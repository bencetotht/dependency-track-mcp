#!/usr/bin/env bun
import { appendFile } from "node:fs/promises";
import { loadConfig } from "./lib/config.ts";
import { detectProjectFromRepo, inspectProject, listFindings, listProjects } from "./lib/projects.ts";

type JsonRpcId = string | number | null;
type JsonRpcMessage = {
  jsonrpc?: string;
  id?: JsonRpcId;
  method?: string;
  params?: Record<string, unknown>;
};
type TransportMode = "content-length" | "newline-json";

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
];

const debugLogPath = process.env.DTRACK_MCP_DEBUG_LOG;

function debugLog(message: string): void {
  if (!debugLogPath) {
    return;
  }
  const line = `[${new Date().toISOString()} pid=${process.pid}] ${message}\n`;
  void appendFile(debugLogPath, line).catch(() => {});
}

class StdioJsonRpcServer {
  private buffer = Buffer.alloc(0);
  private transportMode: TransportMode | null = null;

  start(): void {
    debugLog(`server start cwd=${process.cwd()} argv=${JSON.stringify(process.argv)} configEnv=${process.env.DTRACK_AGENT_CONFIG ? "set" : "unset"} apiKeyEnv=${process.env.DEPENDENCY_TRACK_API_KEY ? "set" : "unset"}`);
    process.stdin.on("data", (chunk) => {
      debugLog(`stdin chunk bytes=${chunk.length}`);
      debugLog(`stdin preview=${JSON.stringify(Buffer.from(chunk).toString("utf8").slice(0, 400))}`);
      this.buffer = Buffer.concat([this.buffer, Buffer.from(chunk)]);
      void this.processBuffer();
    });
  }

  private async processBuffer(): Promise<void> {
    while (true) {
      const messageBody = this.extractMessageBody();
      if (!messageBody) {
        return;
      }

      if (!this.transportMode) {
        this.transportMode = messageBody.mode;
      }

      const message = JSON.parse(messageBody.body) as JsonRpcMessage;
      debugLog(`message method=${message.method || "<none>"} id=${String(message.id ?? "null")}`);
      await this.handleMessage(message);
    }
  }

  private extractMessageBody(): { body: string; mode: TransportMode } | null {
    const contentLengthMessage = this.extractContentLengthMessage();
    if (contentLengthMessage) {
      return {
        body: contentLengthMessage,
        mode: "content-length",
      };
    }

    const newlineMessage = this.extractNewlineDelimitedMessage();
    if (newlineMessage) {
      return {
        body: newlineMessage,
        mode: "newline-json",
      };
    }

    return null;
  }

  private extractContentLengthMessage(): string | null {
    const crlfDelimiterIndex = this.buffer.indexOf("\r\n\r\n");
    const lfDelimiterIndex = this.buffer.indexOf("\n\n");

    if (crlfDelimiterIndex === -1 && lfDelimiterIndex === -1) {
      return null;
    }

    const delimiterIndex = crlfDelimiterIndex !== -1 && (lfDelimiterIndex === -1 || crlfDelimiterIndex < lfDelimiterIndex)
      ? crlfDelimiterIndex
      : lfDelimiterIndex;
    const delimiterLength = delimiterIndex === crlfDelimiterIndex ? 4 : 2;
    const header = this.buffer.slice(0, delimiterIndex).toString("utf8");
    const match = header.match(/Content-Length:\s*(\d+)/i);
    if (!match) {
      return null;
    }

    const contentLength = Number(match[1]);
    const bodyStart = delimiterIndex + delimiterLength;
    const totalLength = bodyStart + contentLength;
    if (this.buffer.length < totalLength) {
      return null;
    }

    const body = this.buffer.slice(bodyStart, totalLength).toString("utf8");
    this.buffer = this.buffer.slice(totalLength);
    return body;
  }

  private extractNewlineDelimitedMessage(): string | null {
    const newlineIndex = this.buffer.indexOf("\n");
    if (newlineIndex === -1) {
      return null;
    }

    const line = this.buffer.slice(0, newlineIndex).toString("utf8").replace(/\r$/, "").trim();
    this.buffer = this.buffer.slice(newlineIndex + 1);

    if (!line) {
      return null;
    }

    return line;
  }

  private async handleMessage(message: JsonRpcMessage): Promise<void> {
    switch (message.method) {
      case "initialize":
        this.reply(message.id ?? null, {
          protocolVersion: "2024-11-05",
          serverInfo: { name: "dtrack-agent-mcp", version: "0.2.0" },
          capabilities: {
            tools: {},
            prompts: {},
          },
        });
        return;
      case "ping":
        this.reply(message.id ?? null, {});
        return;
      case "notifications/initialized":
      case "notifications/cancelled":
        return;
      case "tools/list":
        this.reply(message.id ?? null, { tools: toolDefinitions });
        return;
      case "tools/call":
        debugLog(`tools/call name=${String(message.params?.name || "")}`);
        this.reply(message.id ?? null, {
          content: [
            {
              type: "text",
              text: JSON.stringify(await this.callTool(await loadConfig(), message.params || {}), null, 2),
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
      case "resources/list":
        this.reply(message.id ?? null, { resources: [] });
        return;
      case "resources/templates/list":
        this.reply(message.id ?? null, { resourceTemplates: [] });
        return;
      case "logging/setLevel":
        this.reply(message.id ?? null, {
          ok: true,
        });
        return;
      default:
        if (message.id !== undefined) {
          this.error(message.id ?? null, `Unsupported MCP method: ${message.method}`);
        }
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
                "Return a concise summary of the critical findings and the vulnerability details needed for a follow-up remediation workflow.",
                "Do not expect this toolkit to edit code, commit changes, or create PRs/MRs.",
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
    debugLog(`error id=${String(id)} message=${message}`);
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
    debugLog(`send bytes=${Buffer.byteLength(body, "utf8")}`);
    if (this.transportMode === "newline-json") {
      process.stdout.write(`${body}\n`);
      return;
    }
    process.stdout.write(`Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`);
  }
}

process.on("uncaughtException", (error) => {
  debugLog(`uncaughtException ${error instanceof Error ? error.stack || error.message : String(error)}`);
});

process.on("unhandledRejection", (reason) => {
  debugLog(`unhandledRejection ${reason instanceof Error ? reason.stack || reason.message : String(reason)}`);
});

new StdioJsonRpcServer().start();
