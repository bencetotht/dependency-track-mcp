use std::collections::BTreeMap;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    Human,
    Json,
}

#[derive(
    Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, ValueEnum,
)]
#[serde(rename_all = "UPPERCASE")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
    Unassigned,
}

impl Severity {
    pub fn rank(self) -> u8 {
        match self {
            Self::Critical => 5,
            Self::High => 4,
            Self::Medium => 3,
            Self::Low => 2,
            Self::Info => 1,
            Self::Unassigned => 0,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Critical => "CRITICAL",
            Self::High => "HIGH",
            Self::Medium => "MEDIUM",
            Self::Low => "LOW",
            Self::Info => "INFO",
            Self::Unassigned => "UNASSIGNED",
        }
    }
}

impl From<&str> for Severity {
    fn from(value: &str) -> Self {
        match value.to_ascii_uppercase().as_str() {
            "CRITICAL" => Self::Critical,
            "HIGH" => Self::High,
            "MEDIUM" => Self::Medium,
            "LOW" => Self::Low,
            "INFO" | "INFORMATIONAL" => Self::Info,
            _ => Self::Unassigned,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub uuid: String,
    pub name: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub classifier: Option<String>,
    #[serde(default)]
    pub active: Option<bool>,
    #[serde(default, alias = "latestVersion")]
    pub is_latest: Option<bool>,
    #[serde(default)]
    pub last_bom_import: Option<i64>,
    #[serde(default)]
    pub last_bom_import_format: Option<String>,
    #[serde(default)]
    pub last_inherited_risk_score: Option<f64>,
    #[serde(default)]
    pub tags: Vec<Tag>,
    #[serde(default)]
    pub external_references: Vec<ExternalReference>,
    #[serde(default)]
    pub metrics: Option<Metrics>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Tag {
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ExternalReference {
    #[serde(default)]
    pub r#type: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectProperty {
    #[serde(default)]
    pub group_name: Option<String>,
    #[serde(default)]
    pub property_name: Option<String>,
    #[serde(default)]
    pub property_value: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Finding {
    #[serde(default)]
    pub matrix: Option<String>,
    #[serde(default)]
    pub component: Value,
    #[serde(default)]
    pub vulnerability: Value,
    #[serde(default)]
    pub analysis: Value,
}

impl Finding {
    pub fn component_name(&self) -> &str {
        self.component
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn component_version(&self) -> &str {
        self.component
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn component_purl(&self) -> &str {
        self.component
            .get("purl")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn latest_version(&self) -> &str {
        self.component
            .get("latestVersion")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn vulnerability_id(&self) -> String {
        self.vulnerability
            .get("vulnId")
            .or_else(|| self.vulnerability.get("friendlyVulnId"))
            .or_else(|| self.vulnerability.get("uuid"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    pub fn title(&self) -> &str {
        self.vulnerability
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn severity(&self) -> Severity {
        Severity::from(
            self.vulnerability
                .get("severity")
                .and_then(Value::as_str)
                .unwrap_or("UNASSIGNED"),
        )
    }

    pub fn source(&self) -> &str {
        self.vulnerability
            .get("source")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn cvss_score(&self) -> Option<f64> {
        self.vulnerability
            .get("cvssV3BaseScore")
            .or_else(|| self.vulnerability.get("cvssV2BaseScore"))
            .and_then(Value::as_f64)
    }

    pub fn epss_score(&self) -> Option<f64> {
        self.vulnerability.get("epssScore").and_then(Value::as_f64)
    }

    pub fn analysis_state(&self) -> &str {
        self.analysis
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("UNTRIAGED")
    }

    pub fn component_uuid(&self) -> &str {
        self.component
            .get("uuid")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    pub fn vulnerability_uuid(&self) -> &str {
        self.vulnerability
            .get("uuid")
            .and_then(Value::as_str)
            .unwrap_or("")
    }

    /// Matches a component UUID, PURL (with or without qualifiers), name, or name@version.
    pub fn matches_component(&self, query: &str) -> bool {
        let query = query.trim();
        let purl = self.component_purl();
        let name = self.component_name();
        (!self.component_uuid().is_empty() && self.component_uuid().eq_ignore_ascii_case(query))
            || (!purl.is_empty() && (purl == query || purl.split(['?', '#']).next() == Some(query)))
            || (!name.is_empty() && name.eq_ignore_ascii_case(query))
            || format!("{name}@{}", self.component_version()).eq_ignore_ascii_case(query)
    }

    /// Matches a vulnerability UUID, ID, or alias such as the CVE behind a GHSA.
    pub fn matches_vulnerability(&self, query: &str) -> bool {
        let query = query.trim();
        let uuid = self.vulnerability_uuid();
        if (!uuid.is_empty() && uuid.eq_ignore_ascii_case(query))
            || self.vulnerability_id().eq_ignore_ascii_case(query)
        {
            return true;
        }
        self.vulnerability
            .get("aliases")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .flat_map(|alias| alias.values())
            .filter_map(Value::as_str)
            .any(|alias| alias.eq_ignore_ascii_case(query))
    }

    pub fn is_suppressed(&self) -> bool {
        self.analysis
            .get("isSuppressed")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Vulnerability {
    pub uuid: String,
    pub source: String,
    pub vuln_id: String,
    #[serde(default)]
    pub friendly_vuln_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub recommendation: Option<String>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub cvss_v3_base_score: Option<f64>,
    #[serde(default)]
    pub cvss_v2_base_score: Option<f64>,
    #[serde(default)]
    pub epss_score: Option<f64>,
    #[serde(default)]
    pub epss_percentile: Option<f64>,
    #[serde(default)]
    pub affected_project_count: Option<i64>,
    #[serde(default)]
    pub affected_active_project_count: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    #[serde(default)]
    pub critical: Option<i64>,
    #[serde(default)]
    pub high: Option<i64>,
    #[serde(default)]
    pub medium: Option<i64>,
    #[serde(default)]
    pub low: Option<i64>,
    #[serde(default)]
    pub unassigned: Option<i64>,
    #[serde(default)]
    pub vulnerabilities: Option<i64>,
    #[serde(default)]
    pub vulnerable_components: Option<i64>,
    #[serde(default)]
    pub components: Option<i64>,
    #[serde(default)]
    pub suppressed: Option<i64>,
    #[serde(default)]
    pub findings_total: Option<i64>,
    #[serde(default)]
    pub findings_audited: Option<i64>,
    #[serde(default)]
    pub findings_unaudited: Option<i64>,
    #[serde(default)]
    pub inherited_risk_score: Option<f64>,
    #[serde(default)]
    pub policy_violations_total: Option<i64>,
    #[serde(default)]
    pub first_occurrence: Option<i64>,
    #[serde(default)]
    pub last_occurrence: Option<i64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedProject {
    pub project: Project,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_source: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectedProject {
    pub project: Project,
    pub properties: Vec<ProjectProperty>,
    pub repo_path: String,
    pub remote_url: String,
    pub match_source: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FindingSummary {
    pub project: Project,
    pub total: usize,
    pub by_severity: BTreeMap<String, usize>,
    pub by_analysis_state: BTreeMap<String, usize>,
    pub suppressed: usize,
}

impl FindingSummary {
    pub fn from_findings(project: Project, findings: &[Finding]) -> Self {
        let mut by_severity = BTreeMap::new();
        let mut by_analysis_state = BTreeMap::new();
        let mut suppressed = 0;

        for finding in findings {
            *by_severity
                .entry(finding.severity().as_str().to_string())
                .or_insert(0) += 1;
            *by_analysis_state
                .entry(finding.analysis_state().to_string())
                .or_insert(0) += 1;
            if finding.is_suppressed() {
                suppressed += 1;
            }
        }

        Self {
            project,
            total: findings.len(),
            by_severity,
            by_analysis_state,
            suppressed,
        }
    }
}
