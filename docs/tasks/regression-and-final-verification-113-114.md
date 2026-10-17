---
id: "TASK-bFmQKy8wlXPsJ8TxH83XGQ"
type: "task"
status: "done"
title: "issue-113/114 regression normalize・最終検証・impl 記録"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/worker-schema-structure-not-magnitude.md"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/normalize.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-repair-exhausted.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-repair-success.json"
      - type: "file"
        path: "scripts/test/bulk-reader-skill-limits.test.mjs"
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
    - "docs/tasks/invalid-input-errors-cap.md"
    - "docs/tasks/worker-prose-maxfindings-null.md"
    - "docs/tasks/consumer-docs-schema-magnitude.md"
  blocks: []
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

# issue-113/114 regression normalize・最終検証・impl 記録

## Work

1. `crates/cheaper-cli/tests/regression/normalize.rs` に
   `details.rawOutputPath` の scrub rule を追加(pid/nanos 非決定 —
   `details.checkpoint.path` scrub と同型の新規 rule)。
2. INVALID_OUTPUT を含む golden/regression fixture の expected を
   追随させる。
3. `docs/impl/` に implementation record(`IMPL-Dc1ijUuRtOtwRZigVnv16d`)
   を作成し、実装内容・検証結果・発見事項(stale materialized
   builtin dir の version-keyed cache 問題)を記録する。
4. `mise run check` 全段・workspace test(dev-seam feature 付き)・
   doc-id check・index 整合を pass させる。
5. stale builtin materialize の問題を GitHub issue として起票する
   (本 PR scope 外の発見)。

## Done

- 全検証が実施・pass し、impl record と index が整合する。
