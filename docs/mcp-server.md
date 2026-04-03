# MCP Server

The MCP server exposes the same read-only Dependency-Track queries as the CLI over stdio so tools such as Codex and Claude Code can call them directly.

## What It Does

The server provides:

- four MCP tools for project lookup and findings retrieval
- one MCP prompt for vulnerability triage

It does not edit source code, change dependencies, or create commits, PRs, or MRs.

## Setup

### Prerequisites

- Bun installed locally
- a reachable Dependency-Track instance
- a Dependency-Track API key
- a built server artifact if you want to run `dist/mcp-server.js`

### Configuration

The MCP server uses the same config file and API key environment variable as the CLI.

1. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
2. Set `dependencyTrack.url`.
3. Export the API key environment variable named by `dependencyTrack.apiKeyEnv`.
4. Build the project:

```bash
bun run build
```

You can also run the server directly from source during development:

```bash
bun run src/mcp-server.ts
```

For agent integrations, using an absolute `DTRACK_AGENT_CONFIG` path is recommended so the server can always find the intended config file regardless of its startup working directory.

## Quick Start

Run the built stdio server locally:

```bash
bun dist/mcp-server.js
```

## Tools

### `dtrack_list_projects`

Lists Dependency-Track projects visible to the configured API key.

Arguments:

- `query` optional string

### `dtrack_detect_project`

Detects the Dependency-Track project for a local git repository.

Arguments:

- `repoPath` optional string

### `dtrack_get_project`

Returns the latest project details and vulnerability totals for a Dependency-Track project, or detects the project from a repository path first.

Arguments:

- `projectIdentifier` optional string
- `repoPath` optional string

### `dtrack_get_findings`

Returns current findings for a Dependency-Track project, or detects the project from a repository path first.

Arguments:

- `projectIdentifier` optional string
- `repoPath` optional string
- `severity` optional string, default `CRITICAL`

Supported severities:

- `CRITICAL`
- `HIGH`
- `MEDIUM`
- `LOW`
- `UNASSIGNED`

## Prompt

### `dtrack-triage`

This prompt asks the agent to inspect the latest critical findings for a project and summarize the vulnerability details needed for a separate remediation workflow.

Arguments:

- `projectIdentifier` optional string
- `repoPath` optional string

## Codex Setup

Register the MCP server in Codex with absolute paths:

```bash
codex mcp add dtrack \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

If Codex does not already inherit the Dependency-Track API key, pass it explicitly:

```bash
codex mcp add dtrack \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  --env DEPENDENCY_TRACK_API_KEY=your_api_key_here \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

Verify the registration:

```bash
codex mcp list
codex mcp get dtrack
```

If Codex reports that the server timed out during startup, add a longer startup timeout in `~/.codex/config.toml`:

```toml
[mcp_servers.dtrack]
startup_timeout_sec = 30
```

Equivalent full config:

```toml
[mcp_servers.dtrack]
command = "bun"
args = ["/absolute/path/to/this-repo/dist/mcp-server.js"]
startup_timeout_sec = 30

[mcp_servers.dtrack.env]
DTRACK_AGENT_CONFIG = "/absolute/path/to/this-repo/dtrack-agent.config.json"
```

## Claude Code Setup

Register the same stdio server in Claude Code:

```bash
claude mcp add dtrack \
  --scope local \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

If Claude Code does not already inherit the Dependency-Track API key, pass it explicitly:

```bash
claude mcp add dtrack \
  --scope local \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  --env DEPENDENCY_TRACK_API_KEY=your_api_key_here \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

Verify the registration:

```bash
claude mcp list
claude mcp get dtrack
```

Inside Claude Code, `/mcp` will show the connected server, its tools, and the `dtrack-triage` prompt.

## Debugging

If you need a simple server-side log, set `DTRACK_MCP_DEBUG_LOG` before starting the server:

```bash
export DTRACK_MCP_DEBUG_LOG=/tmp/dtrack-mcp.log
```

The server appends transport and request information to that file.

## Notes And Warnings

- MCP is optional. The toolkit works in CLI-only mode.
- The server is intentionally read-only.
- Use an absolute `DTRACK_AGENT_CONFIG` path for agent integrations.
- If the API key environment variable is not available in the agent process, tool calls will fail.
