import { execCommand, expandHome, fail } from "./utils.ts";

export async function findGitRoot(startPath = process.cwd()): Promise<string> {
  const result = await execCommand(["git", "rev-parse", "--show-toplevel"], {
    cwd: expandHome(startPath),
    quiet: true,
  });
  if (result.exitCode !== 0 || !result.stdout.trim()) {
    fail(`No git repository found from ${startPath}.`);
  }
  return result.stdout.trim();
}

export async function getGitRemoteUrl(repoPath: string, remoteName = "origin"): Promise<string> {
  const result = await execCommand(["git", "remote", "get-url", remoteName], {
    cwd: repoPath,
    quiet: true,
  });
  if (result.exitCode !== 0 || !result.stdout.trim()) {
    fail(`Git remote '${remoteName}' is not configured in ${repoPath}.`);
  }
  return result.stdout.trim();
}

export function normalizeGitRemoteUrl(remoteUrl: string): string {
  const trimmed = remoteUrl.trim();
  if (trimmed.startsWith("git@")) {
    const match = trimmed.match(/^git@([^:]+):(.+?)(\.git)?$/);
    if (!match) {
      return trimmed.replace(/\.git$/, "");
    }
    return `https://${match[1]}/${match[2].replace(/\.git$/, "")}`;
  }
  if (trimmed.startsWith("ssh://git@")) {
    const url = new URL(trimmed);
    return `https://${url.host}${url.pathname.replace(/\.git$/, "")}`;
  }
  try {
    const url = new URL(trimmed);
    return `https://${url.host}${url.pathname.replace(/\.git$/, "")}`;
  } catch {
    return trimmed.replace(/\.git$/, "");
  }
}
