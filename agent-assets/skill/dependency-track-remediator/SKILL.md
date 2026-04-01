---
name: dependency-track-remediator
description: Use when an agent needs to inspect Dependency-Track findings for a configured project, prioritize critical Node dependency issues, run the local dtrack-agent toolkit, and prepare or open a remediation PR or MR. Trigger for Dependency-Track triage, critical dependency remediation, and project-specific vulnerability review using the local CLI or MCP server.
---

# Dependency-Track Remediator

Use this skill when the task is to inspect Dependency-Track data and remediate critical Node ecosystem findings.

## Workflow

1. Resolve the project key from `dtrack-agent.config.json`.
2. Prefer the MCP tools if the MCP server is connected.
3. Otherwise use the CLI:
   - `bun run src/cli.ts projects inspect <project>`
   - `bun run src/cli.ts findings list <project> CRITICAL`
   - `bun run src/cli.ts fix critical <project>` for dry run
   - `bun run src/cli.ts fix critical <project> --apply` to create the branch and PR/MR
4. Summarize:
   - latest Dependency-Track project version
   - critical fixable findings
   - skipped findings and reasons
   - validation outcomes
   - PR/MR URL when created

## Guardrails

- Do not assume the Dependency-Track project name equals the repo slug; use the configured mapping.
- Only auto-remediate supported Node package managers in v1: `bun`, `npm`, `pnpm`, `yarn`.
- Treat dry-run as the default safe mode unless the user clearly asked to apply the fix.
- Report skipped findings explicitly when the package is transitive-only or has no fix version.
- Never merge automatically.

