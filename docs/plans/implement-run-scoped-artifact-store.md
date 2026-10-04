---
id: "PLAN-034zThvLa4bK934vj52jaC"
type: "plan"
status: "completed"
title: "Implement run-scoped artifact store"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/66"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/run-scoped-artifact-store.md"
  implemented-by: ["TASK-034zTn9hnvGsV4G8kGiHvT"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/run-scoped-artifact-store.md"
    - "docs/designs/run-scoped-artifact-store.md"
  derived-by: []
  verifies: []
  verified-by: ["TSPEC-034zTn31vnH33RcM6g7VYS"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/keep-artifact-storage-invocation-local-and-bounded.md"]
  defers: []
  deferred-by: []
---

# Implement run-scoped artifact store

## Goal and Scope

SPEC-034zSQRLxNXgJDxjmUBBB8 AC1-AC7 を 1 coherent slice として実装する。port と store は同一公開契約を分担し独立リリースしないため 1 plan/1 task とする。共有 D4/D5/D7 は維持、Engine/CLI/core/runtime-pi と既存 tool 実装は変更しない。

## File Map

| Path | Responsibility |
| --- | --- |
| packages/tool-runtime/src/artifact.ts | plain JSON reference、producer、explicit read source、bounded ArtifactReader port 型 |
| packages/tool-runtime/src/index.ts | 新 contract の export |
| packages/workflow/src/artifacts.ts | injected caps、opaque ids、atomic copy store、bounded preview/read、dispose/abort/finally、metadata projection |
| packages/workflow/src/index.ts | store と同一 reference 型の export |
| packages/workflow/test/artifacts.test.ts | AC1-AC7 と JSON binding compatibility を検証 |
| docs/test-specs/run-scoped-artifact-boundaries.md | capacity/namespace/lifetime の永続 test intent |
| docs/tasks/implement-bounded-artifact-store.md | 単一契約 slice の手順と verify |
| docs/impl/ir/implement-bounded-artifact-store.md | in-progress から evidence を追記 |

## Dependency Graph

| Step | Depends on | Action |
| --- | --- | --- |
| A | approved plan | contract test を追加 |
| B | A | focused test の failure が未実装 API 由来であることを確認 |
| C | B | port/store/barrel を実装 |
| D | C | focused suite と lint/typecheck/build を実行 |
| E | D | IR/tasks/plan を完了し scoped exit-audit を実行、論理変更を commit |

critical path は A→B→C→D→E。port だけでは store 検証ができず独立 work stream は無し、inline 実行する。

## Implementation Sequence

1. test に metadata/provenance/JsonValue clone、caps の境界と原子性、UTF-8/binary preview、異 run/path escape/source 分離、bounded read/copy、防御 lifecycle を定義。
2. RED を未実装 export/module で確認し evidence に記録。
3. tool-runtime contract を追加し workflow store から import。参照は frozen plain object、data は string|Uint8Array、UUID は組込 node:crypto。新依存は無し。
4. put は cap と producer 一意性を mutation 前に検証。read は run/id/provenance と range を検証し maxReadBytes 内 copy。
5. finally dispose、abort listener の即 clear/解除、live reader 失効、final unavailable metadata projection を実装。
6. focused test を GREEN にし lint/typecheck/build を検証、IR と source relations を整合させる。

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| cleanup で中間 data を失う | medium | medium | disposable lifetime を公開契約にし final は unavailable、必要 data は exit 前 bounded read |
| API breaking change | low | medium | additive port/store/barrel のみ、既存 registry/read/compiler は変更しない |
| toolchain/external dependency 不在 | low | medium | setup 済み、node/pnpm locked toolchain を mise exec 経由で使い新 dependency を追加しない |
| metadata に full content 混入 | medium | high | private fields と explicit projection/JSON serialization test |
| abort 後 delayed consumer が再 read | medium | medium | closed-state が即 fail、drain は engine/runtime 所有 |

## Rollback Strategy

永続 data/migration は無し。公開 API 回帰が trigger。coordinator が関連 implementation commit を git revert し、pnpm typecheck と focused suites で従来 API が回復することを確認する。履歴破壊や現在の branch の reset は行わない。

## Verification Matrix

| Step | Command / Criteria | Pass Condition |
| --- | --- | --- |
| A/B | mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/artifacts.test.ts | RED は未実装 API が原因と確認（verification success と区別） |
| C/D | mise exec -- pnpm --filter @cheaper/workflow test | exit 0 |
| D | mise exec -- pnpm --filter @cheaper/tool-runtime test | exit 0 |
| D | mise exec -- pnpm lint | exit 0 |
| D | mise exec -- pnpm typecheck | exit 0 |
| D | mise exec -- pnpm build | exit 0 |
| E | scoped doc-status + impl audit | chain + shared ADR に blocking findings 0 |

canonical check deferred to coordinator merge run。CLI 変更なし、CLI tests は対象外。

## Self-Review

AC1-AC7 は A/C/D で全て pin。file map は許可範囲のみ、caps は owning engine 注入、future RuntimePolicy schema と InvocationToolRuntimeOptions.artifacts seam は後続。新依存なし、placeholder は無し。

## Implementation Handoff and Review

実装は順序依存の 1 slice として inline 実行し、coordinator の plan approval 後に test-spec/task/IR を作る。review は spec/design/caps/source/lifetime の既承認契約を基準に行う。
