import { describe, expect, test } from "bun:test";
import { parseArgs } from "../src/lib/config.ts";

describe("parseArgs", () => {
  test("parses config and json flags", () => {
    const parsed = parseArgs(["--config", "custom.json", "--json", "findings", "list", "simplemailer"]);
    expect(parsed.configPath).toBe("custom.json");
    expect(parsed.json).toBe(true);
    expect(parsed.command).toEqual(["findings", "list", "simplemailer"]);
  });

  test("parses force for asset installation", () => {
    const parsed = parseArgs(["install-agent-assets", "--force"]);
    expect(parsed.force).toBe(true);
    expect(parsed.installCodex).toBe(false);
    expect(parsed.installClaude).toBe(false);
    expect(parsed.command).toEqual(["install-agent-assets"]);
  });

  test("parses install target flags", () => {
    const parsed = parseArgs(["install-agent-assets", "--codex", "--claude"]);
    expect(parsed.installCodex).toBe(true);
    expect(parsed.installClaude).toBe(true);
    expect(parsed.command).toEqual(["install-agent-assets"]);
  });
});
