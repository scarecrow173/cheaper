---
id: "TASK-034rAcG63A1KaVZH5VyAWA"
type: "task"
status: "done"
title: "Contract matrix and parent-agent guide updates"
created: "2026-09-19"
updated: "2026-09-19"
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
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/cross-agent-native-routing-adapters-implementation.md"
    - "docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md"
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

# Contract matrix and parent-agent guide updates

## Work

実装と実測結果を docs へ反映する:

- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
  — target matrix を更新（Claude `PreToolUse` supported、Cursor
  `beforeReadFile`/`beforeShellExecution` supported、Codex shell-only
  / file-read unsupported + upstream issue、APM pass-through 実測、
  per-target deny / fail semantics）。
- `docs/guides/parent-agent-integration.md` — per-target routing
  support 表、file-read / shell の対応状況、fail-open/fail-closed
  semantics、Codex file-read unsupported の記述。
- spec / design 側で defer した事項（#42 seam、Codex file-read、
  実機 E2E）との整合を確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/cross-agent-native-routing-adapters-implementation.md`
- `docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md`

## Dependencies

- `docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md`
  （実装・test が確定してから doc を更新する）

## Affected Files

- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
- `docs/guides/parent-agent-integration.md`

## Done When

- [ ] matrix の target 行が実装（adapter / event / deny / fail
      semantics）と一致する
- [ ] guide の support matrix が file-read / shell の target 別
      対応を正確に記述する
- [ ] Codex file-read unsupported の根拠（upstream gap）が記録される

## Verification

- [ ] `mise run check` が pass（doc audit を含む）
