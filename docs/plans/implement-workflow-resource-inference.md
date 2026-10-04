---
id: "PLAN-034zTu37tgDGI306OPBitz"
type: "plan"
status: "completed"
title: "Workflow resource inference を実装する"
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
    - "../specs/workflow-resource-inference.md"
  implemented-by:
    - "../tasks/implement-workflow-resource-inference.md"
    - "../tasks/integrate-workflow-resource-inference.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/workflow-resource-inference.md"
    - "../adr/workflow-resource-inference-sources.md"
  derived-by:
    - "../tasks/implement-workflow-resource-inference.md"
    - "../tasks/integrate-workflow-resource-inference.md"
  verifies: []
  verified-by:
    - "../test-specs/workflow-resource-inference.md"
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"

  defers: []
  deferred-by: []
---

# Workflow resource inference を実装する

## Goal and Scope
承認済み SPEC-034zSWvgiB9gHQpjguNJ3M / DESIGN-034zSWw1LYrRvcB471oQm2 と共有 ADR D1/D4/D5 を実装する。単一 inference stream と compiler consumer の二つの順序付き slice。core/runtime-pi/CLI/process executor/engine/policy/dependency は変更しない。

## File Map
| Path | Responsibility |
| --- | --- |
| packages/tool-runtime/src/catalog.ts | optional pure ToolMetadata.resources と built-in access descriptor |
| packages/workflow/src/resource-inference.ts | source extraction、core normalization、ownership/edge/conflict/理由 |
| packages/workflow/src/types.ts | optional process source、optional conflicts/diagnostics 型。固定 core 型は変更しない |
| packages/workflow/src/compiler.ts | 注入された port を全 job で実行し diagnostics/conflicts を clone 保存 |
| packages/workflow/src/index.ts | inference 実装を public export |
| packages/workflow/test/resource-inference.test.ts | source/identity/fallback/ownership/ordering の契約保証 |
| packages/workflow/test/compiler.test.ts | port 注入時の全 job manifest と結果 clone/edges の回帰 |
| docs/test-specs/workflow-resource-inference.md | AC1–AC9 の named rules/examples と保証の意図 |
| docs/tasks/implement-workflow-resource-inference.md | slice 1 の IR/RED/implementation/GREEN/commit と verified-by |
| docs/tasks/integrate-workflow-resource-inference.md | slice 2 の compiler gate/annotation と検証/exit audit |
| docs/impl/ir/workflow-resource-inference-implementation.md | slice 1 の red/green・境界・変更記録 |
| docs/impl/ir/workflow-resource-inference-compilation.md | slice 2 の統合・最終 verification 記録 |

## Dependency Graph
| Slice | Depends on | Blocks |
| --- | --- | --- |
| 1 canonical inference | approved spec/design/ADR | 2 |
| 2 compiler integration and verification | 1 | exit-audit |
critical path は 1→2。契約と export を共有するため同じ worker が inline で順に実行する。追加 worker は不要。

## Implementation Sequence
### Post-approval planning artifacts
1. plan gate の APPROVED を受けて plan status を approved にする。
2. test-spec-doc で docs/test-specs/workflow-resource-inference.md を作成し、下記 named cases を Rule/Example として固定する。verifies は spec と design。
3. plan の relations.verified-by を実在 test-spec へ populate し、spec/design の verified-by にもリンクする。
4. task-doc で docs/tasks/implement-workflow-resource-inference.md と docs/tasks/integrate-workflow-resource-inference.md を作成する（後者 depends-on 前者）。両 task の verified-by に test-spec を指定する。
5. plan relations.implemented-by と derived-by を実在する二 task へ populate、build_task_graph の selected task DAG を audit する。
6. graph が返した runnable slice に限って implementation-flow を開始する。
未作成の artifact に先行して frontmatter link を張らない。planning-flow は plan approved 後に test-spec/tasks を作成する hard gate を持つため、承認前の現在は verified-by/implemented-by を空に保ち、上記 steps で作成直後に populate する。test-spec-skip は使用しない。

### Slice 1: canonical inference
1. in-progress IR を開く。
2. catalog metadata、Worker writeScope、explicit scopes、no-access、fallback、process declaration、producer edge/conflict/duplicate、deterministic reversal を検証する failing tests を書く。
3. focused test を実行して inference export 未実装による RED を記録する。
4. optional metadata と source/result 型を追加する。
5. normalizeResourceManifest/normalizeResourceClaim と claimContains/resourcesOverlap/claimsConflict/resourceKey を使用する canonical 実装を追加する。
6. focused tests が GREEN になるまで causal failures を修正し記録する。
7. logical change を Conventional Commit で commit する。

### Slice 2: compiler consumer
1. in-progress IR を開く。
2. resources port 注入時は全 resources 省略でも全 job に manifest が付く observable change、needs/from dedupe、cycle、result clone の failing tests を追加する。
3. 既存 compiler の skip/result omission による RED を確認する。
4. compiler gate を ports.resources 注入に変更する。未注入 + 非空 explicit resources は従来どおり INVALID_REQUEST を維持する。result optional annotation を clone 保存する。
5. workflow full focused suite、catalog metadata 経由の契約 tests、dependency guard と lint/typecheck/build を実行する。
6. docs/status/index と scoped exit-audit を閉じ、Conventional Commit で commit する。

## Risk Register
| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| data loss | low | high | compile は filesystem mutation/handler 実行を行わない |
| breaking API | low | medium | optional metadata/port fields、未注入 compiler 契約保持 |
| toolchain/dependency availability | low | medium | fresh mise run setup 完了、locked deps を変更しない |
| unknown fallback semantic ordering | medium | high | dependencyReads と safety claims を分離する tests |
| Worker parent mkdir underclaim | medium | high | root exclusive safety 追加、ownership は exact |
| canonical aliases duplicate output | medium | high | core identity と stable pair error |

## Rollback Strategy
Trigger: known dependency/fallback 契約が壊れる場合。Procedure: coordinator がこの stream の logical commits を git revert して旧 port-only behavior に戻す（worker は破壊的 Git を実行しない）。Verification: workflow 既存 suite/typecheck/build。data migration や外部公開はない。

## Verification Matrix
| Step | Command or criteria | Pass condition |
| --- | --- | --- |
| 1 red | mise exec -- pnpm --filter @cheaper/workflow test -- test/resource-inference.test.ts | new inference 未実装による失敗を記録 |
| 1 green | mise exec -- pnpm --filter @cheaper/workflow test -- test/resource-inference.test.ts | exit 0、AC1–AC9 |
| 2 red | mise exec -- pnpm --filter @cheaper/workflow test -- test/compiler.test.ts | 全省略時 port skip / annotation omission の失敗を記録 |
| 2 green | mise exec -- pnpm --filter @cheaper/workflow test | exit 0、既存 bindings/interface/guard も pass |
| catalog | mise exec -- pnpm --filter @cheaper/tool-runtime test | exit 0、catalog extraction の後方互換 |
| lint | mise exec -- pnpm lint | exit 0 |
| types/build | mise exec -- pnpm typecheck / mise exec -- pnpm build | exit 0 |
| scoped docs | doc-status audit findings を chain+共有 ADR に filter | blocking findings 0、全 chain terminal |
canonical check deferred to coordinator merge run。CLI/bundled payload は触らず CLI suite は実行不要。

## Self-Review
AC1/2/6/8/9 は source normalization、AC3/4/5/7 は inference と compiler tests へ対応する。omitted/{} safety fallback と empty array no-access、process undeclared rejection、mutating/read-only capability rules は design と一致。placeholder と未解決 dependency はない。

## Review Handoff
local ADR gate は coordinator が APPROVED（accepted）とした。spec/design は msg_b28e978f1490 で APPROVED、修正項を反映済み。build/test は production provider proof や scheduler/engine 完成を意味しない。

## Implementation Handoff
inline execution で二 slice を順に進める。実装 stack は TDD・incremental implementation・impl-doc・Conventional Commit・verification-before-completion、独立レビューは coordinator の review-protocol に依頼する。planning-flow/implementation-flow の SUBAGENT-STOP に従い dispatch が指定した stack を使い profile 再生成を省略する。

## Coverage: named cases
| AC | Named case | Slice |
| --- | --- | --- |
| AC1 | catalog-read-write-edit-scopes / catalog-subtree-and-default / metadata-absent-fallback | 1 |
| AC2 | worker-write-scope-targets / worker-read-only-and-unknown-mutation / worker-underclaim-rejection / worker-ancestor-mkdir-safety | 1 |
| AC3 | produces-to-read / needs-from-resource-dedupe / inferred-cycle-rejection / no-self-edge | 1/2 |
| AC4 | writers-conflict-without-edge / unknown-read-does-not-order / exclusive-read-write-overlap | 1 |
| AC5 | duplicate-exact-output / subtree-overlapping-owners / canonical-alias-and-case-output / ordered-duplicate-still-rejected | 1/2 |
| AC6 | omitted-empty-object-fallback / declared-empty-read-array | 1/2 |
| AC7 | reversed-job-and-claim-byte-equality / no-input-mutation / full-manifests-on-injected-port / cloned-annotation-results | 1/2 |
| AC8 | core-boundary-escape / glob-safety-and-ownership / component-boundary / normalization-predicate-imports | 1 |
| AC9 | process-config-input-output-edge / undeclared-process-rejection / declared-empty-process-scopes | 1 |

## Plan review amendment（2026-10-04）
msg_6a9e776a22f3 の P1（planning artifact step / coverage 不足）と low（compiler gate 曖昧）を修正した。test-spec/tasks は承認後の必須 steps、skip/waiver は用いない。
