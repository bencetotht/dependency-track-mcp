import { join } from "node:path";
import { cp } from "node:fs/promises";
import { ensureDir, expandHome, fail } from "./utils.ts";

const REPO_ROOT = process.cwd();

export async function installAgentAssets(force: boolean): Promise<Record<string, string>> {
  const home = process.env.HOME;
  if (!home) {
    fail("HOME is not set.");
  }

  const mappings = [
    {
      source: join(REPO_ROOT, "agent-assets", "claude", "commands"),
      target: join(home, ".claude", "commands"),
      label: "claude_commands",
    },
    {
      source: join(REPO_ROOT, "agent-assets", "codex", "prompts"),
      target: join(home, ".codex", "prompts"),
      label: "codex_prompts",
    },
    {
      source: join(REPO_ROOT, "agent-assets", "skill", "dependency-track-remediator"),
      target: join(home, ".codex", "skills", "dependency-track-remediator"),
      label: "codex_skill",
    },
  ];

  const installed: Record<string, string> = {};
  for (const item of mappings) {
    await ensureDir(expandHome(item.target));
    await cp(item.source, item.target, {
      recursive: true,
      force,
    });
    installed[item.label] = item.target;
  }

  return installed;
}

