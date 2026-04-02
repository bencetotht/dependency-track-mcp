import { join } from "node:path";
import { cp } from "node:fs/promises";
import type { InstallTarget } from "./types.ts";
import { ensureDir, expandHome, fail } from "./utils.ts";

const REPO_ROOT = process.cwd();

export async function installAgentAssets(force: boolean, targets: InstallTarget[] = ["codex", "claude"]): Promise<Record<string, string>> {
  const home = process.env.HOME;
  if (!home) {
    fail("HOME is not set.");
  }

  const selectedTargets = targets.length > 0 ? [...new Set(targets)] : ["codex", "claude"];
  const mappings = [
    {
      targetGroup: "claude" as const,
      source: join(REPO_ROOT, "agent-assets", "claude", "commands"),
      target: join(home, ".claude", "commands"),
      label: "claude_commands",
    },
    {
      targetGroup: "codex" as const,
      source: join(REPO_ROOT, "agent-assets", "codex", "prompts"),
      target: join(home, ".codex", "prompts"),
      label: "codex_prompts",
    },
    {
      targetGroup: "codex" as const,
      source: join(REPO_ROOT, "agent-assets", "skill", "dependency-track-query"),
      target: join(home, ".codex", "skills", "dependency-track-query"),
      label: "codex_skill",
    },
  ];

  const installed: Record<string, string> = {};
  for (const item of mappings.filter((mapping) => selectedTargets.includes(mapping.targetGroup))) {
    await ensureDir(expandHome(item.target));
    await cp(item.source, item.target, {
      recursive: true,
      force,
    });
    installed[item.label] = item.target;
  }

  return installed;
}
