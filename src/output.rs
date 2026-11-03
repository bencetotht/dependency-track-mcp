use anyhow::Result;
use chrono::{DateTime, Utc};
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL};
use serde::Serialize;
use serde_json::Value;

use crate::models::{Finding, Metrics, OutputFormat, Project, Severity};

pub fn print_json<T: Serialize + ?Sized>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_value(value: &Value, output: OutputFormat) -> Result<()> {
    match output {
        OutputFormat::Json => print_json(value),
        OutputFormat::Human => {
            render_human(value, 0);
            Ok(())
        }
    }
}

pub fn print_projects(projects: &[Project], output: OutputFormat) -> Result<()> {
    if output == OutputFormat::Json {
        return print_json(projects);
    }

    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header([
        "Name",
        "Version",
        "Classifier",
        "Active",
        "Latest",
        "Risk",
        "Last BOM",
        "UUID",
    ]);
    for project in projects {
        table.add_row([
            Cell::new(&project.name),
            Cell::new(project.version.as_deref().unwrap_or("")),
            Cell::new(project.classifier.as_deref().unwrap_or("")),
            Cell::new(project.active.map(bool_label).unwrap_or("")),
            Cell::new(project.is_latest.map(bool_label).unwrap_or("")),
            Cell::new(
                project
                    .last_inherited_risk_score
                    .map(|score| format!("{score:.1}"))
                    .unwrap_or_default(),
            ),
            Cell::new(format_epoch(project.last_bom_import)),
            Cell::new(&project.uuid),
        ]);
    }
    println!("{table}");
    eprintln!("{} project(s)", projects.len());
    Ok(())
}

pub fn print_findings(project: &Project, findings: &[Finding]) -> Result<()> {
    println!(
        "{} {} ({})",
        project.name,
        project.version.as_deref().unwrap_or(""),
        project.uuid
    );
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header([
        "Severity",
        "Vulnerability",
        "Title",
        "Source",
        "CVSS",
        "EPSS",
        "Component",
        "Current",
        "Latest",
        "PURL",
        "State",
        "Suppressed",
    ]);
    for finding in findings {
        table.add_row([
            severity_cell(finding.severity()),
            Cell::new(finding.vulnerability_id()),
            Cell::new(trim_for_table(finding.title(), 72)),
            Cell::new(finding.source()),
            Cell::new(
                finding
                    .cvss_score()
                    .map(|score| format!("{score:.1}"))
                    .unwrap_or_default(),
            ),
            Cell::new(
                finding
                    .epss_score()
                    .map(|score| format!("{score:.3}"))
                    .unwrap_or_default(),
            ),
            Cell::new(finding.component_name()),
            Cell::new(finding.component_version()),
            Cell::new(finding.latest_version()),
            Cell::new(trim_for_table(finding.component_purl(), 64)),
            Cell::new(finding.analysis_state()),
            Cell::new(bool_label(finding.is_suppressed())),
        ]);
    }
    println!("{table}");
    eprintln!("{} finding(s)", findings.len());
    Ok(())
}

pub fn print_metrics(
    project: Option<&Project>,
    metrics: &Metrics,
    output: OutputFormat,
) -> Result<()> {
    if output == OutputFormat::Json {
        return print_json(&serde_json::json!({ "project": project, "metrics": metrics }));
    }

    if let Some(project) = project {
        println!(
            "{} {} ({})",
            project.name,
            project.version.as_deref().unwrap_or(""),
            project.uuid
        );
    } else {
        println!("Portfolio metrics");
    }
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(["Metric", "Value"]);
    for (name, value) in [
        ("critical", metrics.critical),
        ("high", metrics.high),
        ("medium", metrics.medium),
        ("low", metrics.low),
        ("unassigned", metrics.unassigned),
        ("vulnerabilities", metrics.vulnerabilities),
        ("vulnerable components", metrics.vulnerable_components),
        ("components", metrics.components),
        ("suppressed", metrics.suppressed),
        ("findings total", metrics.findings_total),
        ("findings audited", metrics.findings_audited),
        ("findings unaudited", metrics.findings_unaudited),
        ("policy violations", metrics.policy_violations_total),
    ] {
        if let Some(value) = value {
            table.add_row([Cell::new(name), Cell::new(value)]);
        }
    }
    if let Some(score) = metrics.inherited_risk_score {
        table.add_row([
            Cell::new("inherited risk score"),
            Cell::new(format!("{score:.1}")),
        ]);
    }
    println!("{table}");
    Ok(())
}

fn severity_cell(severity: Severity) -> Cell {
    let color = match severity {
        Severity::Critical => Color::Red,
        Severity::High => Color::DarkRed,
        Severity::Medium => Color::Yellow,
        Severity::Low => Color::Green,
        Severity::Info => Color::Cyan,
        Severity::Unassigned => Color::Grey,
    };
    Cell::new(severity.as_str()).fg(color)
}

fn bool_label(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

fn format_epoch(value: Option<i64>) -> String {
    let Some(value) = value else {
        return String::new();
    };
    DateTime::<Utc>::from_timestamp_millis(value)
        .map(|date| date.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| value.to_string())
}

fn trim_for_table(value: &str, limit: usize) -> String {
    if value.chars().count() <= limit {
        return value.to_string();
    }
    let mut trimmed = value
        .chars()
        .take(limit.saturating_sub(3))
        .collect::<String>();
    trimmed.push_str("...");
    trimmed
}

fn render_human(value: &Value, depth: usize) {
    let indent = "  ".repeat(depth);
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if value.is_null() {
                    continue;
                }
                if value.is_object() || value.is_array() {
                    println!("{indent}{key}:");
                    render_human(value, depth + 1);
                } else {
                    println!(
                        "{indent}{key}: {}",
                        value
                            .as_str()
                            .map(str::to_owned)
                            .unwrap_or_else(|| value.to_string())
                    );
                }
            }
        }
        Value::Array(items) => {
            for (index, value) in items.iter().enumerate() {
                println!("{indent}[{}]", index + 1);
                render_human(value, depth + 1);
            }
        }
        _ => println!(
            "{indent}{}",
            value
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| value.to_string())
        ),
    }
}
