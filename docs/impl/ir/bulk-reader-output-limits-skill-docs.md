---
id: "IMPL-DmstmSWJ77ju7HuPDdrM7h"
type: "impl"
status: "completed"
title: "bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化"
created: "2026-10-17"
updated: "2026-10-17"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "scripts/test/bulk-reader-skill-limits.test.mjs"
    modified:
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/bulk-reader-output-limits-skill-docs.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
    - "docs/designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
    - "docs/specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "docs/adr/document-bulk-reader-output-limits-in-consumer-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# bulk-reader 出力上限・INVALID_OUTPUT 指針の skill 文書化

## Summary

Issue #108 の文書化対応。`cheaper-bulk-read` skill に出力フィールド制約表
（`bulk-reader.output.schema.json` の maxLength/maxItems/enum を網羅、
closed-object・required の注記付き）と `INVALID_OUTPUT` 回復指針
（`errors`/`rawOutput` の意味、scope 縮小・`repairAttempts` 引き上げ・
直接 read フォールバックの順序）を追加した。`cheaper-usage` の exit code
節に worker run 経路の `error.details` 補足を追加した。drift 検知用に
schema 値と skill 記載値を照合する node test を新設した。

## Context

Issue #108 — bulk-reader の出力フィールド制約と `INVALID_OUTPUT` 時の
回復手順が consumer skill へ記載されておらず、caller が制約超過時の
対処を知る術がなかった。schema の制約値を skill 文書として露出し、
drift 検知 test で両者の整合を保証する。

## Implementation

- `cheaper-bulk-read/SKILL.md` — 「Output field limits」表と
  「When it fails with `INVALID_OUTPUT`」節
- `cheaper-usage/SKILL.md` — exit 6 / `INVALID_OUTPUT` の `error.details`
  補足（worker run/alias 経路限定、`workflow run` との shape 差を明記）
- `scripts/test/bulk-reader-skill-limits.test.mjs` — schema JSON ↔
  SKILL.md 表の値照合 + closed-schema/INVALID_OUTPUT 記載の存在確認

## Related Experiments

なし。

## Validation

- `node --test scripts/test/bulk-reader-skill-limits.test.mjs`: 2 pass
- `node --test scripts/test/`: 43 pass（新規 test 含む）
- `node --test benchmarks/test/`: 12 pass
- `mise run check`: 全ステージ pass（biome・fmt・clippy・version・
  doc-id・cargo test・node test・lockfile・payload）

## Risks

- SKILL.md 記載値と schema 制約値の drift は新設の照合 test で検知
  されるが、表にない新規制約の追加時は test 更新も必要。

## Follow-ups

なし(本 task の残件は記録されていない)。
