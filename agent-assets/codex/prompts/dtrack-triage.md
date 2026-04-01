# Dependency-Track Triage

Inspect the latest Dependency-Track findings for the current repo, or for an explicit Dependency-Track project name or UUID.

Use the local MCP server if available. Otherwise use the local CLI:

- `bun run src/cli.ts projects detect`
- `bun run src/cli.ts projects inspect`
- `bun run src/cli.ts findings list`
- or use the explicit project identifier if one was provided

Return the latest project version, critical fixable findings, skipped findings, and the next recommended remediation action.
