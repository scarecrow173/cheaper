---
id: "TASK-034r05iHQeQpDqj5TapRlr"
type: "task"
status: "done"
title: "Shared limits, line framer, and path-only walker/find"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/bulk-reader-huge-file-safe-io.md"
  implemented-by: []
  depends-on:
    - "docs/plans/bulk-reader-huge-file-safe-io.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/bulk-reader-huge-file-safe-io.md"
    - "docs/specs/bulk-reader-huge-file-safe-io.md"
    - "docs/adr/bulk-reader-huge-file-safe-io.md"
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

# Shared limits, line framer, and path-only walker/find

## Work

PLAN-034r05iHE1Fbkp3qMZZB17 / DESIGN-034r05iHy2gdnoNaWH2bos の基盤層。共有上限定数と行フレーマーを新規
作成し、walker を path 列挙専念化、find を path-only + footer に
更新する。本 task の成果物（`limits.ts`・`line-framer.ts`・
`sniffTextFile`・`onIssue`）は TASK-034r05iHQeQpDqkDLrHBxb・TASK-034r05iHQeQpDqlvQrRhaE が利用する。

### 1. `packages/runtime-pi/src/tools/limits.ts`（新規）

DESIGN-034r05iHy2gdnoNaWH2bos「共有上限定数」の表をそのまま export する。すべて入力
ファイル総 byte 数 N と独立した有限値。呼び出し引数で引き上げ
られない（各 tool schema の `maximum` で固定）こと、test 用に
`createXxxTool` の options 経由でのみ縮退できること（既存
`evalPerFileMs` と同じ注入パターン）を前提に定数化する。

| 定数 | 値 | 用途 |
| --- | --- | --- |
| `IO_CHUNK_BYTES` (C) | `64 * 1024` | `FileHandle.read` chunk size。read/grep/sniff 共通 |
| `SNIFF_BYTES` | `8 * 1024` | text/binary 判定の先頭読み取り（walker から移動） |
| `READ_MAX_BYTES` (R) | `50 * 1024` | read 1 call の本文 byte 上限 |
| `READ_DEFAULT_LIMIT` | `2000` | read `limit` 省略時 |
| `READ_MAX_LIMIT` | `2000` | read `limit` 最大値 |
| `GREP_MAX_LINE_BYTES` (L) | `256 * 1024` | grep が評価する 1 行の byte 上限 |
| `GREP_BATCH_MAX_BYTES` (B) | `1024 * 1024` | eval worker へ送る 1 batch の行テキスト合計上限 |
| `GREP_BATCH_MAX_LINES` (K) | `1000` | 1 batch の行数上限 |
| `GREP_PATTERN_MAX_CHARS` | `4096` | `pattern` 文字列長上限（超過は invalid pattern error） |
| `GREP_EVAL_PER_FILE_MS` | `5000` | per-file 評価予算（batch 間で累積） |
| `GREP_EVAL_TOTAL_MS` | `60000` | total 評価予算（batch でリセットしない） |
| `GREP_SCAN_DEADLINE_MS` | `120_000` | 走査全体の wall-clock deadline |
| `GREP_OUTPUT_LINE_MAX_CHARS` | `500` | 出力 1 行の表示文字数上限 |
| `GREP_OUTPUT_MAX_BYTES` (O) | `256 * 1024` | grep 出力本文の総 byte 上限 |
| `SKIP_FOOTER_MAX_ENTRIES` | `20` | skip 理由ごとの代表 path 件数 |
| `SKIP_PATH_MAX_CHARS` | `200` | footer 内 1 path の表示文字数 |
| `SKIP_FOOTER_MAX_BYTES` (S) | `4 * 1024` | skip footer 全体の byte 上限 |
| `GREP_WORKER_MAX_OLD_SPACE_MB` | `64` | eval worker `resourceLimits.maxOldGenerationSizeMb` |

### 2. `packages/runtime-pi/src/tools/line-framer.ts`（新規）

```ts
createLineFramer({
  maxLineBytes: L,
  onLine: (line: string, lineNo: number, rawBytes: number) => void,
  onOversized: (lineNo: number) => void,
}): { feed(buf: Buffer): void; flush(): void }
```

- raw Buffer 上で `0x0A` を走査して行を切り出す（`0x0A` は UTF-8
  multibyte sequence に現れないため byte 走査は UTF-8 安全）。
  行 byte 長は decode 前の raw byte で計測する。
- 行が `maxLineBytes` を超えた時点で蓄積を止め、次の `\n` まで
  破棄モードに入る（保持量 O(C) のまま）。`onOversized(lineNo)`
  を発行し、その行番号は消費する（後続行の番号がずれない）。
- `flush()` は EOF の未終端行を 1 行として emit する。
- `\r` は行内容に残す（既存 `split("\n")` との parity）。

### 3. `packages/runtime-pi/src/tools/walker.ts`

責務を「許可された path の列挙」と「text eligibility」に分離する。

- `walkFiles(opts, startAbs, onIssue?)` を列挙専念化:
  `MAX_FILE_BYTES` / `isEligibleFile` / `isBinaryFile` を yield
  判定から除去し、size・内容に関係なく通常ファイルを yield する。
- `onIssue?: (issue: WalkIssue) => void` で列挙失敗を通知する。
  `WalkIssue = { display: string; reason: "unreadable" }`。
  `stat`/`readdir`/`realpath` 失敗（dangling symlink 含む）は
  silent return ではなく `onIssue` に流す。
- `sniffTextFile(real): Promise<"text" | "binary" | "unreadable">`
  を export する。`open` + 先頭 `SNIFF_BYTES` 読み取り + NUL 検査
  の bounded 判定で、size 判定は持たない。open/read 失敗は
  `"unreadable"` を返す（fail closed だが理由を区別する）。
- 維持するもの: `.git`/`node_modules` skip（`SKIP_DIRS`）、symlink
  policy（`deny`/`within-workspace`）、denylist filter、祖先
  チェーンによる cycle 防止、開始 path の `resolveWorkspacePath`
  失敗は従来どおり throw（footer 化しない）。
- deny 対象は filter 段で消え `onIssue` にも載らない（存在秘匿。
  deny 件数を集計する経路は作らない）。

### 4. `packages/runtime-pi/src/tools/find.ts`

- path-only 化後の `walkFiles` をそのまま使い、内容の
  `open`/`read`/`sniff` を一切行わない。
- `onIssue` を受けて `[skipped: unreadable — <path>]` 系の footer
  を出力末尾に付ける（DESIGN-034r05iHy2gdnoNaWH2bos 有界 skip footer。件数・文字数・
  byte cap は §1 の定数に従う）。
- glob 照合・`maxResults`（既定 200・最大 1000）・
  `[truncated: more results]` marker は維持。
- description から "binary files, and files over 1MB" を外し、
  "lists all allowed file paths regardless of size or content" 系に
  更新する。

### 5. テスト

- `test/tools/line-framer.test.ts`（新規）: chunk 跨ぎの行復元、
  L 超過行の破棄と `onOversized(lineNo)`、行番号消費（後続行番号が
  ずれない）、`flush()` の未終端行 emit、`\r` 残留。
- `test/tools/walker.test.ts`（更新）: `skips files larger than 1MB`
  / `skips binary files` は walker の責務ではなくなるため、巨大・
  binary file を **yield する** 期待に書き換える（eligibility 検証は
  責務移動先の grep 側 = TASK-034r05iHQeQpDqlvQrRhaE の `grep.test.ts` が担う）。
  `onIssue`（unreadable 注入で `WalkIssue` が届く）と
  `sniffTextFile` の 3 分岐（text/binary/unreadable）を追加する。
- `test/tools/find.test.ts`（更新）: 1MB 超・binary file を列挙に
  含めること、`fs/promises` の `open`/`readFile` spy で対象 file の
  内容 I/O が一度も呼ばれないこと、`onIssue` 注入時の
  `[skipped: unreadable — ...]` footer を検証する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/bulk-reader-huge-file-safe-io.md`
- `docs/designs/bulk-reader-huge-file-safe-io.md`
- `docs/specs/bulk-reader-huge-file-safe-io.md`
- `docs/adr/bulk-reader-huge-file-safe-io.md`

## Dependencies

- PLAN-034r05iHE1Fbkp3qMZZB17 のみ。後続 task への提供物: `limits.ts` の定数、
  `line-framer.ts`、`walkFiles` の `onIssue`、`sniffTextFile`。

## Affected Files

- `packages/runtime-pi/src/tools/limits.ts`（新規）
- `packages/runtime-pi/src/tools/line-framer.ts`（新規）
- `packages/runtime-pi/src/tools/walker.ts`
- `packages/runtime-pi/src/tools/find.ts`
- `packages/runtime-pi/test/tools/line-framer.test.ts`（新規）
- `packages/runtime-pi/test/tools/walker.test.ts`
- `packages/runtime-pi/test/tools/find.test.ts`

変更しない file（明示）: `packages/runtime-pi/src/tools/grep.ts`
（TASK-034r05iHQeQpDqlvQrRhaE）、`read.ts`/`registry.ts`/`operations.ts`（TASK-034r05iHQeQpDqkDLrHBxb）、
`count-tokens.ts`、`packages/cli/workers/`、`.apm/hooks/**`。

## Done When

- [ ] `limits.ts` が DESIGN-034r05iHy2gdnoNaWH2bos の 17 定数を確定値どおり export し、
      他 module が同じ上限を別表現で持たない（表現ドリフトなし）
- [ ] `line-framer.ts` が raw Buffer `0x0A` 走査・oversized 破棄
      （行番号消費あり）・`flush()` 未終端 emit を実装する
- [ ] `walkFiles` が size・内容に関係なく通常 file を yield し、
      `onIssue`/`sniffTextFile` が contract どおり動く。deny 対象は
      結果にも `onIssue` にも出ない
- [ ] `find` が巨大/binary file を内容 I/O なしで列挙し、
      unreadable を footer に報告する。description が更新済み

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi build` が exit 0
- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test` が exit 0
      （line-framer/walker/find の新規・更新テストを含む）
- [ ] `grep -n "GREP_SCAN_DEADLINE_MS" packages/runtime-pi/src/tools/limits.ts`
      が定数定義行を返す
