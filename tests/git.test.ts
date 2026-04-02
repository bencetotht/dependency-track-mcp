import { describe, expect, test } from "bun:test";
import { normalizeGitRemoteUrl } from "../src/lib/git.ts";

describe("normalizeGitRemoteUrl", () => {
  test("normalizes https remotes", () => {
    expect(normalizeGitRemoteUrl("https://github.com/example/repo.git")).toBe("https://github.com/example/repo");
  });

  test("normalizes ssh remotes", () => {
    expect(normalizeGitRemoteUrl("git@github.com:example/repo.git")).toBe("https://github.com/example/repo");
  });

  test("normalizes ssh url remotes", () => {
    expect(normalizeGitRemoteUrl("ssh://git@gitlab.example.com/group/repo.git")).toBe("https://gitlab.example.com/group/repo");
  });
});
