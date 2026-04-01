# Dependency-Track Agent Toolkit

This repository now includes a Bun/TypeScript toolkit for agent-driven Dependency-Track triage and remediation.

## Files

- `dtrack-agent.config.example.json`: example project/provider mapping
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
   - project mappings
3. Export required env vars:
   - `DEPENDENCY_TRACK_API_KEY`
   - `GITHUB_TOKEN` and/or `GITLAB_TOKEN`
4. Build the toolkit:

```bash
bun run build
```

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
