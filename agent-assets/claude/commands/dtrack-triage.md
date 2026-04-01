# Dependency-Track Triage

Inspect the latest Dependency-Track findings for a configured project.

## Usage

`/dtrack-triage <project-name-or-uuid>`

## Workflow

1. Use the local MCP server if available:
   - `dtrack_get_project`
   - `dtrack_get_findings`
2. Otherwise use the local CLI:
   - `bun run src/cli.ts projects inspect <project-name-or-uuid>`
   - `bun run src/cli.ts findings list <project-name-or-uuid> CRITICAL`
3. Return:
   - current Dependency-Track project/version
   - fixable critical findings first
   - skipped findings with reasons
   - next recommended remediation action
