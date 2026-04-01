# Dependency-Track Triage

Inspect the latest Dependency-Track findings for the current repo or an explicit Dependency-Track project.

## Usage

`/dtrack-triage [project-name-or-uuid]`

## Workflow

1. If no explicit project is given, detect the matching Dependency-Track project from the current repo path first.
2. In Claude Code, prefer the connected local MCP server if available:
   - `dtrack_detect_project`
   - `dtrack_get_project`
   - `dtrack_get_findings`
   - or use the connected MCP prompt directly with `/mcp__dtrack__dtrack_triage`
3. Otherwise use the local CLI from this toolkit repository checkout:
   - `bun run src/cli.ts projects detect`
   - `bun run src/cli.ts projects inspect`
   - `bun run src/cli.ts findings list`
   - or use the explicit project identifier when provided
4. If the current working directory is not this toolkit repository, explain that the CLI fallback requires running from the repository that contains `src/cli.ts`.
5. Return:
   - current Dependency-Track project/version
   - current critical findings first
   - vulnerability identifiers, titles, descriptions, aliases, and recommendations
   - enough context for the agent to do the fix work separately

The toolkit is query-only. It does not edit code or create PRs/MRs.
