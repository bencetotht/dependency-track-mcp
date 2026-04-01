import { join } from "node:path";
import type { PackageManager } from "./types.ts";

const LOCKFILE_BY_MANAGER: Record<PackageManager, string> = {
  bun: "bun.lock",
  npm: "package-lock.json",
  pnpm: "pnpm-lock.yaml",
  yarn: "yarn.lock",
};

export async function detectPackageManager(repoPath: string): Promise<PackageManager | null> {
  for (const manager of ["bun", "npm", "pnpm", "yarn"] as const) {
    if (await Bun.file(join(repoPath, LOCKFILE_BY_MANAGER[manager])).exists()) {
      return manager;
    }
  }
  return null;
}

export async function readPackageJson(repoPath: string): Promise<Record<string, unknown>> {
  const file = Bun.file(join(repoPath, "package.json"));
  if (!(await file.exists())) {
    throw new Error(`package.json not found in ${repoPath}`);
  }
  return (await file.json()) as Record<string, unknown>;
}

export function findDependencyType(pkg: Record<string, unknown>, packageName: string): string | null {
  const sections = [
    "dependencies",
    "devDependencies",
    "optionalDependencies",
    "peerDependencies",
  ] as const;
  for (const section of sections) {
    const values = pkg[section] as Record<string, string> | undefined;
    if (values && packageName in values) {
      return section;
    }
  }
  return null;
}

export function buildUpgradeCommand(manager: PackageManager, packageName: string, version: string, dependencyType: string): string[] {
  const spec = `${packageName}@${version}`;
  switch (manager) {
    case "bun":
      return dependencyType === "devDependencies" ? ["bun", "add", "--dev", spec] : ["bun", "add", spec];
    case "npm":
      return dependencyType === "devDependencies" ? ["npm", "install", "--save-dev", spec] : ["npm", "install", spec];
    case "pnpm":
      return dependencyType === "devDependencies" ? ["pnpm", "add", "--save-dev", spec] : ["pnpm", "add", spec];
    case "yarn":
      return dependencyType === "devDependencies" ? ["yarn", "add", "--dev", spec] : ["yarn", "add", spec];
  }
}

export function buildTestCommand(manager: PackageManager, pkg: Record<string, unknown>): string[] | null {
  const scripts = (pkg.scripts || {}) as Record<string, string>;
  if (!scripts.test) {
    return null;
  }
  switch (manager) {
    case "bun":
      return ["bun", "run", "test"];
    case "npm":
      return ["npm", "run", "test"];
    case "pnpm":
      return ["pnpm", "run", "test"];
    case "yarn":
      return ["yarn", "test"];
  }
}

