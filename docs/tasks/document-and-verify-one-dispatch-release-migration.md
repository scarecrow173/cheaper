---
id: "TASK-034zTWjm1NjME6ZzgLXNF7"
type: "task"
status: "done"
title: "Document and verify one-dispatch release migration"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: ["../plans/implement-one-dispatch-release-workflow.md"]
  implemented-by: ["../impl/ir/document-and-verify-one-dispatch-release-migration.md"]
  depends-on: ["../plans/implement-one-dispatch-release-workflow.md","./wire-one-dispatch-release-publication.md"]
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/release-manifest-and-snapshot-integrity.md","../test-specs/release-publication-and-native-smoke.md"]
  references: []
  defers: []
  deferred-by: []
---

# Release migration guide and verification

## Work

plan slice3としてguideを1dispatch公開/旧LFS併存へ更新し、最終evidenceとchain終端statusを整合する。

## Scope

release.md/distribution.md、guide review anchors、chain docs/index、指定Temp report。canonical fullcheck / native3 / 実publishはcoordinator/CIへdefer。

## Dependencies

publication workflow taskがdone。

## Done When

- [x] acquisition互換は新経路のみ、旧sums無しfailclosed、CI-only、manualdraftrecovery、snapshotlocalinstallをguideへ記載。
- [x] V6 focused-standard-checks evidence、chain+sharedADR scopedblocker0、IR/index/plan/task終端statusとcommit。
- [x] reportにcanonical check deferred to coordinator merge runとglobalownership外findingsを明記。

## Verification

- [x] `mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs` exits 0。
- [x] `mise exec -- pnpm lint` exits 0。
- [x] `mise exec -- pnpm typecheck` exits 0。
- [x] `mise exec -- pnpm build` exits 0。
- [x] `mise exec -- node scripts/check-doc-ids.mjs` exits 0。
- [x] `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` をchain+sharedADRでfilterしblocking0。
- [x] Manual: named guide checksをreportへ記録（proseのsource-text snapshot testでpassを代用しない）。
