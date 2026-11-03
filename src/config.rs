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

#[derive(Clone, Default, Deserialize, Serialize)]
pub struct Profile {
    pub url: Option<String>,
    pub api_key: Option<String>,
    pub api_key_env: Option<String>,
    pub keyring: Option<bool>,
    pub timeout_seconds: Option<u64>,
    pub retries: Option<u32>,
}
#[derive(Default, Deserialize, Serialize)]
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
    pub timeout: Duration,
    pub retries: u32,
    pub verbose: bool,
    pub config_path: Option<PathBuf>,
    pub context: Option<String>,
    pub project_version: Option<String>,
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
pub fn credential(name: &str) -> Result<keyring::Entry> {
    Ok(keyring::Entry::new("dtrack", name)?)
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
        let context = cli
            .context
            .clone()
            .or(local.context.clone())
            .or(user.current_context.clone());
        let profile = context
            .as_ref()
            .map(|name| {
                user.contexts
                    .get(name)
                    .with_context(|| format!("unknown context '{name}'"))
            })
            .transpose()?;
        let profiles = [Some(&local.profile), profile, Some(&user.profile)];
        let url = cli
            .url
            .clone()
            .or_else(|| profiles.iter().flatten().find_map(|p| p.url.clone()))
            .unwrap_or_default();
        let api_key = cli
            .api_key
            .clone()
            .or_else(|| env::var("DEPENDENCY_TRACK_API_KEY").ok())
            .map(Ok)
            .or_else(|| {
                profiles.iter().flatten().find_map(|p| {
                    if let Some(name) = &p.api_key_env {
                        return Some(env::var(name).with_context(|| {
                            format!("missing API key environment variable {name}")
                        }));
                    }
                    if p.keyring == Some(true) {
                        return Some(
                            credential(context.as_deref().unwrap_or("default"))
                                .and_then(|e| Ok(e.get_password()?)),
                        );
                    }
                    p.api_key.clone().map(Ok)
                })
            })
            .transpose()
            .or_else(|error| {
                if matches!(cli.command, Commands::Config) {
                    Ok(None)
                } else {
                    Err(error)
                }
            })?
            .unwrap_or_default();
        if !matches!(cli.command, Commands::Config) {
            crate::require_non_empty(&url, "Dependency-Track URL")?;
            crate::require_non_empty(&api_key, "Dependency-Track API key")?;
        }
        let base_url = if url.is_empty() {
            url
        } else {
            normalize_base_url(&url)?
        };
        Ok(Self {
            base_url,
            api_key,
            context,
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
        json!({"baseUrl":self.base_url,"apiKeyConfigured":!self.api_key.is_empty(),"context":self.context,
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
    #[test]
    fn contexts_parse_without_plaintext_key() {
        let file: FileConfig = toml::from_str("current_context = 'home'\n[contexts.home]\nurl = 'https://dt.example.com'\nkeyring = true").unwrap();
        assert_eq!(file.current_context.as_deref(), Some("home"));
        assert_eq!(file.contexts["home"].keyring, Some(true));
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
