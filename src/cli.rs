use std::path::PathBuf;
use std::time::Duration;

use clap::{Args, Parser, Subcommand, ValueEnum};

use crate::models::{OutputFormat, Severity};

#[derive(Debug, Parser)]
#[command(
    name = "dtrack",
    version,
    about = "Dependency-Track CLI for humans and AI agents",
    long_about = "dtrack manages Dependency-Track projects, findings, vulnerabilities, and metrics. It is designed for interactive terminal use and for AI agents that need stable JSON output."
)]
pub struct Cli {
    #[arg(
        long,
        global = true,
        help = "Base URL of the Dependency-Track frontend or API server, for example https://dependency-track.example.com",
        long_help = "Base URL of the Dependency-Track instance. The CLI accepts either the frontend URL or the backend API URL and will normalize requests to /api/v1/... Overrides any context. The DTRACK_URL environment variable is used when neither --url nor --context is given."
    )]
    pub url: Option<String>,

    #[arg(
        short = 'c',
        long,
        global = true,
        env = "DTRACK_CONTEXT",
        help = "Named Dependency-Track installation from `dtrack context list`",
        long_help = "Named Dependency-Track installation created with `dtrack context add`. The context's URL and stored API key are used together; DTRACK_URL, DTRACK_API_KEY, DEPENDENCY_TRACK_API_KEY, and repository dtrack.toml connection values are ignored while a context is selected."
    )]
    pub context: Option<String>,

    #[arg(
        long = "project-version",
        global = true,
        help = "Select an exact project version when a project name has several versions"
    )]
    pub project_version: Option<String>,

    #[arg(
        long,
        global = true,
        help = "Dependency-Track API key (prefer contexts or environment variables)",
        long_help = "Dependency-Track API key. Read commands need VIEW_PORTFOLIO and VIEW_VULNERABILITY; `findings triage` also needs VULNERABILITY_ANALYSIS. Without a context, DTRACK_API_KEY or DEPENDENCY_TRACK_API_KEY is used. Passing a key on the command line stores it in shell history."
    )]
    pub api_key: Option<String>,

    #[arg(
        long,
        global = true,
        env = "DTRACK_CONFIG",
        help = "Optional TOML config file with url, api_key_env, timeout_seconds, and retries"
    )]
    pub config: Option<PathBuf>,

    #[arg(
        long,
        global = true,
        default_value = "human",
        value_enum,
        help = "Output format. Use json for stable machine-readable output."
    )]
    pub output: OutputFormat,

    #[arg(
        long,
        global = true,
        env = "DTRACK_TIMEOUT",
        value_parser = humantime::parse_duration,
        help = "Per-request timeout such as 10s, 30s, or 2m"
    )]
    pub timeout: Option<Duration>,

    #[arg(
        long,
        global = true,
        env = "DTRACK_RETRIES",
        help = "Retry count for transient HTTP 429/5xx errors and network failures"
    )]
    pub retries: Option<u32>,

    #[arg(
        long,
        global = true,
        help = "Print retry attempts and request context to stderr"
    )]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    #[command(about = "Find, inspect, and detect Dependency-Track projects")]
    Projects {
        #[command(subcommand)]
        command: ProjectCommands,
    },

    #[command(about = "List and summarize vulnerability findings for a project")]
    Findings {
        #[command(subcommand)]
        command: FindingsCommands,
    },

    #[command(
        aliases = ["vuln", "vulns"],
        about = "Inspect vulnerabilities and list affected projects"
    )]
    Vulnerabilities {
        #[command(subcommand)]
        command: VulnerabilityCommands,
    },

    #[command(about = "Show project or portfolio metrics")]
    Metrics {
        #[command(subcommand)]
        command: MetricsCommands,
    },

    #[command(about = "Show resolved non-secret configuration")]
    Config,

    #[command(
        about = "Add, select, test, and remove named Dependency-Track installations",
        long_about = "Manage named Dependency-Track installations (contexts). Each context stores a URL plus an API key in the OS credential store (macOS Keychain, Linux Secret Service) or a reference to an environment variable. Select one per command with -c NAME or DTRACK_CONTEXT, per repository with `context = \"NAME\"` in dtrack.toml, or by default with `dtrack context use NAME`."
    )]
    Context {
        #[command(subcommand)]
        command: ContextCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProjectCommands {
    #[command(
        alias = "ls",
        about = "List projects, optionally filtered locally by name, classifier, tag, or latest flag"
    )]
    List(ProjectListArgs),

    #[command(
        alias = "get",
        about = "Show one project by UUID or exact/partial name"
    )]
    Show(ProjectShowArgs),

    #[command(about = "Detect a Dependency-Track project for a local Git repository")]
    Detect(ProjectDetectArgs),

    #[command(
        about = "Queue a fresh vulnerability analysis for a project (writes to the server)",
        long_about = "Ask Dependency-Track to re-run vulnerability analysis for a project. The request is asynchronous: success means the work was queued, not finished. Re-query findings after the server has processed it. Requires VIEW_VULNERABILITY permission."
    )]
    Reanalyze(MetricsProjectArgs),
}

#[derive(Debug, Args)]
pub struct ProjectListArgs {
    #[arg(help = "Optional text matched against project name, version, and UUID")]
    pub query: Option<String>,

    #[arg(
        long,
        help = "Ask Dependency-Track to exclude inactive projects server-side"
    )]
    pub active: bool,

    #[arg(
        long,
        help = "Only include projects whose classifier matches this value, such as APPLICATION or CONTAINER"
    )]
    pub classifier: Option<String>,

    #[arg(long, help = "Only include projects with this tag name")]
    pub tag: Option<String>,

    #[arg(
        long,
        help = "Only include projects marked by Dependency-Track as the latest version"
    )]
    pub latest: bool,

    #[arg(
        long,
        default_value = "name",
        value_enum,
        help = "Sort field for human and JSON output"
    )]
    pub sort: ProjectSort,

    #[arg(
        long,
        help = "Maximum number of projects to print after filtering and sorting"
    )]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ProjectShowArgs {
    #[arg(help = "Project UUID, exact project name, or unique name fragment")]
    pub project: String,

    #[arg(
        long,
        help = "Include project properties, useful for repo matching metadata"
    )]
    pub properties: bool,

    #[arg(long, help = "Include current project metrics")]
    pub metrics: bool,

    #[arg(
        long,
        value_name = "MIN_SEVERITY",
        help = "Include findings at or above this severity, for example --findings high"
    )]
    pub findings: Option<Severity>,
}

#[derive(Debug, Args)]
pub struct ProjectDetectArgs {
    #[arg(
        default_value = ".",
        help = "Repository path to inspect with git rev-parse"
    )]
    pub path: String,

    #[arg(
        long,
        default_value = "origin",
        help = "Git remote name used for repository matching"
    )]
    pub remote: String,
}

#[derive(Debug, Subcommand)]
pub enum FindingsCommands {
    #[command(
        alias = "ls",
        about = "List findings for a project or for the current Git repository"
    )]
    List(FindingsListArgs),

    #[command(about = "Summarize findings by severity and analysis state")]
    Summary(FindingsSummaryArgs),

    #[command(
        about = "Record an analysis decision for one finding (writes to the server)",
        long_about = "Record an analysis decision (state, justification, response, details, suppression, audit comment) for one finding. The finding is located by component name, PURL, or UUID and by vulnerability ID or UUID, including suppressed findings. Fields you do not pass keep their current value. Requires VULNERABILITY_ANALYSIS permission. Writes are never retried, to avoid duplicate audit comments."
    )]
    Triage(TriageArgs),
}

#[derive(Debug, Args)]
pub struct FindingsListArgs {
    #[arg(
        help = "Project UUID/name. Omit this with --repo to detect from a local Git repository."
    )]
    pub project: Option<String>,

    #[arg(
        long,
        help = "Repository path for project auto-detection when project is omitted"
    )]
    pub repo: Option<String>,

    #[arg(
        long,
        default_value = "origin",
        help = "Git remote name used with --repo or omitted project"
    )]
    pub remote: String,

    #[arg(
        long,
        default_value = "critical",
        value_enum,
        help = "Lowest severity to include"
    )]
    pub min_severity: Severity,

    #[arg(
        long,
        help = "Include findings suppressed in Dependency-Track analysis"
    )]
    pub include_suppressed: bool,

    #[arg(
        long,
        help = "Only include findings with this analysis state, such as NOT_AFFECTED or EXPLOITABLE"
    )]
    pub state: Option<String>,

    #[arg(
        long,
        help = "Only include findings whose component name contains this text"
    )]
    pub component: Option<String>,

    #[arg(
        long,
        help = "Only include findings whose vulnerability ID contains this text"
    )]
    pub vulnerability: Option<String>,

    #[arg(
        long,
        default_value = "severity",
        value_enum,
        help = "Sort field for findings"
    )]
    pub sort: FindingSort,

    #[arg(
        long,
        help = "Maximum number of findings to print after filtering and sorting"
    )]
    pub limit: Option<usize>,
}

#[derive(Debug, Args)]
pub struct FindingsSummaryArgs {
    #[arg(
        help = "Project UUID/name. Omit this with --repo to detect from a local Git repository."
    )]
    pub project: Option<String>,

    #[arg(
        long,
        help = "Repository path for project auto-detection when project is omitted"
    )]
    pub repo: Option<String>,

    #[arg(
        long,
        default_value = "origin",
        help = "Git remote name used with --repo or omitted project"
    )]
    pub remote: String,

    #[arg(long, help = "Include suppressed findings in summary counts")]
    pub include_suppressed: bool,
}

#[derive(Debug, Subcommand)]
pub enum VulnerabilityCommands {
    #[command(
        alias = "get",
        about = "Show vulnerability details by source and vulnerability ID"
    )]
    Show(VulnerabilityShowArgs),

    #[command(alias = "ls", about = "List projects affected by a vulnerability")]
    Projects(VulnerabilityProjectsArgs),
}

#[derive(Debug, Args)]
pub struct VulnerabilityShowArgs {
    #[arg(help = "Vulnerability source, for example NVD, GITHUB, OSV, or INTERNAL")]
    pub source: String,

    #[arg(help = "Vulnerability ID, for example CVE-2024-12345")]
    pub vulnerability: String,
}

#[derive(Debug, Args)]
pub struct VulnerabilityProjectsArgs {
    #[arg(help = "Vulnerability source, for example NVD, GITHUB, OSV, or INTERNAL")]
    pub source: String,

    #[arg(help = "Vulnerability ID, for example CVE-2024-12345")]
    pub vulnerability: String,

    #[arg(long, help = "Exclude inactive projects server-side")]
    pub exclude_inactive: bool,

    #[arg(
        long,
        default_value = "risk",
        value_enum,
        help = "Sort affected projects"
    )]
    pub sort: ProjectSort,

    #[arg(long, help = "Maximum number of projects to print after sorting")]
    pub limit: Option<usize>,
}

#[derive(Debug, Subcommand)]
pub enum MetricsCommands {
    #[command(about = "Show current metrics for one project")]
    Project(MetricsProjectArgs),

    #[command(about = "Show current portfolio-wide metrics")]
    Portfolio,
}

#[derive(Debug, Args)]
pub struct MetricsProjectArgs {
    #[arg(help = "Project UUID, exact project name, or unique name fragment")]
    pub project: String,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum ProjectSort {
    Name,
    LastBomImport,
    Risk,
}

#[derive(Clone, Debug, ValueEnum)]
pub enum FindingSort {
    Severity,
    Component,
    Vulnerability,
}

#[derive(Debug, Subcommand)]
pub enum ContextCommands {
    #[command(
        about = "Add a context; prompts for missing URL and API key in a terminal",
        long_about = "Add a named Dependency-Track installation. Missing --url and API key values are prompted for when stdin is a terminal (the key is read hidden); non-interactive callers must pass them. The connection is tested before anything is saved. Keys are stored in the OS credential store unless --api-key-env is used, in which case only the variable name is saved. The first context added becomes the current context."
    )]
    Add {
        #[arg(help = "Context name, for example home-dep-track")]
        name: String,
        #[arg(long, help = "Dependency-Track frontend or API URL")]
        url: Option<String>,
        #[arg(
            long,
            help = "API key to store in the OS credential store (visible in shell history; prefer the prompt or --api-key-stdin)"
        )]
        api_key: Option<String>,
        #[arg(
            long,
            conflicts_with_all = ["api_key", "api_key_stdin"],
            help = "Read the API key from this environment variable at run time instead of storing it"
        )]
        api_key_env: Option<String>,
        #[arg(
            long,
            conflicts_with = "api_key",
            help = "Read the API key from stdin and store it in the OS credential store"
        )]
        api_key_stdin: bool,
    },
    #[command(alias = "ls", about = "List contexts and show the current one")]
    List,
    #[command(about = "Make a context the default for future commands")]
    Use {
        #[arg(help = "Context name")]
        name: String,
    },
    #[command(about = "Show the default context")]
    Current,
    #[command(alias = "rm", about = "Remove a context and its stored API key")]
    Remove {
        #[arg(help = "Context name")]
        name: String,
    },
    #[command(about = "Check that a context's URL and API key work")]
    Test {
        #[arg(help = "Context name; defaults to -c/DTRACK_CONTEXT, then the current context")]
        name: Option<String>,
    },
}

#[derive(Debug, Args)]
pub struct TriageArgs {
    #[arg(help = "Project UUID, exact project name, or unique name fragment")]
    pub project: String,
    #[arg(
        long,
        help = "Component name, name@version, PURL, or component UUID from `findings list`"
    )]
    pub component: String,
    #[arg(
        long,
        help = "Vulnerability ID such as CVE-2024-12345 or GHSA-..., or vulnerability UUID"
    )]
    pub vulnerability: String,
    #[arg(long, value_parser = ["NOT_SET", "IN_TRIAGE", "EXPLOITABLE", "NOT_AFFECTED", "FALSE_POSITIVE", "RESOLVED"], help = "Analysis state")]
    pub state: Option<String>,
    #[arg(
        long,
        value_parser = [
            "NOT_SET", "CODE_NOT_PRESENT", "CODE_NOT_REACHABLE", "REQUIRES_CONFIGURATION",
            "REQUIRES_DEPENDENCY", "REQUIRES_ENVIRONMENT", "PROTECTED_BY_COMPILER",
            "PROTECTED_AT_RUNTIME", "PROTECTED_AT_PERIMETER", "PROTECTED_BY_MITIGATING_CONTROL",
        ],
        help = "Why the finding is not exploitable; use with --state NOT_AFFECTED"
    )]
    pub justification: Option<String>,
    #[arg(
        long,
        value_parser = ["NOT_SET", "CAN_NOT_FIX", "WILL_NOT_FIX", "UPDATE", "ROLLBACK", "WORKAROUND_AVAILABLE"],
        help = "Planned vendor/team response"
    )]
    pub response: Option<String>,
    #[arg(long, help = "Analysis details: the evidence behind the decision")]
    pub details: Option<String>,
    #[arg(long, help = "Suppress (true) or unsuppress (false) the finding")]
    pub suppressed: Option<bool>,
    #[arg(long, help = "Audit-trail comment to append")]
    pub comment: Option<String>,
}
