# Dependency-Track CLI

`dtrack` is a read-only Rust CLI for querying Dependency-Track from a terminal or an AI agent workflow. It replaces the old MCP-oriented toolkit with direct commands that agents can discover through `--help`, execute safely, and parse as JSON.

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

