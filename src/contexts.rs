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
            let profile = file.contexts.remove(name).context("unknown context")?;
            if profile.keyring == Some(true) {
                config::credential(name)?.delete_credential()?;
            }
            if file.current_context.as_ref() == Some(name) {
                file.current_context = None;
            }
            save = true;
            serde_json::json!({"removed":name})
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
            let p = file.contexts.get(selected).context("unknown context")?;
            let key = if let Some(var) = &p.api_key_env {
                std::env::var(var)?
            } else if p.keyring == Some(true) {
                config::credential(selected)?.get_password()?
            } else {
                p.api_key.clone().context("no API key")?
            };
            test(p.url.as_deref().context("no URL")?, &key).await?;
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
    http.get(format!("{url}/api/version"))
        .header("X-Api-Key", key)
        .send()
        .await?
        .error_for_status()?;
    // The version endpoint can be public. Check an authenticated endpoint as well.
    http.get(format!("{url}/api/v1/project"))
        .query(&[("pageSize", "1")])
        .header("X-Api-Key", key)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}
