use std::time::Duration;

use anyhow::{Context, Result, bail};
use reqwest::{Client, Method, StatusCode};
use serde::de::DeserializeOwned;
use tokio::time::sleep;

use crate::cli::ProjectListArgs;
use crate::config::Settings;
use crate::git::{find_git_root, get_git_remote_url, normalize_git_remote_url};
use crate::models::{
    DetectedProject, FpfDocument, Metrics, Project, ProjectProperty, ResolvedProject, Vulnerability,
};

#[derive(Clone)]
pub struct DependencyTrackClient {
    http: Client,
    settings: Settings,
}

impl DependencyTrackClient {
    pub fn new(settings: Settings) -> Result<Self> {
        let http = Client::builder()
            .timeout(settings.timeout)
            .user_agent(concat!("dtrack/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to create HTTP client")?;
        Ok(Self { http, settings })
    }

    pub fn safe_settings(&self) -> serde_json::Value {
        self.settings.safe_json()
    }

    pub async fn list_projects(&self, args: &ProjectListArgs) -> Result<Vec<Project>> {
        let mut query = vec![
            ("pageSize", "500".to_string()),
            ("excludeInactive", args.active.to_string()),
        ];
        if let Some(name) = args
            .query
            .as_ref()
            .filter(|_| args.classifier.is_none() && args.tag.is_none())
        {
            query.push(("name", name.clone()));
        }
        self.paged("/api/v1/project", &query).await
    }

    pub async fn project(&self, uuid: &str) -> Result<Project> {
        self.get(&format!("/api/v1/project/{uuid}")).await
    }

    pub async fn project_properties(&self, uuid: &str) -> Result<Vec<ProjectProperty>> {
        self.get(&format!("/api/v1/project/{uuid}/property")).await
    }

    pub async fn project_metrics(&self, uuid: &str) -> Result<Metrics> {
        self.get(&format!("/api/v1/metrics/project/{uuid}/current"))
            .await
    }

    pub async fn portfolio_metrics(&self) -> Result<Metrics> {
        self.get("/api/v1/metrics/portfolio/current").await
    }

    pub async fn findings_export(&self, uuid: &str) -> Result<Vec<crate::models::Finding>> {
        let document: FpfDocument = self
            .get(&format!("/api/v1/finding/project/{uuid}/export"))
            .await?;
        Ok(document.findings)
    }

    pub async fn vulnerability(&self, source: &str, vulnerability: &str) -> Result<Vulnerability> {
        self.get(&format!(
            "/api/v1/vulnerability/source/{}/vuln/{}",
            urlencoding::encode(source),
            urlencoding::encode(vulnerability)
        ))
        .await
    }

    pub async fn vulnerability_projects(
        &self,
        source: &str,
        vulnerability: &str,
        exclude_inactive: bool,
    ) -> Result<Vec<Project>> {
        self.paged(
            &format!(
                "/api/v1/vulnerability/source/{}/vuln/{}/projects",
                urlencoding::encode(source),
                urlencoding::encode(vulnerability)
            ),
            &[("excludeInactive", exclude_inactive.to_string())],
        )
        .await
    }

    pub async fn resolve_project_or_repo(
        &self,
        project: Option<&str>,
        repo: Option<&str>,
        remote: &str,
    ) -> Result<ResolvedProject> {
        if let Some(project) = project {
            return Ok(ResolvedProject {
                project: self.resolve_project(project).await?,
                repo_path: None,
                remote_url: None,
                match_source: None,
            });
        }

        let detected = self
            .detect_project_from_repo(repo.unwrap_or("."), remote)
            .await?;
        Ok(ResolvedProject {
            project: detected.project,
            repo_path: Some(detected.repo_path),
            remote_url: Some(detected.remote_url),
            match_source: Some(detected.match_source),
        })
    }

    pub fn with_version(&self, version: Option<String>) -> Self {
        let mut client = self.clone();
        if version.is_some() {
            client.settings.project_version = version;
        }
        client
    }

    pub async fn resolve_project(&self, identifier: &str) -> Result<Project> {
        if looks_like_uuid(identifier) {
            return self.project(identifier).await;
        }

        let projects = self
            .paged(
                "/api/v1/project",
                &[
                    ("pageSize", "500".to_string()),
                    ("name", identifier.to_string()),
                ],
            )
            .await?;
        let lowered = identifier.to_lowercase();
        let mut candidates: Vec<Project> = projects
            .into_iter()
            .filter(|project| {
                project.name.eq_ignore_ascii_case(identifier)
                    || project.name.to_lowercase().contains(&lowered)
                    || project.uuid.eq_ignore_ascii_case(identifier)
            })
            .collect();

        if candidates.is_empty() {
            bail!("no Dependency-Track project matched '{identifier}'");
        }
        candidates.sort_by(|left, right| {
            right
                .is_latest
                .cmp(&left.is_latest)
                .then(right.last_bom_import.cmp(&left.last_bom_import))
                .then(left.name.cmp(&right.name))
        });

        if candidates
            .iter()
            .any(|p| p.name.eq_ignore_ascii_case(identifier))
        {
            candidates.retain(|p| p.name.eq_ignore_ascii_case(identifier));
        }
        if let Some(version) = &self.settings.project_version {
            candidates.retain(|p| p.version.as_ref() == Some(version));
        }
        if candidates.is_empty() {
            bail!("no matching project version");
        }
        if candidates.len() == 1 {
            return Ok(candidates.remove(0));
        }
        let names = candidates
            .iter()
            .take(10)
            .map(|project| {
                format!(
                    "{} {} ({})",
                    project.name,
                    project.version.as_deref().unwrap_or(""),
                    project.uuid
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        bail!(
            "project identifier '{identifier}' is ambiguous; select --project-version or use UUID. Matches: {names}"
        );
    }

    pub async fn detect_project_from_repo(
        &self,
        path: &str,
        remote: &str,
    ) -> Result<DetectedProject> {
        let repo_path = find_git_root(path)?;
        let remote_url = normalize_git_remote_url(&get_git_remote_url(&repo_path, remote)?);
        let projects: Vec<Project> = self
            .paged(
                "/api/v1/project",
                &[
                    ("pageSize", "500".to_string()),
                    ("excludeInactive", "true".to_string()),
                ],
            )
            .await?;

        use futures_util::{StreamExt, stream};
        let mut pending = stream::iter(projects.into_iter().map(|project| async move {
            let properties = self.project_properties(&project.uuid).await;
            (project, properties)
        }))
        .buffered(8);
        let mut matches = Vec::new();
        while let Some((project, properties)) = pending.next().await {
            let properties = properties
                .with_context(|| format!("loading properties for project {}", project.uuid))?;
            if let Some(source) = match_project_remote(&project, &properties, &remote_url) {
                matches.push((project, properties, source));
            }
        }
        if let Some(version) = &self.settings.project_version {
            matches.retain(|(p, _, _)| p.version.as_ref() == Some(version));
        }
        matches.sort_by(|a, b| {
            b.0.is_latest
                .cmp(&a.0.is_latest)
                .then(b.0.last_bom_import.cmp(&a.0.last_bom_import))
        });
        if let Some((project, properties, match_source)) = matches.into_iter().next() {
            return Ok(DetectedProject {
                project,
                properties,
                repo_path,
                remote_url,
                match_source,
            });
        }

        bail!(
            "no Dependency-Track project matched git remote '{}'. Add a project property such as automation.cloneUrl, automation.repositoryUrl, repo.cloneUrl, repo.repositoryUrl, vcs.cloneUrl, or a Git external reference.",
            remote_url
        );
    }

    async fn paged(&self, path: &str, query: &[(&str, String)]) -> Result<Vec<Project>> {
        let mut all = Vec::new();
        let mut page = 1;
        loop {
            let mut query = query.to_vec();
            query.retain(|(key, _)| *key != "pageSize");
            query.push(("pageSize", "500".into()));
            query.push(("pageNumber", page.to_string()));
            let batch: Vec<Project> = self.get_query(path, &query).await?;
            let count = batch.len();
            if count > 0
                && batch
                    .iter()
                    .all(|p| all.iter().any(|old: &Project| old.uuid == p.uuid))
            {
                bail!("server ignored pagination for {path}");
            }
            all.extend(batch);
            if count < 500 {
                return Ok(all);
            }
            page += 1;
        }
    }

    pub async fn analysis(
        &self,
        project: &str,
        component: &str,
        vulnerability: &str,
    ) -> Result<serde_json::Value> {
        self.get_query(
            "/api/v1/analysis",
            &[
                ("project", project.into()),
                ("component", component.into()),
                ("vulnerability", vulnerability.into()),
            ],
        )
        .await
    }

    pub async fn write(
        &self,
        path: &str,
        body: Option<serde_json::Value>,
    ) -> Result<serde_json::Value> {
        let mut request = self
            .http
            .request(
                if body.is_some() {
                    Method::PUT
                } else {
                    Method::POST
                },
                format!("{}{path}", self.settings.base_url),
            )
            .header("X-Api-Key", &self.settings.api_key);
        if let Some(body) = body {
            request = request.json(&body);
        }
        let response = request.send().await?.error_for_status()?;
        let bytes = response.bytes().await?;
        if bytes.is_empty() {
            Ok(serde_json::json!({"accepted":true}))
        } else {
            Ok(serde_json::from_slice(&bytes)?)
        }
    }

    async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, &[]).await
    }

    async fn get_query<T: DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        self.request(Method::GET, path, query).await
    }

    async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T> {
        let url = format!("{}{}", self.settings.base_url, path);
        let mut attempt = 0;
        loop {
            attempt += 1;
            if self.settings.verbose {
                eprintln!("{} {} attempt {attempt}", method.as_str(), path);
            }
            let response = self
                .http
                .request(method.clone(), &url)
                .query(query)
                .header("Accept", "application/json")
                .header("X-Api-Key", &self.settings.api_key)
                .send()
                .await;

            match response {
                Ok(response) if response.status().is_success() => {
                    return response.json::<T>().await.with_context(|| {
                        format!("failed to decode Dependency-Track response for {path}")
                    });
                }
                Ok(response) => {
                    let status = response.status();
                    let body = response.text().await.unwrap_or_default();
                    if path == "/api/v1/analysis"
                        && status == StatusCode::NOT_FOUND
                        && body.trim() == "No analysis exists."
                    {
                        return Ok(serde_json::from_value(serde_json::json!({}))?);
                    }
                    if should_retry(status) && attempt <= self.settings.retries {
                        self.retry_delay(attempt).await;
                        continue;
                    }
                    bail!(
                        "Dependency-Track request failed: {} {} returned {} {}",
                        method,
                        path,
                        status,
                        body.trim()
                    );
                }
                Err(error) => {
                    if attempt <= self.settings.retries {
                        if self.settings.verbose {
                            eprintln!("request failed: {error}. retrying...");
                        }
                        self.retry_delay(attempt).await;
                        continue;
                    }
                    return Err(error).with_context(|| {
                        format!(
                            "Dependency-Track request failed after {} attempts: {} {}",
                            attempt, method, path
                        )
                    });
                }
            }
        }
    }

    async fn retry_delay(&self, attempt: u32) {
        let millis = 250_u64
            .saturating_mul(2_u64.saturating_pow(attempt.saturating_sub(1)))
            .min(4_000);
        if self.settings.verbose {
            eprintln!("retrying after {}ms", millis);
        }
        sleep(Duration::from_millis(millis)).await;
    }
}

fn should_retry(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

fn looks_like_uuid(value: &str) -> bool {
    value.len() == 36 && value.chars().all(|ch| ch.is_ascii_hexdigit() || ch == '-')
}

fn match_project_remote(
    project: &Project,
    properties: &[ProjectProperty],
    remote_url: &str,
) -> Option<String> {
    let property_names = [
        (
            "automation.cloneurl",
            "project property automation.cloneUrl",
        ),
        ("automation.repourl", "project property automation.repoUrl"),
        (
            "automation.repositoryurl",
            "project property automation.repositoryUrl",
        ),
        ("repo.cloneurl", "project property repo.cloneUrl"),
        ("repo.repositoryurl", "project property repo.repositoryUrl"),
        ("vcs.cloneurl", "project property vcs.cloneUrl"),
        ("vcs.repositoryurl", "project property vcs.repositoryUrl"),
        ("cloneurl", "project property cloneUrl"),
        ("repositoryurl", "project property repositoryUrl"),
    ];

    for property in properties {
        let name = property_key(property);
        let Some(value) = property.property_value.as_deref() else {
            continue;
        };
        for (expected, source) in property_names {
            if name == expected && normalize_git_remote_url(value) == remote_url {
                return Some(source.to_string());
            }
        }
    }

    for reference in &project.external_references {
        let Some(url) = reference.url.as_deref() else {
            continue;
        };
        let reference_type = reference
            .r#type
            .as_deref()
            .unwrap_or_default()
            .to_ascii_uppercase();
        if normalize_git_remote_url(url) == remote_url
            && (reference_type.contains("VCS")
                || reference_type.contains("SCM")
                || url.contains("git"))
        {
            return Some("project external reference".to_string());
        }
    }

    None
}

fn property_key(property: &ProjectProperty) -> String {
    let group = property
        .group_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    let name = property
        .property_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .to_lowercase();
    if group.is_empty() {
        name
    } else {
        format!("{group}.{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn mock(responses: Vec<(u16, String)>) -> (DependencyTrackClient, std::thread::JoinHandle<()>) {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}", listener.local_addr().unwrap());
        let task = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().unwrap();
                let mut buffer = [0; 8192];
                let count = socket.read(&mut buffer).unwrap();
                assert!(count > 0);
                write!(socket, "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).unwrap();
            }
        });
        (
            DependencyTrackClient::new(Settings {
                base_url,
                api_key: "test".into(),
                timeout: Duration::from_secs(2),
                retries: 2,
                verbose: false,
                config_path: None,
                context: None,
                project_version: None,
            })
            .unwrap(),
            task,
        )
    }
    #[tokio::test]
    async fn exact_name_wins_over_newer_partial() {
        let (client, task) = mock(vec![(
            200,
            r#"[{"uuid":"a","name":"ds-image","isLatest":true},{"uuid":"b","name":"ds"}]"#.into(),
        )]);
        assert_eq!(client.resolve_project("ds").await.unwrap().uuid, "b");
        task.join().unwrap();
    }
    #[tokio::test]
    async fn retries_are_additional_attempts() {
        let (client, task) = mock(vec![(503, "busy".into()); 3]);
        assert!(
            client
                .project("x")
                .await
                .unwrap_err()
                .to_string()
                .contains("503")
        );
        task.join().unwrap();
    }
    #[tokio::test]
    async fn projects_fetch_every_page() {
        let first = serde_json::Value::Array(
            (0..500)
                .map(|i| serde_json::json!({"uuid":i.to_string(),"name":"test"}))
                .collect(),
        );
        let (client, task) = mock(vec![
            (200, first.to_string()),
            (200, r#"[{"uuid":"last","name":"test"}]"#.into()),
        ]);
        assert_eq!(
            client.paged("/api/v1/project", &[]).await.unwrap().len(),
            501
        );
        task.join().unwrap();
    }
}
