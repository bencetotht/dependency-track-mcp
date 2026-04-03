# Dependency-Track Agent Toolkit

This repository provides a small Bun/TypeScript toolkit for read-only Dependency-Track queries from an agent workflow. It helps an agent identify the current repository in Dependency-Track, inspect the latest project state, and retrieve current findings without modifying code, dependencies, or Git history.

The toolkit can be used in two ways:

- as a local CLI
- as a stdio MCP server for Codex or Claude Code

The repository also includes an optional Docker Compose stack for running Dependency-Track with PostgreSQL and a private Trivy server.

## Project Structure

- `src/cli.ts`: CLI entrypoint
- `src/mcp-server.ts`: stdio MCP server entrypoint
- `src/lib/`: shared config, git matching, and Dependency-Track query logic
- `agent-assets/`: optional Codex and Claude prompt/command assets
- `docker/compose.yaml`: optional Dependency-Track stack
- `docs/cli.md`: CLI reference and setup
- `docs/mcp-server.md`: MCP server reference and setup
- `docs/docker-compose.md`: Docker Compose stack reference and setup
- `AGENT_AUTOMATION.md`: agent asset installation and workflow notes

## Setup And Quick Start

1. Install Bun locally.
2. Copy `dtrack-agent.config.example.json` to `dtrack-agent.config.json`.
3. Set `dependencyTrack.url`.
4. Export the API key environment variable named by `dependencyTrack.apiKeyEnv`.
5. Run a quick CLI query:

```bash
bun run src/cli.ts projects list
bun run src/cli.ts projects detect /path/to/target-repo
bun run src/cli.ts findings list
```

If you want MCP instead of direct CLI usage, build the project and register `dist/mcp-server.js` with your agent:

```bash
bun run build
```

For a local or self-hosted Dependency-Track stack, see `docker/compose.yaml` and `docs/docker-compose.md`.

## Documentation

- [CLI documentation](docs/cli.md)
- [MCP server documentation](docs/mcp-server.md)
- [Docker Compose documentation](docs/docker-compose.md)
- [Agent asset documentation](AGENT_AUTOMATION.md)
