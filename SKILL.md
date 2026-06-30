---
name: dependency-track-cli-triage
description: Use the dtrack CLI to query Dependency-Track projects and findings, validate whether reported vulnerabilities affect a codebase, fix applicable issues, test the changes, and prepare a pull request.
---

# Dependency-Track CLI Triage

Use this skill when you need to investigate security findings from Dependency-Track for a repository or project. The usual workflow is: find the Dependency-Track project, list findings, validate which findings are applicable, fix the real issues, verify the fix, and prepare a PR.

## Preconditions

- Use the local `dtrack` CLI from this repository, or run it with `cargo run --`.
- Prefer JSON output for agent work: `--output json`.
- Configure the instance with `DTRACK_URL` and `DTRACK_API_KEY`, or pass `--url` and `--api-key`.
- Never print or commit API keys.

If the binary is not built yet:

```bash
cargo build
```

During development, replace `dtrack` with:

```bash
cargo run --quiet --
```

## Core Commands

Discover projects:

```bash
dtrack --output json projects ls --active --sort risk --limit 20
```

Inspect one project:

```bash
dtrack --output json projects get <project-name-or-uuid> --metrics --properties
```

Detect the project for the current Git repository:

```bash
dtrack --output json projects detect .
```

List findings:

```bash
dtrack --output json findings ls <project-name-or-uuid> --min-severity high --sort severity
```

List findings for the current repository if project metadata is configured:

```bash
dtrack --output json findings ls --repo . --min-severity high --sort severity
```

Summarize findings:

```bash
dtrack --output json findings summary <project-name-or-uuid>
```

Look up one vulnerability and affected projects:

```bash
dtrack --output json vuln get NVD CVE-YYYY-NNNN
dtrack --output json vuln ls NVD CVE-YYYY-NNNN --exclude-inactive
```

## Investigation Workflow

1. Identify the project.
   - First try `projects detect .` in the target repository.
   - If detection fails, use `projects ls` with likely name fragments.
   - If multiple projects match, prefer the active latest application project unless the user specified a version, container, or environment.

2. Pull the relevant findings.
   - Start with `--min-severity critical` or `high`.
   - Use `--limit` only for exploration; remove it before making final claims.
   - Include `--include-suppressed` only when auditing suppressed findings.
   - Use `--component`, `--vulnerability`, or `--state` to narrow follow-up queries.

3. Group findings by remediation path.
   - Same vulnerable package across multiple CVEs: usually one dependency update.
   - Same CVE across multiple package versions: check all manifests and lockfiles.
   - Container/base-image findings: inspect Dockerfiles, image tags, and OS package update path.
   - Transitive dependency findings: identify the parent package before changing direct dependencies.

4. Validate applicability before editing.
   - Locate the package in manifest and lock files.
   - Confirm the vulnerable version is actually used by the target build, image, or runtime.
   - Check whether the vulnerable API/path is reachable in this codebase when relevant.
   - Check existing suppressions or analysis state; do not undo an intentional suppression unless evidence shows it is wrong.
   - For false positives, collect concrete evidence: unused dependency, non-reachable code path, dev-only scope, patched downstream package, or non-affected configuration.

5. Fix applicable issues.
   - Prefer the narrowest safe version upgrade that reaches a fixed version.
   - Keep manifest and lockfile changes consistent.
   - For ecosystems with lockfiles, use the package manager instead of hand-editing lockfiles.
   - Avoid unrelated dependency churn.
   - If a major upgrade is required, inspect release notes or migration notes before changing code.

6. Verify locally.
   - Run dependency install/update commands.
   - Run the repository's relevant tests, lint, typecheck, build, or image build.
   - If the project has security scan tooling, run the narrowest scan that confirms the finding is gone.
   - Re-query `dtrack findings ls` when the Dependency-Track instance has refreshed data; if it has not refreshed yet, say that local remediation is verified but server-side evidence is pending.

7. Prepare the PR.
   - Summarize which findings were addressed, including CVE/GHSA IDs, package names, old versions, and new versions.
   - Mention findings intentionally not fixed and why.
   - Include verification commands and results.
   - Do not include secrets, API keys, or irrelevant raw Dependency-Track payloads.

## Output Interpretation

Important fields in `findings ls` JSON:

- `project`: Dependency-Track project metadata.
- `findings[].component.name`: affected component/package.
- `findings[].component.version`: currently detected version.
- `findings[].component.latestVersion`: Dependency-Track's latest known version, not always the fixed version.
- `findings[].component.purl`: package URL; use it to identify ecosystem and exact package.
- `findings[].vulnerability.vulnId`: vulnerability ID.
- `findings[].vulnerability.source`: source such as `NVD`, `GITHUB`, or `OSV`.
- `findings[].vulnerability.severity`: Dependency-Track severity.
- `findings[].vulnerability.cvssV3BaseScore`: prioritization signal, not proof of exploitability.
- `findings[].vulnerability.epssScore`: exploitation-likelihood signal, when available.
- `findings[].analysis.state`: triage state, if set.
- `findings[].analysis.isSuppressed`: whether Dependency-Track suppresses the finding.

## Decision Rules

- Treat Dependency-Track as the alert source, not the sole source of truth.
- Do not claim a vulnerability is fixed just because a package was changed; verify the resolved version.
- Do not claim a finding is not applicable without codebase-specific evidence.
- Do not broaden the PR into dependency modernization unless required for the security fix.
- Prefer small, reviewable PRs grouped by project and remediation path.
- If the fix requires a breaking upgrade, present the risk and the minimum required follow-up work.
- If Dependency-Track data is stale, separate local verification from server-side verification.

## Useful Patterns

Find highest-risk active projects:

```bash
dtrack --output json projects ls --active --sort risk --limit 10
```

Get all high-and-above findings for one project:

```bash
dtrack --output json findings ls <project> --min-severity high --sort severity
```

Focus on one package:

```bash
dtrack --output json findings ls <project> --component <package-name> --min-severity low
```

Focus on one CVE or advisory:

```bash
dtrack --output json findings ls <project> --vulnerability CVE-YYYY-NNNN --min-severity unassigned
```

Check if a vulnerability affects other projects:

```bash
dtrack --output json vuln ls NVD CVE-YYYY-NNNN --exclude-inactive
```

## When to Stop and Ask

Ask the user before proceeding when:

- multiple Dependency-Track projects plausibly match and choosing one could change production dependencies;
- the only fix is a major upgrade with likely functional changes;
- the finding appears false positive but suppressing or documenting it is outside the requested scope;
- required registry, package manager, or test credentials are unavailable;
- local verification cannot be run and the change would be risky without it.
