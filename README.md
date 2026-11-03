# Dependency-Track CLI

`dtrack` is a Rust CLI for querying Dependency-Track from a terminal or an AI agent workflow. It replaces the old MCP-oriented toolkit with direct commands that agents can discover through `--help`, execute safely, and parse as JSON.

The CLI focuses on the Dependency-Track API areas that are useful during vulnerability remediation:

- project discovery and inspection
- Git repository to Dependency-Track project detection
- project finding export with severity/component/state filters
- vulnerability details and affected project lookup
- current project and portfolio metrics

## Install

```bash
cargo build --release
./target/release/dtrack --help
```

During development:

```bash
cargo run -- --help
```

## Configuration

The fastest setup is environment variables:

```bash
export DTRACK_URL="https://dependency-track.example.com"
export DTRACK_API_KEY="odt_..."
dtrack projects list --active
```

For compatibility with existing local setups, `DEPENDENCY_TRACK_API_KEY` is also accepted.

You can also use `dtrack.toml` in the current directory, or pass `--config path/to/dtrack.toml`:

```toml
url = "https://dependency-track.example.com"
api_key_env = "DEPENDENCY_TRACK_API_KEY"
timeout_seconds = 30
retries = 2
```

## Agent-Friendly Usage

Use `--output json` when another program or LLM will consume the output:

```bash
dtrack --output json projects list --active --classifier APPLICATION --sort risk --limit 20
dtrack --output json projects show ds --metrics --properties
dtrack --output json findings list ds --min-severity high --sort severity --limit 50
dtrack --output json findings summary ds
dtrack --output json vulnerabilities projects NVD CVE-2025-62718 --exclude-inactive
```

Common aliases are available for shorter agent commands:

```bash
dtrack projects ls --active
dtrack projects get ds --metrics
dtrack findings ls ds --min-severity high
dtrack vuln ls NVD CVE-2025-62718
```

Use human output for interactive triage:

```bash
dtrack projects list --active --sort risk --limit 10
dtrack metrics project ds
dtrack findings list ds --min-severity critical --limit 10
```

If the current repository has matching Dependency-Track project metadata, omit the project and let the CLI detect it:

```bash
dtrack projects detect .
dtrack findings list --repo .
```

Project detection compares the local Git remote with project properties such as:

- `automation.cloneUrl`
- `automation.repositoryUrl`
- `repo.cloneUrl`
- `repo.repositoryUrl`
- `vcs.cloneUrl`
- `vcs.repositoryUrl`

It also checks Git-like project external references.

## Commands

Run `dtrack --help` or any subcommand help for the full reference:

```bash
dtrack projects --help
dtrack findings list --help
dtrack vulnerabilities --help
dtrack metrics --help
```

Primary command groups:

- `projects list`: list projects with filters for active status, classifier, tags, latest version, sorting, and limits.
- `projects show`: inspect one project by UUID, exact name, or unique name fragment. Optional flags include `--properties`, `--metrics`, and `--findings`.
- `projects detect`: resolve a local Git repository to a Dependency-Track project.
- `findings list`: export findings for a project, filter by severity, component, vulnerability ID, analysis state, and suppression status.
- `findings summary`: count findings by severity and analysis state.
- `vulnerabilities show`: fetch vulnerability details by source and ID.
- `vulnerabilities projects`: list projects affected by a vulnerability.
- `metrics project`: show current metrics for one project.
- `metrics portfolio`: show current portfolio metrics.
- `config`: show resolved non-secret configuration.

Aliases: `ls` for list-like commands, `get` for show-like commands, and `vuln`/`vulns` for `vulnerabilities`.

## Reliability

Every HTTP request has a configurable timeout and retry count:

```bash
dtrack --timeout 10s --retries 4 --verbose projects list --active
```

Retries are used for transient network failures, HTTP 429, and HTTP 5xx responses. HTTP errors include the method, API path, status code, and response body where available.

## Development

```bash
cargo fmt
cargo test
cargo run -- --url https://dt.bencetotht.hu --output json projects list --active --limit 5
```

The CLI uses Dependency-Track's documented OpenAPI endpoints under `/api/v1`. The public Dependency-Track docs note that the OpenAPI spec is served by the backend at `/api/openapi.json` or `/api/openapi.yaml`.


## Contexts and credentials

Install from a checkout with `cargo install --path . --locked`. Release binaries
are built by the Rust CI workflow for Linux and macOS.

`dtrack context add home --url https://dt.example.com` prompts for a hidden key,
validates the connection, and stores the key in macOS Keychain or Linux Secret
Service. Linux desktops need a running, unlocked Secret Service (such as GNOME
Keyring). Headless Linux can use `--api-key-env HOME_DT_KEY` instead.
`--api-key-stdin` accepts a piped key; keys are never saved by these commands
in TOML. Fully specified commands do not prompt.

Manage installations with `context list`, `context use NAME`,
`context current`, `context remove NAME`, and `context test [NAME]`.
Select one per invocation with `-c NAME` or `DTRACK_CONTEXT`.
A repository's `dtrack.toml` may contain `context = "company"`.
Selection order is flag/environment, repository context, then user
`current_context`. Individual connection flags/environment override file values.
Repository flat values override the selected context, which overrides user flat
values. Explicit timeout and retry flags always win.

The user config is `~/Library/Application Support/dtrack/config.toml` on macOS
and `$XDG_CONFIG_HOME/dtrack/config.toml` (normally
`~/.config/dtrack/config.toml`) on Linux. `--config` selects an alternative
file for both context management and command execution.
`dtrack config` works without a URL or key and never displays credentials.

## Record triage and reanalyze

Use project UUIDs or select a version with `--project-version VERSION`.
Exact names take priority over partial matches; multiple versions require a
version selector or UUID.

```sh
dtrack findings triage PROJECT --component COMPONENT_UUID --vulnerability VULNERABILITY_UUID \
  --state NOT_AFFECTED --suppressed true --comment "Reason and supporting evidence"
dtrack projects reanalyze PROJECT
```

Triage preserves existing fields you did not specify. These commands need
server write permissions (VULNERABILITY_ANALYSIS for triage; permissions for
the finding analysis endpoint for reanalysis). Reanalysis is asynchronous:
acceptance does not mean processing has finished. Writes are not automatically
retried, to avoid duplicate audit comments. Read retries mean additional attempts:
`--retries 2` makes at most three requests.
