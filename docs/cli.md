# CLI Reference

`dtrack` is intentionally command-discoverable. Prefer the built-in help for the exact current interface:

```bash
dtrack --help
dtrack projects list --help
dtrack findings list --help
```

## Global Options

- `--url`: Dependency-Track base URL. Environment: `DTRACK_URL`.
- `--api-key`: Dependency-Track API key. Environment: `DTRACK_API_KEY`; `DEPENDENCY_TRACK_API_KEY` is also supported. Environment keys are ignored while a context is selected.
- `-c`, `--context`: named installation from `dtrack context add`. Environment: `DTRACK_CONTEXT`.
- `--project-version`: exact project version when a name has several versions.
- `--config`: TOML config path. Environment: `DTRACK_CONFIG`.
- `--output human|json`: human tables or stable JSON. Default: `human`.
- `--timeout`: per-request timeout such as `10s` or `2m`. Default: `30s`.
- `--retries`: transient failure retry count. Default: `2`.
- `--verbose`: print request and retry context to stderr.

## Useful Agent Commands

List risky active application projects:

```bash
dtrack --output json projects list --active --classifier APPLICATION --sort risk --limit 20
```

Short form:

```bash
dtrack --output json projects ls --active --classifier APPLICATION --sort risk --limit 20
```

Inspect a project and include metrics:

```bash
dtrack --output json projects show <project-name-or-uuid> --metrics --properties
```

List high and critical findings:

```bash
dtrack --output json findings list <project-name-or-uuid> --min-severity high --sort severity
```

Short form:

```bash
dtrack --output json findings ls <project-name-or-uuid> --min-severity high --sort severity
```

Get a compact finding summary:

```bash
dtrack --output json findings summary <project-name-or-uuid>
```

Find all projects affected by a vulnerability:

```bash
dtrack --output json vulnerabilities projects NVD CVE-2025-62718 --exclude-inactive
```

Short form:

```bash
dtrack --output json vuln ls NVD CVE-2025-62718 --exclude-inactive
```

Detect the current repository:

```bash
dtrack --output json projects detect .
dtrack --output json findings list --repo .
```

## Output Guidance

Use `--output json` for automation. JSON keys use Dependency-Track's camelCase style where possible and preserve raw finding component/vulnerability payloads so agents can inspect fields the CLI does not render in tables.

Human output uses tables and severity coloring where the terminal supports it. Counts are printed to stderr for list commands so JSON stdout remains clean in machine-readable mode.

## Failure Behavior

The CLI exits non-zero and prints an `error:` line on failure. Common failures:

- missing URL or API key
- HTTP 401/403 from insufficient Dependency-Track permissions
- ambiguous project name or fragment
- unknown project or vulnerability
- Git remote mismatch during `projects detect`
- timeout or exhausted retries

## Context and write commands

See [README](../README.md#contexts-and-credentials) for credential storage and
platform config paths. Context commands: `add`, `list`, `use`, `current`,
`remove`, `test`. All accept `--output json`.

`-c, --context NAME` (or `DTRACK_CONTEXT`) selects an installation. See the
README for how the URL and key are chosen.

`--project-version VERSION` selects an exact project version for every command
that takes a project. Without it, the version marked latest is used.

`findings triage PROJECT --component NAME|PURL|UUID --vulnerability ID|UUID`
accepts `--state`, `--justification`, `--response`, `--details`,
`--suppressed true|false`, and `--comment`. `projects reanalyze PROJECT` queues
a fresh analysis. See `--help` on each for the accepted values and permissions.
