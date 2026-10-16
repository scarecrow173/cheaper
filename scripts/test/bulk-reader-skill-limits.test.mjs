// cheaper-bulk-read skill の出力上限表と canonical schema の drift 検知
// (issue #108 / SPEC-7GjKOMzR6m3HfBdpPmzSIe)。SKILL.md に記載する上限値が
// bulk-reader.output.schema.json と一致することを検証する — schema 側を
// 変更した場合は skill 文書も追随させるためのガード。

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

const finding = schema.properties.findings.items;
const location = finding.properties.locations.items;

test("bulk-read skill documents the schema's output limits", () => {
  const expectations = [
    // [skill に含まれるべき文字列, schema 由来の実際値]
    ["1500", schema.properties.summary.maxLength],
    ["1200", finding.properties.detail.maxLength],
    ["120", finding.properties.title.maxLength],
    ["600", location.properties.excerpt.maxLength],
    ["30", schema.properties.findings.maxItems],
    ["10", finding.properties.locations.maxItems],
    ["30", schema.properties.relevantFiles.maxItems],
    ["10", schema.properties.suggestedReads.maxItems],
    ["10", schema.properties.openQuestions.maxItems],
    ["300", schema.properties.relevantFiles.items.properties.reason.maxLength],
  ];
  for (const [needle, expected] of expectations) {
    assert.equal(String(expected), needle, `schema value changed for ${needle}`);
    assert.match(
      skill,
      new RegExp(`\\b${needle}\\b`),
      `SKILL.md is missing the ${needle} limit from ${schemaPath}`,
    );
  }
});

test("bulk-read skill states the closed-schema requirement", () => {
  assert.equal(schema.additionalProperties, false);
  assert.match(skill, /additionalProperties:\s*false/);
  assert.match(skill, /600/);
  assert.match(skill, /INVALID_OUTPUT/);
  assert.match(skill, /rawOutput/);
});
