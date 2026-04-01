export type Severity = "CRITICAL" | "HIGH" | "MEDIUM" | "LOW" | "UNASSIGNED";
export type InstallTarget = "codex" | "claude";

export interface ProjectMapping {
  key: string;
  dependencyTrackName: string;
  projectUuid?: string;
  classifier?: string;
  cloneUrl?: string;
}

export interface ToolkitConfig {
  dependencyTrack: {
    url: string;
    apiKeyEnv: string;
  };
  projects?: ProjectMapping[];
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
  tags?: Array<{ name?: string }>;
  externalReferences?: Array<{
    type?: string;
    url?: string;
    comment?: string;
  }>;
}

export interface DtrackProjectProperty {
  groupName?: string;
  propertyName?: string;
  propertyValue?: string;
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

export interface ParsedArgs {
  configPath?: string;
  json: boolean;
  force: boolean;
  installCodex: boolean;
  installClaude: boolean;
  command: string[];
}
