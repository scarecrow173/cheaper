import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { loadPricingSnapshot } from "../src/pricing.js";

let dir: string | null = null;
afterEach(() => {
  if (dir !== null) fs.rmSync(dir, { recursive: true, force: true });
  dir = null;
});

function makeDir(): string {
  dir = fs.mkdtempSync(path.join(os.tmpdir(), "cheaper-pricing-"));
  return dir;
}

function writePricing(body: unknown): void {
  fs.writeFileSync(
    path.join(dir as string, "pricing.json"),
    typeof body === "string" ? body : JSON.stringify(body),
    "utf8",
  );
}

describe("loadPricingSnapshot", () => {
  it("returns null when the file is missing", () => {
    expect(loadPricingSnapshot(makeDir())).toBeNull();
  });

  it("returns null for malformed JSON or missing fields", () => {
    const d = makeDir();
    writePricing("{ not json");
    expect(loadPricingSnapshot(d)).toBeNull();
    writePricing({ version: "2026-09-18" });
    expect(loadPricingSnapshot(d)).toBeNull();
    writePricing({ version: "v", updatedAt: "2026-09-18", models: "nope" });
    expect(loadPricingSnapshot(d)).toBeNull();
  });

  it("parses a snapshot file matching the core PricingSnapshot contract", () => {
    const d = makeDir();
    writePricing({
      version: "2026-09-18",
      updatedAt: "2026-09-18T00:00:00.000Z",
      models: {
        "ollama/qwen3.5:9b": {
          input: 0.1,
          output: 0.3,
          cacheRead: 0.05,
          cacheWrite: 0.2,
          updatedAt: "2026-09-18",
        },
      },
    });
    expect(loadPricingSnapshot(d)).toEqual({
      version: "2026-09-18",
      updatedAt: "2026-09-18T00:00:00.000Z",
      models: {
        "ollama/qwen3.5:9b": {
          input: 0.1,
          output: 0.3,
          cacheRead: 0.05,
          cacheWrite: 0.2,
          updatedAt: "2026-09-18",
        },
      },
    });
  });

  it("skips invalid model entries while keeping valid ones", () => {
    const d = makeDir();
    writePricing({
      version: "v1",
      updatedAt: "2026-09-18",
      models: {
        "a/b": { input: 1, output: 2 },
        "c/d": { input: -1, output: 2 },
        "e/f": { input: 1 },
        "g/h": "garbage",
      },
    });
    expect(loadPricingSnapshot(d)).toEqual({
      version: "v1",
      updatedAt: "2026-09-18",
      models: { "a/b": { input: 1, output: 2 } },
    });
  });
});
