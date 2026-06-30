use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result, bail};

pub fn find_git_root(path: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(expand_path(path))
        .output()
        .context("failed to execute git rev-parse")?;
    if !output.status.success() {
        bail!("no Git repository found from '{}'", path);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn get_git_remote_url(repo_path: &str, remote: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["remote", "get-url", remote])
        .current_dir(repo_path)
        .output()
        .with_context(|| format!("failed to read git remote '{remote}' in {repo_path}"))?;
    if !output.status.success() {
        bail!("git remote '{remote}' is not configured in {repo_path}");
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn normalize_git_remote_url(remote_url: &str) -> String {
    let trimmed = remote_url.trim().trim_end_matches(".git");
    if let Some(rest) = trimmed.strip_prefix("git@")
        && let Some((host, path)) = rest.split_once(':')
    {
        return format!("https://{}/{}", host, path.trim_end_matches(".git"));
    }
    if let Some(rest) = trimmed.strip_prefix("ssh://git@")
        && let Some((host, path)) = rest.split_once('/')
    {
        return format!("https://{}/{}", host, path.trim_end_matches(".git"));
    }
    if let Ok(mut url) = reqwest::Url::parse(trimmed) {
        url.set_scheme("https").ok();
        url.set_query(None);
        url.set_fragment(None);
        return url
            .as_str()
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_string();
    }
    trimmed.to_string()
}

fn expand_path(path: &str) -> String {
    if path == "~" {
        return dirs::home_dir()
            .unwrap_or_else(|| Path::new(path).to_path_buf())
            .display()
            .to_string();
    }
    if let Some(rest) = path.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest).display().to_string();
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::normalize_git_remote_url;

    #[test]
    fn normalizes_common_git_urls() {
        assert_eq!(
            normalize_git_remote_url("git@example.com:org/repo.git"),
            "https://example.com/org/repo"
        );
        assert_eq!(
            normalize_git_remote_url("ssh://git@example.com/org/repo.git"),
            "https://example.com/org/repo"
        );
        assert_eq!(
            normalize_git_remote_url("http://example.com/org/repo.git"),
            "https://example.com/org/repo"
        );
    }
}
