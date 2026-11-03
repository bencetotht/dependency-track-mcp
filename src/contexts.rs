use crate::{
    cli::{Cli, ContextCommands},
    config::{self, FileConfig, Profile},
    output::print_value,
};
use anyhow::{Context, Result, bail};
use std::io::{self, IsTerminal, Read, Write};

pub async fn run(cli: &Cli, command: &ContextCommands) -> Result<()> {
    let path = cli.config.clone().unwrap_or(config::user_config_path()?);
    let mut file = if path.exists() {
        config::read_config(&path)?
    } else {
        FileConfig::default()
    };
    let mut save = false;
    let value = match command {
        ContextCommands::List => {
            serde_json::json!({"contexts":file.contexts.keys().collect::<Vec<_>>(),"current":file.current_context})
        }
        ContextCommands::Current => serde_json::json!({"context":file.current_context}),
        ContextCommands::Use { name } => {
            if !file.contexts.contains_key(name) {
                bail!("unknown context '{name}'");
            }
            file.current_context = Some(name.clone());
            save = true;
            serde_json::json!({"context":name})
        }
        ContextCommands::Remove { name } => {
            let profile = file
                .contexts
                .remove(name)
                .with_context(|| format!("unknown context '{name}'"))?;
            let credential_deleted = if profile.keyring == Some(true) {
                delete_credential(name)
            } else {
                false
            };
            if file.current_context.as_ref() == Some(name) {
                file.current_context = None;
            }
            save = true;
            serde_json::json!({"removed":name,"credentialDeleted":credential_deleted})
        }
        ContextCommands::Add {
            name,
            url,
            api_key,
            api_key_env,
            api_key_stdin,
        } => {
            if file.contexts.contains_key(name) {
                bail!("context already exists; remove it first");
            }
            let url = config::normalize_base_url(&match url {
                Some(u) => u.clone(),
                None => prompt("URL")?,
            })?;
            let key = if let Some(var) = api_key_env {
                std::env::var(var).context("API key environment variable is not set")?
            } else if *api_key_stdin {
                let mut key = String::new();
                io::stdin().read_to_string(&mut key)?;
                key.trim_end().to_owned()
            } else if let Some(key) = api_key {
                key.clone()
            } else {
                if !io::stdin().is_terminal() {
                    bail!("missing API key; use --api-key-env or --api-key-stdin");
                }
                rpassword::prompt_password("API key: ")?
            };
            crate::require_non_empty(&key, "API key")?;
            test(&url, &key).await?;
            if api_key_env.is_none() {
                config::credential(name)?.set_password(&key).context("could not store key in OS credential store; use --api-key-env on headless Linux")?;
            }
            file.contexts.insert(
                name.clone(),
                Profile {
                    url: Some(url),
                    api_key_env: api_key_env.clone(),
                    keyring: Some(api_key_env.is_none()),
                    ..Default::default()
                },
            );
            if file.current_context.is_none() {
                file.current_context = Some(name.clone());
            }
            save = true;
            serde_json::json!({"added":name})
        }
        ContextCommands::Test { name } => {
            let selected = name
                .as_ref()
                .or(cli.context.as_ref())
                .or(file.current_context.as_ref())
                .context("no context selected")?;
            let profile = file
                .contexts
                .get(selected)
                .with_context(|| format!("unknown context '{selected}'"))?;
            let key = config::profile_key(profile, Some(selected))
                .with_context(|| format!("context '{selected}' has no API key configured"))?;
            let url = profile
                .url
                .as_deref()
                .with_context(|| format!("context '{selected}' has no url"))?;
            test(url, &config::resolve_key(&key)?).await?;
            serde_json::json!({"context":selected,"ok":true})
        }
    };
    if save {
        std::fs::create_dir_all(path.parent().context("invalid config path")?)?;
        let temporary = path.with_extension("toml.tmp");
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut output = options.open(&temporary).context(
            "opening temporary config (remove stale .toml.tmp if a prior save was interrupted)",
        )?;
        output.write_all(toml::to_string_pretty(&file)?.as_bytes())?;
        output.sync_all()?;
        std::fs::rename(temporary, path)?;
    }
    print_value(&value, cli.output)
}
/// Deletes a stored key. A missing entry counts as deleted; other credential
/// store failures are reported but do not prevent removing the context.
fn delete_credential(name: &str) -> bool {
    match config::credential(name).and_then(|entry| Ok(entry.delete_credential()?)) {
        Ok(()) => true,
        Err(error)
            if matches!(
                error.downcast_ref::<keyring::Error>(),
                Some(keyring::Error::NoEntry)
            ) =>
        {
            true
        }
        Err(error) => {
            eprintln!(
                "warning: removed context '{name}' but could not delete its stored API key: {error:#}"
            );
            false
        }
    }
}
fn prompt(label: &str) -> Result<String> {
    if !io::stdin().is_terminal() {
        bail!("missing {label}; supply it with a flag in non-interactive mode");
    }
    eprint!("{label}: ");
    io::stderr().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    Ok(value.trim().to_owned())
}
async fn test(url: &str, key: &str) -> Result<()> {
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    // The version endpoint can be public. Check an authenticated endpoint as well.
    for request in [
        http.get(format!("{url}/api/version")),
        http.get(format!("{url}/api/v1/project"))
            .query(&[("pageSize", "1")]),
    ] {
        let response = request
            .header("X-Api-Key", key)
            .send()
            .await
            .with_context(|| format!("connecting to {url}"))?;
        let status = response.status();
        if !status.is_success() {
            let path = response.url().path().to_owned();
            let body = response.text().await.unwrap_or_default();
            bail!(
                "connection test failed: GET {path} returned {status} {}",
                body.trim()
            );
        }
    }
    Ok(())
}
