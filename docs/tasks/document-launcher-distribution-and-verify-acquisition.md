---
id: "TASK-034zQTXMNDaUeqIR3aCRNf"
type: "task"
status: "todo"
title: "Document launcher distribution and verify acquisition"
created: "2026-10-04"
updated: "2026-10-04"
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
    - "../plans/implement-version-pinned-launcher-acquisition.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-version-pinned-launcher-acquisition.md"
    - "../tasks/implement-native-launcher-acquisition-and-hook-routing.md"
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
    - "../test-specs/version-pinned-launcher-integrity-and-stream-preservation.md"
  references: []
  defers: []
  deferred-by: []
---

# Distribution guide and final verification
## Work
1. 新規distribution guideとusage skillへの取得経路説明を作成する。
2. 既存deployed e2eとcanonical checkを実行しreportへevidenceを保存する。
3. doc-maintenance preview/applyとfocused exit auditを行いchainを終端にする。
## Classification
normal-plan-task
## Files
docs/guides/distribution.md、packages/apm/.apm/skills/cheaper-usage/SKILL.md の説明追記、own chain/index。
## Done When
guideがproxy/offline pair/migration/recovery/unsupported/Copilot欠落制約を説明しcanonical checkとfocused auditがpass。
## Verification
- [ ] mise exec -- pnpm exec vitest run packages/cli/test/e2e/deployed-executable.test.ts exits 0。
- [ ] mise run check exits 0。
- [ ] mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js --current exit-audit --focus docs/plans/implement-version-pinned-launcher-acquisition.md --signal exit-audit-pass --json がcompleteを返す。
## Scope
release.mdの手順変更と公開Release検証は#73-B/C。
