mod cli;
mod client;
mod config;
mod git;
mod models;
mod output;

use anyhow::{Result, bail};
use clap::Parser;
use cli::{
    Cli, Commands, FindingsCommands, MetricsCommands, ProjectCommands, VulnerabilityCommands,
};
use client::DependencyTrackClient;
use config::Settings;
use models::{Finding, OutputFormat, Project, Severity};
use output::{print_findings, print_json, print_metrics, print_projects, print_value};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let settings = Settings::load(&cli)?;
    let client = DependencyTrackClient::new(settings)?;

    match cli.command {
        Commands::Projects { command } => match command {
            ProjectCommands::List(args) => {
                let mut projects = client.list_projects(&args).await?;
                filter_projects(
                    &mut projects,
                    args.query.as_deref(),
                    args.classifier.as_deref(),
                    args.tag.as_deref(),
                    args.latest,
                );
                sort_projects(&mut projects, &args.sort);
                truncate(&mut projects, args.limit);
                print_projects(&projects, cli.output)?;
            }
            ProjectCommands::Show(args) => {
                let project = client.resolve_project(&args.project).await?;
                let properties = if args.properties {
                    Some(client.project_properties(&project.uuid).await?)
                } else {
                    None
                };
                let metrics = if args.metrics {
                    Some(client.project_metrics(&project.uuid).await?)
                } else {
                    None
                };
                let findings = if let Some(min_severity) = args.findings {
                    Some(load_findings(&client, &project, min_severity, false, None, None).await?)
                } else {
                    None
                };
                let value = serde_json::json!({
                    "project": project,
                    "properties": properties,
                    "metrics": metrics,
                    "findings": findings,
                });
                print_value(&value, cli.output)?;
            }
            ProjectCommands::Detect(args) => {
                let detected = client
                    .detect_project_from_repo(&args.path, &args.remote)
                    .await?;
                print_value(&serde_json::to_value(detected)?, cli.output)?;
            }
        },
        Commands::Findings { command } => match command {
            FindingsCommands::List(args) => {
                let resolved = client
                    .resolve_project_or_repo(
                        args.project.as_deref(),
                        args.repo.as_deref(),
                        &args.remote,
                    )
                    .await?;
                let mut findings = load_findings(
                    &client,
                    &resolved.project,
                    args.min_severity,
                    args.include_suppressed,
                    args.state.as_deref(),
                    args.component.as_deref(),
                )
                .await?;
                if let Some(vulnerability) = args.vulnerability.as_deref() {
                    let needle = vulnerability.to_lowercase();
                    findings.retain(|finding| {
                        finding.vulnerability_id().to_lowercase().contains(&needle)
                    });
                }
                sort_findings(&mut findings, &args.sort);
                truncate(&mut findings, args.limit);

                let value = serde_json::json!({
                    "project": resolved.project.clone(),
                    "repoPath": resolved.repo_path,
                    "remoteUrl": resolved.remote_url,
                    "matchSource": resolved.match_source,
                    "minSeverity": args.min_severity,
                    "includeSuppressed": args.include_suppressed,
                    "findings": findings,
                });
                if cli.output == OutputFormat::Json {
                    print_json(&value)?;
                } else {
                    print_findings(&resolved.project, &findings)?;
                }
            }
            FindingsCommands::Summary(args) => {
                let project = client
                    .resolve_project_or_repo(
                        args.project.as_deref(),
                        args.repo.as_deref(),
                        &args.remote,
                    )
                    .await?;
                let findings = load_findings(
                    &client,
                    &project.project,
                    Severity::Low,
                    args.include_suppressed,
                    None,
                    None,
                )
                .await?;
                let summary = models::FindingSummary::from_findings(project.project, &findings);
                print_value(&serde_json::to_value(summary)?, cli.output)?;
            }
        },
        Commands::Vulnerabilities { command } => match command {
            VulnerabilityCommands::Show(args) => {
                let vulnerability = client
                    .vulnerability(&args.source, &args.vulnerability)
                    .await?;
                print_value(&serde_json::to_value(vulnerability)?, cli.output)?;
            }
            VulnerabilityCommands::Projects(args) => {
                let mut projects = client
                    .vulnerability_projects(
                        &args.source,
                        &args.vulnerability,
                        args.exclude_inactive,
                    )
                    .await?;
                sort_projects(&mut projects, &args.sort);
                truncate(&mut projects, args.limit);
                print_projects(&projects, cli.output)?;
            }
        },
        Commands::Metrics { command } => match command {
            MetricsCommands::Project(args) => {
                let project = client.resolve_project(&args.project).await?;
                let metrics = client.project_metrics(&project.uuid).await?;
                print_metrics(Some(&project), &metrics, cli.output)?;
            }
            MetricsCommands::Portfolio => {
                let metrics = client.portfolio_metrics().await?;
                print_metrics(None, &metrics, cli.output)?;
            }
        },
        Commands::Config => {
            print_value(&client.safe_settings(), cli.output)?;
        }
    }

    Ok(())
}

async fn load_findings(
    client: &DependencyTrackClient,
    project: &Project,
    min_severity: Severity,
    include_suppressed: bool,
    state: Option<&str>,
    component: Option<&str>,
) -> Result<Vec<Finding>> {
    let mut findings = client.findings_export(&project.uuid).await?;
    findings.retain(|finding| {
        finding.severity().rank() >= min_severity.rank()
            && (include_suppressed || !finding.is_suppressed())
    });
    if let Some(state) = state {
        let expected = state.to_uppercase();
        findings.retain(|finding| finding.analysis_state().eq_ignore_ascii_case(&expected));
    }
    if let Some(component) = component {
        let needle = component.to_lowercase();
        findings.retain(|finding| finding.component_name().to_lowercase().contains(&needle));
    }
    Ok(findings)
}

fn filter_projects(
    projects: &mut Vec<Project>,
    query: Option<&str>,
    classifier: Option<&str>,
    tag: Option<&str>,
    latest: bool,
) {
    if let Some(query) = query {
        let needle = query.to_lowercase();
        projects.retain(|project| {
            project.name.to_lowercase().contains(&needle)
                || project
                    .version
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
                || project.uuid.to_lowercase().contains(&needle)
        });
    }
    if let Some(classifier) = classifier {
        projects.retain(|project| {
            project
                .classifier
                .as_deref()
                .unwrap_or_default()
                .eq_ignore_ascii_case(classifier)
        });
    }
    if let Some(tag) = tag {
        projects.retain(|project| {
            project
                .tags
                .iter()
                .any(|item| item.name.eq_ignore_ascii_case(tag))
        });
    }
    if latest {
        projects.retain(|project| project.is_latest.unwrap_or(false));
    }
}

fn sort_projects(projects: &mut [Project], sort: &cli::ProjectSort) {
    match sort {
        cli::ProjectSort::Name => projects.sort_by(|left, right| {
            left.name
                .cmp(&right.name)
                .then(right.last_bom_import.cmp(&left.last_bom_import))
        }),
        cli::ProjectSort::LastBomImport => projects.sort_by(|left, right| {
            right
                .last_bom_import
                .cmp(&left.last_bom_import)
                .then(left.name.cmp(&right.name))
        }),
        cli::ProjectSort::Risk => projects.sort_by(|left, right| {
            right
                .last_inherited_risk_score
                .partial_cmp(&left.last_inherited_risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }
}

fn sort_findings(findings: &mut [Finding], sort: &cli::FindingSort) {
    match sort {
        cli::FindingSort::Severity => findings.sort_by(|left, right| {
            right
                .severity()
                .rank()
                .cmp(&left.severity().rank())
                .then(left.component_name().cmp(right.component_name()))
        }),
        cli::FindingSort::Component => findings.sort_by(|left, right| {
            left.component_name()
                .cmp(right.component_name())
                .then(right.severity().rank().cmp(&left.severity().rank()))
        }),
        cli::FindingSort::Vulnerability => {
            findings.sort_by_key(|finding| finding.vulnerability_id())
        }
    }
}

fn truncate<T>(items: &mut Vec<T>, limit: Option<usize>) {
    if let Some(limit) = limit {
        items.truncate(limit);
    }
}

pub(crate) fn require_non_empty(value: &str, field: &str) -> Result<()> {
    if value.trim().is_empty() {
        bail!("{field} cannot be empty");
    }
    Ok(())
}
