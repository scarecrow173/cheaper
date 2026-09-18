---
id: "TASK-034r05iHQeQpDqkDLrHBxb"
type: "task"
status: "done"
title: "Bounded read tool and registry swap"
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
    - "docs/tasks/shared-limits-line-framer-and-path-only-walker-find.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/bulk-reader-huge-file-safe-io.md"
    - "docs/specs/bulk-reader-huge-file-safe-io.md"
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

# Bounded read tool and registry swap

## Work

Pi の `createReadToolDefinition` + `createReadOperations`（全文
`readFile`）を、DESIGN-034r05iHy2gdnoNaWH2bos の自前 bounded read tool に置き換える。
返却は `READ_MAX_BYTES`（R = 50KiB）以下に有界化し、巨大 file は
`offset`/`byteInLine` で継続読み取りする。

### 1. `packages/runtime-pi/src/tools/read.ts`（新規）

`createReadTool(opsOpts)` を実装する。境界検証は `count-tokens.ts`
と同一経路: `resolveWorkspacePath(root, params.path, symlinkPolicy)`
→ `rethrowAsDenied` → `denylist.isDenied(display)` なら
`POLICY_DENIED_MESSAGE`。迂回経路は作らない。

パラメータ schema（DESIGN-034r05iHy2gdnoNaWH2bos）:

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

- `offset` は 1-indexed 行番号（`\n` 区切り、先頭が 1）。
  `byteInLine` はその行内の byte 位置で別 namespace。`offset` 省略時の
  `byteInLine` は tool error `byteInLine requires offset`。
- `byteInLine` が行 byte 長以上 → `byteInLine K is beyond end of
  line N (M bytes)`。`offset` が EOF 超過 → `Offset N is beyond end
  of file (M lines total)`（先行走査で総行数は判明済み）。
- `byteInLine` が UTF-8 文字境界でない場合は前方へ次の境界に
  スナップ（最大 3 byte）。後退はしない。

応答: `content` はテキスト 1 ブロック + `details` に
`ReadToolDetails { startLine, endLine, eof, truncatedBy:
"limit" | "bytes" | null, continuation?: { offset, byteInLine? },
totalLines? }`。`totalLines` は `eof === true` のときのみ返す
（行数を知るための前方走査 O(N) はしない）。本文は選択行の
`\n` 連結 verbatim（`\r` 除去しない）。`byteInLine > 0` で再開
した場合は先頭に `[partial line N: continuing from byte K]` を付ける。
継続 trailer（EOF 時は付けない）:

- limit 到達: `[Showing lines S-E. Use offset=N to continue.]`
- byte cap が行境界: `[Reached 50KB limit. Use offset=N to continue.]`
- byte cap が行途中: `[Line E emitted bytes A-B (reached 50KB
  limit). Use offset=E byteInLine=C to continue.]`

I/O 手順:

1. `open(resolved.real, "r")` → `stat` で `isFile()` 確認（directory
   等は `read: not a regular file`）。
2. 先頭 chunk を `SNIFF_BYTES` 分走査し NUL を含めば
   `read: binary file (NUL byte within first 8KiB)`。sniff は prefix
   走査で読む最初の chunk を再利用し追加 I/O なし。
3. 拡張子が `IMAGE_EXTENSIONS`（`.png .jpg .jpeg .gif .webp .bmp`、
   大小文字非依存）なら走査前に `read does not support image files`。
   画像経路・全文 fallback は設けない。
4. `offset` 行まで `IO_CHUNK_BYTES` 単位の `FileHandle.read` で
   `\n` を数えながら prefix を破棄（保持量 O(C)）。目的行の開始
   byte 位置に `byteInLine` を加算・スナップして emit 開始。
5. emit ループは行ごとに残り byte 予算を引き、`limit` 行または
   `READ_MAX_BYTES` で継続情報を確定して終了。1 行が残り予算を
   超える場合は収まる分だけ返し `byteInLine` 継続を返す。行全体の
   蓄積はせず保持量は O(C + R)。
6. `execute` 第 3 引数 `signal` で abort を受け、`finally` で
   FileHandle を必ず `close()` する。

decode は `StringDecoder("utf8")` を chunk 跨ぎで継続。不正 byte
列は U+FFFD で置換して返す（欠落・全文 fallback なし）。
`byteInLine` は常にファイル byte 基準。tool description に
"changed files may need re-reading from an earlier offset" を明記
し、FileHandle は呼び出しごとに open して cache しない。Pi read の
綴り変種解決（`~`/`@`/NFD）・画像 attachment・
`detectImageMimeType` 経路は引き継がない。

### 2. `packages/runtime-pi/src/tools/registry.ts`

`case "read"` を `createReadTool(opsOpts)` に置き換える。
`createReadToolDefinition` / `createReadOperations` /
`ReadOperations` の import を除去。write/edit/ls の Pi
definition + operations 注入、grep/find/count_tokens の mapping は
不変。

### 3. `packages/runtime-pi/src/tools/operations.ts`

`createReadOperations` を削除する（`ReadOperations` import も除去）。
write/edit/ls operations、`resolve`/`resolveForWrite`/`atomicWrite`
helper は不変。

### 4. テスト

- `test/tools/read.test.ts`（新規）:
  - `fs/promises` の `readFile` spy が不呼出、`open` を wrap して
    `fh.read` の `length ≤ IO_CHUNK_BYTES` と呼出位置を記録する。
    16MB 超・100MB 以上 fixture を `beforeAll` で固定生成し、
    返却 ≤ R・`truncatedBy`/`continuation` の正確さを検証。
    `maxBytes`/`chunkBytes` の options 縮退注入で小 fixture の
    決定的テストも併用。
  - 境界: 先頭/中間/末尾 offset、空 file、改行なし長大行の
    `byteInLine` 連続呼出で欠落・重複なし、UTF-8 文字が chunk/
    byte cap 境界に跨る fixture、CRLF parity、`offset` EOF 超過
    error、`byteInLine` 単独指定 error、binary/image の明示 error。
  - boundary/deny: workspace escape・symlink escape・built-in/custom
    deny path の直接 read が `ACCESS_DENIED_MESSAGE`/
    `POLICY_DENIED_MESSAGE` で reject される。
- `test/tools/operations.test.ts`（更新）: `createReadOperations`
  を import・`ops()` helper から除去し、read 系 boundary assert
  （`../`・絶対 path・symlink escape の read 拒否）は残る
  operations（edit/ls）経路または新 read tool 側で代替する。
- `test/tools/registry.test.ts`（更新）: `read tool reads via
  boundary ops` 等を新 tool 契約に更新（応答は verbatim テキスト
  1 ブロックで互換の見込み。PLAN-034r05iHE1Fbkp3qMZZB17 Open Questions 参照）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/bulk-reader-huge-file-safe-io.md`
- `docs/designs/bulk-reader-huge-file-safe-io.md`
- `docs/specs/bulk-reader-huge-file-safe-io.md`

## Dependencies

- `docs/tasks/shared-limits-line-framer-and-path-only-walker-find.md`
  （`limits.ts` の `IO_CHUNK_BYTES`/`SNIFF_BYTES`/`READ_MAX_BYTES`/
  `READ_DEFAULT_LIMIT`/`READ_MAX_LIMIT` を利用）

## Affected Files

- `packages/runtime-pi/src/tools/read.ts`（新規）
- `packages/runtime-pi/src/tools/registry.ts`
- `packages/runtime-pi/src/tools/operations.ts`
- `packages/runtime-pi/test/tools/read.test.ts`（新規）
- `packages/runtime-pi/test/tools/operations.test.ts`
- `packages/runtime-pi/test/tools/registry.test.ts`

変更しない file（明示）: `walker.ts`/`find.ts`/`grep.ts`/
`line-framer.ts`/`limits.ts`（TASK-034r05iHQeQpDqj5TapRlr/0091 の領域）、
`count-tokens.ts`、`operations.ts` の edit/write/ls 系、
`packages/cli/workers/`、`.apm/hooks/**`。

## Done When

- [ ] `read` が全文 `readFile` を呼ばず `fh.read` 長 ≤ C、返却 ≤ R、
      `offset`/`limit`/`byteInLine` 継続が contract どおり
- [ ] binary/image/非 regular file/`byteInLine` 単独/EOF 超過が
      規定の明示 error になる
- [ ] boundary/symlink/deny 拒否が `resolveWorkspacePath` +
      `rethrowAsDenied` + `denylist.isDenied` 経路で維持される
- [ ] `registry.ts` の `case "read"` が `createReadTool` で、
      `operations.ts` に `createReadOperations` が残っていない

## Verification

- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi build` が exit 0
- [ ] `mise exec -- pnpm --filter @cheaper/runtime-pi test` が exit 0
      （read/operations/registry の新規・更新テストを含む）
- [ ] `grep -n "createReadOperations" packages/runtime-pi/src/tools/`
      が一致なし（exit 1）を返す
