---
id: "IMPL-034zTneiPeW23igPepp7ph"
type: "impl"
status: "completed"
title: "Implement bounded artifact store"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - { type: "file", path: "packages/tool-runtime/src/artifact.ts" }
      - { type: "file", path: "packages/workflow/src/artifacts.ts" }
      - { type: "file", path: "packages/workflow/test/artifacts.test.ts" }
    modified:
      - { type: "file", path: "packages/tool-runtime/src/index.ts" }
      - { type: "file", path: "packages/workflow/src/index.ts" }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-bounded-artifact-store.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement bounded artifact store

## Summary

承認済み run-scoped artifact contract の実装と focused 検証を完了した。dispatch-specific override: implementation meta profile/delegation は SUBAGENT-STOP に従って省略、直接 TDD/検証/Git conventions を適用した。

## Context

issue #66 の slice は store 本体・reference・cap・cleanup・port contract のみ。承認済み spec/design/plan、local ADR と共有 D4/D5/D7 が実装境界を定める。

## Implementation

### Changes Made

IR を code 変更前に開いた。AC1-AC7 の contract test を先に追加し、port/store/barrel を実装した。port は tool-runtime、store は workflow、共有 D4/D5/D7 を維持。全 data/metadata cap は owning invocation 注入、source byte 計測は UTF-8/string と Uint8Array byteLength。

## Validation and Evidence

mise run setup 成功（pnpm install、apm install、全 package build）。spec/design と plan approval を coordinator から受領。local ADR は msg_a8d4489914f3 で作成/accepted を承認された。

- 最初の RED: test/artifacts.test.ts は 12 failures。未実装 API（RunArtifactStore/withArtifactStore/projectArtifactMetadata）による failure と確認。
- 実装後 GREEN: 12 contract tests pass。
- 境界追加 RED: BOM decode が先頭文字を落とす、callback が options.signal を変更すると abort が成功扱いになる 2 failures を再現。
- 修正: TextDecoder の ignoreBOM により BOM を保持、helper は開始時 signal を固定。
- 最終 focused workflow suite: 5 files / 89 tests pass（14 artifact tests）。
- focused tool-runtime suite: 3 files / 18 tests pass。
- pnpm lint: exit 0、既存 18 warnings。初回の新規 2 format errors は該当 5 files の biome check --write で修正。
- pnpm typecheck / pnpm build: exit 0。CLI test は CLI 無変更のため対象外。
- read-only review: Orca 子 Run run_fad8d26a0ba9 / task_a7d395a774bb / ctx_e997c1b50237、msg_325ba9430bf9。承認済み契約との照合で確定 P0/P1/P2 findings なし。reviewer は検証を独立再実行していない。settlement 後に release/ack 完了。
- 最終 scoped doc audit: 今回 chain の findings 0、共有 ADR に既存 reciprocal relation warnings 2（non-blocking）、scope blocking 0。global audit は findings 455 / blocking 41 で ownership 外の既存 findings を維持し、global 全通過は主張しない。
- impl audit: 75 files / findings []。git diff --check pass。
- index は doc-maintenance plan → apply で再生成。impl/ir は承認済み例外の方法 (1): new_impl_record.js の standalone main() を除いた repo 内一時 module から updateIndexForMarkdownDir を呼び、実行後に一時 module を削除した。

## Graph Execution Trace

run-until-yield、taskBudgetCount=1、maxHops=10。probe-to-briefing → briefing-to-design → design-to-planning → planning-to-task-graph → task-graph-to-implementation → implementation-to-followup-triage → followup-triage-terminal → exit-audit-to-complete。spec/design と plan の approval-required は coordinator ask で停止し承認後に再開。shared ADR lineage の focus-required は共有 ADR を references のみに戻し、承認済み local ADR を作成して解消した。commitGate baseline は bdacfc04cbfcf96f4ea070b3ff0524ab00546638、code 変更前の docs dirty を保持。implementation checkpoint の commit-required は Conventional Commit で解消する。followup-terminal は本 slice の追加作業なしを表し、既定の後続 engine/policy/inference scope は拡大しない。承認済み scoped exit-audit-pass を記録した。

## Related Experiments

既知の解を TDD で実装したため experiment log は不要。BOM/signal の RED/GREEN は上記 Validation and Evidence に記録した。

## Follow-ups

### Risks or Follow-ups

future engine seam InvocationToolRuntimeOptions.artifacts、RuntimePolicy artifact cap schema、#65-B inference と実 adapter 配線は後続 issue が所有。canonical check deferred to coordinator merge run。
