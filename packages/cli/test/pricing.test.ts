import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { loadPricingEntry, PRICING_STALE_MS } from "../src/pricing.js";

const NOW = Date.parse("2026-09-19T00:00:00.000Z");
const entry = {
  inputPerMillionUsd: 3,
  outputPerMillionUsd: 15,
  checkedAt: "2026-09-18T00:00:00.000Z",
  source: "test",
};

function write(dir: string, body: unknown): void {
  fs.writeFileSync(path.join(dir, "pricing.json"), JSON.stringify(body));
}

describe("loadPricingEntry", () => {
  let dir: string;
  beforeEach(() => {
    dir = fs.mkdtempSync(path.join(os.tmpdir(), "pricing-"));
  });
  afterEach(() => {
    fs.rmSync(dir, { recursive: true, force: true });
  });

  it("returns null when pricing.json does not exist", () => {
    expect(loadPricingEntry({ userConfigDir: dir, model: "a/b" })).toBeNull();
  });

  it("returns null when pricing.json is malformed", () => {
    fs.writeFileSync(path.join(dir, "pricing.json"), "not json{");
    expect(loadPricingEntry({ userConfigDir: dir, model: "a/b" })).toBeNull();
  });

  it("returns null when the model entry is missing or invalid", () => {
    write(dir, { models: { "other/x": entry, "bad/entry": { inputPerMillionUsd: "x" } } });
    expect(loadPricingEntry({ userConfigDir: dir, model: "a/b" })).toBeNull();
    expect(loadPricingEntry({ userConfigDir: dir, model: "bad/entry" })).toBeNull();
  });

  it("returns a fresh entry as non-stale", () => {
    write(dir, { models: { "openai/gpt-5": entry } });
    const r = loadPricingEntry({ userConfigDir: dir, model: "openai/gpt-5", now: () => NOW });
    expect(r).toEqual({ entry, stale: false });
  });

  it("marks entries older than 30 days as stale", () => {
    const old = { ...entry, checkedAt: "2026-08-01T00:00:00.000Z" };
    write(dir, { models: { "openai/gpt-5": old } });
    const r = loadPricingEntry({ userConfigDir: dir, model: "openai/gpt-5", now: () => NOW });
    expect(NOW - Date.parse(old.checkedAt)).toBeGreaterThan(PRICING_STALE_MS);
    expect(r?.stale).toBe(true);
  });

  it("marks an unparseable checkedAt as stale", () => {
    write(dir, { models: { "openai/gpt-5": { ...entry, checkedAt: "not-a-date" } } });
    expect(
      loadPricingEntry({ userConfigDir: dir, model: "openai/gpt-5", now: () => NOW })?.stale,
    ).toBe(true);
  });
});
