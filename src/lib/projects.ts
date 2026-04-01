import { findProjectMapping } from "./config.ts";
import { DependencyTrackClient } from "./dtrack.ts";
import { findGitRoot, getGitRemoteUrl, normalizeGitRemoteUrl } from "./git.ts";
import type { DtrackFinding, DtrackProject, DtrackProjectProperty, ProjectMapping, Severity, ToolkitConfig } from "./types.ts";
import { fail } from "./utils.ts";

type ResolvedProject = {
  project: DtrackProject;
  properties: DtrackProjectProperty[];
  override?: ProjectMapping;
};

type RepoProjectMatch = ResolvedProject & {
  repoPath: string;
  remoteUrl: string;
  matchSource: string;
};

function normalizeSeverity(input?: string): Severity {
  return ((input || "UNASSIGNED").toUpperCase()) as Severity;
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

function inferCloneUrlFromProject(project: DtrackProject): string | undefined {
  const ref = (project.externalReferences || []).find((item) => {
    const type = (item.type || "").toUpperCase();
    return type.includes("VCS") || type.includes("SCM") || type.includes("DISTRIBUTION");
  }) || (project.externalReferences || []).find((item) => typeof item.url === "string" && item.url.includes("git"));
  return ref?.url;
}

function groupProjects(projects: DtrackProject[]): DtrackProject[] {
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
  return [...grouped.values()];
}

function projectCloneUrlCandidates(
  project: DtrackProject,
  properties: DtrackProjectProperty[],
  override?: ProjectMapping,
): Array<{ url: string; source: string }> {
  const lookup = propertyLookup(properties);
  const candidates = [
    { value: override?.cloneUrl, source: "config.projects[].cloneUrl" },
    { value: lookup.get("automation.cloneurl"), source: "project property automation.cloneUrl" },
    { value: lookup.get("automation.repourl"), source: "project property automation.repoUrl" },
    { value: lookup.get("automation.repositoryurl"), source: "project property automation.repositoryUrl" },
    { value: lookup.get("repo.cloneurl"), source: "project property repo.cloneUrl" },
    { value: lookup.get("repo.repositoryurl"), source: "project property repo.repositoryUrl" },
    { value: lookup.get("vcs.cloneurl"), source: "project property vcs.cloneUrl" },
    { value: lookup.get("vcs.repositoryurl"), source: "project property vcs.repositoryUrl" },
    { value: lookup.get("cloneurl"), source: "project property cloneUrl" },
    { value: lookup.get("repositoryurl"), source: "project property repositoryUrl" },
    { value: inferCloneUrlFromProject(project), source: "project external reference" },
  ];

  const seen = new Set<string>();
  const normalized: Array<{ url: string; source: string }> = [];
  for (const candidate of candidates) {
    if (!candidate.value || !candidate.value.trim()) {
      continue;
    }
    const url = normalizeGitRemoteUrl(candidate.value);
    if (seen.has(url)) {
      continue;
    }
    seen.add(url);
    normalized.push({ url, source: candidate.source });
  }
  return normalized;
}

function mapProperties(properties: DtrackProjectProperty[]): Array<{ groupName?: string; propertyName?: string; propertyValue?: string }> {
  return properties.map((item) => ({
    groupName: item.groupName,
    propertyName: item.propertyName,
    propertyValue: item.propertyValue,
  }));
}

function mapFinding(finding: DtrackFinding) {
  return {
    id: finding.vulnerability.vulnId || finding.vulnerability.uuid || finding.matrix,
    matrix: finding.matrix,
    package: finding.component.name,
    version: finding.component.version,
    latestVersion: finding.component.latestVersion,
    purl: finding.component.purl,
    severity: normalizeSeverity(finding.vulnerability.severity),
    source: finding.vulnerability.source,
    vulnerabilityId: finding.vulnerability.vulnId,
    title: finding.vulnerability.title,
    description: finding.vulnerability.description,
    recommendation: finding.vulnerability.recommendation,
    aliases: finding.vulnerability.aliases || [],
  };
}

async function resolveProjectTarget(config: ToolkitConfig, identifier: string): Promise<ResolvedProject> {
  const client = new DependencyTrackClient(config);
  const override = findProjectMapping(config, identifier);
  const project = override?.projectUuid
    ? await client.getProject(override.projectUuid)
    : override
      ? await client.resolveProjectIdentifier(override.dependencyTrackName, override.classifier)
      : await client.resolveProjectIdentifier(identifier);
  const properties = await client.getProjectProperties(project.uuid);
  return { project, properties, override };
}

async function resolveProjectFromRepo(config: ToolkitConfig, repoPath?: string): Promise<RepoProjectMatch> {
  const gitRoot = await findGitRoot(repoPath || process.cwd());
  const remoteUrl = normalizeGitRemoteUrl(await getGitRemoteUrl(gitRoot));
  const client = new DependencyTrackClient(config);
  const projects = groupProjects(await client.listProjects());

  for (const project of projects) {
    const override = findProjectMapping(config, project.uuid) || findProjectMapping(config, project.name);
    const properties = await client.getProjectProperties(project.uuid);
    const candidates = projectCloneUrlCandidates(project, properties, override);
    const matched = candidates.find((candidate) => candidate.url === remoteUrl);
    if (matched) {
      return {
        project,
        properties,
        override,
        repoPath: gitRoot,
        remoteUrl,
        matchSource: matched.source,
      };
    }
  }

  fail([
    `No Dependency-Track project matched the current repository remote '${remoteUrl}'.`,
    "Add one of these project properties to the correct Dependency-Track project:",
    "- automation.cloneUrl",
    "- automation.repositoryUrl",
    "- repo.cloneUrl",
    "Or add a matching cloneUrl override in dtrack-agent.config.json.",
  ].join("\n"));
}

export async function listProjects(config: ToolkitConfig, query?: string): Promise<unknown[]> {
  const client = new DependencyTrackClient(config);
  const loweredQuery = query?.toLowerCase().trim();

  return groupProjects(await client.listProjects())
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

export async function detectProjectFromRepo(config: ToolkitConfig, repoPath?: string): Promise<unknown> {
  const detected = await resolveProjectFromRepo(config, repoPath);
  return {
    project: detected.project,
    repoPath: detected.repoPath,
    remoteUrl: detected.remoteUrl,
    matchSource: detected.matchSource,
    properties: mapProperties(detected.properties),
  };
}

export async function inspectProject(config: ToolkitConfig, identifier?: string, repoPath?: string): Promise<unknown> {
  const resolved = identifier
    ? await resolveProjectTarget(config, identifier)
    : await resolveProjectFromRepo(config, repoPath);
  const client = new DependencyTrackClient(config);
  const { findings } = await client.getFindingsForProject(resolved.project.uuid, "LOW");

  const totals = findings.reduce<Record<string, number>>((acc, finding) => {
    const severity = normalizeSeverity(finding.vulnerability.severity);
    acc[severity] = (acc[severity] || 0) + 1;
    return acc;
  }, {});

  const response: Record<string, unknown> = {
    project: resolved.project,
    properties: mapProperties(resolved.properties),
    totals,
  };

  if ("repoPath" in resolved) {
    response.repoPath = resolved.repoPath;
    response.remoteUrl = resolved.remoteUrl;
    response.matchSource = resolved.matchSource;
  }

  return response;
}

export async function listFindings(
  config: ToolkitConfig,
  identifier?: string,
  severity: Severity = "CRITICAL",
  repoPath?: string,
): Promise<unknown> {
  const resolved = identifier
    ? await resolveProjectTarget(config, identifier)
    : await resolveProjectFromRepo(config, repoPath);
  const client = new DependencyTrackClient(config);
  const { project, findings } = await client.getFindingsForProject(resolved.project.uuid, severity);

  const response: Record<string, unknown> = {
    project,
    severity,
    findings: findings.map(mapFinding),
  };

  if ("repoPath" in resolved) {
    response.repoPath = resolved.repoPath;
    response.remoteUrl = resolved.remoteUrl;
    response.matchSource = resolved.matchSource;
  }

  return response;
}
