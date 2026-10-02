---
id: "IMPL-034z57iY91Zc8YChIAuBty"
type: "impl"
status: "completed"
title: "port routing policy to cli hook module"
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
    - "docs/tasks/port-routing-policy-js-to-packages-cli-src-hook-routing-policy-ts.md"
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

# port routing policy to cli hook module

## Summary

`packages/apm/.apm/hooks/routing-policy.js`（393 行）を
`packages/cli/src/hook/routing-policy.ts` に挙動不変で移植し、
`count-tokens` 子 process spawn を in-process の
`countFileTokens` 経路に置き換えた。計測 pipeline は新規
`packages/cli/src/count.ts` に抽出して `count-tokens` command と
共有した。

## Context

- task: docs/tasks/port-routing-policy-js-to-packages-cli-src-hook-routing-policy-ts.md
- plan: docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md
- design: docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md

## Implementation

### Changes

- `packages/cli/src/hook/routing-policy.ts`（新規）: lexical
  containment → `statSync().isFile()` → byte fast path
  （`CHEAPER_READ_TOKEN_THRESHOLD`、default 8192・`parseInt` 部分
  parse 許容）の順序を verbatim 維持。shell tokenizer・
  `extractReadCandidates`・`CWD_CHANGING_COMMANDS`・deny 文言を
  JS 版と同一ロジックで移植。`evaluate()` は async 化
- `packages/cli/src/count.ts`（新規）: `countMaxBytes` と
  `countWorkspaceFileTokens`（canonicalize → config dirs → policy
  → `countFileTokens`）を切り出し、`defaultBuiltinDir` を export
- `packages/cli/src/cli.ts`: `count-tokens` command を
  `countWorkspaceFileTokens` 経由に変更、局所の
  `countMaxBytes`/`defaultBuiltinDir` を共有実装に置換
- `packages/cli/test/hook/routing-policy.test.ts`（新規）: 14 件
  （threshold parse・containment・fast path・deny 文言・
  FILE_TOO_LARGE fail-closed・shell candidate・cwd-changing
  fail-open・malformed request）

### Decisions During Implementation

- **30s subprocess timeout の復元**: `countFileTokens` は signal を
  受け付けないため、measured path 全体を `Promise.race` +
  `AbortSignal.timeout(30_000)` で囲み timeout → allow とした
  （FIFO / 死んだ network FS の hang 対策）
- **lazy dynamic import**: `../count.js`・`@cheaper/core` は
  measured path 内でのみ import。byte fast path の hook 起動に
  tokenizer eager init（module scope `getEncoding`）を載せない
- **count pipeline の共有先**: `cli.ts` から直接 import すると
  runtime-pi を含む全 graph を引き込むため `count.ts` を新設
- **error mapping**: `FileTooLargeError` → deny（JS 版の
  FILE_TOO_LARGE envelope 解析と同文言）、それ以外は全て allow
  （JS 版の spawn error / non-0 exit / parse 失敗 → allow と同値）
- **symlinked cwd quirk の再現**: raw cwd で resolve した abs path
  を canonical root へ渡すため、cwd が realpath と一致しない環境
  では `WorkspaceEscapeError` → allow（JS 版と同じ fail-open）

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `pnpm build`（tsc -b）pass
- `vitest run test/hook/routing-policy.test.ts`: 14 pass
- `pnpm -F @cheaper/cli test`: 22 files / 249 pass / 3 skipped
- `pnpm bundle` 再生成済み（freshness 維持）

## Risks

- in-process 化により `run.error` 系の fail-open 経路が
  `measureTokens` の catch に集約された。新規 error 型が deny に
  化けないことを contract test（task 3）で二重確認する
- unhandled rejection: `evaluate` 呼び出し側の catch が必須
  （task 2 の hook entry catch-all で担保）

## Follow-ups

- task 2（target adapter + `cheaper hook` command 配線）
