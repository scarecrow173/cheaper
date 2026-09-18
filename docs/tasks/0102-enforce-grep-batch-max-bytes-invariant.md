---
id: "TASK-0102"
type: "task"
status: "done"
title: "Enforce grep batchMaxBytes invariant at construction"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/0018-grep-batch-max-bytes-invariant.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0018-grep-batch-max-bytes-invariant.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0023-grep-batch-max-bytes-invariant-enforcement.md"
    - "docs/specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
    - "docs/adr/0021-grep-batch-max-bytes-invariant-reject-at-construction.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Enforce grep batchMaxBytes invariant at construction

## Work

DESIGN-0023 どおり、`grep.ts` に `batchMaxBytes >= 3 * maxLineBytes`
invariant の構築時検証と option JSDoc を追加し、違反注入を使う
`grep.test.ts` の test を整合化する。

### 1. `packages/runtime-pi/src/tools/grep.ts`（修正）

- `createGrepTool(opts)` の冒頭で、既定値適用後の実効値を使って
  検証する:

  ```ts
  const maxLineBytes = opts.maxLineBytes ?? GREP_MAX_LINE_BYTES;
  const batchMaxBytes = opts.batchMaxBytes ?? GREP_BATCH_MAX_BYTES;
  if (batchMaxBytes < 3 * maxLineBytes) {
    throw new Error(
      `grep: batchMaxBytes (${batchMaxBytes}) must be >= 3 * maxLineBytes ` +
        `(${3 * maxLineBytes}); a single decoded line payload must fit in one batch`,
    );
  }
  ```

- `GrepToolOptions.batchMaxBytes` / `maxLineBytes` の JSDoc に
  invariant `batchMaxBytes >= 3 * maxLineBytes`、根拠（invalid
  UTF-8 byte → U+FFFD 3 byte 展開で decoded payload は最大
  `3 * maxLineBytes`）、違反は構築時に reject されることを追記する。
- `execute` 内の既存解決コード・batch 締めロジック・既定値・
  framer は変更しない。

### 2. `packages/runtime-pi/test/tools/grep.test.ts`（修正）

- `closes a batch before adding a line that would exceed
  batchMaxBytes`: `tool({ batchMaxBytes: 10, batchMaxLines: 100 })`
  を `tool({ maxLineBytes: 4, batchMaxBytes: 12, batchMaxLines: 100 })`
  に修正する。行 fixture はそのまま（最大 4 byte 行）。
  `bytes <= 12` と `startLine` `[1, 4]` の期待を維持するため、
  byte 上限の assertion を `toBeLessThanOrEqual(12)` に合わせる。
- `still sends a single line whose payload exceeds batchMaxBytes
  as a one-line batch`: 違反設定が構築できなくなるため、構築時
  reject を検証する test に置き換える。
  `expect(() => tool({ batchMaxBytes: 4, maxLineBytes: 100 })).toThrow(...)`
  と、`batchMaxBytes` のみ小さい場合
  （`tool({ batchMaxBytes: 10 })`）も reject されることを確認する。

### 3. 回帰

- `maxLineBytes` のみ注入する既存 test（`maxLineBytes: 10` 等）は
  `B` 既定 1MiB で `3 * 10 < 1MiB` を満たすため無修正で通ることを
  確認する。

## Acceptance

- [x] `grep.ts` に構築時 invariant 検証と JSDoc が追加され、
      batch 締めロジック・既定値は不変。
- [x] 違反 option（`B < 3 * L`）での `createGrepTool` 呼び出しが
      `batchMaxBytes` を含む error message で throw する test が
      ある。
- [x] batch 締め test が invariant 適合 fixture で通る。
- [x] `packages/runtime-pi` の test・typecheck・lint が pass する。

## Result

実施済み（2026-09-18）。`createGrepTool` 冒頭に `B >= 3 * L` の構築時
検証を追加し、両 option の JSDoc に invariant と根拠を明記。batch
締めロジック・既定値・framer は不変（diff で確認: batch 関連行
0 変更）。`grep.test.ts`: batch 締め test は `maxLineBytes: 4` +
`batchMaxBytes: 12` の適合 fixture へ、単独行 over-B test は構築時
reject 検証（2 ケース + 境界値受理）へ置き換え。runtime-pi 255 test
全 pass、typecheck・lint clean。
