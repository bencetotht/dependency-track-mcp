#!/usr/bin/env bun
import { installAgentAssets } from "./lib/agent-assets.ts";
import { loadConfig, parseArgs } from "./lib/config.ts";
import { inspectProject, listFindings, listProjects, remediateCritical } from "./lib/remediation.ts";
import { printOutput } from "./lib/utils.ts";

function usage(): string {
  return [
    "Usage:",
    "  dtrack-agent [--config path] [--json] projects list",
    "  dtrack-agent [--config path] [--json] projects inspect <project>",
    "  dtrack-agent [--config path] [--json] findings list <project> [severity]",
    "  dtrack-agent [--config path] [--json] fix critical <project> [--apply]",
    "  dtrack-agent [--config path] [--json] fix finding <project> <finding-id> [--apply]",
    "  dtrack-agent install-agent-assets [--force]",
  ].join("\n");
}

async function main(): Promise<void> {
  const parsed = parseArgs(process.argv.slice(2));
  const [group, action, ...rest] = parsed.command;

  if (!group) {
    console.log(usage());
    return;
  }

  if (group === "install-agent-assets") {
    const installed = await installAgentAssets(parsed.force);
    printOutput(installed, parsed.json);
    return;
  }

  const config = await loadConfig(parsed.configPath);

  if (group === "projects" && action === "list") {
    printOutput(await listProjects(config), parsed.json);
    return;
  }

  if (group === "projects" && action === "inspect") {
    printOutput(await inspectProject(config, rest[0]), parsed.json);
    return;
  }

  if (group === "findings" && action === "list") {
    printOutput(await listFindings(config, rest[0], (rest[1] || "CRITICAL").toUpperCase() as never), parsed.json);
    return;
  }

  if (group === "fix" && action === "critical") {
    printOutput(await remediateCritical(config, rest[0], parsed.apply), parsed.json);
    return;
  }

  if (group === "fix" && action === "finding") {
    printOutput(await remediateCritical(config, rest[0], parsed.apply, rest[1]), parsed.json);
    return;
  }

  console.log(usage());
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});

