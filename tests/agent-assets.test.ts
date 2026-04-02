import { describe, expect, test } from "bun:test";
import { mkdtemp, readdir, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { installAgentAssets } from "../src/lib/agent-assets.ts";

describe("installAgentAssets", () => {
  test("installs all Claude command assets", async () => {
    const originalHome = process.env.HOME;
    const tempHome = await mkdtemp(join(tmpdir(), "dtrack-agent-assets-"));

    process.env.HOME = tempHome;

    try {
      const installed = await installAgentAssets(false, ["claude"]);
      expect(installed).toEqual({
        claude_commands: join(tempHome, ".claude", "commands"),
      });

      const files = (await readdir(installed.claude_commands)).sort();
      expect(files).toEqual([
        "dtrack-install-assets.md",
        "dtrack-install-claude-assets.md",
        "dtrack-install-codex-assets.md",
        "dtrack-triage.md",
      ]);
    } finally {
      process.env.HOME = originalHome;
      await rm(tempHome, { recursive: true, force: true });
    }
  });
});
