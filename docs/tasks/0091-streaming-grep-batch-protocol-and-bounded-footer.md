---
id: "TASK-0091"
type: "task"
status: "done"
title: "Streaming grep with batch protocol and bounded footer"
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
    - "docs/plans/0014-bulk-reader-huge-file-safe-io.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0014-bulk-reader-huge-file-safe-io.md"
    - "docs/tasks/0089-shared-limits-line-framer-and-path-only-walker-find.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0019-bulk-reader-huge-file-safe-io.md"
    - "docs/specs/0014-bulk-reader-huge-file-safe-io.md"
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

# Streaming grep with batch protocol and bounded footer

## Work

`grep.ts` を全文 `readFile` + `file` メッセージから、固定 chunk
読み取り + `line-framer` + batch protocol の streaming 評価へ
書き換える（DESIGN-0019）。schema
（`pattern`/`path`/`glob`/`ignoreCase`/`maxResults`/`context`）と
出力フォーマット（`path:line: text`、`path-line- text`、`--`）は
維持する。

### 1. `packages/runtime-pi/src/tools/grep.ts`（書き換え）

データフロー: `walkFiles`（path-only）→ glob → `sniffTextFile`
（binary/unreadable は skip footer に記録）→ `fh.read` chunk
（C = `IO_CHUNK_BYTES`）→ `createLineFramer`（L =
`GREP_MAX_LINE_BYTES`）→ batch → worker eval → ring/pending
context → 出力（≤ O）→ footer。

- `pattern` が `GREP_PATTERN_MAX_CHARS`（4096）超なら invalid
  pattern error（worker 起動前に検査）。
- **batch protocol**: worker source の `file` メッセージを `batch`
  に置き換える。送信 `{type:"batch", id, display, startLine,
  lines: string[]}`、返信 `{type:"result", id, matched: number[]}`
  （batch 内 0-indexed。`startLine` で絶対行番号に写像）。
- batch は `GREP_BATCH_MAX_LINES`（K = 1000）行または
  `GREP_BATCH_MAX_BYTES`（B = 1MiB）の先に到達した方で締める。
  **未応答 batch は常に最大 1**: 送信 → result 待ち → 次 batch。
  eval 待機中は I/O を進めず、受け渡し queue は増えない
  （backpressure）。
- regex 評価は引き続き worker 内のみ。worker 生成に
  `resourceLimits: { maxOldGenerationSizeMb:
  GREP_WORKER_MAX_OLD_SPACE_MB }`（64）を付ける。
- **eval 時間計測**: per-file 予算は同一 file の全 batch の eval
  時間を累積して `GREP_EVAL_PER_FILE_MS`（5000）と比較する
  （batch ごとのリセットはしない）。超過時は worker を
  `terminate()` し、その file を `timeout` skip として記録、後続
  file 用に worker を再生成する。`GREP_EVAL_TOTAL_MS`（60000）超過
  で走査全体を止め `[truncated: evaluation time limit reached]`
  を出す。
- **scan deadline**: 呼び出し開始から `GREP_SCAN_DEADLINE_MS`
  （120000）の wall-clock deadline を設け、walk・I/O・eval 待機の
  すべてを含む。到達時は部分結果 + footer `timeout`（`[truncated:
  scan deadline reached]`）を返す（error にしない）。
  `signal` abort は cleanup 後に `Operation aborted` で reject。
- **cleanup**: `finally` で FileHandle close・worker terminate・
  timer 解除を保証する。timeout・abort・error のいずれでも
  handle/worker が残らない。
- **context と出力**: `context`（0–3）の前 context は直近
  `context` 行の ring buffer（各行 ≤ L）から供給。後 context は
  「あと `context` 行を context として拾う」pending countdown で
  処理する（1 batch in-flight のため eval 待ちで読み取りが止まり、
  後 context 行は次 batch で届く）。block merge 規則（`from <=
  last.to + 1` で結合）は維持し、close 済み block だけを出力へ
  flush する。保持するのは ring 分 + pending 分のみで、全行配列・
  全一致配列は作らない。
- 1 行の表示は `GREP_OUTPUT_LINE_MAX_CHARS`（500）文字 +
  `... [truncated]`。出力全体は `GREP_OUTPUT_MAX_BYTES`（O =
  256KiB）で締め、超過時は `[truncated: output limit reached]` で
  停止する。
- `maxResults`（既定 100・最大 500）到達は従来どおり
  `[truncated: more matches]`（reason `result-cap`）。
- **有界 skip footer**: reason 分類は `binary` / `unreadable` /
  `oversized-line` / `timeout` / `result-cap`。`timeout`・
  `result-cap` は `[truncated: ...]` marker、binary/unreadable/
  oversized-line は `[skipped: <reason> — <path>[, path:line]...]`
  行。oversized-line は `path:行番号` で報告する。各 reason の
  代表例は `SKIP_FOOTER_MAX_ENTRIES`（20）件まで、残りは
  `and N more` の集計。1 path は `SKIP_PATH_MAX_CHARS`（200）文字
  で切り詰め、footer 全体が `SKIP_FOOTER_MAX_BYTES`（S = 4KiB）を
  超えたら `[skipped-report: truncated]` で締める。一致 0 件でも
  skip があれば `(no matches)` に続けて footer を出し、「一致なし」
  と「未検索あり」を区別可能にする。
- **存在秘匿**: deny 対象は walker filter 段で消え、結果・footer・
  `and N more` 件数に一切現れない。直接指定 path の
  boundary/policy 違反は従来どおり tool error で、unreadable と
  して飲み込まない。
- description から "binary files, and files over 1MB" の記述を外し
  path-only 列挙前提の記述に更新する（find と同じ方針）。

### 2. `test/tools/grep.test.ts`（全面更新）

既存 12 case を新 protocol の期待値へ全面的に更新し、赤→緑で進める
（PLAN-0014 Risk #4）。

- 巨大 file: 1MB 超・100MB 以上の text fixture 後半に置いた既知
  一致を返す。
- 小さな C/B へ options 縮退して chunk/batch 境界を跨ぐ行の一致、
  `ignoreCase`・`glob`・`context` ±3・`--` merge を検証。
- oversized-line（> L の行）は footer `oversized-line`（`path:行番号`）
  になり `(no matches)` と区別できる。行番号がずれないこと。
- postMessage spy で in-flight batch ≤1 を構造検証し、batch 内
  0-indexed matched が `startLine` で正しく絶対行番号に写像される
  ことを確認。
- `evalPerFileMs`/`evalTotalMs` 縮退注入で予算が batch 間で累積
  されること（リセットされない）、per-file 超過で worker
  terminate → 当該 file が `timeout` skip → 後続 file 用に worker
  再生成されることを検証。
- `scanDeadlineMs` 縮退注入で deadline 発火 → 部分結果 +
  `[truncated: scan deadline reached]`。wall-clock 実待機に依存する
  assert は避ける。abort で `Operation aborted` reject + handle/
  worker/timer cleanup。
- `[skipped: binary/unreadable/oversized-line]`、
  `[truncated: evaluation time limit reached]`、
  `[truncated: more matches]`、output cap、`and N more`、
  `[skipped-report: truncated]` をそれぞれ区別できることを検証。
- `pattern` > `GREP_PATTERN_MAX_CHARS` で invalid pattern error。
- メモリ instrumentation: spy が記録した最大 chunk/batch/出力が
  C/L/B/O/S 以下であることを構造的に検証し、16MB→100MB 入力増大
  の `heapUsed` delta（目安 4×(C+L+B+O+S)）を補助指標として記録
  （RSS 一回観測は証拠にしない）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0014-bulk-reader-huge-file-safe-io.md`
- `docs/designs/0019-bulk-reader-huge-file-safe-io.md`
- `docs/specs/0014-bulk-reader-huge-file-safe-io.md`

## Dependencies

- `docs/tasks/0089-shared-limits-line-framer-and-path-only-walker-find.md`
  （`limits.ts` 定数・`createLineFramer`・`sniffTextFile`・
  `walkFiles` の `onIssue` を利用）

## Affected Files

- `packages/runtime-pi/src/tools/grep.ts`
- `packages/runtime-pi/test/tools/grep.test.ts`

変更しない file（明示）: `walker.ts`/`find.ts`/`line-framer.ts`/
`limits.ts`（TASK-0089）、`read.ts`/`registry.ts`/`operations.ts`
（TASK-0090）、`registry.test.ts`/`scope-deny.test.ts`
（TASK-0090/0092）、`packages/cli/workers/`、`.apm/hooks/**`。

## Done When

- [ ] `file` メッセージを廃止し `batch` protocol（K 行 / B byte
      締め、in-flight ≤1、batch 内 0-indexed matched）に移行済み
- [ ] per-file/total eval 予算が batch 間で累積され、deadline・
      abort で部分結果または reject + 完全 cleanup になる
- [ ] binary/unreadable/oversized-line/timeout/result-cap が有界
      footer で `(no matches)` と区別可能に報告される
- [ ] deny 対象が結果・footer・件数に一切現れない
- [ ] schema・出力フォーマット（`path:line:`/`path-line-`/`--`）と
      既存 12 case の期待動作が維持される

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi build` が exit 0
- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test` が exit 0
      （grep.test.ts の全面更新を含む）
