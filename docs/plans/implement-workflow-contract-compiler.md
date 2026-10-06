---
id: "PLAN-034zQ9Pet2QAuh0FP3ku3u"
type: "plan"
status: "completed"
title: "Workflow contract compiler を実装する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/64"]
  implements: ["../specs/workflow-contract-compiler.md"]
  derives-from: ["../designs/workflow-contract-compiler.md"]
  verified-by: ["../test-specs/workflow-compiler-contract-guarantees.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Workflow contract compiler を実装する

## Goal / Scope

承認済み spec/design と共有 ADR D1/D4/D5/D6 に従う単一 compiler stream。engine/CLI/Artifact Store/resource 推論実装/依存変更は含まない。hot files と他 package を編集しない。

## File Map

| Path | 責任 |
| --- | --- |
| `packages/workflow/src/types.ts` | Workflow JSON、ports、CompiledJob.manifest、compiled hooks、projection 型 |
| `packages/workflow/src/json.ts` | JSON 境界、clone、own-key、安全な object 構築 |
| `packages/workflow/src/bindings.ts` | exact wrapper scan/materialization、result-root RFC 6901、明確に不可能な schema path |
| `packages/workflow/src/schema.ts` | WORKFLOW_JSON_SCHEMA、core compileSchema、catalog/default 検査 |
| `packages/workflow/src/dag.ts` | edge dedupe/self/unknown/cycle、code-unit 昇順 Kahn order |
| `packages/workflow/src/compiler.ts` | defaults/snapshot/async compile/resource port/job/output validation |
| `packages/workflow/src/interface.ts` | invocation 不要の public-only projection |
| `packages/workflow/src/index.ts` | public exports |
| `packages/workflow/test/{bindings,compiler,interface}.test.ts` | AC1–AC5/resource/snapshot の contract tests |
| 自分の test-spec/tasks/impl chain | AC7 fixture/guidance、検証実績、graph trace |

既存 dependency guard は変更せず実行する。

## Contract Clarifications

- order/edges が順序の正本。Record の integer-like key に対する JS enumeration 順は ordering contract にしない。tie-break は code-unit 昇順。
- unknown jobId は INVALID_REQUEST、job hook の producer result missing は INVALID_INPUT、output 側 missing は INVALID_OUTPUT。
- invocation 非 object/JSON 非互換値、resource path の非 string 解決値は INVALID_INPUT。
- default:null は schema が null を受理する場合のみ optional。nested default 補完なし。
- resource port 実行時は全 job manifest 必須・CompiledJob に保持。未実行は manifest 無しで fallback。空配列宣言は port 必須。
- 公開 field mutation は private snapshot/validators に影響しない。元 binding から一度だけ解決し挿入値は再 scan しない。

## Implementation Sequence / Dependency Graph

| Step | Action | Depends on |
| --- | --- | --- |
| A | plan 承認後 test-spec をリンクする | plan gate |
| B | AC1–AC5/resource/snapshot の failing tests を作成する | A |
| C | targeted tests で未実装の failure を確認する | B |
| D | types/json/bindings/schema/dag を実装する | C |
| E | compiler/interface/index を実装する | D |
| F | targeted tests と workflow build を実行する | E |
| G | 実在 schema 互換 fixture と token-efficiency guidance を IR に記録する | F |
| H | canonical check（host contention 時は承認済み委譲方針）、doc-maintenance plan→apply、exit-audit を実行する | G |
| I | 論理 commit、report と worker_done を送信する | H |

critical path は A→B→C→D→E→F→G→H→I。型と snapshot を共有するため inline 実行。新たな worker は起動しない。task docs の各 slice は red/implementation/green を別 action として記録する。

## Risk Register

| Risk | Likelihood / Impact | 対策 |
| --- | --- | --- |
| data loss | low / low | metadata 参照と計算のみ、filesystem/handler/process 実行なし |
| breaking change | medium / medium | 承認済み #62/#70 consumer API を export、placeholder から追加 |
| #63/#65 availability | high / medium | fake catalog/async inference port、未注入 resource は fail closed |
| schema path 過剰推論 | medium / medium | union/ref/open schema は runtime 検査へ |
| snapshot/non-scan 破綻 | medium / high | 偽 $from、compiled 表示 mutation regression tests |
| cli e2e timeout | medium / low | 既知 timeout file の単独再実行と report 証跡 |

## Rollback Strategy

永続データ変換なし。統合後 contract regression 時は coordinator が compiler の論理 commit を revert し、guard と canonical check で scaffolding 状態を確認。worker は破壊的 Git 操作を行わない。

## Verification Matrix

| Step | Command / Evidence | Pass condition |
| --- | --- | --- |
| A | plan/test-spec/task doc-status | relation/status 整合 |
| B/C | `mise exec -- pnpm --filter @cheaper/workflow test` | red: 未実装 contract failures、既存 guard pass |
| D/E/F | 同 targeted test と `mise exec -- pnpm --filter @cheaper/workflow build` | AC1–AC5/resource/snapshot/型/guard pass |
| G | IR fixture を compiler test と一致させる | 実在 schema 互換 fixture、AC7 guidance |
| H | canonical check、doc-maintenance、focused exit-audit | focused suite/CLI 単独再実行/lint/typecheck/build pass、chain terminal、audit pass。canonical check deferred to coordinator merge run |
| I | git status/log | scope 限定、logical commits、push なし |

## Self-Review / Review Handoff

AC1=inputs/default、AC2=bindings、AC3=DAG、AC4=job/output hooks、AC5=projection、AC6=guard/canonical、AC7=IR fixture。全 requirement を covered とする。未解決仕様はない。resource policy/normalization は #65-B port が所有する。

## Graph Resume

run-until-yield。共有 ADR lineage による非 plan focus-required は coordinator 承認済み例外で手動 plan-doc→approval を実施。本 plan focus で router を再開する。承認未回答時は同じ ask を再開し待機する。



AC7 guidance は本 chain の IR に記録する。coordinator plan review が IR のみも受理可と確認し、fixture を tests に一致させた。


