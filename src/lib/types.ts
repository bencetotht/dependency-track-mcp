export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "UNASSIGNED";
export type Provider = "github" | "gitlab";
export type PackageManager = "bun" | "npm" | "pnpm" | "yarn";

export interface ProjectMapping {
  key: string;
  dependencyTrackName: string;
  projectUuid?: string;
  imageProjectUuid?: string;
  classifier?: string;
  provider: Provider;
  cloneUrl: string;
  defaultBranch: string;
  localPath?: string;
}

export interface ToolkitConfig {
  dependencyTrack: {
    url: string;
    apiKeyEnv: string;
  };
  workspaceRoot: string;
  github?: {
    tokenEnv: string;
    apiBaseUrl?: string;
  };
  gitlab?: {
    tokenEnv: string;
    apiBaseUrl?: string;
  };
  remediation?: {
    branchPrefix?: string;
    maxFindingsPerRun?: number;
    severity?: Severity;
  };
  projects: ProjectMapping[];
}

export interface DtrackProject {
  uuid: string;
  name: string;
  version: string;
  classifier?: string;
  active?: boolean;
  latestVersion?: boolean;
  lastBomImport?: number | string | null;
  metrics?: Record<string, unknown>;
}

export interface DtrackFinding {
  matrix: string;
  component: {
    uuid?: string;
    name: string;
    version: string;
    purl?: string;
    latestVersion?: string;
  };
  vulnerability: {
    uuid?: string;
    source?: string;
    vulnId?: string;
    title?: string;
    description?: string;
    severity?: Severity;
    recommendation?: string;
    aliases?: Array<Record<string, string>>;
  };
  analysis?: {
    state?: string;
    isSuppressed?: boolean;
  };
}

export interface FpfDocument {
  version: string;
  project: {
    uuid: string;
    name: string;
    version: string;
  };
  findings: DtrackFinding[];
}

export interface ResolvedProject {
  mapping: ProjectMapping;
  project: DtrackProject;
}

export interface ValidationStep {
  name: string;
  status: "passed" | "failed" | "skipped";
  detail: string;
}

export interface RemediationOutcome {
  mode: "dry-run" | "apply";
  projectKey: string;
  projectUuid: string;
  projectVersion: string;
  repoPath: string;
  branchName?: string;
  prUrl?: string;
  mrUrl?: string;
  updatedPackages: Array<{
    name: string;
    fromVersion: string;
    toVersion: string;
    dependencyType: string;
    vulnerabilityIds: string[];
  }>;
  skippedFindings: Array<{
    findingId: string;
    reason: string;
  }>;
  validations: ValidationStep[];
}

export interface ParsedArgs {
  configPath?: string;
  json: boolean;
  apply: boolean;
  force: boolean;
  command: string[];
}

