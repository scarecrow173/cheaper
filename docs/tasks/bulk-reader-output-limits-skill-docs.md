---
id: "TASK-pPgGrBzIU7zBVDTCFkV4aV"
type: "task"
status: "done"
title: "bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/108"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  implemented-by: []
  depends-on:
    - "docs/plans/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
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
    - "docs/designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  defers: []
  deferred-by: []
---

# bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化

## Work

1. `packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md` — 「Consuming the
   result」に出力フィールド制約表と `INVALID_OUTPUT` 回復節を追加
   （design §1 の表・文言どおり）
2. `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — exit code 表直後に
   `INVALID_OUTPUT` の `error.details` 補足を追加（worker run/alias 経路
   限定、`workflow run` との差異を明記）
3. `scripts/test/bulk-reader-skill-limits.test.mjs` — schema JSON と
   skill 記載値の照合 test を新規追加

## Done criteria

- 3ファイルの変更が完了し、design の表・文言と一致
- `node --test scripts/test/` で新規 test を含め全 pass
- `mise run check` が pass
