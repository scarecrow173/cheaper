---
id: "TASK-034r05iHQeQpDq84iLFgT3"
type: "task"
status: "done"
title: "Runtime-pi scope and deny enforcement in sandboxed tools"
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
    - "docs/plans/write-scope-and-privacy-deny.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/core-deny-matcher-and-write-scope-field.md"
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

# Runtime-pi scope and deny enforcement in sandboxed tools

## Work

- `operations.ts`: `resolve()` に deny check、`resolveForWrite` で scope
  check（atomicWrite の target、mkdir の祖先規則、tmp は scope 対象外）
- `walker.ts`: deny file を yield しない・deny dir に潜らない、
  `DENIED_MESSAGE` と `isAccessDeniedError` 拡張
- `ls.readdir` の deny entry 除外、`count_tokens` tool の deny check
- `registry.ts`: `denylist`/`writeScope` opts の plumbing
- `run.ts`: `worker.cheaper.writeScope` → input field から allowlist 抽出
  （string または `{path}` items、fail closed で INVALID_INPUT）
- unit test: 各経路の拒否・許可・rejectedToolCalls 計上

## Acceptance Criteria

- [x] scope 外 write/edit が tool error になる
- [x] deny path の read/count_tokens が拒否され walk/ls で除外される
- [x] scope 未宣言 worker は従来どおり write 可能

## Done

operations/walker/registry/run への強制実装と unit tests。
