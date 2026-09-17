---
id: "DESIGN-0019"
type: "design"
status: "approved"
title: "Bulk-reader huge-file safe I/O"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
    - "../adr/0017-bulk-reader-huge-file-safe-io.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0014-bulk-reader-huge-file-safe-io.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "./0006-bulk-reader-contract.md"
    - "../discovery/0010-bulk-reader-huge-file-safe-io.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0017-bulk-reader-huge-file-safe-io.md"
  derived-by:
    - "../plans/0014-bulk-reader-huge-file-safe-io.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Bulk-reader huge-file safe I/O

## Context

SPEC-0014 / ADR-0017 の実装設計。ADR-0017 が選定した組み合わせ
（独自 bounded read + 行単位 streaming grep + path-only find +
有界な除外通知）に対し、ADR が後続設計へ委譲した数値・schema・
プロトコル・メモリ予算をすべて確定する。対象は
`packages/runtime-pi/src/tools/` の sandboxed tool 層であり、
外部プロセス・追加依存・新しい env 設定は導入しない。

## Scope

### In Scope

- 新規 `tools/read.ts`（bounded read `defineTool`）と `registry.ts` の
  read mapping 置き換え
- `tools/walker.ts` の path 列挙 / text eligibility 分離
- `tools/grep.ts` の streaming 化（行フレーマー + batch protocol +
  有界 footer）
- `tools/find.ts` の path-only 化と列挙失敗通知
- 共有上限定数モジュール `tools/limits.ts`、tool description と
  bulk-reader worker guidance の更新
- `packages/runtime-pi/test/tools/` の境界・有界性テスト

### Out of Scope

- `count_tokens` の既定 16MB cap、`EditOperations` の全文
  `readFile`、親側 `pre-read-code` hook の block 契約
- bulk-reader の output schema・excerpt 制約（DESIGN-0006 のまま）
- 画像読み取りの代替経路（既定経路から外す。再導入には同じ有界性を
  証明する別契約が必要）
- 依存追加、新しい環境変数、外部プロセス（`rg`/`fd`/shell）

## Components and Boundaries

### 共有上限定数 `packages/runtime-pi/src/tools/limits.ts`（新規）

I/O・出力・報告の上限を一箇所に定義する。すべて入力ファイルの総
byte 数 N と独立した有限値であり、呼び出し引数では上限を超えて
引き上げられない（schema の `maximum` で固定）。テスト用に
`createXxxTool` の options 経由でのみ小さな値へ縮退できる
（既存 `evalPerFileMs` と同じ注入パターン）。

| 定数 | 値 | 用途 |
| --- | --- | --- |
| `IO_CHUNK_BYTES` (C) | `64 * 1024` | FileHandle.read の chunk サイズ。read/grep/sniff 共通 |
| `SNIFF_BYTES` | `8 * 1024` | text/binary 判定の先頭読み取り（walker から移動） |
| `READ_MAX_BYTES` (R) | `50 * 1024` | read 1 call が返す本文 byte 上限（Pi `DEFAULT_MAX_BYTES` 準拠） |
| `READ_DEFAULT_LIMIT` | `2000` | read `limit` 省略時（Pi `DEFAULT_MAX_LINES` 準拠） |
| `READ_MAX_LIMIT` | `2000` | read `limit` の最大値 |
| `GREP_MAX_LINE_BYTES` (L) | `256 * 1024` | grep が評価する 1 行の byte 上限。超過行は未評価で破棄 |
| `GREP_BATCH_MAX_BYTES` (B) | `1024 * 1024` | eval worker へ送る 1 batch の行テキスト合計 byte 上限 |
| `GREP_BATCH_MAX_LINES` (K) | `1000` | 1 batch の行数上限 |
| `GREP_PATTERN_MAX_CHARS` | `4096` | `pattern` の文字列長上限。超過は invalid pattern error |
| `GREP_EVAL_PER_FILE_MS` | `5000` | 既存 per-file 評価予算（維持。batch 間で累積） |
| `GREP_EVAL_TOTAL_MS` | `60000` | 既存 total 評価予算（維持。batch でリセットしない） |
| `GREP_SCAN_DEADLINE_MS` | `120_000` | walk+I/O+行分割+eval 待機を含む走査全体の wall-clock deadline |
| `GREP_OUTPUT_LINE_MAX_CHARS` | `500` | 出力 1 行の表示文字数上限（Pi `GREP_MAX_LINE_LENGTH` 準拠） |
| `GREP_OUTPUT_MAX_BYTES` (O) | `256 * 1024` | grep 出力本文の総 byte 上限 |
| `SKIP_FOOTER_MAX_ENTRIES` | `20` | skip 理由ごとの代表 path 件数 |
| `SKIP_PATH_MAX_CHARS` | `200` | footer 内 1 path の表示文字数 |
| `SKIP_FOOTER_MAX_BYTES` (S) | `4 * 1024` | skip footer 全体の byte 上限 |
| `GREP_WORKER_MAX_OLD_SPACE_MB` | `64` | eval worker の `resourceLimits.maxOldGenerationSizeMb` |

### bounded read `packages/runtime-pi/src/tools/read.ts`（新規）

`registry.ts` の `case "read"` を `createReadToolDefinition` から自前の
`createReadTool` に置き換える。`operations.ts` の `createReadOperations`
は不要になり削除する。境界検証は `count-tokens.ts` と同じ経路、すなわち
`resolveWorkspacePath(root, params.path, symlinkPolicy)` +
`rethrowAsDenied` + `denylist.isDenied(display)` →
`POLICY_DENIED_MESSAGE` をそのまま使う。迂回経路は作らない。

#### パラメータ schema

```ts
const readSchema = Type.Object({
  path: Type.String({ description: "Workspace-relative file path" }),
  offset: Type.Optional(
    Type.Integer({ minimum: 1, description: "1-indexed start line (default 1)" }),
  ),
  limit: Type.Optional(
    Type.Integer({
      minimum: 1,
      maximum: READ_MAX_LIMIT,
      default: READ_DEFAULT_LIMIT,
    }),
  ),
  byteInLine: Type.Optional(
    Type.Integer({
      minimum: 0,
      description:
        "Byte offset within the `offset` line, for continuing an oversized line",
    }),
  ),
});
```

- `offset` は **1-indexed の行番号**（`\n` 区切り、ファイル先頭が 1）。
  `byteInLine` は **その行内の byte 位置**。両者は別の名前空間であり、
  混同しない。
- `byteInLine` は `offset` との同時指定でのみ有効。`offset` 省略時の
  `byteInLine` は tool error `byteInLine requires offset` とする
  （黙った precedence は設けない）。
- `byteInLine` が行の byte 長以上 → tool error
  `byteInLine K is beyond end of line N (M bytes)`。
- `offset` が EOF 超過 → tool error `Offset N is beyond end of file
  (M lines total)`。先行走査が EOF に達した時点で総行数 M は判明済み
  なので追加コストなしで報告できる。
- `byteInLine` が UTF-8 文字境界でない場合は**前方へ**次の境界に
  スナップする（最大 3 byte）。後退は継続の進行性を壊すため行わない。

#### 応答形状

`content` はテキスト 1 ブロック、`details` に構造化情報を載せる。

```ts
interface ReadToolDetails {
  startLine: number;              // 返した先頭行（1-indexed）
  endLine: number;                // 返した末尾行（inclusive）
  eof: boolean;                   // この呼び出しで EOF に到達したか
  truncatedBy: "limit" | "bytes" | null;
  continuation?: { offset: number; byteInLine?: number };
  totalLines?: number;            // eof === true のときのみ
}
```

- 本文は選択行を `\n` で連結した verbatim テキスト。`\r` は除去しない
  （既存の `split("\n")` 意味論と一致）。
- `byteInLine > 0` で再開した場合、先頭に marker 行
  `[partial line N: continuing from byte K]` を付ける。
- 継続が残る場合の trailer（EOF 時は付けない）:
  - 行数 limit 到達: `[Showing lines S-E. Use offset=N to continue.]`
  - byte cap が行境界で到達: `[Reached 50KB limit. Use offset=N to
    continue.]`
  - byte cap が行途中で到達:
    `[Line E emitted bytes A-B (reached 50KB limit). Use offset=E
    byteInLine=C to continue.]`
- **総行数の方針**: `totalLines` は EOF 到達時のみ返す。行数を知るためだけに
  前方走査（O(N) 時間）は行わない。cap 到達と EOF は `details.eof` と
  trailer の有無で必ず区別できる。

#### I/O 手順

1. `open(resolved.real, "r")` → `stat` で `isFile()` を確認
   （directory 等は tool error `read: not a regular file`）。
2. 先頭 chunk を `SNIFF_BYTES` 分だけ走査し NUL を含めば tool error
   `read: binary file (NUL byte within first 8KiB)`。sniff は prefix
   走査で読む最初の chunk を再利用し、追加 I/O は発生しない。
3. 拡張子が `.png .jpg .jpeg .gif .webp .bmp`（大小文字非依存、
   `IMAGE_EXTENSIONS` 定数）なら走査前に tool error
   `read does not support image files`。画像経路・全文 fallback は
   いずれも設けない（ADR-0017）。
4. `offset` 行まで `IO_CHUNK_BYTES` 単位の `FileHandle.read` で
   `\n` を数えながら prefix を破棄（保持量 O(C)）。目的行の開始
   byte 位置を特定したら `byteInLine` を加算・スナップして emit 開始。
5. emit ループは行ごとに「残り byte 予算」を引き、`limit` 行または
   `READ_MAX_BYTES` に達したら継続情報を確定して終了。1 行が残り
   予算を超える場合は収まる分だけ返し `byteInLine` 継続を返す。
   行全体の蓄積は行わず、保持量は O(C + R)。
6. `execute` の第 3 引数 `signal` で abort を受け、`finally` で
   FileHandle を必ず `close()` する。

#### UTF-8 と不正 byte

- decode は `string_decoder` の `StringDecoder("utf8")` を chunk を
  またいで継続させる。改行 byte `0x0A` は multibyte sequence に
  現れないため、行分割は raw Buffer 上で行い、行単位で decode する。
- 不正 byte 列は U+FFFD で置換して返す（欠落させない・黙って全文
  読み込みに fallback しない）。`byteInLine` は常にファイル byte
  基準であり、decode 後の文字列 index ではない。

#### 読み取り中の変更（ caveat ）

継続位置は「呼び出し時点のファイルに対する byte/行座標」であり
snapshot 保証ではない。読み取り間にファイルが短縮されれば
`offset` 超過 error、変更されれば継続位置が別内容を指しうる。
tool description に「changed files may need re-reading from an
earlier offset」と明記する。FileHandle は呼び出しごとに open し、
キャッシュしない。

#### 互換メモ

- Pi read の `resolveReadPathAsync` による綴り変種（`~`・`@`・NFD）
  解決は引き継がない。`path` は workspace 相対として直接
  `resolveWorkspacePath` へ渡す（count_tokens と同じ）。
- Pi read の画像 attachment・`detectImageMimeType` 経路は廃止し、
  上記の明示 error に置き換える。

### walker 分割 `packages/runtime-pi/src/tools/walker.ts`

責務を「許可された path の列挙」と「text eligibility」に分離する。

- `walkFiles(opts, startAbs, onIssue?)`: 列挙専念。
  `MAX_FILE_BYTES` / `isEligibleFile` / `isBinaryFile` を yield 判定から
  外し、size・内容に関係なく通常ファイルを yield する。
  `.git`/`node_modules` skip、symlink policy、denylist、祖先チェーン
  による cycle 防止は維持。
- `onIssue?: (issue: WalkIssue) => void` で列挙失敗を通知する。
  `WalkIssue = { display: string; reason: "unreadable" }`。
  `stat`/`readdir`/`realpath` 失敗（dangling symlink 含む）は
  silent return ではなく onIssue に流す。deny 対象は filter 済みで
  onIssue にも載らない（存在秘匿）。
- `sniffTextFile(real): Promise<"text" | "binary" | "unreadable">` を
  export する。`open` + 先頭 `SNIFF_BYTES` 読み取り + NUL 検査の
  bounded 判定で、size 判定は持たない。open/read 失敗は
  `"unreadable"` を返す（fail closed だが理由を区別する）。
- 開始 path の `resolveWorkspacePath` 失敗は従来どおり throw し、
  footer 化しない（boundary/policy 違反は error 契約）。

### streaming grep `packages/runtime-pi/src/tools/grep.ts`

全文 `readFile` + `file` メッセージを廃止し、固定 chunk 読み取り +
上限付き行フレーマー + batch protocol に置き換える。schema
（`pattern`/`path`/`glob`/`ignoreCase`/`maxResults`/`context`）と
出力フォーマット（`path:line: text`、`path-line- text`、`--`）は維持。
`pattern` が `GREP_PATTERN_MAX_CHARS` 超なら invalid pattern error。

#### 行フレーマー（`tools/line-framer.ts`）

```ts
createLineFramer({
  maxLineBytes: L,
  onLine: (line: string, lineNo: number, rawBytes: number) => void,
  onOversized: (lineNo: number) => void,
}): { feed(buf: Buffer): void; flush(): void }
```

- raw Buffer 上で `0x0A` を走査して行を切り出す（UTF-8 安全）。
  行 byte 長は decode 前の raw byte で計測する。
- 行が L を超えた時点で蓄積を止め、次の `\n` まで破棄モードに
  入る（保持量 O(C) のまま）。`onOversized(lineNo)` を発行し、
  行番号は消費する（後続行の番号がずれない）。
- `flush()` は EOF 時の未終端行を 1 行として emit する。
- `\r` は行内容に残す（既存 `split("\n")` との parity）。

#### batch protocol（eval worker）

- worker source の `file` メッセージを `batch` に置き換える:
  送信 `{type:"batch", id, display, startLine, lines: string[]}`、
  返信 `{type:"result", id, matched: number[]}`（batch 内 0-indexed）。
- batch は `K` 行または `B` byte の先に到達した方で締める。
  **未応答 batch は常に最大 1**：送信 → result 待ち → 次 batch。
  eval 待機中は I/O を進めず、受け渡しキューは増えない。
- regex 評価は引き続き worker 内のみ。worker 生成に
  `resourceLimits: { maxOldGenerationSizeMb: GREP_WORKER_MAX_OLD_SPACE_MB }`
  を付ける。
- eval 時間計測: per-file 予算は同一ファイルの全 batch の eval 時間を
  累積して `GREP_EVAL_PER_FILE_MS` と比較する（batch ごとの
  リセットはしない）。超過時は worker を `terminate()` し、その
  ファイルを `timeout` skip として記録、後続ファイル用に worker を
  再生成する。`GREP_EVAL_TOTAL_MS` 超過で走査全体を止め
  `[truncated: evaluation time limit reached]` を出す。
- **走査 deadline**: 呼び出し開始から `GREP_SCAN_DEADLINE_MS` の
  wall-clock deadline を設け、walk・I/O・eval 待機のすべてを含む。
  到達時は部分結果 + footer `timeout` を返す（error にしない）。
  `signal` abort は cleanup 後に `Operation aborted` で reject する。
- **cleanup**: `finally` で FileHandle close・worker terminate・
  timer 解除を保証する。timeout・abort・error のいずれでも
  stream/handle/worker が残らない。

#### context と出力

- `context`（0–3）の前 context は直近 `context` 行の ring buffer
  （各行 ≤ L）から供給する。
- match 後の後 context は「あと `context` 行を context として拾う」
  pending countdown で処理する。eval 待ち中に読み取りを止めるため
  （1 batch in-flight）、後 context 行は次 batch で届く。
  既存の block merge 規則（`from <= last.to + 1` で結合）は維持し、
  close 済み block だけを出力へ flush する。保持するのは ring 分 +
  pending 分のみで、全行配列・全一致配列は作らない。
- 1 行の表示は `GREP_OUTPUT_LINE_MAX_CHARS` 文字 + `... [truncated]`
  （Pi `truncateLine` 相当）。出力全体は `GREP_OUTPUT_MAX_BYTES` で
  締め、超過時は `[truncated: output limit reached]` で停止する。
- `maxResults`（既定 100・最大 500）到達は従来どおり
  `[truncated: more matches]` で停止する（reason `result-cap`）。

### find `packages/runtime-pi/src/tools/find.ts`

- `walkFiles`（path-only 化後）をそのまま使う。内容の
  `open`/`read`/`sniff` は一切行わない。
- `onIssue` を footer に流し、列挙失敗を報告する。
- glob 照合・`maxResults`（既定 200・最大 1000）・
  `[truncated: more results]` marker・`.git`/`node_modules` skip・
  symlink/deny は維持。
- description から "binary files, and files over 1MB" の記述を外し
  "lists all allowed file paths regardless of size or content" 系に
  更新する（grep description も同様に更新）。

### 有界 skip footer

grep/find の出力末尾に、reason 別の footer を付ける。

```text
[skipped: binary — b.dat, c.bin, and 3 more]
[skipped: unreadable — locked.txt]
[skipped: oversized-line — min.js:1, min.js:7]
[truncated: evaluation time limit reached]
[truncated: scan deadline reached]
[truncated: more matches]
[skipped-report: truncated]
```

- reason 分類: `binary` / `unreadable` / `oversized-line` /
  `timeout` / `result-cap`。`timeout`・`result-cap` は既存の
  `[truncated: ...]` marker に対応し、`skipped:` 行は
  binary/unreadable/oversized-line を担う。
- 各 reason につき代表例は `SKIP_FOOTER_MAX_ENTRIES` 件まで、
  残りは `and N more` の集計。1 path は `SKIP_PATH_MAX_CHARS` 文字で
  切り詰め。footer 全体は `SKIP_FOOTER_MAX_BYTES` を超えたら
  `[skipped-report: truncated]` で締める。
- 一致 0 件でも skip があれば `(no matches)` に続けて footer を出す。
  「一致なし」と「未検索あり」を必ず区別できる。
- **存在秘匿**: deny 対象は walker の filter 段で消えるため、結果・
  footer・`and N more` の件数にも一切現れない。deny 件数を集計して
  漏らす経路は作らない（SPEC-0010）。直接指定した path の
  boundary/policy 違反は従来どおり tool error で、unreadable として
  飲み込まない。

### registry / operations / worker guidance

- `registry.ts`: `case "read"` を `createReadTool(opsOpts)` に
  置き換える。`createReadToolDefinition` / `createReadOperations` /
  `ReadOperations` の import を除去。他 tool の mapping は不変。
- `operations.ts`: `createReadOperations` を削除。write/edit/ls の
  operations は不変。
- `packages/cli/workers/bulk-reader.md`: procedure に
  「`read` は `offset`/`byteInLine` で継続、巨大 file は grep の
  `[skipped: ...]` footer と `(no matches)` を区別する」旨を追記。
  tool 名・schema 宣言は不変。配布物（`.apm/hooks/cli/workers/` 等）は
  bundle/apm install の生成物であり手編集しない。

## Data and Control Flow

```text
read:  params → resolveWorkspacePath + deny check
       → ext/regular-file check → open
       → chunk 走査 (O(C)): NUL sniff + offset 行まで prefix 破棄
       → emit (≤ limit 行, ≤ R byte) → eof/truncatedBy + continuation
       → finally: handle close

grep:  walkFiles(path-only) → glob → sniffTextFile
         ├ binary/unreadable → skip footer に記録
         └ text → fh.read chunk (C) → line framer (L)
                 → batch (≤K lines, ≤B bytes) → worker eval (1 in flight)
                 → ring/pending context → 出力 (≤O) → footer
       finally: handle close / worker terminate / timers

find:  walkFiles(path-only) → glob → display 列挙 (内容 I/O なし)
       → onIssue → footer
```

## Sandbox Invariants

- bounded read・streaming grep・find はいずれも
  `resolveWorkspacePath` + `denylist.isDenied` を既存 tool と同じ
  入口で通す。byte 継続・深い offset を使った新規経路に
  boundary/deny の迂回は存在しない。
- 外部プロセスを spawn しない（`child_process` を使わない）。
- read-only の維持: 新規コードは `open(..., "r")` と `stat`/`readdir`
  のみを使い、write 系 syscall を呼ばない。write-scope・tool budget・
  `rejectedToolCalls` 計上は変更しない。
- `count_tokens` の既定 16MB cap・`FileTooLargeError`、
  `EditOperations` の全文読み込み、親 hook の fail-closed は
  本設計で一切変更しない。
- DESIGN-0006 の出力 schema・excerpt 制約（≤600 文字・≤20 行）は
  維持。tool の返却量上限 R/O は worker の excerpt policy と独立。

## Memory Budget

入力ファイル総 byte 数を N とし、同じ設定での作業メモリを O(N) に
しないことを予算として固定する。

| 経路 | 保持量の上限 |
| --- | --- |
| read 1 call | O(C + R) ≈ 64KiB + 50KiB + decoder/定数分 |
| grep 1 call | O(C + L + B + context×L + O + S)。`context ≤ 3` で ring+pending は ≤ 2×3×L。概算 ~2.6MiB + worker 側 |
| grep eval worker | 1 batch ≤ B + pattern ≤ `GREP_PATTERN_MAX_CHARS` + `resourceLimits.maxOldGenerationSizeMb = 64` |
| find 1 call | O(maxResults × path 文字数) ≤ 1000 × `SKIP_PATH_MAX_CHARS` 程度の表示文字列 |
| footer | ≤ S = 4KiB |

- worker への複製（postMessage の structured clone）、decode 後の
  文字列表現、結果 formatting の一時コピーも上記の枠内で見る。
- watchdog は時間制限でありメモリ上限の証明ではない。メモリは
  buffer/蓄積の構造的上限で保証し、テストで instrumentation する。
- tool call が並行実行される場合の worst case は
  `per-call 上限 × 同時実行数 M`。runtime は tool 並列度を明示的に
  制限しないため、M の設計上の上限は tool vocabulary 数（≤7）とする。
  7 call が同時に grep 相当を実行しても ~20MiB 級に収まる。

## Non-Functional Constraints

- 行 offset が深い read / 巨大 file の grep は O(N) の走査時間を
  要しうる。有界性は高速性の保証ではなく、走査 deadline で打ち切る。
- 単一長大行（>L）の完全な regex 検索は保証しない制限であり、
  footer で必ず報告する。内容確認は bounded read の `byteInLine`
  継続で行う（ADR-0017 の受け入れ済み制約）。
- 互換性: `offset`/`limit` の意味と出力形式は既存利用と互換を保つ。
  grep の 1 行表示 cap（500 chars）は新規の表示制限であり、
  従来 1MB 以下のファイルでは実質到達しない。
- 依存追加なし。Node.js 標準（`fs/promises`・`worker_threads`・
  `string_decoder`）と既存の `defineTool`/`typebox` のみで実装する。

## File Map

| file | 変更 |
| --- | --- |
| `packages/runtime-pi/src/tools/limits.ts` | 新規。共有上限定数 |
| `packages/runtime-pi/src/tools/read.ts` | 新規。bounded read tool |
| `packages/runtime-pi/src/tools/line-framer.ts` | 新規。grep 用の上限付き行フレーマー |
| `packages/runtime-pi/src/tools/registry.ts` | read mapping 置き換え |
| `packages/runtime-pi/src/tools/operations.ts` | `createReadOperations` 削除 |
| `packages/runtime-pi/src/tools/walker.ts` | path 列挙専念化 + `onIssue` + `sniffTextFile` |
| `packages/runtime-pi/src/tools/grep.ts` | streaming/batch protocol へ書き換え + footer |
| `packages/runtime-pi/src/tools/find.ts` | path-only + footer、description 更新 |
| `packages/cli/workers/bulk-reader.md` | 継続読み取り・footer 解釈の guidance 追記 |
| `packages/runtime-pi/test/tools/read.test.ts` | 新規 |
| `packages/runtime-pi/test/tools/{walker,find,grep,registry,scope-deny}.test.ts` | 移動した eligibility の更新・footer/skip/秘匿の追加テスト |
| `packages/runtime-pi/test/tools/operations.test.ts` | `createReadOperations` 削除に伴う更新 |

## Test Plan

SPEC-0014 の各受け入れ条件に対応する。

- **有界 read**: `fs/promises` の `readFile` を spy して不呼出を確認し、
  `open` を wrap して `fh.read` の `length ≤ C` と呼出位置を記録する。
  16MB 超 + 100MB 以上の fixture を `beforeAll` で固定生成し、
  返却 byte ≤ R・`truncatedBy`/`continuation` の正確さを検証する。
  低予算への縮退（`maxBytes`/`chunkBytes` options 注入）で小 fixture
  による決定的テストも併用する。
- **境界ケース**: 先頭/中間/末尾 offset、空 file、改行なし長大行
  （`byteInLine` 継続の連続呼出で欠落・重複なし）、UTF-8 文字が
  chunk/byte cap 境界に跨る fixture、CRLF、`offset` EOF 超過 error、
  `byteInLine` 単独指定 error、binary/image の明示 error。
- **grep 巨大 file**: 1MB 超・100MB 以上の text fixture の後半に置いた
  既知一致を返す。小さな C/B に縮退して batch/chunk 境界を跨ぐ行の
  一致・`ignoreCase`・`glob`・context ±3・`--` merge を検証する。
  長大行は footer `oversized-line`（path:行番号）になり `(no matches)`
  と区別できる。
- **find**: 1MB 超・100MB・binary file を列挙し、`open`/`readFile`
  spy が対象 file の内容 I/O を一度も呼ばないことを確認する。
- **skip/不完全性**: binary・`open` 失敗を mock 注入した unreadable・
  `evalTotalMs`/`scanDeadlineMs` 縮退の timeout・多数一致の
  result-cap・skip 20 件超の `and N more` と `[skipped-report:
  truncated]` をそれぞれ区別できることを検証する。
- **メモリ**: spy が記録した最大 chunk/batch/出力が C/L/B/O/S 以下
  であること、未応答 batch が常に ≤1 であること（postMessage spy で
  in-flight 計数）を構造的に検証する。16MB→100MB の入力増大に対し
  `process.memoryUsage().heapUsed` の delta が予算数倍（目安:
  4×(C+L+B+O+S)）以内に収まることを補助指標として記録する。
  RSS の一回観測だけを証拠にしない（ADR-0017 Confirmation）。
- **boundary/deny 回帰**: workspace escape・symlink escape・
  built-in/custom deny の直接 read 拒否、grep/find の結果と footer・
  集計に deny path が一切出ないこと。read-only（journal 差分なし）と
  write-scope の既存テストが維持される。
- **parity**: 既存 126 テストのうち eligibility を検証していたものは
  責務の移動先（walker → grep）に合わせて更新し、残りは期待動作を
  変えずに全件通す。`count_tokens` の 16MB cap error と cap 未満の
  canonical count、親 hook の exit 2 + 委譲 guidance の既存検証を
  維持する。
- **隔離環境**: 実 provider / CLI / hook の検証は TASK-0086/0087 と
  同様に一時ディレクトリへ `apm install` した隔離環境でのみ行う。
  本リポジトリ内の bundled cheaper CLI・`.apm/hooks`・cheaper-*
  スキルによる自己委譲・検証は実行しない。

## Risks and Trade-offs

- 独自 read は Pi の綴り変種解決・画像表示を失う。worker への
  影響は tool description と bulk-reader guidance で吸収する。
- batch 応答待ちで I/O が止まるため grep は単純 streaming より
  低速になる。backpressure による有界性を優先する。
- 深い `offset` や巨大 file 後半の一致探索は O(N) 時間を要し、
  deadline 到達時は部分結果になる。footer で完全性を判断可能にする。
- `byteInLine` はファイル変更に対して unsafe である。snapshot を
  取らない代わりに caveat を契約化した。

## Verification Notes

```bash
mise exec -- pnpm --filter @cheaper/runtime-pi build
mise exec -- pnpm --filter @cheaper/runtime-pi test
```

全体検証は従来どおり `mise exec -- pnpm -r build` /
`mise exec -- pnpm -r test` / `mise exec -- pnpm lint` で行う。

- [ ] 16MB 超・100MB fixture で全文 `readFile` が呼ばれず、
      `fh.read` 長 ≤ C、返却 ≤ R、継続位置が正しい
- [ ] 改行なし 100MB 単一行で read は `byteInLine` 継続を返し、
      grep は `oversized-line` を footer に報告する
- [ ] 空/先頭/末尾/EOF/CRLF/UTF-8 境界/巨大 offset/不正範囲の
      文書化どおりの結果になる
- [ ] 100MB 後半の既知一致、batch/chunk 境界を跨ぐ行、
      ignoreCase/glob/context/merge が正しい
- [ ] maxResults 100/500・出力 cap O・未応答 batch ≤1・
      footer 件数/byte cap が守られる
- [ ] binary/unreadable/timeout/abort 注入で部分結果 + 理由が返り、
      handle/worker/timer が解放される
- [ ] per-file 5s / total 60s の eval 予算が batch でリセットされず、
      ReDoS は worker terminate で停止する
- [ ] find が巨大/binary file を列挙し内容 I/O を行わない
- [ ] escape/symlink/deny の回帰と、deny の存在・件数が結果/footer に
      漏れないこと
- [ ] `count_tokens` 16MB cap・canonical count・親 hook fail-closed の
      既存検証が維持される
- [ ] provider/CLI/hook の実検証は隔離 `apm install` 環境で行い、
      本リポジトリの cheaper CLI/hook/skill は実行しない
