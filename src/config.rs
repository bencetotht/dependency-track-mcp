use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use serde_json::json;

use crate::cli::Cli;
use crate::require_non_empty;

#[derive(Debug, Deserialize)]
struct FileConfig {
    url: Option<String>,
    api_key: Option<String>,
    api_key_env: Option<String>,
    timeout_seconds: Option<u64>,
    retries: Option<u32>,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub base_url: String,
    pub api_key: String,
    pub timeout: Duration,
    pub retries: u32,
    pub verbose: bool,
    pub config_path: Option<PathBuf>,
}

impl Settings {
    pub fn load(cli: &Cli) -> Result<Self> {
        let file_config = match cli.config.as_ref() {
            Some(path) => Some(read_config(path)?),
            None => default_config_path()
                .filter(|path| path.exists())
                .map(|path| read_config(&path))
                .transpose()?,
        };

        let url = cli
            .url
            .clone()
            .or_else(|| file_config.as_ref().and_then(|config| config.url.clone()));
        let api_key = cli
            .api_key
            .clone()
            .or_else(|| env::var("DEPENDENCY_TRACK_API_KEY").ok())
            .or_else(|| {
                file_config
                    .as_ref()
                    .and_then(|config| config.api_key.clone())
            })
            .or_else(|| api_key_from_named_env(file_config.as_ref()));

        let url = url.context(
            "Dependency-Track URL is required. Set --url, DTRACK_URL, or url in dtrack.toml.",
        )?;
        let api_key = api_key.context("Dependency-Track API key is required. Set --api-key, DTRACK_API_KEY, DEPENDENCY_TRACK_API_KEY, api_key, or api_key_env in dtrack.toml.")?;
        require_non_empty(&url, "Dependency-Track URL")?;
        require_non_empty(&api_key, "Dependency-Track API key")?;

        let timeout = if cli.timeout != Duration::from_secs(30) {
            cli.timeout
        } else {
            file_config
                .as_ref()
                .and_then(|config| config.timeout_seconds)
                .map(Duration::from_secs)
                .unwrap_or(cli.timeout)
        };

        let retries = if cli.retries != 2 {
            cli.retries
        } else {
            file_config
                .as_ref()
                .and_then(|config| config.retries)
                .unwrap_or(cli.retries)
        };

        Ok(Self {
            base_url: normalize_base_url(&url)?,
            api_key,
            timeout,
            retries,
            verbose: cli.verbose,
            config_path: cli
                .config
                .clone()
                .or_else(|| default_config_path().filter(|path| path.exists())),
        })
    }

    pub fn safe_json(&self) -> serde_json::Value {
        json!({
            "baseUrl": self.base_url,
            "apiKeyConfigured": !self.api_key.is_empty(),
            "timeoutSeconds": self.timeout.as_secs_f64(),
            "retries": self.retries,
            "verbose": self.verbose,
            "configPath": self.config_path.as_ref().map(|path| path.display().to_string()),
        })
    }
}

fn read_config(path: &Path) -> Result<FileConfig> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("failed to read config file {}", path.display()))?;
    toml::from_str(&text)
        .with_context(|| format!("failed to parse TOML config file {}", path.display()))
}

fn api_key_from_named_env(file_config: Option<&FileConfig>) -> Option<String> {
    file_config
        .and_then(|config| config.api_key_env.as_deref())
        .and_then(|name| env::var(name).ok())
}

fn default_config_path() -> Option<PathBuf> {
    let local = PathBuf::from("dtrack.toml");
    if local.exists() {
        return Some(local);
    }
    dirs::config_dir().map(|dir| dir.join("dtrack").join("config.toml"))
}

fn normalize_base_url(input: &str) -> Result<String> {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        bail!("Dependency-Track URL cannot be empty");
    }
    let without_api = trimmed.trim_end_matches("/api/v1").trim_end_matches("/api");
    Ok(without_api.to_string())
}
