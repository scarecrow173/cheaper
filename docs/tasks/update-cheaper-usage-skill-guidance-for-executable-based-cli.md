---
id: "TASK-034z3zQ6V08kM7gUKnUzfL"
type: "task"
status: "done"
title: "Update cheaper-usage skill guidance for executable-based CLI"
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
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  implemented-by:
    - "docs/impl/ir/update-cheaper-usage-guidance.md"
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
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
    - "docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Update cheaper-usage skill guidance for executable-based CLI

## Work

`packages/apm/.apm/skills/cheaper-usage/`（または該当する
skill 定義）の CLI location guidance を executable 基準に更新する。

- `node ./cli/bin/cheaper.mjs` 前提の記述を、launcher
  `./cheaper` / `./cheaper.ps1` 経由の executable 呼び出しに
  合わせる
- transitional 期間中は bundle（`cheaper.mjs`）も同梱される点を
  明記し、Phase 4 で hook 経路が切り替わる旨を簡潔に記す

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md

## Dependencies

- なし（launcher/スクリプトと独立）

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/`（変更）

## Done When

- [ ] <!-- completion criterion -->

## Verification

- [ ] <!-- command, test, review step, or observable behavior -->
