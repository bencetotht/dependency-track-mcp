---
description: Install or refresh only the Dependency-Track Codex assets from this repository.
disable-model-invocation: true
---

# Install Codex Dependency-Track Assets

Install only the Codex assets from this repository with the CLI.

## Usage

`/dtrack-install-codex-assets [--force]`

## Workflow

1. Verify the current working directory is this toolkit repository by checking that `src/cli.ts`, `src/mcp-server.ts`, and `agent-assets/` exist.
2. If the current working directory is not this repository, explain that this command must be run from the toolkit checkout and stop.
3. Run `bun run src/cli.ts install-agent-assets --codex`, appending any extra arguments that were passed to the slash command.
4. Report which paths were updated.

Use `--force` when you want to overwrite previously installed Codex prompt and skill files.
