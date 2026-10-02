---
id: "TSPEC-034z50PbuzTYDWvQXYdvaG"
type: "test-spec"
status: "proposed"
title: "cheaper hook subcommands and hooks.json executable contract"
created: "2026-10-02"
updated: "2026-10-02"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md"
  derived-by: []
  verifies:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper hook subcommands and hooks.json executable contract

## Purpose

issue #71 Phase 4 として、`cheaper hook <event> --target <agent>` が
現行 JS adapter（`pre-read-code*.js` / `pre-run-command.js` +
`routing-policy.js`）と挙動不変の判定を Node.js 非依存で行い、
`hooks.json` が executable 経由で CLI を呼ぶことを保証する。
routing ルールの変更や新 target の追加ではなく、既存 contract の
移植 parity を守ることが目的。hook 経路が別形態に再設計された場合
にのみ retire する。

## Feature

Node.js 非依存の `cheaper hook` 経路による agent hook 判定

## Rules

### Canonical routing policy（移植 parity）

- workspace containment は raw cwd に対する lexical 判定
  （`path.resolve` + `path.relative`）。workspace 外・`rel === ""`・
  解決不能な path は allow
- `statSync` 失敗・非 regular file（dir / FIFO / device）は allow
- byte fast path: `size <= CHEAPER_READ_TOKEN_THRESHOLD` の場合は
  計測せず allow（count 系 import を起こさない）
- measured path は in-process `countFileTokens`（count-tokens command
  と同一経路: canonicalize workspace root → policy load → maxBytes）。
  `FileTooLargeError` → fail-closed deny、`WorkspaceEscapeError`・
  ENOENT・その他全 error → allow。measured path 全体を
  `AbortSignal.timeout(30_000)` で囲み timeout → allow
- shell command の read candidate 抽出（cat/head/tail/less/more、
  quoting・`#` comment・`|`,`&&`,`;`・redirection・unsafe token・
  `ENV_PREFIX`・`--`・cwd-changing command 後段 fail-open・operand
  dedup）は JS 版と同一 fixture で挙動一致
- deny 文言は JS 版と同一形式（`'<rel>' has N tokens (threshold M).`
  ・FILE_TOO_LARGE 版・shell 版 `blocked shell command '<cmd>'.`）

### Target adapter contract

- windsurf（`pre-read-code` / `pre-run-command`）: deny は stderr
  guidance + exit 2。event 名不一致・malformed payload・field 欠落・
  allow verdict は無言 exit 0
- copilot（`pre-tool-use --target copilot`）: deny は stdout
  `{"permissionDecision":"deny",...}` + exit 0。**絶対に非 0 exit
  しない**。異形 payload（`tool_name`/`file_path`/`command` を持つ
  claude/cursor 風）には dual-schema allow
  `{"permission":"allow","permissionDecision":"allow"}` を emit
- pretooluse（`pre-tool-use --target pretooluse`、claude・codex 共用）:
  deny は stdout `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"deny",...}}` + exit 0。全 fail-open 経路は
  無言 exit 0（stdout に何も出さない）
- cursor（`before-read-file` / `before-shell-execution`）: deny は
  stdout `{"permission":"deny",...}` + exit 0。**常に schema-valid
  JSON を emit**（malformed stdin・内部 error でも
  `{"permission":"allow"}`）。無言 exit 0 は禁止

### `cheaper hook` command 経路

- hook 経路は runCli の envelope JSON / `recordUsage` / 汎用 catch に
  到達しない。args parse 失敗・未知 `--target`・policy 破損・cwd
  不存在・内部例外を含む全 error は target 適合の allow + exit 0
  （target 判別不能なら無言 exit 0）
- `usage.jsonl` に hook 計測は記録されない
- `--target` は `hook` command 以外では拒否される（通常の option
  誤用ガード）

### hooks.json executable 配線

- 全 entry に `node ` / `.js` adapter 参照を含まない
- POSIX `command` は `./cheaper hook <event> --target <t>` の形
- `powershell` は `& "./cli/bin/win32-x64/cheaper.exe" hook ...; exit
  $LASTEXITCODE` の形（`.ps1` launcher を介さない）
- copilot entry: `|| exit 0` 相当の shell fail-open。cursor entry:
  `|| echo '{"permission":"allow"}'`。windsurf / pretooluse entry は
  exit code を潰す suffix を持たない
- launcher `cheaper` は payload 先頭の `version https://git-lfs`
  prefix（LFS pointer）を検出して明確な stderr error で終了する

## Examples

- Given: workspace 内に 8192 byte の file。When: windsurf の
  `pre_read_code` payload を stdin に流す。Then: byte fast path で
  無言 exit 0（tokenizer を load しない）
- Given: 閾値超過 token の file。When: copilot payloadで `pre-tool-use
  --target copilot`。Then: stdout に deny JSON、exit 0
- Given: 閾値超過 token の file。When: pretooluse payloadで
  `--target pretooluse`。Then: `hookSpecificOutput.permissionDecision`
  が `"deny"`、exit 0
- Given: malformed JSON stdin。When: cursor の `before-read-file`。
  Then: `{"permission":"allow"}` を emit、exit 0
- Given: 未知の `--target` 値や引数不足。When: `cheaper hook` を
  実行。Then: 非 0 exit を返さない（silent exit 0 or target 適合
  allow）
- Given: workspace 外 path を指す file-read request。Then: 計測せず
  allow
- Given: 閾値超過の file に対し `cat big.ts` の shell command。
  Then: `blocked shell command 'cat big.ts'.` prefix の deny verdict
- Given: `cd /tmp && cat big.ts` のように cwd-changing command を
  挟む shell。Then: 後段は fail-open で allow

## Guarantees

- infra error・malformed payload・例外は常に allow 方向（fail-open）
- measured deny と FILE_TOO_LARGE のみが fail-closed
- `cheaper hook` の起動に Node.js は不要（SEA executable / launcher
  経由で動作）
- `hooks.json` 切替後も target 別の deny/allow contract が維持される
- POSIX hook 呼び出しの slow path（token 計測）は 30s で打ち切られる

## Non-goals

- routing 閾値・文言・policy ルール自体の変更検証（挙動不変が前提）
- macOS executable 配布（payload matrix 外）
- `apm install` の remote transport での LFS smudge 実測（consumer
  初回 install に依存、deferred）
- Claude/Codex/Cursor 実 agent での発火検証（upstream 依存）

## Risk

- この保証が落ちると、hook が誤って全 read/shell を block したり
  （copilot の非 0 exit = deny-all）、逆に巨大 file の deny が漏れて
  routing policy の存在意義が失われる
