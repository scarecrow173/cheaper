---
id: "TASK-034z52KS4mSEHAIf28JHuj"
type: "task"
status: "done"
title: "add hook contract tests and port existing adapter e2e"
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
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  implemented-by:
    - "docs/impl/ir/hook-contract-tests-and-adapter-e2e-port.md"
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
    - "docs/tasks/add-cheaper-hook-command-with-target-adapters.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# add hook contract tests and port existing adapter e2e

## Work

target 別 contract test を新設し、既存 adapter e2e を
`cheaper hook` 呼び出しに移植する。

- `packages/cli/test/hook/` に target 別 contract test を追加:
  deny/allow/fail-open 各系統、malformed payload → allow、
  FILE_TOO_LARGE → deny、catch-all（policy 破損・cwd 不存在・
  parse 失敗）で非 0 exit を返さない
- contract test は `node packages/cli/bin/cheaper.js hook ...`
  （bundled mjs 可）を直接 spawn。launcher 経由は host exe 存在時の
  smoke に分離（CI ubuntu に linux-x64 exe 無し）
- 既存 `pre-read-hook.test.ts` / `copilot-pre-read-hook.test.ts` /
  `cross-agent-routing-adapters.test.ts`（~900 行、`node <adapter>`
  spawn）を `cheaper hook` 呼び出しに書き換え、JS adapter 削除前に
  両経路の parity を確認
- `apm-install.test.ts` / `apm-install-targets.test.ts` /
  `delegation-loop.test.ts` の hooks.json assert を executable
  呼び出し形に更新
- SEA exe 実機 smoke: `dist/sea/cheaper-windows-x64.exe hook ...` で
  fixture payload が Node 非依存で処理されることを確認

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md
- docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md

## Dependencies

- docs/tasks/add-cheaper-hook-command-with-target-adapters.md

## Affected Files

- `packages/cli/test/hook/`（新規）
- `packages/cli/test/e2e/{pre-read-hook,copilot-pre-read-hook,
  cross-agent-routing-adapters,apm-install,apm-install-targets,
  delegation-loop}.test.ts`

## Done When

- [ ] test-spec の Rules/Examples が全て test に写像されている
- [ ] 既存 adapter e2e が `cheaper hook` 経路で pass（JS adapter と
      同一 fixture で parity）
- [ ] SEA exe での hook smoke が Node 不在で動く

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/cli test` 緑
- [ ] `dist/sea/cheaper-windows-x64.exe hook <event> --target <t>` が
      PATH から Node を除去しても動作
