import type { DtrackFinding, DtrackProject, FpfDocument, ProjectMapping, ResolvedProject, Severity, ToolkitConfig } from "./types.ts";
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

  async exportFindings(projectUuid: string): Promise<FpfDocument> {
    return this.requestJson<FpfDocument>("GET", `/api/v1/finding/project/${projectUuid}/export`);
  }

  async resolveProject(mapping: ProjectMapping): Promise<ResolvedProject> {
    if (mapping.projectUuid) {
      const project = await this.getProject(mapping.projectUuid);
      return { mapping, project };
    }

    const projects = await this.listProjects();
    const candidates = projects.filter((project) =>
      project.name === mapping.dependencyTrackName &&
      (!mapping.classifier || (project.classifier || "").toUpperCase() === mapping.classifier.toUpperCase()),
    );
    if (candidates.length === 0) {
      fail(`No Dependency-Track project found for '${mapping.dependencyTrackName}'.`);
    }

    const latestFlag = candidates.find((project) => project.latestVersion === true);
    const project = latestFlag || sortByDateDesc(candidates, (item) => item.lastBomImport || item.version)[0];
    return { mapping, project };
  }

  async getFindings(mapping: ProjectMapping, minSeverity?: Severity): Promise<{ project: DtrackProject; findings: DtrackFinding[] }> {
    const resolved = await this.resolveProject(mapping);
    const exported = await this.exportFindings(resolved.project.uuid);
    const findings = exported.findings
      .filter((finding) => !finding.analysis?.isSuppressed)
      .filter((finding) => !minSeverity || severityRank(finding.vulnerability.severity) >= severityRank(minSeverity))
      .sort((left, right) => severityRank(right.vulnerability.severity) - severityRank(left.vulnerability.severity));
    return {
      project: resolved.project,
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

