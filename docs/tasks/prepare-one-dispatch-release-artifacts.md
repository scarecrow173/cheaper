---
id: "TASK-034zTWibd7OKesgJgoGxJu"
type: "task"
status: "done"
title: "Prepare one-dispatch release artifacts"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  implements: ["../plans/implement-one-dispatch-release-workflow.md"]
  implemented-by: ["../impl/ir/prepare-one-dispatch-release-artifacts.md"]
  depends-on: ["../plans/implement-one-dispatch-release-workflow.md"]
  blocks: ["./wire-one-dispatch-release-publication.md"]
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/release-manifest-and-snapshot-integrity.md"]
  references: []
  defers: []
  deferred-by: []
---

# Release artifact preparation

## Work

plan slice1としてIRを開き、strict manifest/tag/hash/snapshot/dry-runのREDを確認してstdlib libraryと2 CLIを実装する。

## Scope

scripts/lib/release-artifacts.mjs、generate-release-checksums.mjs、assemble-release-bundle.mjs、scripts/test/release-artifacts.test.mjs。packages/dependencies/LFS変更とpublisherは対象外。

## Dependencies

PLAN-034zTJptHTdovrvYW7wQ1k の承認済み契約。完了後publication taskをunblockする。

## Done When

- [x] planのslice1 named testsがGREEN、tarroundtrip / source非変更evidenceをIRに記録。0755はLinux producer専用でWindowsではskip。
- [x] IR completed / focusedreview / Conventional Commit。

## Verification

- [x] `mise exec -- node --test scripts/test/release-artifacts.test.mjs` exits 0（15 pass / 1 Linux-only skip）。
- [x] scoped Biomeの対象を確認。root設定がscripts/**をexcludeするためlint evidenceは最終sliceのpnpm lintに記録する（config変更なし）。
