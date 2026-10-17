---
id: "TASK-mCX3oPsAX62vs24tiU1ilI"
type: "task"
status: "done"
title: "consumer skill/guides の magnitude 撤廃・rawOutputPath 追随"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added: []
    modified:
      - type: "file"
        path: ".agents/skills/cheaper-bulk-read/SKILL.md"
      - type: "file"
        path: ".agents/skills/cheaper-code-write/SKILL.md"
      - type: "file"
        path: ".agents/skills/cheaper-doc-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-code-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-doc-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
    - "docs/tasks/worker-schema-magnitude-removal.md"
    - "docs/tasks/invalid-output-raw-output-persistence.md"
  blocks:
    - "docs/tasks/regression-and-final-verification-113-114.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  defers: []
  deferred-by: []
---

# consumer skill/guides の magnitude 撤廃・rawOutputPath 追随

## Work

1. `.agents/skills/cheaper-bulk-read/SKILL.md` と
   `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` —
   `task` の `≤4000` 表記を除去、`INVALID_OUTPUT` 節の canonical
   limits 記述(#108 追加)を構造 contract 説明へ更新し、
   `rawOutputPath` 回収手順を追記。
2. `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` —
   `task` の `≤4000` 表記を除去。
3. `packages/apm/.apm/skills/cheaper-usage/SKILL.md` —
   `INVALID_OUTPUT` details 説明に `rawOutputPath` を追記。
4. `docs/guides/` 配下に schema 上限値の言及があれば更新する
   (調査時点で上限値の明記は見つかっていない — 再確認のみ)。

## Done

- consumer 向け文書が「構造 contract のみ強制」の説明に更新され、
  `INVALID_OUTPUT` 時の file 回収経路が読める。
