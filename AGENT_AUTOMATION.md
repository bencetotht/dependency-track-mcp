# Dependency-Track Agent Workflow

This toolkit exists to give an agent Dependency-Track context for the repository it is currently working in.

## Responsibility Boundary

The toolkit does:

- resolve the current git repository to the matching Dependency-Track project
- inspect the latest project state
- list current findings, typically `CRITICAL`

The toolkit does not:

- research the vulnerability outside Dependency-Track
- modify dependencies or source code
- run package manager upgrades
- create commits
- open PRs or MRs

Those actions belong to the agent operating in the checked-out target repository.

## Access Modes

There are two supported ways for an agent to query this toolkit:

- CLI mode: the agent runs local commands such as `bun run src/cli.ts findings list`
- MCP mode: the agent talks to the stdio MCP server in `src/mcp-server.ts`

MCP is optional. The toolkit works without MCP.

## Default Agent Behavior

By default, the installed agent assets prefer MCP when an MCP connection to this server is available.

If the MCP server is not configured or not connected, the agent falls back to the local CLI.

So when you ask an installed agent to investigate a vulnerability, the expected order is:

1. use MCP tools if the `dtrack-agent-mcp` server is connected
2. otherwise run the local CLI commands directly

This is the current default in:

- `agent-assets/skill/dependency-track-query/SKILL.md`
- `agent-assets/codex/prompts/dtrack-triage.md`
- `agent-assets/claude/commands/dtrack-triage.md`

## Intended Agent Flow

From inside project `xyz`, the agent should:

1. detect the Dependency-Track project for the current repo
2. inspect the project and retrieve the latest critical findings
3. use the returned vulnerability details to plan and implement the fix in project `xyz`
4. run the repo's tests and validation
5. create the commit and PR/MR with a detailed explanation of:
   - the vulnerability or CVE
   - the affected dependency
   - the chosen fix
   - validation performed

Only steps 1 and 2 are handled by this toolkit.

## Setup

1. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
2. Set `dependencyTrack.url`.
3. Export `DEPENDENCY_TRACK_API_KEY` or whichever env var you configured as `dependencyTrack.apiKeyEnv`.
4. Build the toolkit:

```bash
bun run build
```

## Installing Agent Assets

The repository includes small agent-specific assets so Codex or Claude can discover the Dependency-Track workflow without you rewriting the instructions each time.

Install both Codex and Claude assets:

```bash
bun run src/cli.ts install-agent-assets
```

Install only Codex assets:

```bash
bun run src/cli.ts install-agent-assets --codex
```

Install only Claude assets:

```bash
bun run src/cli.ts install-agent-assets --claude
```

Replace already-installed assets:

```bash
bun run src/cli.ts install-agent-assets --force
```

You can combine `--force` with `--codex` or `--claude`.

Examples:

```bash
bun run src/cli.ts install-agent-assets --codex --force
bun run src/cli.ts install-agent-assets --claude --force
```

### Default Behavior

- If you pass neither `--codex` nor `--claude`, the installer copies both sets of assets.
- If you pass `--codex`, only Codex assets are installed.
- If you pass `--claude`, only Claude assets are installed.
- If you pass both flags, both sets are installed explicitly.

### What Gets Installed

Codex installation copies:

- `agent-assets/codex/prompts/dtrack-triage.md` to `~/.codex/prompts`
- `agent-assets/skill/dependency-track-query/` to `~/.codex/skills/dependency-track-query`

Claude installation copies:

- `agent-assets/claude/commands/dtrack-triage.md` to `~/.claude/commands`

### Why These Files Exist

The installed files are intentionally small and opinionated. They give the agent a repeatable Dependency-Track lookup workflow without turning this repository into the component that performs the code remediation.

### File-by-File Purpose

`agent-assets/codex/prompts/dtrack-triage.md`

- Provides a reusable Codex prompt for looking up the current repo in Dependency-Track.
- Tells Codex which local CLI/MCP commands to use.
- Frames the toolkit as a source of vulnerability context, not the component that edits code or opens PRs.

`agent-assets/skill/dependency-track-query/SKILL.md`

- Registers a Codex skill dedicated to Dependency-Track project lookup and findings retrieval.
- Defines the expected workflow, guardrails, and handoff to the normal coding workflow after the lookup is complete.
- Helps Codex consistently detect the current repo, fetch findings, and summarize the relevant CVE details.

`agent-assets/skill/dependency-track-query/agents/openai.yaml`

- Supplies the Codex skill metadata and default prompt text.
- Makes the skill appear with a useful name and short description in the local Codex skill system.
- Provides a default instruction that points the agent toward “query first, remediate separately.”

`agent-assets/claude/commands/dtrack-triage.md`

- Adds a Claude slash-command style instruction for the same Dependency-Track lookup flow.
- Keeps Claude aligned with the same responsibility boundary as the Codex assets.

### Why Install Them Locally

- They make the Dependency-Track workflow discoverable inside the agent you actually use.
- They reduce repeated prompt-writing and keep the lookup flow consistent across repositories.
- They keep the logic lightweight: the assets teach the agent how to use the toolkit, while the agent still performs the actual remediation in the checked-out project.

### What The Installed Assets Prefer By Default

- Codex skill and prompt: prefer MCP if connected, otherwise use the CLI
- Claude command: prefer MCP if connected, otherwise use the CLI

Installing the assets does not automatically configure an MCP server connection. The assets only define the preferred behavior once the server is available to the agent.

## Query Examples

From inside a checked-out repository:

```bash
bun run src/cli.ts projects detect
bun run src/cli.ts projects inspect
bun run src/cli.ts findings list
```

For an explicit Dependency-Track project:

```bash
bun run src/cli.ts projects inspect simplemailer
bun run src/cli.ts findings list simplemailer CRITICAL
```

## CLI Setup

CLI mode is the simplest option and requires no MCP integration.

1. Build the toolkit:

```bash
bun run build
```

2. Make sure `dtrack-agent.config.json` exists and `DEPENDENCY_TRACK_API_KEY` is exported.
3. From the repo you want to investigate, run commands such as:

```bash
bun run src/cli.ts projects detect
bun run src/cli.ts projects inspect
bun run src/cli.ts findings list
```

Use CLI mode when:

- you want the simplest setup
- your agent can run shell commands directly
- you do not want to configure MCP integration

## MCP Server Setup

MCP mode is also supported, but it is optional.

The MCP server is a stdio process started from this repository:

```bash
bun run build
bun dist/mcp-server.js
```

To use it with Codex, add the MCP server to Codex directly.

Replace the example paths below with the absolute path to your own checkout of this repository.

Recommended command:

```bash
codex mcp add dtrack \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

If Codex does not already inherit `DEPENDENCY_TRACK_API_KEY`, add it too:

```bash
codex mcp add dtrack \
  --env DTRACK_AGENT_CONFIG=/absolute/path/to/this-repo/dtrack-agent.config.json \
  --env DEPENDENCY_TRACK_API_KEY=your_api_key_here \
  -- bun /absolute/path/to/this-repo/dist/mcp-server.js
```

Verify the setup:

```bash
codex mcp list
codex mcp get dtrack
```

If Codex reports that startup timed out, set a larger startup timeout in `~/.codex/config.toml`:

```toml
[mcp_servers.dtrack]
startup_timeout_sec = 30
```

Equivalent `~/.codex/config.toml` entry:

```toml
[mcp_servers.dtrack]
command = "bun"
args = ["/absolute/path/to/this-repo/dist/mcp-server.js"]
startup_timeout_sec = 30

[mcp_servers.dtrack.env]
DTRACK_AGENT_CONFIG = "/absolute/path/to/this-repo/dtrack-agent.config.json"
```

Why this is the recommended setup:

- `dist/mcp-server.js` is the built local stdio MCP server
- Codex can launch local stdio MCP servers with `codex mcp add <name> -- <command...>`
- `DTRACK_AGENT_CONFIG` should point to an absolute path so the server can always find the correct config file
- the built file starts more predictably than pointing Codex at the TypeScript source file directly

If you prefer, you can also configure the same server directly in `~/.codex/config.toml`, but `codex mcp add` is the simplest path.

What the MCP server provides:

- `dtrack_list_projects`
- `dtrack_detect_project`
- `dtrack_get_project`
- `dtrack_get_findings`
- `dtrack-triage` prompt

What the MCP setup still depends on:

- the same `dtrack-agent.config.json` file used by the CLI
- the same Dependency-Track API key env var
- access to this repository on disk so the MCP server process can start

Use MCP mode when:

- your agent environment already supports MCP cleanly
- you want Dependency-Track access exposed as tools instead of shell commands
- you want the installed skill/prompt assets to use their preferred path

Use CLI mode instead when:

- MCP is not configured
- you want fewer moving parts
- you are testing the toolkit quickly from a terminal

## MCP Tools

Exposed tools:

- `dtrack_list_projects`
- `dtrack_detect_project`
- `dtrack_get_project`
- `dtrack_get_findings`

Recommended MCP usage:

1. Call `dtrack_detect_project` with the current repo path.
2. Call `dtrack_get_project` for additional project metadata.
3. Call `dtrack_get_findings` with `severity=CRITICAL`.

## Making Codex Prefer This MCP Server

The installed assets already prefer MCP first and CLI second.

If you want to reinforce that preference at the repository level, add this to your local `AGENTS.md`:

```md
Always use the `dtrack` MCP server for Dependency-Track lookups if it is available. If it is not available, fall back to the local `dtrack-agent` CLI.
```

That makes the intended behavior explicit when you ask Codex to investigate a vulnerability from inside a checked-out project.

## Matching Behavior

Repo-to-project detection uses the normalized git `origin` remote and matches it against:

- `projects[].cloneUrl` from local config
- Dependency-Track project properties such as `automation.cloneUrl`, `automation.repositoryUrl`, or `repo.cloneUrl`
- Dependency-Track external references when available

This keeps the workflow centered on "current repo to current Dependency-Track project" rather than manual project selection.
