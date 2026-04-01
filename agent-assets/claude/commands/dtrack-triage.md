# Dependency-Track Triage

Inspect the latest Dependency-Track findings for the current repo or an explicit Dependency-Track project.

## Usage

`/dtrack-triage [project-name-or-uuid]`

## Workflow

1. If no explicit project is given, detect the matching Dependency-Track project from the current repo path first.
2. Use the local MCP server if available:
   - `dtrack_detect_project`
   - `dtrack_get_project`
   - `dtrack_get_findings`
3. Otherwise use the local CLI:
   - `bun run src/cli.ts projects detect`
   - `bun run src/cli.ts projects inspect`
   - `bun run src/cli.ts findings list`
   - or use the explicit project identifier when provided
4. Return:
   - current Dependency-Track project/version
   - current critical findings first
   - vulnerability identifiers, titles, descriptions, aliases, and recommendations
   - enough context for the agent to do the fix work separately

The toolkit is query-only. It does not edit code or create PRs/MRs.
