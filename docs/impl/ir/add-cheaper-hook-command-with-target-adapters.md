---
id: "IMPL-034z57jprjlY9PUYkFPIdl"
type: "impl"
status: "completed"
title: "add cheaper hook command with target adapters"
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
  implements:
    - "docs/tasks/add-cheaper-hook-command-with-target-adapters.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
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

# add cheaper hook command with target adapters

## Summary

`cheaper hook <event> --target <agent>` command を実装し、4 target
（windsurf / copilot / pretooluse / cursor）の adapter 層を
`packages/cli/src/hook/` に移植した。hook 経路は `runCli` の
envelope・recordUsage・汎用 catch を完全に bypass する。

## Context

- task: docs/tasks/add-cheaper-hook-command-with-target-adapters.md
- design: docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md

## Implementation

### Changes

- `packages/cli/src/hook/targets.ts`（新規）: 4 adapter を JS 版と
  1:1 移植。windsurf（`agent_action_name` 照合、deny = stderr +
  exit 2）、copilot（`view`/`bash`/`powershell`/`shell`、toolArgs
  object/string 両対応、cross-agent dual-schema allow、deny =
  `permissionDecision` JSON + exit 0）、pretooluse（`Read`/`Bash`/
  `PowerShell`、deny = `hookSpecificOutput` JSON + exit 0）、
  cursor（`file_path`/`command` key で event 判別、payload `cwd`
  fallback、全経路で schema-valid permission JSON + exit 0）。
  `fallbackAllow(target)` で catch-all の allow 応答を target 別に
  定義（cursor は明示 allow JSON、他は silent exit 0）
- `packages/cli/src/hook/hook.ts`（新規）: `executeHook` entry。
  独自の緩い引数解析（`--target` / `--target=` 形式、未知 flag・
  欠損は throw せず undefined）、raw stdin 読み取り（isTTY→空、
  JSON parse 失敗→fail-open）、最深部 catch-all で全 error を
  `fallbackAllow` + exit 0 に変換。**runCli の汎用 catch には
  到達しない**構造
- `packages/cli/src/cli.ts`: `argv[0] === "hook"` を dispatch
  直前で早期 return（`parseCliArgs`・envelope・usage 記録を迂回）、
  HELP に `hook` の行を追加

### Decisions During Implementation

- **`parseCliArgs` を通さない**: hook の引数解析失敗が
  `INVALID_REQUEST` envelope（exit 2）に化ける経路を根絶するため、
  hook 独自の緩い parser を hook.ts 内に閉じ込めた。`--target` は
  `args.ts` に追加せず、非 hook command では従来どおり未知 option
  として strict parse が拒否する
- **`argv[0] === "hook"` で早期 dispatch**: `cheaper hook` は常に
  command 名が先頭に来るため位置固定で十分（flag 前置の組み合わせ
  は hook contract に存在しない）

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `pnpm build`（tsc -b）pass
- 実機 smoke（`/tmp/hooksmoke` で 132KB ファイル = 36000 tokens）:
  - windsurf deny: stderr 文言一致 + exit 2 / allow: silent 0
  - windsurf shell（`pre_run_command` + `cat big.txt`）: exit 2
  - copilot deny / shell deny: `permissionDecision` JSON + exit 0
  - copilot cross-agent payload: dual-schema allow JSON + exit 0
  - pretooluse Read/Bash deny: `hookSpecificOutput` JSON + exit 0
  - cursor file/shell deny: `{"permission":"deny",...}` + exit 0、
    allow / malformed stdin: `{"permission":"allow"}` + exit 0
  - malformed stdin・未知 event・未知 target・引数無し・未知
    flag: 全て silent exit 0（cursor は allow JSON）
- `test/args.test.ts` 31 pass（HELP 変更の回帰なし）

## Risks

- payload `cwd` が POSIX 形式のまま Windows で届くと
  `path.resolve` が誤解釈して fail-open 化する（JS 版も同じ制約、
  parity 維持済み。実機で Cursor が送る形式に依存）
- contract test（task 3）で stdout/stderr/exit を網羅検証するまで
  parity は実機 smoke ベース

## Follow-ups

- task 3: contract test 新設 + 既存 e2e 移植
