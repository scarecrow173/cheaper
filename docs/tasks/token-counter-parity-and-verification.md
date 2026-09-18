---
id: "TASK-034r05iHQeQpDpUao5Hjj3"
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
    - "docs/plans/canonical-token-counter.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/implement-core-token-counter.md"
    - "docs/tasks/add-count-tokens-tool.md"
    - "docs/tasks/add-count-tokens-cli.md"
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

- `docs/designs/token-counter-contract.md`
- `docs/specs/canonical-token-counter.md`

## Dependencies

- TASK-034r05iHQeQpDpQjb4veqT、TASK-034r05iHQeQpDpSeJahHce、TASK-034r05iHQeQpDpSS51up2r

## Affected Files

- `packages/cli/test/e2e/`（parity fixture/test）
- `docs/{specs,adr,designs,plans,tasks}/README.md` index

## Done When

- [ ] CLI/tool parity test が green
- [ ] escape/symlink/missing/binary の振る舞いが test 済み
- [ ] 各 index README に SPEC-034r05iHK8bA7gdbCbWupr/ADR-034r05iHrZUE6W3U1rbJq4/DESIGN-034r05iHy2gdnoEfk3jERu/PLAN-034r05iHE1FbkoVrzp9sac/TASK-034r05iHQeQpDpQjb4veqT–0069 が登録される

## Verification

- [ ] `mise exec -- pnpm -r build && pnpm -r test && pnpm lint && pnpm typecheck` 全 green
