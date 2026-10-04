---
id: "TASK-034zTWiLbk14ZhGMpiNo7M"
type: "task"
status: "todo"
title: "Wire one-dispatch release publication"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: ["../plans/implement-one-dispatch-release-workflow.md"]
  implemented-by: []
  depends-on: ["../plans/implement-one-dispatch-release-workflow.md","./prepare-one-dispatch-release-artifacts.md"]
  blocks: ["./document-and-verify-one-dispatch-release-migration.md"]
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/release-publication-and-native-smoke.md"]
  references: []
  defers: []
  deferred-by: []
---

# One-dispatch publication workflow

## Work

plan slice2としてIRを開き、smoke failure / workflow gatesのREDを確認し、stdlib smoke helperと新workflow / optional expected_version gateを実装する。

## Scope

scripts/smoke-release.mjs、scripts/test/release-workflow.test.mjs、release-acquisition.yml、build-executables.ymlの最小連携。旧release.yml/LFS/packages/依存は変更しない。実Releaseは発火しない。

## Dependencies

artifact preparationがdoneでありsharedlibraryが使えること。

## Done When

- [ ] slice2 namedtestsでDAG/tagref/draft順序/permission/oldcaller互換を確認。
- [ ] IRにCI-only境界を記録しcompleted / Conventional Commit。

## Verification

- [ ] `mise exec -- node --test scripts/test/release-workflow.test.mjs` exits 0。
- [ ] `mise exec -- pnpm exec biome check scripts/smoke-release.mjs scripts/test/release-workflow.test.mjs` exits 0。

