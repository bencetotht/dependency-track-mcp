# Dependency-Track Triage

Look up the current repo in Dependency-Track, inspect the latest critical findings, and return the vulnerability details needed for remediation.

Use the local MCP server if available. Otherwise use the local CLI:

- `bun run src/cli.ts projects detect`
- `bun run src/cli.ts projects inspect`
- `bun run src/cli.ts findings list`
- or use the explicit project identifier if one was provided

Return the resolved project, latest project/version context, and the current critical findings with package names, versions, identifiers, descriptions, aliases, and recommendations.

The toolkit only provides Dependency-Track lookup data. Use normal agent capabilities for code changes, testing, commits, and PR/MR creation.
