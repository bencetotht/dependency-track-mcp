# Dependency-Track Fix Critical

Inspect and remediate critical Dependency-Track findings for a configured project.

## Usage

`/dtrack-fix-critical [project-name-or-uuid]`

## Workflow

1. If no explicit project is given, detect the matching Dependency-Track project from the current repo path first.
2. Inspect the latest critical findings first.
3. Start with a dry run:
   - MCP: `dtrack_detect_project` if needed
   - MCP: `dtrack_fix_critical` with `apply=false`
   - CLI: `bun run src/cli.ts fix critical`
   - or `bun run src/cli.ts fix critical <project-name-or-uuid>`
4. Summarize what will change.
5. If the user wants execution, run:
   - MCP: `dtrack_fix_critical` with `apply=true`
   - CLI: `bun run src/cli.ts fix critical --apply`
   - or `bun run src/cli.ts fix critical <project-name-or-uuid> --apply`
6. Report:
   - updated packages
   - validations
   - branch name
   - PR/MR URL
   - skipped findings
