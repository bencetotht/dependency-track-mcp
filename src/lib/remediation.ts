import { join } from "node:path";
import { findProjectMapping } from "./config.ts";
import { DependencyTrackClient } from "./dtrack.ts";
import { commitAndPush, ensureRepository, hasChanges, prepareBranch } from "./git.ts";
import { buildTestCommand, buildUpgradeCommand, detectPackageManager, findDependencyType, readPackageJson } from "./package-managers.ts";
import { openPullRequestOrMr } from "./providers.ts";
import type { DtrackFinding, RemediationOutcome, Severity, ToolkitConfig, ValidationStep } from "./types.ts";
import { execCommand, fail, severityRank, slugify } from "./utils.ts";

function normalizeSeverity(input?: string): Severity {
  return ((input || "UNASSIGNED").toUpperCase()) as Severity;
}

function inferFixVersion(finding: DtrackFinding): string | undefined {
  if (finding.component.latestVersion) {
    return finding.component.latestVersion;
  }
  const recommendation = finding.vulnerability.recommendation || "";
  const match = recommendation.match(/version\s+([0-9A-Za-z._-]+)\s+or\s+later/i);
  return match?.[1];
}

function findingIdentifier(finding: DtrackFinding): string {
  return finding.vulnerability.vulnId || finding.vulnerability.uuid || finding.matrix;
}

function groupFixableFindings(findings: DtrackFinding[], threshold: Severity, requestedId?: string): DtrackFinding[] {
  return findings.filter((finding) => {
    if (severityRank(normalizeSeverity(finding.vulnerability.severity)) < severityRank(threshold)) {
      return false;
    }
    if (finding.analysis?.isSuppressed) {
      return false;
    }
    if (!inferFixVersion(finding)) {
      return false;
    }
    if (!requestedId) {
      return true;
    }
    const identifier = findingIdentifier(finding);
    return identifier === requestedId || finding.matrix === requestedId;
  });
}

export async function listProjects(config: ToolkitConfig): Promise<unknown[]> {
  const client = new DependencyTrackClient(config);
  return client.listProjects();
}

export async function inspectProject(config: ToolkitConfig, projectKey: string): Promise<unknown> {
  const mapping = findProjectMapping(config, projectKey);
  const client = new DependencyTrackClient(config);
  const resolved = await client.resolveProject(mapping);
  const findings = await client.getFindings(mapping, "LOW");
  return {
    mapping,
    project: resolved.project,
    totals: findings.findings.reduce<Record<string, number>>((acc, finding) => {
      const severity = normalizeSeverity(finding.vulnerability.severity);
      acc[severity] = (acc[severity] || 0) + 1;
      return acc;
    }, {}),
  };
}

export async function listFindings(config: ToolkitConfig, projectKey: string, severity: Severity = "CRITICAL"): Promise<unknown> {
  const mapping = findProjectMapping(config, projectKey);
  const client = new DependencyTrackClient(config);
  const { project, findings } = await client.getFindings(mapping, severity);
  return {
    project,
    findings: findings.map((finding) => ({
      id: findingIdentifier(finding),
      matrix: finding.matrix,
      package: finding.component.name,
      version: finding.component.version,
      latestVersion: finding.component.latestVersion,
      purl: finding.component.purl,
      severity: finding.vulnerability.severity,
      vulnerabilityId: finding.vulnerability.vulnId,
      title: finding.vulnerability.title,
      recommendation: finding.vulnerability.recommendation,
    })),
  };
}

async function maybeRunValidationStep(
  repoPath: string,
  stepName: string,
  argv: string[] | null,
): Promise<ValidationStep> {
  if (!argv) {
    return {
      name: stepName,
      status: "skipped",
      detail: "No command configured",
    };
  }
  try {
    await execCommand(argv, { cwd: repoPath });
    return {
      name: stepName,
      status: "passed",
      detail: argv.join(" "),
    };
  } catch (error) {
    return {
      name: stepName,
      status: "failed",
      detail: error instanceof Error ? error.message : String(error),
    };
  }
}

async function maybeRunTrivy(repoPath: string): Promise<ValidationStep> {
  try {
    await execCommand(["which", "trivy"], { cwd: repoPath, quiet: true });
  } catch {
    return {
      name: "trivy",
      status: "skipped",
      detail: "trivy is not installed locally",
    };
  }
  try {
    await execCommand(["trivy", "fs", "--no-progress", "--severity", "CRITICAL", "."], { cwd: repoPath });
    return {
      name: "trivy",
      status: "passed",
      detail: "trivy fs --severity CRITICAL .",
    };
  } catch (error) {
    return {
      name: "trivy",
      status: "failed",
      detail: error instanceof Error ? error.message : String(error),
    };
  }
}

export async function remediateCritical(
  config: ToolkitConfig,
  projectKey: string,
  apply: boolean,
  findingId?: string,
): Promise<RemediationOutcome> {
  const mapping = findProjectMapping(config, projectKey);
  const client = new DependencyTrackClient(config);
  const { project, findings } = await client.getFindings(mapping, config.remediation?.severity || "CRITICAL");
  const candidateFindings = groupFixableFindings(findings, config.remediation?.severity || "CRITICAL", findingId)
    .slice(0, config.remediation?.maxFindingsPerRun || 5);

  const repoPath = await ensureRepository(config, mapping);
  const packageManager = await detectPackageManager(repoPath);
  if (!packageManager) {
    fail(`No supported Node package manager lockfile found in ${repoPath}.`);
  }
  const packageJson = await readPackageJson(repoPath);

  const updatedPackages: RemediationOutcome["updatedPackages"] = [];
  const skippedFindings: RemediationOutcome["skippedFindings"] = [];
  const seenPackages = new Set<string>();

  for (const finding of candidateFindings) {
    const packageName = finding.component.name;
    const version = inferFixVersion(finding);
    const id = findingIdentifier(finding);
    if (!version) {
      skippedFindings.push({ findingId: id, reason: "No fix version available from Dependency-Track finding data." });
      continue;
    }
    if (seenPackages.has(packageName)) {
      continue;
    }
    const dependencyType = findDependencyType(packageJson, packageName);
    if (!dependencyType) {
      skippedFindings.push({ findingId: id, reason: `Package '${packageName}' is not a direct dependency in package.json.` });
      continue;
    }
    seenPackages.add(packageName);
    updatedPackages.push({
      name: packageName,
      fromVersion: finding.component.version,
      toVersion: version,
      dependencyType,
      vulnerabilityIds: [id],
    });
  }

  const validations: ValidationStep[] = [];
  const branchName = `${config.remediation?.branchPrefix || "dtrack/fix"}/${slugify(project.name)}-${slugify(project.version).slice(0, 12)}`;

  if (!apply) {
    return {
      mode: "dry-run",
      projectKey: mapping.key,
      projectUuid: project.uuid,
      projectVersion: project.version,
      repoPath,
      branchName,
      updatedPackages,
      skippedFindings,
      validations,
    };
  }

  if (updatedPackages.length === 0) {
    return {
      mode: "apply",
      projectKey: mapping.key,
      projectUuid: project.uuid,
      projectVersion: project.version,
      repoPath,
      updatedPackages,
      skippedFindings,
      validations,
    };
  }

  await prepareBranch(repoPath, mapping.defaultBranch, branchName);

  for (const item of updatedPackages) {
    await execCommand(buildUpgradeCommand(packageManager, item.name, item.toVersion, item.dependencyType), { cwd: repoPath });
  }

  validations.push(await maybeRunValidationStep(repoPath, "tests", buildTestCommand(packageManager, packageJson)));
  validations.push(await maybeRunTrivy(repoPath));

  if (!(await hasChanges(repoPath))) {
    skippedFindings.push({ findingId: "none", reason: "Package manager completed but no file changes were produced." });
    return {
      mode: "apply",
      projectKey: mapping.key,
      projectUuid: project.uuid,
      projectVersion: project.version,
      repoPath,
      branchName,
      updatedPackages,
      skippedFindings,
      validations,
    };
  }

  await commitAndPush(
    config,
    mapping,
    repoPath,
    branchName,
    "fix(deps): remediate critical dependency-track findings",
  );

  const prBody = [
    `Automated remediation for Dependency-Track project \`${project.name}\` version \`${project.version}\`.`,
    "",
    "Updated packages:",
    ...updatedPackages.map((item) => `- \`${item.name}\`: \`${item.fromVersion}\` -> \`${item.toVersion}\``),
    "",
    "Validation:",
    ...validations.map((item) => `- ${item.name}: ${item.status} (${item.detail})`),
  ].join("\n");

  const prUrl = await openPullRequestOrMr(
    config,
    mapping,
    branchName,
    `fix(deps): remediate critical findings for ${project.name}`,
    prBody,
  );

  return {
    mode: "apply",
    projectKey: mapping.key,
    projectUuid: project.uuid,
    projectVersion: project.version,
    repoPath,
    branchName,
    prUrl: mapping.provider === "github" ? prUrl : undefined,
    mrUrl: mapping.provider === "gitlab" ? prUrl : undefined,
    updatedPackages,
    skippedFindings,
    validations,
  };
}

