# Dependency-Track Triage

Inspect the latest Dependency-Track findings for a configured project.

## Usage

`/dtrack-triage <projectKey>`

## Workflow

1. Use the local MCP server if available:
   - `dtrack_get_project`
   - `dtrack_get_findings`
2. Otherwise use the local CLI:
   - `bun run src/cli.ts projects inspect <projectKey>`
   - `bun run src/cli.ts findings list <projectKey> CRITICAL`
3. Return:
   - current Dependency-Track project/version
   - fixable critical findings first
   - skipped findings with reasons
   - next recommended remediation action

