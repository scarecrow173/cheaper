---
id: "TASK-034r05iHQeQpDqac6FjqrJ"
type: "task"
status: "done"
title: "Writer writeScope declaration and e2e verification"
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
    - "docs/tasks/runtime-scope-and-deny-enforcement.md"
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

# Writer writeScope declaration and e2e verification

## Work

- `code-writer.md`/`doc-writer.md` frontmatter に `writeScope: targets` を宣言
- e2e: fake-stream で targets 外 write が tool error になる・
  `.env` read が denied になる・targets 内 write は成功することを検証
- index README 更新・`pnpm -r build/test`・lint・typecheck・doc audit

## Acceptance Criteria

- [x] 両 writer に writeScope 宣言
- [x] e2e で scope/deny が観測可能
- [x] 全 baseline + doc audit clean

## Done

frontmatter 宣言、e2e 追加、index/README 更新、全検証 pass。
