use crate::cli::{Cli, Commands};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Profile {
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub api_key_command: Option<String>,
    pub keyring: Option<bool>,
    pub timeout_seconds: Option<u64>,
    pub retries: Option<u32>,
}
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct FileConfig {
    #[serde(flatten)]
    pub profile: Profile,
    pub context: Option<String>,
    pub current_context: Option<String>,
    #[serde(default)]
    pub contexts: BTreeMap<String, Profile>,
}
#[derive(Clone)]
pub struct Settings {
    pub base_url: String,
    pub api_key: String,
    pub api_key_configured: bool,
    pub timeout: Duration,
    pub retries: u32,
    pub verbose: bool,
    pub config_path: Option<PathBuf>,
    pub context: Option<String>,
    pub connection_source: Option<&'static str>,
    pub project_version: Option<String>,
}

/// Where an API key comes from. Resolved lazily so that the URL and key of a
/// connection are always chosen together, from the same source.
#[derive(Clone, Debug, PartialEq)]
pub enum KeySource {
    Value(String),
    Env(String),
    Command(String),
    Keyring(String),
}

#[derive(Debug, Default, PartialEq)]
pub struct Connection {
    pub url: Option<String>,
    pub key: Option<KeySource>,
    pub context: Option<String>,
    pub source: Option<&'static str>,
    pub warnings: Vec<String>,
}

pub fn user_config_path() -> Result<PathBuf> {
    Ok(dirs::config_dir()
        .context("cannot determine user config directory")?
        .join("dtrack/config.toml"))
}
pub fn read_config(path: &Path) -> Result<FileConfig> {
    toml::from_str(
        &fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?,
    )
    .with_context(|| format!("parsing {}", path.display()))
}
/// The key configured for a profile. Contexts may use a command or the OS
/// credential store; legacy flat profiles (`name == None`) cannot, so a
/// repository dtrack.toml can never make the CLI run a command.
pub fn profile_key(profile: &Profile, name: Option<&str>) -> Option<KeySource> {
    if let Some(var) = &profile.api_key_env {
        return Some(KeySource::Env(var.clone()));
    }
    if let Some(name) = name {
        if let Some(command) = &profile.api_key_command {
            return Some(KeySource::Command(command.clone()));
        }
        if profile.keyring == Some(true) {
            return Some(KeySource::Keyring(name.to_owned()));
        }
    }
    profile.api_key.clone().map(KeySource::Value)
}

pub fn resolve_key(key: &KeySource) -> Result<String> {
    match key {
        KeySource::Value(value) => Ok(value.clone()),
        KeySource::Env(name) => env::var(name)
            .with_context(|| format!("API key environment variable {name} is not set")),
        KeySource::Command(command) => run_key_command(command),
        KeySource::Keyring(name) => crate::credentials::get(name).with_context(|| {
            format!("reading API key for context '{name}' from the OS credential store")
        }),
    }
}

/// Runs an `api_key_command` with `sh -c` and returns its trimmed stdout.
/// Stdin and stderr are inherited so secret managers can prompt.
fn run_key_command(command: &str) -> Result<String> {
    let output = std::process::Command::new("sh")
        .args(["-c", command])
        .stderr(std::process::Stdio::inherit())
        .output()
        .with_context(|| format!("running api_key_command `{command}`"))?;
    if !output.status.success() {
        bail!("api_key_command `{command}` failed with {}", output.status);
    }
    let key = String::from_utf8(output.stdout).context("api_key_command output is not UTF-8")?;
    let key = key.trim();
    if key.is_empty() {
        bail!("api_key_command `{command}` printed nothing");
    }
    Ok(key.to_owned())
}

/// Picks the URL and API key for this invocation. The first matching source
/// supplies both values:
///
/// 1. `--url` flag, with `--api-key` or DTRACK_API_KEY / DEPENDENCY_TRACK_API_KEY
/// 2. `--context` / DTRACK_CONTEXT
/// 3. DTRACK_URL, with `--api-key` or the API key environment variables
/// 4. `context = "..."` in the repository dtrack.toml
/// 5. `current_context` in the user config
/// 6. legacy flat `url` in the repository dtrack.toml, then the user config
///
/// A selected context never takes its URL from a repository file or its key
/// from the environment, so a context's key is only sent to that context's URL.
pub fn select_connection(
    url_flag: Option<&str>,
    key_flag: Option<&str>,
    context_flag: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
    local: &FileConfig,
    user: &FileConfig,
) -> Result<Connection> {
    let explicit_key = || key_flag.map(|key| KeySource::Value(key.to_owned()));
    let env_key = || {
        explicit_key().or_else(|| {
            ["DTRACK_API_KEY", "DEPENDENCY_TRACK_API_KEY"]
                .into_iter()
                .find(|name| env(name).is_some_and(|value| !value.trim().is_empty()))
                .map(|name| KeySource::Env(name.to_owned()))
        })
    };
    let from_context = |name: &str, source: &'static str| -> Result<Connection> {
        let profile = user
            .contexts
            .get(name)
            .with_context(|| format!("unknown context '{name}'; see `dtrack context list`"))?;
        let mut warnings = Vec::new();
        if local.profile.url.is_some() {
            warnings.push(format!(
                "ignoring url in ./dtrack.toml because context '{name}' is selected"
            ));
        }
        Ok(Connection {
            url: Some(
                profile
                    .url
                    .clone()
                    .with_context(|| format!("context '{name}' has no url"))?,
            ),
            key: explicit_key().or_else(|| profile_key(profile, Some(name))),
            context: Some(name.to_owned()),
            source: Some(source),
            warnings,
        })
    };

    if let Some(url) = url_flag {
        return Ok(Connection {
            url: Some(url.to_owned()),
            key: env_key(),
            source: Some("--url"),
            ..Default::default()
        });
    }
    if let Some(name) = context_flag {
        return from_context(name, "--context/DTRACK_CONTEXT");
    }
    if let Some(url) = env("DTRACK_URL").filter(|url| !url.trim().is_empty()) {
        return Ok(Connection {
            url: Some(url),
            key: env_key(),
            source: Some("DTRACK_URL"),
            ..Default::default()
        });
    }
    if let Some(name) = &local.context {
        return from_context(name, "./dtrack.toml context");
    }
    if let Some(name) = &user.current_context {
        return from_context(name, "current_context");
    }
    for (profile, source) in [
        (&local.profile, "./dtrack.toml"),
        (&user.profile, "user config"),
    ] {
        if let Some(url) = &profile.url {
            return Ok(Connection {
                url: Some(url.clone()),
                key: env_key().or_else(|| profile_key(profile, None)),
                source: Some(source),
                ..Default::default()
            });
        }
    }
    Ok(Connection {
        key: env_key(),
        ..Default::default()
    })
}

impl Settings {
    pub fn load(cli: &Cli) -> Result<Self> {
        let path = cli.config.clone().unwrap_or(user_config_path()?);
        let user = if path.exists() {
            read_config(&path)?
        } else {
            FileConfig::default()
        };
        let local = if cli.config.is_none() && Path::new("dtrack.toml").exists() {
            read_config(Path::new("dtrack.toml"))?
        } else {
            FileConfig::default()
        };
        let inspecting = matches!(cli.command, Commands::Config);
        let connection = select_connection(
            cli.url.as_deref(),
            cli.api_key.as_deref(),
            cli.context.as_deref(),
            &|name| env::var(name).ok(),
            &local,
            &user,
        )?;
        for warning in &connection.warnings {
            eprintln!("warning: {warning}");
        }
        // `dtrack config` never needs the key itself, so it does not touch
        // the credential store or run a command.
        let api_key_configured = match &connection.key {
            Some(KeySource::Env(name)) => env::var(name).is_ok_and(|v| !v.trim().is_empty()),
            Some(KeySource::Value(value)) => !value.trim().is_empty(),
            Some(_) => true,
            None => false,
        };
        let api_key = match &connection.key {
            Some(key) if !inspecting => resolve_key(key)?,
            _ => String::new(),
        };
        let url = connection.url.unwrap_or_default();
        if !inspecting {
            if url.trim().is_empty() {
                bail!(
                    "Dependency-Track URL is required. Run `dtrack context add NAME`, pass --context or --url, or set DTRACK_URL."
                );
            }
            crate::require_non_empty(&api_key, "Dependency-Track API key")?;
        }
        let base_url = if url.is_empty() {
            url
        } else {
            normalize_base_url(&url)?
        };
        let context_profile = connection
            .context
            .as_ref()
            .and_then(|name| user.contexts.get(name));
        let profiles = [context_profile, Some(&local.profile), Some(&user.profile)];
        Ok(Self {
            base_url,
            api_key,
            api_key_configured,
            context: connection.context,
            connection_source: connection.source,
            project_version: cli.project_version.clone(),
            timeout: cli.timeout.unwrap_or_else(|| {
                Duration::from_secs(
                    profiles
                        .iter()
                        .flatten()
                        .find_map(|p| p.timeout_seconds)
                        .unwrap_or(30),
                )
            }),
            retries: cli.retries.unwrap_or_else(|| {
                profiles
                    .iter()
                    .flatten()
                    .find_map(|p| p.retries)
                    .unwrap_or(2)
            }),
            verbose: cli.verbose,
            config_path: Some(path),
        })
    }
    pub fn safe_json(&self) -> serde_json::Value {
        json!({"baseUrl":self.base_url,"apiKeyConfigured":self.api_key_configured,"context":self.context,
            "connectionSource":self.connection_source,
            "timeoutSeconds":self.timeout.as_secs_f64(),"retries":self.retries,"verbose":self.verbose,
            "configPath":self.config_path})
    }
}
pub fn normalize_base_url(input: &str) -> Result<String> {
    let trimmed = input
        .trim()
        .trim_end_matches('/')
        .trim_end_matches("/api/v1")
        .trim_end_matches("/api");
    let url = reqwest::Url::parse(trimmed).context("invalid Dependency-Track URL")?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        bail!("URL must use HTTP or HTTPS");
    }
    Ok(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn files(local: &str) -> (FileConfig, FileConfig) {
        let user = toml::from_str(
            "current_context = 'home'\n\
             [contexts.home]\nurl = 'https://home.example.com'\nkeyring = true\n\
             [contexts.company]\nurl = 'https://company.example.com'\napi_key_env = 'COMPANY_KEY'\n",
        )
        .unwrap();
        (toml::from_str(local).unwrap(), user)
    }
    fn env_with_home_key(name: &str) -> Option<String> {
        matches!(name, "DEPENDENCY_TRACK_API_KEY" | "DTRACK_API_KEY").then(|| "home-secret".into())
    }

    #[test]
    fn contexts_parse_without_plaintext_key() {
        let file: FileConfig = toml::from_str("current_context = 'home'\n[contexts.home]\nurl = 'https://dt.example.com'\nkeyring = true").unwrap();
        assert_eq!(file.current_context.as_deref(), Some("home"));
        assert_eq!(file.contexts["home"].keyring, Some(true));
    }
    #[test]
    fn repository_url_cannot_redirect_selected_context() {
        let (local, user) = files("url = 'https://evil.example.com'");
        for context in [Some("company"), None] {
            let connection =
                select_connection(None, None, context, &|_| None, &local, &user).unwrap();
            assert_ne!(connection.url.as_deref(), Some("https://evil.example.com"));
            assert_eq!(connection.warnings.len(), 1);
        }
    }
    #[test]
    fn environment_key_is_not_sent_to_context_url() {
        let (local, user) = files("");
        let connection = select_connection(
            None,
            None,
            Some("company"),
            &env_with_home_key,
            &local,
            &user,
        )
        .unwrap();
        assert_eq!(
            connection.url.as_deref(),
            Some("https://company.example.com")
        );
        assert_eq!(connection.key, Some(KeySource::Env("COMPANY_KEY".into())));
        let connection =
            select_connection(None, None, None, &env_with_home_key, &local, &user).unwrap();
        assert_eq!(connection.key, Some(KeySource::Keyring("home".into())));
    }
    #[test]
    fn repository_context_and_explicit_overrides() {
        let (local, user) = files("context = 'company'");
        let connection = select_connection(None, None, None, &|_| None, &local, &user).unwrap();
        assert_eq!(connection.context.as_deref(), Some("company"));
        let connection =
            select_connection(None, Some("typed"), Some("home"), &|_| None, &local, &user).unwrap();
        assert_eq!(connection.key, Some(KeySource::Value("typed".into())));
        let connection = select_connection(
            Some("https://other.example.com"),
            None,
            Some("home"),
            &env_with_home_key,
            &local,
            &user,
        )
        .unwrap();
        assert_eq!(connection.context, None);
        assert_eq!(
            connection.key,
            Some(KeySource::Env("DTRACK_API_KEY".into()))
        );
    }
    #[test]
    fn key_command_is_only_used_for_contexts() {
        let user: FileConfig = toml::from_str(
            "[contexts.op]\nurl = 'https://dt.example.com'\napi_key_command = 'printf \"  key\\n\"'\n",
        )
        .unwrap();
        let connection = select_connection(
            None,
            None,
            Some("op"),
            &|_| None,
            &FileConfig::default(),
            &user,
        )
        .unwrap();
        let key = connection.key.unwrap();
        assert_eq!(key, KeySource::Command("printf \"  key\\n\"".into()));
        assert_eq!(resolve_key(&key).unwrap(), "key");
        assert!(resolve_key(&KeySource::Command("false".into())).is_err());
        assert!(resolve_key(&KeySource::Command("true".into())).is_err());
        let local: FileConfig =
            toml::from_str("url = 'https://dt.example.com'\napi_key_command = 'touch pwned'")
                .unwrap();
        let connection =
            select_connection(None, None, None, &|_| None, &local, &FileConfig::default()).unwrap();
        assert_eq!(connection.key, None);
    }
    #[test]
    fn legacy_flat_config_still_works() {
        let local: FileConfig =
            toml::from_str("url = 'https://dt.example.com'\napi_key_env = 'MY_KEY'").unwrap();
        let connection =
            select_connection(None, None, None, &|_| None, &local, &FileConfig::default()).unwrap();
        assert_eq!(connection.url.as_deref(), Some("https://dt.example.com"));
        assert_eq!(connection.key, Some(KeySource::Env("MY_KEY".into())));
    }
    #[test]
    fn explicit_defaults_remain_present() {
        let cli = Cli::try_parse_from(["dtrack", "--timeout", "30s", "--retries", "2", "config"])
            .unwrap();
        assert_eq!(cli.timeout, Some(Duration::from_secs(30)));
        assert_eq!(cli.retries, Some(2));
        let cli = Cli::try_parse_from(["dtrack", "config"]).unwrap();
        assert!(cli.timeout.is_none());
    }
    #[test]
    fn config_values_and_explicit_overrides() {
        let path =
            std::env::temp_dir().join(format!("dtrack-config-test-{}.toml", std::process::id()));
        fs::write(
            &path,
            "url = 'https://example.com'\ntimeout_seconds = 90\nretries = 7\n",
        )
        .unwrap();
        let cli = Cli::try_parse_from([
            "dtrack",
            "--config",
            path.to_str().unwrap(),
            "--timeout",
            "30s",
            "--retries",
            "2",
            "config",
        ])
        .unwrap();
        let settings = Settings::load(&cli).unwrap();
        assert_eq!(settings.timeout, Duration::from_secs(30));
        assert_eq!(settings.retries, 2);
        assert!(settings.safe_json().get("api_key").is_none());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn url_normalization() {
        assert_eq!(
            normalize_base_url("https://dt.example.com/api/v1/").unwrap(),
            "https://dt.example.com"
        );
        assert!(normalize_base_url("file:///tmp/key").is_err());
    }
}
