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

## Matching Behavior

Repo-to-project detection uses the normalized git `origin` remote and matches it against:

- `projects[].cloneUrl` from local config
- Dependency-Track project properties such as `automation.cloneUrl`, `automation.repositoryUrl`, or `repo.cloneUrl`
- Dependency-Track external references when available

This keeps the workflow centered on "current repo to current Dependency-Track project" rather than manual project selection.
