---
id: "DESIGN-034r05iHy2gdnoLt52txFG"
type: "design"
status: "approved"
title: "Huge-file byte cap guard and fail-closed hook"
created: "2026-09-17"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/huge-file-token-count-guard-with-fail-closed-hook-blocking.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../adr/treat-unmeasurable-huge-files-as-over-threshold-via-a-core-byte-cap-fail-closed.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Huge-file byte cap guard and fail-closed hook

## Context

SPEC-034r05iHK8bA7gjwZTji8t / ADR-034r05iHrZUE6W82VsDbPq の実装設計。`countFileTokens` への byte cap、CLI
`count-tokens` の error map と env 上書き、hook の fail-closed 扱いの
3 面を定める。

## Scope

### In Scope

- core: `FileTooLargeError` + `maxBytes` option + stat 事前チェック
- cli: `CHEAPER_COUNT_MAX_BYTES` 解釈・`FILE_TOO_LARGE` error code map
- hook: fast-path（bytes ≤ threshold）と cap error の block 扱い
- 単体 / e2e test

### Out of Scope

- bulk-reader の巨大 file 読み込み対策（別 chain）
- hook の spawn timeout 値（30s のまま）

## Components and Boundaries

### core `packages/core/src/token/count.ts`

- `export const DEFAULT_COUNT_MAX_BYTES = 16 * 1024 * 1024`（16MB）
- `export class FileTooLargeError extends Error` — `{path, bytes, maxBytes}`
  を保持（`WorkspaceEscapeError` と同じ typed error パターン）
- `CountFileTokensOptions.maxBytes?: number` — 省略時は default
- `resolveWorkspacePath` 後・`readFile` 前に `stat(resolved.real)` を取り、
  `size > maxBytes` なら `FileTooLargeError` を投げる。read/encode しない

### core `packages/core/src/errors.ts`

- `ERROR_TABLE` に `FILE_TOO_LARGE: { exitCode: 2, retryable: false }` を
  追加（request-level の拒否 = exit 2、envelope.code で判別）

### cli `packages/cli/src/cli.ts`

- `count-tokens` 分岐で `CHEAPER_COUNT_MAX_BYTES` を解釈:
  正の整数のみ有効、不正値・未設定は `DEFAULT_COUNT_MAX_BYTES`
  （hook の `threshold()` と同じ parse パターン）
- `countFileTokens({..., maxBytes})` へ渡す
- `FileTooLargeError` を `CheaperError("FILE_TOO_LARGE", ..., {details:
  {path, bytes, maxBytes}})` に map（`WorkspaceEscapeError` map と同型）

### tool `packages/runtime-pi/src/tools/count-tokens.ts`

- 変更不要。`FileTooLargeError` は `rethrowAsDenied` を素通りして tool
  error として worker に返る（同一 core 実装で自動的に guard される）

### hook `.apm/hooks/pre-read-code.js`

- **fast-path**: `statSync(resolved).size <= threshold()` なら `tokens ≤
  bytes ≤ threshold` が保証されるため CLI spawn せず `exit(0)`
- **envelope 判定の順序変更**: spawn 後、stdout が JSON envelope として
  parse できたら先に `envelope.error && envelope.error.code ===
  "FILE_TOO_LARGE"` を検査し、一致すれば block。既存の
  `run.status !== 0 → exit(0)` の前に envelope を見る形にする
- block guidance（FILE_TOO_LARGE 用）: `'<rel>' is N bytes (exceeds the
  N-byte measurement cap). Do not read it directly. Delegate ...` +
  `workspace-relative path:` 行。非実行型テキストのみの契約を維持
- spawn error / timeout / parse 不能 / ok:false（FILE_TOO_LARGE 以外）は
  従来どおり `exit(0)`（infra 障害の fail-open 契約は維持）

## Data and Control Flow

```text
hook: stat → bytes ≤ threshold → exit 0 (fast path, no spawn)
           └ bytes > threshold → spawn count-tokens
CLI:  resolveWorkspacePath → stat → size > maxBytes → FileTooLargeError
        └→ CheaperError FILE_TOO_LARGE → envelope {ok:false,error:{code}}
hook: envelope.error.code === FILE_TOO_LARGE → exit 2 + 委譲 guidance
      envelope.ok && tokens > threshold      → exit 2 + 委譲 guidance
      その他（infra 障害等）                   → exit 0
```

## Non-Functional Constraints

- cap 超過 file は read/encode しないため OOM/timeout が構造的に消える
- cap 未満 file の count 結果・envelope shape は不変（parity）
- `CHEAPER_COUNT_MAX_BYTES` は正の整数のみ。不正値は default に倒す
- guidance は実行可能な shell command を含めない（既存契約）

## Risks and Trade-offs

- 高圧縮率 file（bytes 大・tokens 小）の false positive block →
  parent context は保護されるが、現状の bulk-reader は whole-file
  Buffer 化 + >1MB skip walker で huge-file-safe I/O を保証しない。
  bounded/chunked 対応は follow-up であり、委譲自体が失敗しうる。
  cap は env で調整可
- fast-path は「bytes ≤ threshold → tokens ≤ threshold」の決定的性質に
  依存。tokenizer 差異で崩れない（1 token ≥ 1 byte は encoding に依らず
  成立）

## Verification Notes

- [ ] `maxBytes` 超過で `FileTooLargeError`・readFile が呼ばれない
      （core unit test、小さな maxBytes で検証）
- [ ] `CHEAPER_COUNT_MAX_BYTES` 上書き・不正値フォールバック（cli test）
- [ ] FILE_TOO_LARGE envelope で hook が exit 2 + guidance（e2e）
- [ ] bytes ≤ threshold の file で spawn なし exit 0（e2e、CLI を不在に
      しても exit 0 になることで検証可能…は不可: CLI 不在は既に
      fail-open。代わりに CHEAPER_CLI を marker を書く fake に向け、
      marker が無いことを確認）
- [ ] cap 未満 file の parity（既存 test が pass し続ける）
