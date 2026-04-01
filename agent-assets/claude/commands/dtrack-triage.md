# Dependency-Track Triage

Inspect the latest Dependency-Track findings for a configured project.

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
   - fixable critical findings first
   - skipped findings with reasons
   - next recommended remediation action
