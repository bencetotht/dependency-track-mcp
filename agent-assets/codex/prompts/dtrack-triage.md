# Dependency-Track Triage

Inspect the latest Dependency-Track findings for a configured project key.

Use the local MCP server if available. Otherwise use the local CLI:

- `bun run src/cli.ts projects inspect <projectKey>`
- `bun run src/cli.ts findings list <projectKey> CRITICAL`

Return the latest project version, critical fixable findings, skipped findings, and the next recommended remediation action.

