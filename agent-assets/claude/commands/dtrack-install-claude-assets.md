---
description: Install or refresh only the Dependency-Track Claude slash commands from this repository.
disable-model-invocation: true
---

# Install Claude Dependency-Track Assets

Install only the Claude command assets from this repository with the CLI.

## Usage

`/dtrack-install-claude-assets [--force]`

## Workflow

1. Verify the current working directory is this toolkit repository by checking that `src/cli.ts`, `src/mcp-server.ts`, and `agent-assets/` exist.
2. If the current working directory is not this repository, explain that this command must be run from the toolkit checkout and stop.
3. Run `bun run src/cli.ts install-agent-assets --claude`, appending any extra arguments that were passed to the slash command.
4. Report which paths were updated.

Use `--force` when you want to overwrite previously installed Claude command files.
