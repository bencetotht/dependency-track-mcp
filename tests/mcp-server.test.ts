import { describe, expect, test } from "bun:test";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

type JsonRpcPayload = Record<string, unknown>;

function frame(payload: JsonRpcPayload): string {
  const body = JSON.stringify(payload);
  return `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`;
}

function parseResponses(output: string): JsonRpcPayload[] {
  const messages: JsonRpcPayload[] = [];
  let buffer = output;

  while (buffer.length > 0) {
    const delimiterIndex = buffer.indexOf("\r\n\r\n");
    expect(delimiterIndex).toBeGreaterThanOrEqual(0);
    const header = buffer.slice(0, delimiterIndex);
    const match = header.match(/Content-Length:\s*(\d+)/i);
    expect(match).toBeTruthy();
    const contentLength = Number(match?.[1]);
    const bodyStart = delimiterIndex + 4;
    const bodyEnd = bodyStart + contentLength;
    messages.push(JSON.parse(buffer.slice(bodyStart, bodyEnd)) as JsonRpcPayload);
    buffer = buffer.slice(bodyEnd);
  }

  return messages;
}

describe("mcp server startup", () => {
  test("lists tools and prompts without loading config", async () => {
    const isolatedCwd = await mkdtemp(join(tmpdir(), "dtrack-mcp-test-"));
    const serverPath = new URL("../src/mcp-server.ts", import.meta.url).pathname;
    try {
      const proc = Bun.spawn([process.execPath, serverPath], {
        cwd: isolatedCwd,
        env: {},
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
      });

      proc.stdin.write(frame({
        jsonrpc: "2.0",
        id: 1,
        method: "initialize",
        params: { protocolVersion: "2024-11-05" },
      }));
      proc.stdin.write(frame({
        jsonrpc: "2.0",
        method: "notifications/initialized",
      }));
      proc.stdin.write(frame({
        jsonrpc: "2.0",
        id: 2,
        method: "tools/list",
        params: {},
      }));
      proc.stdin.write(frame({
        jsonrpc: "2.0",
        id: 3,
        method: "prompts/list",
        params: {},
      }));
      proc.stdin.write(frame({
        jsonrpc: "2.0",
        id: 4,
        method: "ping",
        params: {},
      }));
      proc.stdin.end();

      const stdout = await new Response(proc.stdout).text();
      const stderr = await new Response(proc.stderr).text();
      const exitCode = await proc.exited;

      expect(exitCode).toBe(0);
      expect(stderr).toBe("");

      const responses = parseResponses(stdout);
      expect(responses).toHaveLength(4);
      expect(responses[0]).toMatchObject({
        jsonrpc: "2.0",
        id: 1,
        result: {
          protocolVersion: "2024-11-05",
          capabilities: {
            tools: {},
            prompts: {},
          },
        },
      });
      expect((responses[1].result as { tools: Array<{ name: string }> }).tools.map((tool) => tool.name)).toEqual([
        "dtrack_list_projects",
        "dtrack_get_project",
        "dtrack_get_findings",
        "dtrack_detect_project",
      ]);
      expect((responses[2].result as { prompts: Array<{ name: string }> }).prompts.map((prompt) => prompt.name)).toEqual([
        "dtrack-triage",
      ]);
      expect(responses[3]).toMatchObject({
        jsonrpc: "2.0",
        id: 4,
        result: {},
      });
    } finally {
      await rm(isolatedCwd, { recursive: true, force: true });
    }
  });
});
