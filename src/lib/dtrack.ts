import type { DtrackFinding, DtrackProject, DtrackProjectProperty, FpfDocument, Severity, ToolkitConfig } from "./types.ts";
import { getDependencyTrackApiKey } from "./config.ts";
import { fail, severityRank, sortByDateDesc } from "./utils.ts";

export class DependencyTrackClient {
  private readonly baseUrl: string;
  private readonly apiKey: string;

  constructor(config: ToolkitConfig) {
    this.baseUrl = config.dependencyTrack.url.replace(/\/$/, "");
    this.apiKey = getDependencyTrackApiKey(config);
  }

  async listProjects(): Promise<DtrackProject[]> {
    return this.requestJson<DtrackProject[]>("GET", "/api/v1/project");
  }

  async getProject(projectUuid: string): Promise<DtrackProject> {
    return this.requestJson<DtrackProject>("GET", `/api/v1/project/${projectUuid}`);
  }

  async getProjectProperties(projectUuid: string): Promise<DtrackProjectProperty[]> {
    return this.requestJson<DtrackProjectProperty[]>("GET", `/api/v1/project/${projectUuid}/property`);
  }

  async exportFindings(projectUuid: string): Promise<FpfDocument> {
    return this.requestJson<FpfDocument>("GET", `/api/v1/finding/project/${projectUuid}/export`);
  }

  async resolveProjectIdentifier(identifier: string, classifier?: string): Promise<DtrackProject> {
    const projects = await this.listProjects();
    const lowered = identifier.toLowerCase();
    const candidates = projects.filter((project) =>
      project.uuid.toLowerCase() === lowered ||
      project.name.toLowerCase() === lowered,
    ).filter((project) =>
      !classifier || (project.classifier || "").toUpperCase() === classifier.toUpperCase(),
    );

    if (candidates.length === 0) {
      fail(`No Dependency-Track project found for identifier '${identifier}'.`);
    }

    const exactUuid = candidates.find((project) => project.uuid.toLowerCase() === lowered);
    if (exactUuid) {
      return exactUuid;
    }

    const latestFlag = candidates.find((project) => project.latestVersion === true);
    const activeCandidates = candidates.filter((project) => project.active !== false);
    return latestFlag || sortByDateDesc(activeCandidates.length > 0 ? activeCandidates : candidates, (item) => item.lastBomImport || item.version)[0];
  }

  async getFindingsForProject(projectUuid: string, minSeverity?: Severity): Promise<{ project: DtrackProject; findings: DtrackFinding[] }> {
    const project = await this.getProject(projectUuid);
    const exported = await this.exportFindings(projectUuid);
    const findings = exported.findings
      .filter((finding) => !finding.analysis?.isSuppressed)
      .filter((finding) => !minSeverity || severityRank(finding.vulnerability.severity) >= severityRank(minSeverity))
      .sort((left, right) => severityRank(right.vulnerability.severity) - severityRank(left.vulnerability.severity));
    return {
      project,
      findings,
    };
  }

  private async requestJson<T>(method: string, path: string): Promise<T> {
    const response = await fetch(`${this.baseUrl}${path}`, {
      method,
      headers: {
        "Accept": "application/json",
        "X-Api-Key": this.apiKey,
      },
    });
    if (!response.ok) {
      const text = await response.text();
      fail(`Dependency-Track request failed (${method} ${path}): ${response.status} ${text}`);
    }
    return (await response.json()) as T;
  }
}
