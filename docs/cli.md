# CLI

The CLI is the simplest way to use this toolkit. It runs local read-only Dependency-Track queries and does not require MCP integration.

## What It Does

The CLI can:

- list visible Dependency-Track projects
- detect the Dependency-Track project for the current repository
- inspect the latest state of a project
- list current findings for a project
- install the optional Codex and Claude agent assets

The CLI does not modify Dependency-Track, source code, dependencies, or Git history.

## Setup

### Prerequisites

- Bun installed locally
- a reachable Dependency-Track instance
- a Dependency-Track API key

### Configuration

1. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
2. Set `dependencyTrack.url`.
3. Set `dependencyTrack.apiKeyEnv` to the environment variable name that will hold your Dependency-Track API key.
4. Export that environment variable before running the CLI.

Example config:

```json
{
  "dependencyTrack": {
    "url": "https://dep.example.com",
    "apiKeyEnv": "DEPENDENCY_TRACK_API_KEY"
  }
}
```

Export the API key:

```bash
export DEPENDENCY_TRACK_API_KEY=your_api_key_here
```

You can point the CLI at a different config file with `--config path`.

## Quick Start

From the toolkit checkout:

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects detect /path/to/repo
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts findings list
```

If you prefer built entrypoints, run:

```bash
bun run build
```

Then use the generated CLI:

```bash
bun dist/cli.js projects detect
```

## Command Reference

Global options:

- `--config <path>`: load a config file from a non-default location
- `--json`: print machine-readable JSON output

### `projects list [query]`

Lists Dependency-Track projects visible to the configured API key. If `query` is provided, it filters by project name or UUID.

Example:

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects list billing
```

### `projects detect [repo-path]`

Detects the Dependency-Track project that matches a local git repository. If `repo-path` is omitted, the current working directory is used.

The detection flow:

1. finds the git root
2. reads the `origin` remote URL
3. normalizes the remote URL
4. compares it with Dependency-Track project metadata

Matching uses, in order of availability:

- `projects[].cloneUrl` from local config
- project properties such as `automation.cloneUrl`, `automation.repoUrl`, `automation.repositoryUrl`, `repo.cloneUrl`, `repo.repositoryUrl`, `vcs.cloneUrl`, and `vcs.repositoryUrl`
- project external references

This avoids guessing by repository name alone.

Examples:

```bash
bun run src/cli.ts projects detect
bun run src/cli.ts projects detect /path/to/repo
```

### `projects inspect [project-name-or-uuid]`

Returns the latest project details, project properties, and vulnerability totals.

If you pass no identifier, the CLI first detects the project from the current repository path.

Examples:

```bash
bun run src/cli.ts projects inspect
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts projects inspect 00000000-0000-0000-0000-000000000000
```

### `findings list [project-name-or-uuid] [severity]`

Returns current findings for the selected project. Suppressed findings are filtered out.

Default severity is `CRITICAL`. Supported severities are:

- `CRITICAL`
- `HIGH`
- `MEDIUM`
- `LOW`
- `UNASSIGNED`

Examples:

```bash
bun run src/cli.ts findings list
bun run src/cli.ts findings list HIGH
bun run src/cli.ts findings list simplemailer CRITICAL
```

### `install-agent-assets [--force] [--codex] [--claude]`

Installs the optional Codex and Claude assets from this repository into your home directory.

Behavior:

- if you pass neither `--codex` nor `--claude`, both are installed
- `--codex` installs only Codex assets
- `--claude` installs only Claude assets
- `--force` replaces already-installed files

Examples:

```bash
bun run src/cli.ts install-agent-assets
bun run src/cli.ts install-agent-assets --codex
bun run src/cli.ts install-agent-assets --claude
bun run src/cli.ts install-agent-assets --force
```

## Output

Human-readable output is the default. Use `--json` when you want stable output for automation, wrapper scripts, or agent consumption.

## Notes And Warnings

- The CLI is intentionally query-only.
- It is designed to provide Dependency-Track context to a follow-up remediation workflow, not to perform remediation itself.
- If the configured API key environment variable is missing, the CLI exits with an error.
