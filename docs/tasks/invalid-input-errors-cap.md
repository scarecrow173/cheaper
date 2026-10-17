---
id: "TASK-Y48pGJwcxZmcMbiClyRyd5"
type: "task"
status: "done"
title: "INVALID_INPUT details.errors の 20 件 cap 揃え"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/worker/schema/mod.rs"
      - type: "file"
        path: "crates/cheaper-core/src/worker/validate.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/repair.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
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

# INVALID_INPUT details.errors の 20 件 cap 揃え

## Work

1. `crates/cheaper-core/src/worker/validate.rs` の
   `validate_worker_input` が `details.errors` を無制限に serialize
   しているのを、`INVALID_OUTPUT` の `MAX_DETAIL_ERRORS`(20 件)と
   同じ cap に揃える。
2. unit test: 21 件超の構造違反を持つ入力で `details.errors` が
   20 件に収まること。構造違反の instancePath が details に残ること
   (既存 pin を維持)。

## Done

- `INVALID_INPUT`/`INVALID_OUTPUT` 両経路で `details.errors` の
  診断面が一致する。
