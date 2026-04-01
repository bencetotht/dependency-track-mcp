# Dependency-Track Agent Toolkit

This repository now includes a Bun/TypeScript toolkit for agent-driven Dependency-Track triage and remediation.

## Files

- `dtrack-agent.config.example.json`: example config with optional local project overrides
- `src/cli.ts`: CLI entrypoint
- `src/mcp-server.ts`: stdio MCP server
- `agent-assets/skill/dependency-track-remediator`: repo-local Codex skill
- `agent-assets/claude/commands`: Claude slash-command assets
- `agent-assets/codex/prompts`: Codex prompt/slash-command assets

## Setup

1. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
2. Fill in:
   - `dependencyTrack.url`
   - `workspaceRoot`
3. Export required env vars:
   - `DEPENDENCY_TRACK_API_KEY`
   - `GITHUB_TOKEN` and/or `GITLAB_TOKEN`
4. Build the toolkit:

```bash
bun run build
```

## Project Discovery

The toolkit now queries Dependency-Track live. You can pass a Dependency-Track project name or UUID directly to the CLI and MCP tools.

Examples:

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts findings list simplemailer CRITICAL
```

You no longer need to maintain a full local project registry just to inspect or triage projects.

## Remediation Metadata

To remediate a project without a local override, store repo metadata directly on the Dependency-Track project using project properties.

Recommended properties:

- `automation.provider`: `github` or `gitlab`
- `automation.cloneUrl`: clone URL for the repo
- `automation.defaultBranch`: usually `main`
- `automation.localPath`: optional local checkout path override

The tool reads these via Dependency-Track's project property API.

Local `projects` entries in `dtrack-agent.config.json` are now optional and act only as overrides when you want to pin or override project metadata locally.

## CLI

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts findings list simplemailer CRITICAL
bun run src/cli.ts fix critical simplemailer
bun run src/cli.ts fix critical simplemailer --apply
```

## MCP Server

Run over stdio:

```bash
bun run src/mcp-server.ts
```

Exposed tools:

- `dtrack_list_projects`
- `dtrack_get_project`
- `dtrack_get_findings`
- `dtrack_fix_critical`
- `dtrack_fix_finding`

The MCP tools accept a Dependency-Track project name or UUID directly.

## Agent Assets

Install repo-local agent assets into your local Codex and Claude directories:

```bash
bun run src/cli.ts install-agent-assets --force
```

This copies:

- Claude commands to `~/.claude/commands`
- Codex prompts to `~/.codex/prompts`
- Codex skill to `~/.codex/skills/dependency-track-remediator`

## Scope

The remediation engine currently supports Node ecosystem repositories only:

- `bun`
- `npm`
- `pnpm`
- `yarn`

It only upgrades direct dependencies with fix versions discoverable from Dependency-Track finding data.
