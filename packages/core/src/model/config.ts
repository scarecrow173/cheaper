import { readFile } from "node:fs/promises";
import path from "node:path";
import Ajv2020Ns, { type ValidateFunction } from "ajv/dist/2020.js";
import { parse as parseYaml } from "yaml";
import { CheaperError } from "../errors.js";
import { mergePolicyLayers } from "../policy/merge.js";
import { TIERS, type Tier } from "./tier.js";

export interface ModelConfig {
  /** "<provider>/<modelId>" → tier */
  overrides: Record<string, Tier>;
  /** tier → ["<provider>/<modelId>" | "<provider>/*"] */
  preferences: Partial<Record<Tier, string[]>>;
}

const Ajv2020 = Ajv2020Ns.default;

const TIER_PREFERENCE = { type: "array", items: { type: "string" } };

export const MODELS_CONFIG_SCHEMA = {
  type: "object",
  additionalProperties: false,
  required: ["version"],
  properties: {
    version: { type: "integer", const: 1 },
    overrides: {
      type: "object",
      additionalProperties: { type: "string", enum: [...TIERS] },
    },
    preferences: {
      type: "object",
      additionalProperties: false,
      properties: {
        low: TIER_PREFERENCE,
        medium: TIER_PREFERENCE,
        high: TIER_PREFERENCE,
      },
    },
  },
} as const;

const ajv = new Ajv2020({ strict: true, allErrors: true });
const VALIDATE: ValidateFunction = ajv.compile(MODELS_CONFIG_SCHEMA);

function isPlainObject(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

function validateModelsConfig(raw: unknown, filePath: string): Record<string, unknown> {
  if (!VALIDATE(raw)) {
    throw new CheaperError("INVALID_MODEL_CONFIG", `invalid models config: ${filePath}`, {
      details: {
        path: filePath,
        errors: (VALIDATE.errors ?? []).map((e) => ({
          instancePath: e.instancePath,
          message: e.message ?? "validation error",
        })),
      },
    });
  }
  return raw as Record<string, unknown>;
}

async function readModelsConfig(filePath: string): Promise<Record<string, unknown> | undefined> {
  let text: string;
  try {
    text = await readFile(filePath, "utf8");
  } catch (e) {
    const errno = (e as NodeJS.ErrnoException).code;
    // 未配置（ENOENT/ENOTDIR）のみ skip。その他の読み込み失敗は silent にしない。
    if (errno === "ENOENT" || errno === "ENOTDIR") return undefined;
    throw new CheaperError("INVALID_MODEL_CONFIG", `models config read failed: ${filePath}`, {
      details: { path: filePath, reason: "read-failed", errno },
      cause: e,
    });
  }
  let parsed: unknown;
  try {
    parsed = parseYaml(text);
  } catch (e) {
    throw new CheaperError("INVALID_MODEL_CONFIG", `models yaml parse failed: ${filePath}`, {
      details: { path: filePath, reason: "yaml-parse" },
      cause: e,
    });
  }
  if (!isPlainObject(parsed)) {
    throw new CheaperError("INVALID_MODEL_CONFIG", `models config is not an object: ${filePath}`, {
      details: { path: filePath, reason: "not-object" },
    });
  }
  return validateModelsConfig(parsed, filePath);
}

/**
 * models.yaml（user → project、後勝ち）。merge は policy と同規則
 * （overrides deep merge・preferences.<tier> array 全体 replace）で、
 * `mergePolicyLayers` を共有する。
 */
export async function loadModelConfig(opts: {
  userConfigDir: string;
  projectDir: string;
}): Promise<ModelConfig> {
  let merged: Record<string, unknown> = {};
  for (const filePath of [
    path.join(opts.userConfigDir, "models.yaml"),
    path.join(opts.projectDir, "models.yaml"),
  ]) {
    const raw = await readModelsConfig(filePath);
    if (raw === undefined) continue;
    merged = mergePolicyLayers(merged, raw);
  }
  const overrides = (isPlainObject(merged.overrides) ? merged.overrides : {}) as Record<
    string,
    Tier
  >;
  const prefRaw = isPlainObject(merged.preferences) ? merged.preferences : {};
  const preferences: Partial<Record<Tier, string[]>> = {};
  for (const tier of TIERS) {
    const v = prefRaw[tier];
    if (v !== undefined) preferences[tier] = v as string[];
  }
  return { overrides, preferences };
}
