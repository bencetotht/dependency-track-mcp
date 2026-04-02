import { resolve } from "node:path";
import type { ParsedArgs, ProjectMapping, ToolkitConfig } from "./types.ts";
import { expandHome, fail } from "./utils.ts";

const DEFAULT_CONFIG_FILE = "dtrack-agent.config.json";

export async function loadConfig(configPath?: string): Promise<ToolkitConfig> {
  const candidate = configPath || process.env.DTRACK_AGENT_CONFIG || DEFAULT_CONFIG_FILE;
  const resolved = resolve(expandHome(candidate));
  const file = Bun.file(resolved);
  if (!(await file.exists())) {
    fail(`Config not found at ${resolved}. Copy dtrack-agent.config.example.json to ${DEFAULT_CONFIG_FILE} and edit it.`);
  }
  const config = (await file.json()) as ToolkitConfig;
  validateConfig(config);
  config.projects = config.projects || [];
  return config;
}

function validateConfig(config: ToolkitConfig): void {
  if (!config.dependencyTrack?.url || !config.dependencyTrack?.apiKeyEnv) {
    fail("Config must define dependencyTrack.url and dependencyTrack.apiKeyEnv.");
  }
  if (config.projects && !Array.isArray(config.projects)) {
    fail("Config projects must be an array when provided.");
  }
}

export function findProjectMapping(config: ToolkitConfig, identifier: string): ProjectMapping | undefined {
  const lowered = identifier.toLowerCase();
  return (config.projects || []).find((project) =>
    project.key.toLowerCase() === lowered ||
    project.dependencyTrackName.toLowerCase() === lowered ||
    (project.projectUuid || "").toLowerCase() === lowered,
  );
}

export function getDependencyTrackApiKey(config: ToolkitConfig): string {
  const apiKey = process.env[config.dependencyTrack.apiKeyEnv];
  if (!apiKey) {
    fail(`Environment variable ${config.dependencyTrack.apiKeyEnv} is not set.`);
  }
  return apiKey;
}

export function parseArgs(argv: string[]): ParsedArgs {
  const command: string[] = [];
  let configPath: string | undefined;
  let json = false;
  let force = false;
  let installCodex = false;
  let installClaude = false;

  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--config") {
      configPath = argv[index + 1];
      index += 1;
      continue;
    }
    if (token === "--json") {
      json = true;
      continue;
    }
    if (token === "--force") {
      force = true;
      continue;
    }
    if (token === "--codex") {
      installCodex = true;
      continue;
    }
    if (token === "--claude") {
      installClaude = true;
      continue;
    }
    command.push(token);
  }

  return { configPath, json, force, installCodex, installClaude, command };
}
