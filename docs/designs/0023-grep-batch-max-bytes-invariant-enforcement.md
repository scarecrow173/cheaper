---
id: "DESIGN-0023"
type: "design"
status: "approved"
title: "grep batchMaxBytes invariant enforcement"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
    - "../adr/0021-grep-batch-max-bytes-invariant-reject-at-construction.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0019-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0021-grep-batch-max-bytes-invariant-reject-at-construction.md"
  derived-by:
    - "../plans/0018-grep-batch-max-bytes-invariant.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# grep batchMaxBytes invariant enforcement

## Context

SPEC-0018 / ADR-0021 の実装設計。sandboxed `grep`（`tools/grep.ts`）の
`batchMaxBytes`（B）と 1 行の最大 decoded payload（`3 * maxLineBytes`、L）
の invariant `B >= 3 * L` を option 定義に明記し、違反を `createGrepTool`
構築時に reject する。

対象は `packages/runtime-pi/src/tools/grep.ts` と
`packages/runtime-pi/test/tools/grep.test.ts` のみ。外部依存・新規 module・
env 設定は導入しない。

## Scope

### In Scope

- `GrepToolOptions.batchMaxBytes` / `maxLineBytes` の JSDoc に invariant と
  その根拠を明記する。
- `createGrepTool` 冒頭で実効値（既定値適用後）の `B < 3 * L` を検出して
  `Error` を投げる。
- 違反注入を使う既存 test 2 件の整合化と reject 検証 test の追加。

### Out of Scope

- `GREP_MAX_LINE_BYTES` / `GREP_BATCH_MAX_BYTES` の既定値変更。
- 単独行 over-B batch 送出経路（forward-progress 防御コード）の削除。
- 他の上限 option（`batchMaxLines`・eval 予算・deadline・出力 cap）の
  invariant 整備。

## Design

### Invariant の定義

行フレーマーは raw byte を `maxLineBytes` で制限し、確定行を
`raw.toString("utf8")` で decode する。invalid UTF-8 byte は U+FFFD
（UTF-8 で 3 byte）へ置換されるため、decoded payload の最大 byte 数は
`3 * maxLineBytes` となる。`batchMaxBytes >= 3 * maxLineBytes` が成立する
とき、どの 1 行も空 batch に収まり、「1 batch ≤ B」が厳密に保たれる。

### 構築時検証

`createGrepTool(opts)` の冒頭で、既定値適用後の実効値を用いて検証する:

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

`execute` 内で既に解決している `maxLineBytes` / `batchMaxBytes` は
`GrepToolOptions` 固定値から導かれるため、構築時検証と execute 時の解釈は
一致する。`execute` 内の既存解決コードは変更しない。

### Option JSDoc

`batchMaxBytes` と `maxLineBytes` の JSDoc に、invariant
`batchMaxBytes >= 3 * maxLineBytes` と、`3 *` が invalid UTF-8 byte の
U+FFFD 3 byte 展開による decoded payload 最悪値であること、違反は構築時に
reject されることを追記する。

### 防御コードの維持

`drain` 内の `batchLines.length > 0` ガード（空 batch は常に次の行を積む）
は invariant 強制後も残す。forward-progress 保証の防御であり、invariant が
将来の変更で再び破られた場合にも停止しない。

## Test Design

- **reject 検証**: `createGrepTool({ batchMaxBytes: 4, maxLineBytes: 100 })`
  （`4 < 300`）が `batchMaxBytes` を含む error message で throw することを
  確認する test を追加する。`batchMaxBytes` のみ小さい場合
  （`batchMaxBytes: 10`、L は既定 256KiB）も同様に reject されることを
  確認する。
- **既存 test の整合化**:
  - `closes a batch before adding a line that would exceed batchMaxBytes`:
    fixture は行 "aaa"/"bbb"/"cccc"/"ddd"（最大 4 byte）。`maxLineBytes: 4`
    `batchMaxBytes: 12`（`= 3 * 4`）へ修正し、B 締め挙動を維持する。
    期待値: batch 1 が "aaa"+"bbb"+"cccc"（10 byte ≤ 12）、batch 2 が "ddd"。
    `bytes <= 12` の assertion と `startLine` `[1, 4]` は不変。
  - `still sends a single line whose payload exceeds batchMaxBytes ...`:
    違反設定が構築できなくなるため、この test は上記 reject 検証 test に
    置き換える。
- **回帰**: invariant 適合な既存 test（`maxLineBytes: 10` のみ注入する
  oversized 系など、`B` 既定 1MiB で `3 * 10 = 30 < 1MiB` を満たすもの）は
  無修正で通る。

## Verification

- `pnpm typecheck` / `pnpm lint` / `packages/runtime-pi` の test 全件。
- `git diff` で `grep.ts` の batch 締めロジック・既定値・framer に
  変更がないことを確認する。
