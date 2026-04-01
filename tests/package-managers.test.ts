import { describe, expect, test } from "bun:test";
import { findDependencyType } from "../src/lib/package-managers.ts";

describe("findDependencyType", () => {
  test("finds regular dependencies", () => {
    const pkg = {
      dependencies: {
        handlebars: "^4.7.8",
      },
    };
    expect(findDependencyType(pkg, "handlebars")).toBe("dependencies");
  });

  test("finds dev dependencies", () => {
    const pkg = {
      devDependencies: {
        vitest: "^1.0.0",
      },
    };
    expect(findDependencyType(pkg, "vitest")).toBe("devDependencies");
  });
});

