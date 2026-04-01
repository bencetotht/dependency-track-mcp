# Dependency-Track Fix Critical

Inspect and remediate critical Dependency-Track findings for a configured project.

## Usage

`/dtrack-fix-critical <projectKey>`

## Workflow

1. Inspect the latest critical findings first.
2. Start with a dry run:
   - MCP: `dtrack_fix_critical` with `apply=false`
   - CLI: `bun run src/cli.ts fix critical <projectKey>`
3. Summarize what will change.
4. If the user wants execution, run:
   - MCP: `dtrack_fix_critical` with `apply=true`
   - CLI: `bun run src/cli.ts fix critical <projectKey> --apply`
5. Report:
   - updated packages
   - validations
   - branch name
   - PR/MR URL
   - skipped findings

