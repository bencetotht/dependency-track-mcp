import { mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";

export function fail(message: string): never {
  throw new Error(message);
}

export function expandHome(input: string): string {
  if (input === "~") {
    return process.env.HOME || input;
  }
  if (input.startsWith("~/")) {
    return join(process.env.HOME || "~", input.slice(2));
  }
  return input;
}

export async function ensureDir(path: string): Promise<void> {
  await mkdir(path, { recursive: true });
}

export async function execCommand(
  argv: string[],
  options: {
    cwd?: string;
    env?: Record<string, string | undefined>;
    quiet?: boolean;
  } = {},
): Promise<{ stdout: string; stderr: string; exitCode: number }> {
  const proc = Bun.spawn(argv, {
    cwd: options.cwd,
    env: {
      ...process.env,
      ...(options.env || {}),
    },
    stdout: "pipe",
    stderr: "pipe",
  });
  const stdout = await new Response(proc.stdout).text();
  const stderr = await new Response(proc.stderr).text();
  const exitCode = await proc.exited;
  if (!options.quiet && exitCode !== 0) {
    throw new Error(`Command failed (${argv.join(" ")}): ${stderr || stdout}`);
  }
  return { stdout, stderr, exitCode };
}

export function sortByDateDesc<T>(items: T[], getValue: (item: T) => string | number | null | undefined): T[] {
  return [...items].sort((a, b) => {
    const left = getValue(a);
    const right = getValue(b);
    const leftTs = left ? new Date(left).getTime() : 0;
    const rightTs = right ? new Date(right).getTime() : 0;
    return rightTs - leftTs;
  });
}

export function severityRank(severity?: string): number {
  switch ((severity || "").toUpperCase()) {
    case "CRITICAL":
      return 5;
    case "HIGH":
      return 4;
    case "MEDIUM":
      return 3;
    case "LOW":
      return 2;
    default:
      return 1;
  }
}

export function slugify(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "")
    .slice(0, 48);
}

export async function writeTextFile(path: string, content: string): Promise<void> {
  await ensureDir(dirname(path));
  await Bun.write(path, content);
}

export function printOutput(payload: unknown, asJson: boolean): void {
  if (asJson) {
    console.log(JSON.stringify(payload, null, 2));
    return;
  }
  console.log(typeof payload === "string" ? payload : JSON.stringify(payload, null, 2));
}

