---
id: "IMPL-034zR3fwmlg2mtdTgJxi2v"
type: "impl"
status: "completed"
title: "Core resource manifest verification"
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
    - "docs/tasks/core-resource-manifest-verification.md"
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



# Core resource manifest verification

## Summary

core 正規化の focused GREEN と lint/typecheck/build、CLI flake の単独再実行結果を記録した。full canonical は coordinator の明示指示により deferred-to-coordinator。

## Context

spec/design approval msg_51dda51d3eb5、plan approval msg_4f465c6e0d66（7afaad2）。verification IR は task 開始時に作成した。full canonical 例外は msg_5463b4cce5a1、共有 ADR marker 限定変更は追加 user pre-authorization。

## Implementation

### Changes

spec / plan の verification amendment、終端 status、reciprocal verified-by、generated index、report を更新した。root の旧 untracked gate JSON 13件は resume 指示に従い削除した。固定型・paths.ts・policy・deps・runtime source は変更していない。

### Decisions During Implementation

plan 作成前の focus-required は resume-65a.md の承認済み例外で delegate 順を手動継続。plan approved 後は plan focus に router 復帰。script / relation は変更しない。
exit-audit の briefing gate は considered-options のみ fail。共有 ADR の marker 1/2/3 のみ "-" に変え、option text と marker 4 は維持した（6064c6c）。router が unordered list item のみ数えるため。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- focused RED: 未実装 resources/index.js、exit 1。
- focused GREEN: 77 passed / 2 skipped（79）、exit 0。POSIX case / backslash は host 条件 skip。
- core build: tsc -b、exit 0。lint / full typecheck は両 canonical attempts の前段で exit 0。lint の既存 12.3MiB bundle size warning 1件。
- doc IDs: 476 markdown files、exit 0。git diff --check: exit 0。
- first canonical: core 368 pass / 4 skip、tool-runtime 2 pass、workflow 2 pass、runtime-pi 342 pass。CLI 260 pass / 5 skip / 6 timeout、4 files failed、exit 1（333.75s）。
- worker cap retry: VITEST_MAX_WORKERS=1 でも CLI cross-agent 1 timeout。coordinator の指示により自分の retry Vitest tree のみ停止し、環境変数を finally で復元した。全 canonical pass は主張しない。bundle / APM freshness stage 未到達。
- CLI 単独再実行（pnpm --dir packages/cli exec vitest run <file> --maxWorkers 1）:
  - copilot-pre-read-hook.test.ts: 11 pass、58.03s、exit 0。
  - exec.test.ts: 12 pass、71.70s、exit 0。
  - cross-agent-routing-adapters.test.ts: 32 pass、217.89s、exit 0。
  - hook-contract.test.ts: 15 pass / 1 skip / 1 timeout（20.641s）、file exit 1。
  - 最後の残存 case を -t 'emits permissionDecision deny JSON' で単独実行: 1 pass / 16 skip、18.39s、exit 0。file 全体 pass とは表現しない。
- 最初の誤った @cheaper/cli filter は No projects matched で未実行だったため、検証証拠に含めていない。
- 初回 focused の symlink fixture timeout は case-only と file 再実行で pass。source の特例 / timeout 変更は加えていない。

## Graph Checkpoints

run-until-yield、taskBudgetCount=3、maxHops=12。probe focus-required → 承認済み手動 briefing/design → design-to-planning → planning-to-task-graph → build_task_graph（3 tasks / 2 edges / issues=[]）→ task-graph-to-active-implementation → implementation-retry（RED/GREEN/verification の各 task）。commit gates は 6c5f17e / 2447347 / f22319d で処理。marker 修正は事前承認された単発 repair で、新たな feature / policy の決定を加えない。
followup-triage-terminal → exit-audit-to-complete を確認した（blockers=[]、3 tasks done、dependency issues=[]）。named all audit は399 files / 460 findings / 48既存 blocking。自 chain は blocking=0、共有 ADR の既存 reciprocal warning 2件のみ。spec/design/plan/task/test-spec の対象 findings は0、impl-record audit は67 files / findings=[]。repository-wide doc audit の pass とは主張しない。
impl index は doc-maintenance が impl 型を扱わないため coordinator 承認の代替を使用。既存 new_impl_record.js の updateIndexForMarkdownDir を一時 module から実行し、canonical に再生成した。tracked script は不変、一時 module は削除、manual cell fallback は使用していない。

## Risks

full canonical は統合時に coordinator が逐次実行する。POSIX の2条件はこの Windows host では未実行。
今回の root probe: root=C:\\、candidate=C:\\resource-probe、isInsideWorkspaceRoot(...,{caseInsensitive:true})=false。paths.ts を変更せず別 issue に引き継ぐ。
within-workspace literal exact/subtree 内の symlink 到達先や produces ownership の到達集合保証は行わず、ownership も拡大しない。hard-link alias / 外部差替えも既存限界。

## Follow-ups

terminal classification: #65-B compiler inference / #61 lease は既存 scope exclusion。workspace-root boundary と共有 ADR owner への within-workspace literal subtree / produces exclusion 追記は coordinator owner の deferred concern（resume 指示済み）。worker が実装する未分類 follow-up はない。
