#!/usr/bin/env bun
import { installAgentAssets } from "./lib/agent-assets.ts";
import { loadConfig, parseArgs } from "./lib/config.ts";
import { detectProjectFromRepo, inspectProject, listFindings, listProjects, remediateCritical } from "./lib/remediation.ts";
import { printOutput } from "./lib/utils.ts";

function usage(): string {
  return [
    "Usage:",
    "  dtrack-agent [--config path] [--json] projects list [query]",
    "  dtrack-agent [--config path] [--json] projects detect [repo-path]",
    "  dtrack-agent [--config path] [--json] projects inspect [project-name-or-uuid]",
    "  dtrack-agent [--config path] [--json] findings list [project-name-or-uuid] [severity]",
    "  dtrack-agent [--config path] [--json] fix critical [project-name-or-uuid] [--apply]",
    "  dtrack-agent [--config path] [--json] fix finding [project-name-or-uuid] <finding-id> [--apply]",
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
    printOutput(await listProjects(config, rest[0]), parsed.json);
    return;
  }

  if (group === "projects" && action === "detect") {
    printOutput(await detectProjectFromRepo(config, rest[0]), parsed.json);
    return;
  }

  if (group === "projects" && action === "inspect") {
    printOutput(await inspectProject(config, rest[0]), parsed.json);
    return;
  }

  if (group === "findings" && action === "list") {
    const severity = (rest.length > 1 ? rest[1] : rest[0] && /^(critical|high|medium|low|unassigned)$/i.test(rest[0]) ? rest[0] : "CRITICAL").toUpperCase();
    const projectIdentifier = rest.length > 1 ? rest[0] : rest[0] && !/^(critical|high|medium|low|unassigned)$/i.test(rest[0]) ? rest[0] : undefined;
    printOutput(await listFindings(config, projectIdentifier, severity as never), parsed.json);
    return;
  }

  if (group === "fix" && action === "critical") {
    printOutput(await remediateCritical(config, rest[0], parsed.apply), parsed.json);
    return;
  }

  if (group === "fix" && action === "finding") {
    const projectIdentifier = rest.length > 1 ? rest[0] : undefined;
    const findingId = rest.length > 1 ? rest[1] : rest[0];
    printOutput(await remediateCritical(config, projectIdentifier, parsed.apply, undefined, findingId), parsed.json);
    return;
  }

  console.log(usage());
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
