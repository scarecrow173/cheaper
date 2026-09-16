---
id: "TASK-0069"
type: "task"
status: "done"
title: "Token counter CLI/tool parity tests and full verification"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/0007-canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0066-implement-core-token-counter.md"
    - "docs/tasks/0067-add-count-tokens-tool.md"
    - "docs/tasks/0068-add-count-tokens-cli.md"
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
---

# Token counter parity tests and full verification

## Work

同一 file について `count-tokens` CLI と `count_tokens` tool が同一 count を返す
ことを e2e（fake stream で tool call を script する）で検証する。あわせて
workspace escape・symlink・不存在 file・binary file・version 定数一致の
回帰を揃え、docs index を更新して全 baseline を確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0012-token-counter-contract.md`
- `docs/specs/0007-canonical-token-counter.md`

## Dependencies

- TASK-0066、TASK-0067、TASK-0068

## Affected Files

- `packages/cli/test/e2e/`（parity fixture/test）
- `docs/{specs,adr,designs,plans,tasks}/README.md` index

## Done When

- [ ] CLI/tool parity test が green
- [ ] escape/symlink/missing/binary の振る舞いが test 済み
- [ ] 各 index README に SPEC-0007/ADR-0011/DESIGN-0012/PLAN-0007/TASK-0066–0069 が登録される

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` 全 green
