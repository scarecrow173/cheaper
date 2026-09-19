---
id: "TASK-034rBt4e20Wrlc3FH3NQWd"
type: "task"
status: "in-progress"
title: "Shell-read docs and final verification"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/shell-read-candidate-extraction-implementation.md"
    - "docs/tasks/shell-read-contract-tests.md"
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

# Shell-read docs and final verification

## Work

`docs/guides/parent-agent-integration.md` を更新し、shell 経由 read の
検出対象 command・target 別対応状況（shell hook を emit できる target
一覧、Codex の file-read unsupported は継続）・fail-open/fail-closed
挙動・既知制限（`cd` 追跡なし、解釈不能構文は fail-open、対象外
command 経路は非目標）を記述する。最後に `mise run check` で canonical
verification を通す。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/shell-read-candidate-extraction-implementation.md`
- `docs/specs/shell-read-large-file-blocking.md`

## Dependencies

- `docs/tasks/shell-read-contract-tests.md`（実装と test の完了が先）

## Affected Files

- `docs/guides/parent-agent-integration.md`

## Done When

- [ ] guide に shell 経由検出の挙動・target 別対応・既知制限が記載される
- [ ] `mise run check` が pass する

## Verification

- [ ] `mise run check`
