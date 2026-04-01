# Dependency-Track Fix Critical

Inspect and remediate critical Dependency-Track findings for the current repo, or for an explicit Dependency-Track project name or UUID.

Start with a dry run:

- MCP: `dtrack_detect_project` if needed
- MCP: `dtrack_fix_critical` with `apply=false`
- CLI: `bun run src/cli.ts fix critical`
- or `bun run src/cli.ts fix critical <project-name-or-uuid>`

Only apply changes when explicitly requested:

- MCP: `dtrack_fix_critical` with `apply=true`
- CLI: `bun run src/cli.ts fix critical --apply`
- or `bun run src/cli.ts fix critical <project-name-or-uuid> --apply`

Report updated packages, validation results, branch name, PR/MR URL, and skipped findings.
