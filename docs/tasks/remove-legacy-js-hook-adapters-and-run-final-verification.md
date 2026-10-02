---
id: "TASK-034z52W87FtawtD6bYW6RR"
type: "task"
status: "done"
title: "remove legacy js hook adapters and run final verification"
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
    - "docs/impl/ir/remove-legacy-js-adapters-and-final-verify.md"
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
    - "docs/tasks/add-hook-contract-tests-and-port-existing-adapter-e2e.md"
    - "docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md"
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

# remove legacy js hook adapters and run final verification

## Work

旧 JS hook adapter を削除し、最終検証を実施する。

- `pre-read-code.js`・`pre-read-code-copilot.js`・
  `pre-read-code-cursor.js`・`pre-read-code-pretooluse.js`・
  `pre-run-command.js`・`routing-policy.js` の 6 file を削除
- `pnpm bundle` 再生成 + `mise run check` で freshness 通過
- `dist/sea/cheaper-windows-x64.exe hook ...` で全 target の
  smoke を再確認
- IR に timeout 復元・usage.jsonl 非記録・`cheaper.mjs` 残置の
  決定を記録

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md

## Dependencies

- docs/tasks/add-hook-contract-tests-and-port-existing-adapter-e2e.md
- docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md

## Affected Files

- `packages/apm/.apm/hooks/*.js`（削除）

## Done When

- [ ] JS adapter 6 file が削除されている
- [ ] `mise run check` 緑（bundle freshness 含む）
- [ ] SEA exe で hook が動作する

## Verification

- [ ] `mise run check` pass
- [ ] `hooks.json` が `node ` を含まない
- [ ] 削除後の executable 経路で deny/allow が再現できる
