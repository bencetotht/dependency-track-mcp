# Dependency-Track Triage

Inspect the latest Dependency-Track findings for a Dependency-Track project name or UUID.

Use the local MCP server if available. Otherwise use the local CLI:

- `bun run src/cli.ts projects inspect <project-name-or-uuid>`
- `bun run src/cli.ts findings list <project-name-or-uuid> CRITICAL`

Return the latest project version, critical fixable findings, skipped findings, and the next recommended remediation action.
