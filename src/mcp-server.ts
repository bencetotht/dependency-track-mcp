#!/usr/bin/env bun
import { loadConfig } from "./lib/config.ts";
import { detectProjectFromRepo, inspectProject, listFindings, listProjects, remediateCritical } from "./lib/remediation.ts";

type JsonRpcId = string | number | null;
type JsonRpcMessage = {
  jsonrpc?: string;
  id?: JsonRpcId;
  method?: string;
  params?: Record<string, unknown>;
};

const toolDefinitions = [
  {
    name: "dtrack_list_projects",
    description: "List Dependency-Track projects visible to the configured API key.",
    inputSchema: {
      type: "object",
      properties: {
        query: { type: "string" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "dtrack_get_project",
    description: "Inspect the latest Dependency-Track project for a project name/UUID, or infer it from a repo path.",
    inputSchema: {
      type: "object",
      properties: {
        projectIdentifier: { type: "string" },
        repoPath: { type: "string" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "dtrack_get_findings",
    description: "Get current findings for a Dependency-Track project name/UUID, or infer it from a repo path.",
    inputSchema: {
      type: "object",
      properties: {
        projectIdentifier: { type: "string" },
        repoPath: { type: "string" },
        severity: { type: "string" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "dtrack_detect_project",
    description: "Detect the Dependency-Track project associated with a local git repository path.",
    inputSchema: {
      type: "object",
      properties: {
        repoPath: { type: "string" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "dtrack_fix_critical",
    description: "Remediate critical Dependency-Track findings for a Dependency-Track project name/UUID, or infer it from a repo path.",
    inputSchema: {
      type: "object",
      properties: {
        projectIdentifier: { type: "string" },
        repoPath: { type: "string" },
        apply: { type: "boolean" },
      },
      additionalProperties: false,
    },
  },
  {
    name: "dtrack_fix_finding",
    description: "Remediate a specific Dependency-Track finding id or matrix for a Dependency-Track project name/UUID, or infer it from a repo path.",
    inputSchema: {
      type: "object",
      properties: {
        projectIdentifier: { type: "string" },
        repoPath: { type: "string" },
        findingId: { type: "string" },
        apply: { type: "boolean" },
      },
      required: ["findingId"],
      additionalProperties: false,
    },
  },
];

const promptDefinitions = [
  {
    name: "dtrack-triage",
    description: "Inspect the latest critical Dependency-Track findings for a Dependency-Track project name/UUID, or infer it from the current repo path.",
    arguments: [
      { name: "projectIdentifier", description: "Dependency-Track project name or UUID", required: false },
      { name: "repoPath", description: "Local repo path to match against Dependency-Track", required: false },
    ],
  },
  {
    name: "dtrack-fix-critical",
    description: "Inspect and optionally remediate critical Dependency-Track findings for a Dependency-Track project name/UUID, or infer it from the current repo path.",
    arguments: [
      { name: "projectIdentifier", description: "Dependency-Track project name or UUID", required: false },
      { name: "repoPath", description: "Local repo path to match against Dependency-Track", required: false },
      { name: "apply", description: "Use true to allow branch/PR creation", required: false },
    ],
  },
];

class StdioJsonRpcServer {
  private buffer = Buffer.alloc(0);

  start(): void {
    process.stdin.on("data", (chunk) => {
      this.buffer = Buffer.concat([this.buffer, Buffer.from(chunk)]);
      void this.processBuffer();
    });
  }

  private async processBuffer(): Promise<void> {
    while (true) {
      const delimiterIndex = this.buffer.indexOf("\r\n\r\n");
      if (delimiterIndex === -1) {
        return;
      }

      const header = this.buffer.slice(0, delimiterIndex).toString("utf8");
      const match = header.match(/Content-Length:\s*(\d+)/i);
      if (!match) {
        throw new Error("Missing Content-Length header.");
      }

      const contentLength = Number(match[1]);
      const totalLength = delimiterIndex + 4 + contentLength;
      if (this.buffer.length < totalLength) {
        return;
      }

      const body = this.buffer.slice(delimiterIndex + 4, totalLength).toString("utf8");
      this.buffer = this.buffer.slice(totalLength);
      const message = JSON.parse(body) as JsonRpcMessage;
      await this.handleMessage(message);
    }
  }

  private async handleMessage(message: JsonRpcMessage): Promise<void> {
    const config = await loadConfig();
    switch (message.method) {
      case "initialize":
        this.reply(message.id ?? null, {
          protocolVersion: "2024-11-05",
          serverInfo: { name: "dtrack-agent-mcp", version: "0.1.0" },
          capabilities: {
            tools: {},
            prompts: {},
          },
        });
        return;
      case "notifications/initialized":
        return;
      case "tools/list":
        this.reply(message.id ?? null, { tools: toolDefinitions });
        return;
      case "tools/call":
        this.reply(message.id ?? null, {
          content: [
            {
              type: "text",
              text: JSON.stringify(await this.callTool(config, message.params || {}), null, 2),
            },
          ],
        });
        return;
      case "prompts/list":
        this.reply(message.id ?? null, { prompts: promptDefinitions });
        return;
      case "prompts/get":
        this.reply(message.id ?? null, await this.getPrompt(message.params || {}));
        return;
      default:
        this.error(message.id ?? null, `Unsupported MCP method: ${message.method}`);
    }
  }

  private async callTool(config: Awaited<ReturnType<typeof loadConfig>>, params: Record<string, unknown>): Promise<unknown> {
      const name = String(params.name || "");
      const args = (params.arguments || {}) as Record<string, unknown>;
      switch (name) {
      case "dtrack_list_projects":
        return listProjects(config, args.query ? String(args.query) : undefined);
      case "dtrack_detect_project":
        return detectProjectFromRepo(config, args.repoPath ? String(args.repoPath) : undefined);
      case "dtrack_get_project":
        return inspectProject(config, args.projectIdentifier ? String(args.projectIdentifier) : undefined, args.repoPath ? String(args.repoPath) : undefined);
      case "dtrack_get_findings":
        return listFindings(config, args.projectIdentifier ? String(args.projectIdentifier) : undefined, String(args.severity || "CRITICAL").toUpperCase() as never, args.repoPath ? String(args.repoPath) : undefined);
      case "dtrack_fix_critical":
        return remediateCritical(config, args.projectIdentifier ? String(args.projectIdentifier) : undefined, Boolean(args.apply), args.repoPath ? String(args.repoPath) : undefined);
      case "dtrack_fix_finding":
        return remediateCritical(config, args.projectIdentifier ? String(args.projectIdentifier) : undefined, Boolean(args.apply), args.repoPath ? String(args.repoPath) : undefined, String(args.findingId));
      default:
        throw new Error(`Unsupported tool: ${name}`);
      }
  }

  private async getPrompt(params: Record<string, unknown>): Promise<unknown> {
    const name = String(params.name || "");
    const args = (params.arguments || {}) as Record<string, string | undefined>;
    if (name === "dtrack-triage") {
      const target = args.projectIdentifier || args.repoPath || "<project-name-or-uuid-or-repo-path>";
      return {
        description: "Inspect the latest critical Dependency-Track findings for a Dependency-Track project name or UUID.",
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: [
                `Inspect Dependency-Track target '${target}'.`,
                args.repoPath ? `Use repo path '${args.repoPath}' to detect the matching Dependency-Track project if needed.` : "If no project identifier is given, detect the project from the current repo path first.",
                "Use the MCP tools to fetch the latest project state and current critical findings.",
                "Return a concise summary of fixable critical findings, skipped items, and the most urgent next action.",
              ].join("\n"),
            },
          },
        ],
      };
    }

    if (name === "dtrack-fix-critical") {
      const apply = args.apply === "true";
      const target = args.projectIdentifier || args.repoPath || "<project-name-or-uuid-or-repo-path>";
      return {
        description: "Inspect and remediate critical Dependency-Track findings for a Dependency-Track project name or UUID.",
        messages: [
          {
            role: "user",
            content: {
              type: "text",
              text: [
                `Inspect Dependency-Track target '${target}'.`,
                args.repoPath ? `Use repo path '${args.repoPath}' to detect the matching Dependency-Track project if needed.` : "If no project identifier is given, detect the project from the current repo path first.",
                "Use the MCP tools to list critical findings and assess fixability.",
                apply
                  ? "Proceed with remediation using apply=true, then report the branch and PR/MR URL."
                  : "Do a dry run first and report exactly what would be changed. Only switch to apply=true if explicitly requested.",
              ].join("\n"),
            },
          },
        ],
      };
    }

    throw new Error(`Unsupported prompt: ${name}`);
  }

  private reply(id: JsonRpcId, result: unknown): void {
    this.send({ jsonrpc: "2.0", id, result });
  }

  private error(id: JsonRpcId, message: string): void {
    this.send({
      jsonrpc: "2.0",
      id,
      error: {
        code: -32000,
        message,
      },
    });
  }

  private send(payload: unknown): void {
    const body = JSON.stringify(payload);
    process.stdout.write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
  }
}

new StdioJsonRpcServer().start();
