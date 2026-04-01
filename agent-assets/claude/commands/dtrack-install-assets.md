---
description: Install or refresh the Dependency-Track Codex and Claude assets from this repository.
disable-model-invocation: true
---

# Install Dependency-Track Agent Assets

Install the local agent assets from this repository with the CLI.

## Usage

`/dtrack-install-assets [--force]`

## Workflow

1. Verify the current working directory is this toolkit repository by checking that `src/cli.ts`, `src/mcp-server.ts`, and `agent-assets/` exist.
2. If the current working directory is not this repository, explain that this command must be run from the toolkit checkout and stop.
3. Run `bun run src/cli.ts install-agent-assets`, appending any extra arguments that were passed to the slash command.
4. Report which asset targets were installed and which paths were updated.

If no extra arguments were provided, install both Codex and Claude assets.
