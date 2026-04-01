import { findProjectMapping } from "./config.ts";
import { DependencyTrackClient } from "./dtrack.ts";
import { commitAndPush, ensureRepository, hasChanges, prepareBranch } from "./git.ts";
import { buildTestCommand, buildUpgradeCommand, detectPackageManager, findDependencyType, readPackageJson } from "./package-managers.ts";
import { openPullRequestOrMr } from "./providers.ts";
import type {
  DtrackFinding,
  DtrackProject,
  DtrackProjectProperty,
  ProjectMapping,
  RemediationOutcome,
  ResolvedProject,
  Severity,
  ToolkitConfig,
  ValidationStep,
} from "./types.ts";
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

function propertyLookup(properties: DtrackProjectProperty[]): Map<string, string> {
  const map = new Map<string, string>();
  for (const property of properties) {
    const groupName = (property.groupName || "").trim();
    const propertyName = (property.propertyName || "").trim();
    const propertyValue = (property.propertyValue || "").trim();
    if (!propertyName || !propertyValue) {
      continue;
    }
    map.set(propertyName.toLowerCase(), propertyValue);
    if (groupName) {
      map.set(`${groupName}.${propertyName}`.toLowerCase(), propertyValue);
    }
  }
  return map;
}

function firstDefined(...values: Array<string | undefined>): string | undefined {
  return values.find((value) => value && value.trim().length > 0);
}

function inferProviderFromCloneUrl(cloneUrl?: string): "github" | "gitlab" | undefined {
  if (!cloneUrl) {
    return undefined;
  }
  const lowered = cloneUrl.toLowerCase();
  if (lowered.includes("github.com")) {
    return "github";
  }
  if (lowered.includes("gitlab")) {
    return "gitlab";
  }
  return undefined;
}

function inferCloneUrlFromProject(project: DtrackProject): string | undefined {
  const ref = (project.externalReferences || []).find((item) => {
    const type = (item.type || "").toUpperCase();
    return type.includes("VCS") || type.includes("SCM") || type.includes("DISTRIBUTION");
  }) || (project.externalReferences || []).find((item) => typeof item.url === "string" && item.url.includes("git"));
  return ref?.url;
}

async function resolveProjectTarget(config: ToolkitConfig, identifier: string): Promise<{
  resolved: ResolvedProject;
  properties: DtrackProjectProperty[];
}> {
  const client = new DependencyTrackClient(config);
  const override = findProjectMapping(config, identifier);
  const project = override
    ? (await client.resolveProject(override)).project
    : await client.resolveProjectIdentifier(identifier);
  const properties = await client.getProjectProperties(project.uuid);
  return {
    resolved: {
      mapping: override,
      project,
    },
    properties,
  };
}

function buildRemediationMapping(
  project: DtrackProject,
  properties: DtrackProjectProperty[],
  override?: ProjectMapping,
): ProjectMapping {
  const lookup = propertyLookup(properties);
  const cloneUrl = firstDefined(
    override?.cloneUrl,
    lookup.get("automation.cloneurl"),
    lookup.get("automation.repourl"),
    lookup.get("automation.repositoryurl"),
    lookup.get("repo.cloneurl"),
    lookup.get("repo.repositoryurl"),
    lookup.get("vcs.cloneurl"),
    lookup.get("vcs.repositoryurl"),
    lookup.get("cloneurl"),
    lookup.get("repositoryurl"),
    inferCloneUrlFromProject(project),
  );
  const provider = (firstDefined(
    override?.provider,
    lookup.get("automation.provider"),
    lookup.get("repo.provider"),
    lookup.get("vcs.provider"),
    lookup.get("provider"),
    inferProviderFromCloneUrl(cloneUrl),
  ) || "") as ProjectMapping["provider"];
  const defaultBranch = firstDefined(
    override?.defaultBranch,
    lookup.get("automation.defaultbranch"),
    lookup.get("repo.defaultbranch"),
    lookup.get("vcs.defaultbranch"),
    lookup.get("defaultbranch"),
    "main",
  )!;
  const localPath = firstDefined(
    override?.localPath,
    lookup.get("automation.localpath"),
    lookup.get("repo.localpath"),
    lookup.get("vcs.localpath"),
    lookup.get("localpath"),
  );

  if (!cloneUrl) {
    fail([
      `No repository clone URL is available for Dependency-Track project '${project.name}'.`,
      "Set one of these project properties in Dependency-Track:",
      "- automation.cloneUrl",
      "- automation.repositoryUrl",
      "- repo.cloneUrl",
      "Or add a local override in dtrack-agent.config.json.",
    ].join("\n"));
  }

  if (provider !== "github" && provider !== "gitlab") {
    fail([
      `No supported VCS provider is available for Dependency-Track project '${project.name}'.`,
      "Set automation.provider to 'github' or 'gitlab' in Dependency-Track project properties,",
      "or add a local override in dtrack-agent.config.json.",
    ].join("\n"));
  }

  return {
    key: override?.key || project.name,
    dependencyTrackName: override?.dependencyTrackName || project.name,
    projectUuid: project.uuid,
    imageProjectUuid: override?.imageProjectUuid,
    classifier: override?.classifier || project.classifier,
    provider,
    cloneUrl,
    defaultBranch,
    localPath,
  };
}

export async function listProjects(config: ToolkitConfig, query?: string): Promise<unknown[]> {
  const client = new DependencyTrackClient(config);
  const projects = await client.listProjects();
  const loweredQuery = query?.toLowerCase().trim();
  const grouped = new Map<string, DtrackProject>();
  for (const project of projects) {
    const groupKey = `${project.name.toLowerCase()}::${(project.classifier || "").toLowerCase()}`;
    const existing = grouped.get(groupKey);
    if (!existing) {
      grouped.set(groupKey, project);
      continue;
    }
    if (project.latestVersion === true && existing.latestVersion !== true) {
      grouped.set(groupKey, project);
      continue;
    }
    const existingTs = existing.lastBomImport ? new Date(existing.lastBomImport).getTime() : 0;
    const projectTs = project.lastBomImport ? new Date(project.lastBomImport).getTime() : 0;
    if (projectTs > existingTs) {
      grouped.set(groupKey, project);
    }
  }

  return [...grouped.values()]
    .filter((project) => !loweredQuery ||
      project.name.toLowerCase().includes(loweredQuery) ||
      project.uuid.toLowerCase().includes(loweredQuery))
    .sort((left, right) => left.name.localeCompare(right.name) || String(right.lastBomImport || "").localeCompare(String(left.lastBomImport || "")))
    .map((project) => ({
      uuid: project.uuid,
      name: project.name,
      version: project.version,
      classifier: project.classifier,
      active: project.active,
      latestVersion: project.latestVersion,
      lastBomImport: project.lastBomImport,
    }));
}

export async function inspectProject(config: ToolkitConfig, identifier: string): Promise<unknown> {
  const { resolved, properties } = await resolveProjectTarget(config, identifier);
  const client = new DependencyTrackClient(config);
  const { findings } = await client.getFindings(
    resolved.mapping || {
      key: resolved.project.name,
      dependencyTrackName: resolved.project.name,
      projectUuid: resolved.project.uuid,
      provider: "github",
      cloneUrl: "https://example.invalid/placeholder.git",
      defaultBranch: "main",
    },
    "LOW",
  );

  let remediation: Record<string, unknown>;
  try {
    const mapping = buildRemediationMapping(resolved.project, properties, resolved.mapping);
    remediation = {
      ready: true,
      provider: mapping.provider,
      cloneUrl: mapping.cloneUrl,
      defaultBranch: mapping.defaultBranch,
      localPath: mapping.localPath || null,
    };
  } catch (error) {
    remediation = {
      ready: false,
      reason: error instanceof Error ? error.message : String(error),
    };
  }

  return {
    project: resolved.project,
    remediation,
    properties: properties.map((item) => ({
      groupName: item.groupName,
      propertyName: item.propertyName,
      propertyValue: item.propertyValue,
    })),
    totals: findings.reduce<Record<string, number>>((acc, finding) => {
      const severity = normalizeSeverity(finding.vulnerability.severity);
      acc[severity] = (acc[severity] || 0) + 1;
      return acc;
    }, {}),
  };
}

export async function listFindings(config: ToolkitConfig, identifier: string, severity: Severity = "CRITICAL"): Promise<unknown> {
  const { resolved } = await resolveProjectTarget(config, identifier);
  const client = new DependencyTrackClient(config);
  const { project, findings } = await client.getFindings(
    resolved.mapping || {
      key: resolved.project.name,
      dependencyTrackName: resolved.project.name,
      projectUuid: resolved.project.uuid,
      provider: "github",
      cloneUrl: "https://example.invalid/placeholder.git",
      defaultBranch: "main",
    },
    severity,
  );
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
  identifier: string,
  apply: boolean,
  findingId?: string,
): Promise<RemediationOutcome> {
  const { resolved, properties } = await resolveProjectTarget(config, identifier);
  const mapping = buildRemediationMapping(resolved.project, properties, resolved.mapping);
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
