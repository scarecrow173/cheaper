// cheaper-bulk-read skill の contract 記述と canonical schema の drift 検知
// (issue #113/#114 — SPEC worker-schema-structure-not-magnitude)。
// schema は magnitude(文字数・件数)を検証せず構造のみを検証する。
// この test は (a) schema に magnitude 制約が再混入しないこと、
// (b) SKILL.md が構造契約と rawOutputPath 回復経路を記載していることを
// guard する。

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { test } from "node:test";

import { repoRoot } from "./lib/apm-fixture.mjs";

const schemaPath = path.join(
  repoRoot,
  "crates",
  "cheaper-cli",
  "resources",
  "workers",
  "schemas",
  "bulk-reader.output.schema.json",
);
const skillPath = path.join(
  repoRoot,
  "packages",
  "apm",
  ".apm",
  "skills",
  "cheaper-bulk-read",
  "SKILL.md",
);

const schema = JSON.parse(readFileSync(schemaPath, "utf8"));
const skill = readFileSync(skillPath, "utf8");

function* walk(node) {
  if (node && typeof node === "object") {
    yield node;
    for (const value of Object.values(node)) {
      yield* walk(value);
    }
  }
}

test("bulk-reader output schema carries no magnitude constraints", () => {
  // payload magnitude 制約(maxLength/maxItems/maximum)は撤廃済み
  // (issue #113/#114) — 再混入を防ぐ。構造制約(minimum/minItems/
  // pattern/required/enum)はこの対象外。
  for (const node of walk(schema)) {
    for (const key of ["maxLength", "maxItems", "maximum"]) {
      assert.equal(
        node[key],
        undefined,
        `magnitude constraint ${key} reappeared at ${JSON.stringify(node).slice(0, 120)}`,
      );
    }
  }
});

test("bulk-read skill documents the structural contract", () => {
  assert.equal(schema.additionalProperties, false);
  assert.match(skill, /additionalProperties:\s*false/);
  assert.match(skill, /INVALID_OUTPUT/);
  assert.match(skill, /rawOutput/);
  // issue #114 — 全量 raw output の file 回収経路。
  assert.match(skill, /rawOutputPath/);
  assert.match(skill, /invalid-outputs/);
  // 構造制約の記載 — confidence enum と workspace-relative path。
  assert.match(skill, /confidence/);
  assert.match(skill, /workspace-relative/);
});
