import { describe, expect, it } from "vitest";
import type { DevinCli } from "../../../src/providers/devin/cli.js";
import {
  type DevinCatalog,
  FALLBACK_MODELS,
  loadCliCatalog,
  modelsFromCatalog,
  resolveModelUid,
} from "../../../src/providers/devin/models.js";

function cliReturning(stdout: string, code = 0, stderr = ""): DevinCli {
  return {
    bin: "devin",
    run: () => Promise.resolve({ stdout, stderr, code }),
    version: () => Promise.resolve(null),
  };
}

const CATALOG: DevinCatalog = {
  families: [
    {
      family_label: "Claude Opus 5",
      family_uid: "claude-opus-5",
      slug: "claude-opus-5",
      variants: [
        {
          model_uid: "claude-opus-5-low",
          label: "Low",
          max_context_tokens: 1_000_000,
          max_output_tokens: 64_000,
          cost_summary: "$5.00 / MTok In · $25.00 / MTok Out",
        },
        {
          model_uid: "claude-opus-5-high",
          label: "High",
          max_context_tokens: 1_000_000,
          max_output_tokens: 128_000,
          cost_summary: "$5.00 / MTok In · $25.00 / MTok Out",
        },
        {
          model_uid: "claude-opus-5-high-priority",
          label: "High (priority)",
          max_context_tokens: 1_000_000,
          max_output_tokens: 128_000,
        },
      ],
    },
    {
      family_label: "SWE",
      family_uid: "swe-1-7",
      slug: "swe",
      variants: [
        {
          model_uid: "swe-1-7",
          label: "SWE-1.7",
          max_context_tokens: 262_000,
          max_output_tokens: 32_000,
        },
      ],
    },
  ],
};

describe("modelsFromCatalog", () => {
  it("falls back to the bootstrap list for null/empty catalogs", () => {
    expect(modelsFromCatalog(null)).toBe(FALLBACK_MODELS);
    expect(modelsFromCatalog({ families: [] })).toBe(FALLBACK_MODELS);
  });

  it("maps families to Pi models with thinking level maps", () => {
    const models = modelsFromCatalog(CATALOG);
    expect(models).toHaveLength(2);

    const opus = models[0];
    // reasoning family の Pi-facing id は family id（variant uid ではない）
    expect(opus?.id).toBe("claude-opus-5");
    expect(opus?.name).toBe("Claude Opus 5");
    expect(opus?.reasoning).toBe(true);
    expect(opus?.thinkingLevelMap?.low).toBe("claude-opus-5-low");
    expect(opus?.thinkingLevelMap?.high).toBe("claude-opus-5-high");
    expect(opus?.contextWindow).toBe(1_000_000);
    expect(opus?.maxTokens).toBe(128_000);
    expect(opus?.cost.input).toBe(5);
    expect(opus?.cost.output).toBe(25);
  });

  it("marks levels the family does not ship as explicit null", () => {
    const models = modelsFromCatalog(CATALOG);
    const map = models[0]?.thinkingLevelMap;
    // catalog が low/high のみ持つ family では、未対応 level は
    // undefined ではなく null — Pi が「対応あり」と誤認しない
    expect(map?.off).toBeNull();
    expect(map?.minimal).toBeNull();
    expect(map?.medium).toBeNull();
    expect(map?.xhigh).toBeNull();
    expect(map?.max).toBeNull();
  });

  it("filters priority/fast variants from the thinking map source", () => {
    const models = modelsFromCatalog(CATALOG);
    const levels = Object.values(models[0]?.thinkingLevelMap ?? {});
    expect(levels).not.toContain("claude-opus-5-high-priority");
  });

  it("marks single-variant families as non-reasoning", () => {
    const models = modelsFromCatalog(CATALOG);
    const swe = models[1];
    expect(swe?.id).toBe("swe-1-7");
    expect(swe?.reasoning).toBe(false);
    expect(swe?.contextWindow).toBe(262_000);
  });
});

describe("loadCliCatalog", () => {
  it("parses `devin models list --format json` output", async () => {
    const cli = cliReturning(JSON.stringify(CATALOG));
    const catalog = await loadCliCatalog(cli);
    expect(catalog.families).toHaveLength(2);
  });

  it("invokes the exact catalog argv", async () => {
    let seen: string[] | undefined;
    const cli: DevinCli = {
      bin: "devin",
      run: (args) => {
        seen = args;
        return Promise.resolve({ stdout: '{"families":[]}', stderr: "", code: 0 });
      },
      version: () => Promise.resolve(null),
    };
    await loadCliCatalog(cli);
    expect(seen).toEqual(["models", "list", "--format", "json"]);
  });

  it("surfaces stderr on non-zero exit", async () => {
    const cli = cliReturning("", 1, "not authenticated");
    await expect(loadCliCatalog(cli)).rejects.toThrow(/not authenticated/);
  });

  it("rejects malformed JSON", async () => {
    const cli = cliReturning("{not json");
    await expect(loadCliCatalog(cli)).rejects.toThrow(/malformed JSON/);
  });

  it("rejects a catalog without a families array", async () => {
    const cli = cliReturning('{"unexpected": true}');
    await expect(loadCliCatalog(cli)).rejects.toThrow(/malformed catalog/);
  });
});

describe("resolveModelUid", () => {
  const map = { low: "m-low", high: "m-high" };

  it("maps the requested reasoning level", () => {
    expect(resolveModelUid("m", map, "low")).toBe("m-low");
  });

  it("falls back to the preferred default level", () => {
    expect(resolveModelUid("m", map, undefined)).toBe("m-high");
    expect(resolveModelUid("m", map, "xhigh")).toBe("m-high");
  });

  it("returns the model id unchanged without a map", () => {
    expect(resolveModelUid("m", undefined, "high")).toBe("m");
  });

  it("maps the family id to the wire variant uid", () => {
    const models = modelsFromCatalog(CATALOG);
    const opus = models[0];
    // Pi-facing id（family id）が wire 上の variant UID に解決される
    expect(resolveModelUid(opus?.id ?? "", opus?.thinkingLevelMap, "low")).toBe(
      "claude-opus-5-low",
    );
    expect(resolveModelUid(opus?.id ?? "", opus?.thinkingLevelMap, "high")).toBe(
      "claude-opus-5-high",
    );
  });

  it("falls back to the preferred default for explicitly-null levels", () => {
    const models = modelsFromCatalog(CATALOG);
    const opus = models[0];
    // null level は Pi 側で選択不可になる前提の defensive fallback —
    // wire に流す場合も存在しない variant UID にはならない
    expect(resolveModelUid(opus?.id ?? "", opus?.thinkingLevelMap, "off")).toBe(
      "claude-opus-5-high",
    );
  });
});
