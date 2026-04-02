---
name: dependency-track-query
description: Use when an agent needs to identify the current repository in Dependency-Track and retrieve the latest findings, especially critical vulnerabilities, before handling remediation in the checked-out project itself.
---

# Dependency-Track Query

Use this skill when the task is to fetch Dependency-Track project and vulnerability data for the current repository or for an explicit Dependency-Track project.

## Workflow

1. If you are already inside the target repository, detect the matching Dependency-Track project from the current repo first.
2. If needed, fall back to an explicit Dependency-Track project name or UUID.
3. Prefer the MCP tools if the MCP server is connected.
4. Otherwise use the CLI:
   - `bun run src/cli.ts projects detect`
   - `bun run src/cli.ts projects inspect`
   - `bun run src/cli.ts findings list`
5. For explicit project targeting:
   - `bun run src/cli.ts projects inspect <project-name-or-uuid>`
   - `bun run src/cli.ts findings list <project-name-or-uuid> CRITICAL`
6. Summarize:
   - detected project match or explicit project target
   - latest Dependency-Track project version
   - current critical findings
   - CVE or GHSA identifiers, descriptions, aliases, and recommendations
   - the dependency/package versions involved

## Guardrails

- Do not assume the Dependency-Track project name equals the repo slug.
- When operating from a checked-out repo, prefer matching by git remote URL instead of guessing by name.
- Prefer repository metadata stored on the Dependency-Track project:
  - `automation.cloneUrl`
  - `automation.repositoryUrl`
  - `repo.cloneUrl`
- Use local config project overrides only when Dependency-Track project metadata is missing or intentionally overridden.
- This toolkit is read-only. It does not upgrade packages, create branches, commit changes, or open PRs/MRs.
- After collecting the findings, use normal agent capabilities in the checked-out repo to research, fix, test, commit, and open the PR/MR.
