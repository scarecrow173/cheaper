---
id: "IMPL-034r05iHy2gdnpwO94elRp"
type: "impl"
status: "completed"
title: "grep batchMaxBytes invariant enforcement"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/tools/grep.ts"
      - type: "file"
        path: "packages/runtime-pi/test/tools/grep.test.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/enforce-grep-batch-max-bytes-invariant.md"
    - "docs/tasks/grep-batch-invariant-final-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/grep-batch-max-bytes-line-payload-invariant.md"
    - "docs/adr/grep-batch-max-bytes-invariant-reject-at-construction.md"
    - "docs/designs/grep-batch-max-bytes-invariant-enforcement.md"
    - "docs/plans/grep-batch-max-bytes-invariant.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/grep-batch-max-bytes-line-payload-invariant.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# grep batchMaxBytes invariant enforcement

## Summary

sandboxed `grep` tool の暗黙の invariant `batchMaxBytes >= 3 * maxLineBytes`
を明文化し、`createGrepTool` 構築時に違反を reject するようにした
（issue #14、PR #6 review の non-blocker 解消）。1 行の decoded payload は
invalid UTF-8 byte の U+FFFD 展開（1 byte → 3 byte）により最大
`3 * maxLineBytes` に達するため、この条件が満たされれば「1 batch ≤ B」が
単独行を含めて厳密に保たれる。

## Context

TASK-034r05iHQeQpDqzPOrhXsY / TASK-034r05iHQeQpDqAVvmTPpC（PLAN-034r05iHE1Fbkp8emdVve6 / SPEC-034r05iHK8bA7gqGaD6qpk / ADR-034r05iHrZUE6WgelFx89a / DESIGN-034r05iHy2gdnoS2PWvNo0）
に従う。branch: `feature/grep-batch-max-bytes-invariant`。

## Implementation

### Changes

- `packages/runtime-pi/src/tools/grep.ts`:
  - `createGrepTool` 冒頭に invariant 検証を追加。既定値適用後の実効値
    （`opts.maxLineBytes ?? GREP_MAX_LINE_BYTES`、
    `opts.batchMaxBytes ?? GREP_BATCH_MAX_BYTES`）で
    `B < 3 * L` を検出したら `Error` を throw。`execute` 内の既存解決
    コードと同じ既定適用式を使うため解釈は一致する。
  - `GrepToolOptions.maxLineBytes` / `batchMaxBytes` の JSDoc に
    invariant と `3 *` の根拠（U+FFFD 3 byte 展開）、違反時の構築時
    reject を明記。
  - batch 締めロジック・既定値・framer は不変（diff で batch 関連行
    0 変更を確認）。単独行 over-B の forward-progress 経路は防御コード
    として維持。
- `packages/runtime-pi/test/tools/grep.test.ts`:
  - `closes a batch before adding a line that would exceed batchMaxBytes`:
    `batchMaxBytes: 10` 単独注入（既定 L=256KiB で違反）→
    `maxLineBytes: 4` + `batchMaxBytes: 12`（`= 3 * 4`、invariant 適合）
    へ修正。byte 上限 assertion を `<= 12` に合わせ、batch 分割
    `startLine [1, 4]` の検証を維持。
  - `still sends a single line whose payload exceeds batchMaxBytes ...`:
    違反設定が構築できなくなったため、構築時 reject を検証する test
    `rejects tool construction when batchMaxBytes < 3 * maxLineBytes`
    に置き換え（違反 2 ケース + 境界値 `B = 3 * L` 受理）。

### Decisions During Implementation

- **reject を選択**（記録のみ/clamp ではなく）: issue の懸念は「test-only
  注入の production 格上げ時に invariant が暗黙のまま残る」こと。
  fail-fast の構築時 reject がこのリスクを直接解消する（ADR-034r05iHrZUE6WgelFx89a）。
- **検証位置は `createGrepTool`**: option は構築時に固定されるため
  execute 内ではなく構築時に検査。`execute` の解決式と同じ既定適用を
  使うため不整合なし。
- **防御コードは維持**: `batchLines.length > 0` ガードは invariant
  適合時も batch 締め判定に必要な条件であり、将来の変更で invariant が
  再び破られた場合の forward-progress 保護として残す。

## Related Experiments

### Adopted

- invariant の構築時 fail-fast 化（option の production 格上げに備える）

### Rejected

- clamp（`batchMaxBytes = max(B, 3*L)`）: 注入者の意図を黙って変え、
  test が観測したい batch 締め条件を破壊するため不採用（ADR-034r05iHrZUE6WgelFx89a）。

## Validation

- `pnpm --filter @cheaper/runtime-pi test`: 255 tests 全 pass
- `pnpm typecheck`: pass
- `pnpm lint`: pass（131 files、warning のみ）
- `audit_docs.js --type all`: 183 files、no findings
- `git diff` 構造確認: batch 締め・既定値・`limits.ts` 変更なし、
  `.apm/hooks/cli` diff 空（bundle 非影響）

## Risks

- なし。

## Follow-ups

- なし。
