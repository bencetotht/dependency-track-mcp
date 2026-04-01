---
name: dependency-track-remediator
description: Use when an agent needs to inspect Dependency-Track findings for a configured project, prioritize critical Node dependency issues, run the local dtrack-agent toolkit, and prepare or open a remediation PR or MR. Trigger for Dependency-Track triage, critical dependency remediation, and project-specific vulnerability review using the local CLI or MCP server.
---

# Dependency-Track Remediator

Use this skill when the task is to inspect Dependency-Track data and remediate critical Node ecosystem findings.

## Workflow

1. If you are already inside the target repository, prefer current-repo detection first.
2. If needed, fall back to an explicit Dependency-Track project name or UUID.
3. Prefer the MCP tools if the MCP server is connected.
4. Otherwise use the CLI:
   - `bun run src/cli.ts projects detect`
   - `bun run src/cli.ts projects inspect`
   - `bun run src/cli.ts findings list`
   - `bun run src/cli.ts fix critical` for dry run
   - `bun run src/cli.ts fix critical --apply` to create the branch and PR/MR
5. For explicit project targeting:
   - `bun run src/cli.ts projects inspect <project-name-or-uuid>`
   - `bun run src/cli.ts findings list <project-name-or-uuid> CRITICAL`
   - `bun run src/cli.ts fix critical <project-name-or-uuid>`
6. Summarize:
   - detected project match or explicit project target
   - latest Dependency-Track project version
   - critical fixable findings
   - skipped findings and reasons
   - validation outcomes
   - PR/MR URL when created

## Guardrails

- Do not assume the Dependency-Track project name equals the repo slug.
- When operating from a checked-out repo, prefer matching by git remote URL instead of guessing by name.
- For remediation, prefer repo metadata stored on the Dependency-Track project as properties:
  - `automation.provider`
  - `automation.cloneUrl`
  - `automation.defaultBranch`
  - `automation.localPath`
- Use local config project overrides only when Dependency-Track project properties are missing or intentionally overridden.
- Only auto-remediate supported Node package managers in v1: `bun`, `npm`, `pnpm`, `yarn`.
- Treat dry-run as the default safe mode unless the user clearly asked to apply the fix.
- Report skipped findings explicitly when the package is transitive-only or has no fix version.
- Never merge automatically.
