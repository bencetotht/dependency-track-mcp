# Dependency-Track Agent Toolkit

This repository provides a small Bun/TypeScript toolkit that lets an agent identify the current repository in Dependency-Track and retrieve the latest findings for that project.

The toolkit is intentionally query-only. It does not modify source code, upgrade dependencies, create commits, or open PRs/MRs. Those steps belong to the agent working in the checked-out target repository.

## Intended Workflow

The target workflow is:

1. Open Codex inside project `xyz`.
2. Ask Codex to look up that project in Dependency-Track.
3. Codex uses this toolkit to:
   - detect the matching Dependency-Track project from the current repo path
   - inspect the latest project state
   - retrieve the latest critical findings
4. Codex then uses the returned vulnerability details to:
   - research the CVE or advisory
   - change code or dependency manifests in project `xyz`
   - run tests
   - commit the fix
   - create the PR or MR with a detailed description

This repository only covers step 3.

## What This Repo Contains

- `src/cli.ts`: read-only CLI for project lookup and findings retrieval
- `src/mcp-server.ts`: stdio MCP server exposing the same read-only operations
- `src/lib/projects.ts`: project detection and findings query logic
- `agent-assets/skill/dependency-track-query`: repo-local Codex skill
- `agent-assets/codex/prompts`: Codex prompt assets
- `agent-assets/claude/commands`: Claude command assets
- `dtrack-agent.config.example.json`: minimal config example
- `docker/compose.yaml`: optional local Dependency-Track stack for development/testing

## Configuration

1. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
2. Set `dependencyTrack.url`.
3. Export the Dependency-Track API key env var referenced by `dependencyTrack.apiKeyEnv`.
4. Optionally add `projects[]` overrides when you want to pin a project UUID or provide a clone URL mapping.

Minimal example:

```json
{
  "dependencyTrack": {
    "url": "https://dep.example.com",
    "apiKeyEnv": "DEPENDENCY_TRACK_API_KEY"
  },
  "projects": [
    {
      "key": "simplemailer",
      "dependencyTrackName": "simplemailer",
      "projectUuid": "00000000-0000-0000-0000-000000000000",
      "cloneUrl": "https://github.com/example/simplemailer.git"
    }
  ]
}
```

## Project Matching

From inside a checked-out repository, the toolkit:

1. finds the git root
2. reads the `origin` remote URL
3. normalizes that URL
4. searches Dependency-Track for the current project entry

It matches using, in order of availability:

- local config override `projects[].cloneUrl`
- Dependency-Track project properties such as `automation.cloneUrl`, `automation.repositoryUrl`, or `repo.cloneUrl`
- relevant Dependency-Track external references

This avoids guessing by repo name alone.

## CLI

Examples:

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects detect
bun run src/cli.ts projects inspect
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts findings list
bun run src/cli.ts findings list simplemailer CRITICAL
bun run src/cli.ts install-agent-assets --force
bun run src/cli.ts install-agent-assets --codex
bun run src/cli.ts install-agent-assets --claude
```

Available commands:

- `projects list [query]`
- `projects detect [repo-path]`
- `projects inspect [project-name-or-uuid]`
- `findings list [project-name-or-uuid] [severity]`
- `install-agent-assets [--force] [--codex] [--claude]`

Removed in `0.2.0`:

- `fix critical`
- `fix finding`
- `--apply`

## MCP Server

Run it over stdio:

```bash
bun run src/mcp-server.ts
```

Exposed tools:

- `dtrack_list_projects`
- `dtrack_detect_project`
- `dtrack_get_project`
- `dtrack_get_findings`

Exposed prompt:

- `dtrack-triage`

Recommended MCP flow for an agent:

1. `dtrack_detect_project` with the current repo path
2. `dtrack_get_project` for the resolved project
3. `dtrack_get_findings` with `severity=CRITICAL`

## Agent Assets

Install repo-local agent assets into your local Codex and Claude directories:

```bash
bun run src/cli.ts install-agent-assets --force
```

Install only Codex assets:

```bash
bun run src/cli.ts install-agent-assets --codex
```

Install only Claude assets:

```bash
bun run src/cli.ts install-agent-assets --claude
```

This copies:

- Claude commands to `~/.claude/commands`
- Codex prompts to `~/.codex/prompts`
- Codex skill to `~/.codex/skills/dependency-track-query`

## Output Shape

The findings output is designed to be directly useful to an agent performing remediation elsewhere. Each finding includes:

- package name and current version
- latest known component version from Dependency-Track when available
- PURL
- severity
- vulnerability source and identifier
- title and description
- aliases
- recommendation text

## Local Development

Build:

```bash
bun run build
```

Test:

```bash
bun test
```
