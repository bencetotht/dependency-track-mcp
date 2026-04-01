import { describe, expect, test } from "bun:test";
import { parseArgs } from "../src/lib/config.ts";

describe("parseArgs", () => {
  test("parses config and apply flags", () => {
    const parsed = parseArgs(["--config", "custom.json", "fix", "critical", "simplemailer", "--apply"]);
    expect(parsed.configPath).toBe("custom.json");
    expect(parsed.apply).toBe(true);
    expect(parsed.command).toEqual(["fix", "critical", "simplemailer"]);
  });
});

