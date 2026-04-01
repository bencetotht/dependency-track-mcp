import type { ProjectMapping, ToolkitConfig } from "./types.ts";
import { getProviderToken } from "./config.ts";
import { fail } from "./utils.ts";

function parseRepoPath(cloneUrl: string): { host: string; owner: string; repo: string; path: string } {
  const url = new URL(cloneUrl);
  const path = url.pathname.replace(/^\/+/, "").replace(/\.git$/, "");
  const segments = path.split("/");
  if (segments.length < 2) {
    fail(`Unsupported cloneUrl path: ${cloneUrl}`);
  }
  return {
    host: url.host,
    owner: segments[0],
    repo: segments[segments.length - 1],
    path,
  };
}

export async function openPullRequestOrMr(
  config: ToolkitConfig,
  mapping: ProjectMapping,
  branchName: string,
  title: string,
  body: string,
): Promise<string> {
  const token = getProviderToken(config, mapping.provider);
  if (!token) {
    fail(`Missing ${mapping.provider} token for PR/MR creation.`);
  }

  if (mapping.provider === "github") {
    const { owner, repo } = parseRepoPath(mapping.cloneUrl);
    const apiBaseUrl = config.github?.apiBaseUrl || "https://api.github.com";
    const response = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/repos/${owner}/${repo}/pulls`, {
      method: "POST",
      headers: {
        "Authorization": `Bearer ${token}`,
        "Accept": "application/vnd.github+json",
        "Content-Type": "application/json",
      },
      body: JSON.stringify({
        title,
        head: branchName,
        base: mapping.defaultBranch,
        body,
      }),
    });
    if (!response.ok) {
      fail(`GitHub PR creation failed: ${response.status} ${await response.text()}`);
    }
    const payload = await response.json() as { html_url: string };
    return payload.html_url;
  }

  const { host, path } = parseRepoPath(mapping.cloneUrl);
  const apiBaseUrl = config.gitlab?.apiBaseUrl || `https://${host}/api/v4`;
  const response = await fetch(`${apiBaseUrl.replace(/\/$/, "")}/projects/${encodeURIComponent(path)}/merge_requests`, {
    method: "POST",
    headers: {
      "PRIVATE-TOKEN": token,
      "Content-Type": "application/json",
    },
    body: JSON.stringify({
      source_branch: branchName,
      target_branch: mapping.defaultBranch,
      title,
      description: body,
      remove_source_branch: true,
    }),
  });
  if (!response.ok) {
    fail(`GitLab MR creation failed: ${response.status} ${await response.text()}`);
  }
  const payload = await response.json() as { web_url: string };
  return payload.web_url;
}

