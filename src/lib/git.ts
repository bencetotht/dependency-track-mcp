import { basename, join } from "node:path";
import { stat } from "node:fs/promises";
import type { ProjectMapping, ToolkitConfig } from "./types.ts";
import { getProviderToken } from "./config.ts";
import { ensureDir, execCommand, expandHome, fail, slugify } from "./utils.ts";

function getRepoFolderName(cloneUrl: string): string {
  const trimmed = cloneUrl.replace(/\/$/, "").replace(/\.git$/, "");
  return slugify(basename(trimmed));
}

function buildAuthenticatedCloneUrl(mapping: ProjectMapping, token: string): string {
  if (mapping.provider === "github") {
    const url = new URL(mapping.cloneUrl);
    return `https://x-access-token:${encodeURIComponent(token)}@${url.host}${url.pathname}`;
  }
  const url = new URL(mapping.cloneUrl);
  return `https://oauth2:${encodeURIComponent(token)}@${url.host}${url.pathname}`;
}

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

export async function ensureRepository(config: ToolkitConfig, mapping: ProjectMapping): Promise<string> {
  if (mapping.localPath) {
    return expandHome(mapping.localPath);
  }

  const targetDir = join(expandHome(config.workspaceRoot), mapping.key || getRepoFolderName(mapping.cloneUrl));
  await ensureDir(expandHome(config.workspaceRoot));

  try {
    const gitStat = await stat(join(targetDir, ".git"));
    if (gitStat.isDirectory()) {
      await execCommand(["git", "fetch", "--all"], { cwd: targetDir });
      return targetDir;
    }
  } catch {
    // Fall through to clone when the repo does not exist yet.
  }

  if (await Bun.file(join(targetDir, ".git")).exists()) {
    await execCommand(["git", "fetch", "--all"], { cwd: targetDir });
    return targetDir;
  }

  const token = getProviderToken(config, mapping.provider);
  const cloneUrl = token ? buildAuthenticatedCloneUrl(mapping, token) : mapping.cloneUrl;
  await execCommand(["git", "clone", cloneUrl, targetDir]);
  return targetDir;
}

export async function prepareBranch(repoPath: string, defaultBranch: string, branchName: string): Promise<void> {
  await execCommand(["git", "checkout", defaultBranch], { cwd: repoPath });
  await execCommand(["git", "pull", "--ff-only"], { cwd: repoPath });
  await execCommand(["git", "checkout", "-B", branchName], { cwd: repoPath });
}

export async function hasChanges(repoPath: string): Promise<boolean> {
  const result = await execCommand(["git", "status", "--porcelain"], { cwd: repoPath, quiet: true });
  return result.stdout.trim().length > 0;
}

export async function commitAndPush(
  config: ToolkitConfig,
  mapping: ProjectMapping,
  repoPath: string,
  branchName: string,
  commitMessage: string,
): Promise<void> {
  await execCommand(["git", "add", "-A"], { cwd: repoPath });
  await execCommand(["git", "commit", "-m", commitMessage], { cwd: repoPath });

  const token = getProviderToken(config, mapping.provider);
  if (!token) {
    fail(`Cannot push branch for ${mapping.provider}: provider token is missing.`);
  }
  const remoteUrl = buildAuthenticatedCloneUrl(mapping, token);
  await execCommand(["git", "remote", "set-url", "origin", remoteUrl], { cwd: repoPath });
  await execCommand(["git", "push", "--set-upstream", "origin", branchName], { cwd: repoPath });
}
