---
id: "IMPL-034ALJN6e40bNeMK86YJ6Y"
type: "impl"
status: "completed"
title: "Implement injected workflow engine"
created: "2026-10-06"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - { type: file, path: packages/workflow/src/engine.ts }
      - { type: file, path: packages/workflow/test/engine.test.ts }
    modified:
      - { type: file, path: packages/workflow/src/index.ts }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../../tasks/implement-injected-workflow-engine.md"
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

# Implement injected workflow engine

## Summary

S1 APPROVED の直接 verdict を受け、S2 injected engine を実装した。code 変更前に本 IR を開いた。taskBudgetCount=6/maxHops=15 と commitGate を維持する。focused と package-wide workflow tests は GREEN、独立 S2 review は未承認のため IR/task は in-progress とする。

## Context

承認済み plan/design/test-spec の ports、compiled-only edges、admission と executor capacity の分離、collect-all、bounded outer cancellation、public projection を実装する。nested workers は使わず inline TDD/incremental/commit/coordinator review に従う。

## Implementation

### Changes

engine.ts に設計済み ports/context/result と runWorkflow を導入した。ID 昇順の event-driven pump は compiled.edges の predecessor counter と成功 result snapshots を使い、admission/ready を executor slot から分離する。input/admission/executor/output failure は affected descendants のみ skip、全 terminal 後に public outputs を名前ごとに投影し、独立成功を保持する。private input/result と error details/cause は jobs に載せない。

outer signal/timeout は first reason と一回の ports.abort を持ち、新規起動を止める。取得済み未実行 lease と late admission grant は release、開始済み lease は actual executor finally だけで release。共通 grace 後の論理 terminal と late result/meta を分離した。job/overall metrics は owner/invocation snapshot の copy であり合算しない。

WorkflowExecutorError は、adapter が選択した failure metadata を generic JSON と CheaperError channel で運ぶ additive subtype。runtime-pi の WorkerRunError/runMeta は import しない。runtime/store/CLI の生成と wiring は S3-S5 所有であり S2 では扱わない。

### Decisions During Implementation

main be660a0 への rebase は行わない。merge.ts の #60 restrict-key union は coordinator の merge 所有。settled Dispatch の lifecycle IDs は使わず、今回の直接指示を user-owned continuation として扱う。

timeout/grace は設計の有限値 contract を維持するため Node の2147483647ms timer上限を越えた場合のみ再armする。scheduling の polling は導入しない。now は duration 用、deadline は monotonic performance.now を使う。

## Related Experiments

### Adopted

既存の承認済み設計を実装するため exp は不要。

### Rejected

なし。

## Validation

fresh task-graph-to-implementation route: issues []、S2/S3 runnable。直接指示に従い S2 のみを開始した。IR IMPL-034ALJN6e40bNeMK86YJ6Y を code 前に作成済み。

- 初回 RED: engine module 未導入で engine.test suite が fail、既存 no-pi/resource-inference の30 testsはpass。
- 初回 GREEN: scheduling/projection/options の14 tests pass。
- cancellation RED: preabort/timeout/grace の3 tests fail、16 pass。GREEN:19 pass。
- failure metadata/long timer RED:2 fail、19 pass。GREEN:21 pass。
- inferred-edge fixture は manifests 不足で compiler が正しく INVALID_REQUEST を返した。各jobに explicit empty manifest を供給し fixture を修正した。compiler/source は変更していない。
- 最終 focused: engine/no-pi/resource-inference 3 files /62 tests pass（engine32、既存30）。
- producer snapshot mutation pin 後の workflow package全体:9 files /168 tests pass。
- workflow build、root pnpm lint（既存36 warnings）、root pnpm build:exit0。初回 build の TS2412 は optional lease への undefined assignment が exactOptionalPropertyTypes に反したため、状態除去を delete に直した。
- focused biome check/write:2 files、formattingのみ。source import guard は runtime-pi/Pi SDK/CLI dependencies を拒否する既存 test を維持。
- root `mise exec -- pnpm typecheck`:exit0。
- 最終 source review で、timeout/grace の起点が arm 時点にずれる問題を発見。engine開始／first abortのdeadlineを固定する2 testsを追加して2 fail/32 passのREDを確認し、monotonic起点を先に捕捉して修正した。修正後 workflow package全体9 files /170 tests pass（engine34）。root lint/buildを再実行してexit0。
- 上記source修正後の `mise exec -- pnpm typecheck`:exit0。task指定focused commandも再実行し3 files /64 tests pass。
- impl audit:96 files/findings[]。doc-status all:550 files、own findings0、共有ADRの既存nonblocking warnings2、global blocking48は変更していない。task indexはdoc-maintenance plan→apply、IR indexはcreation scriptのhelperで追加。

## Risks

source commit `a2d1861` に対する coordinator の独立 review（review-protocol）は APPROVED、P0/P1 なし。status message `msg_7f94504f94f3` は admission 非slot、maxJobs bound、fan-in、failure/abort/grace/late release、timeout、port purity、宣言 outputs の投影を確認した。fresh Dispatch で承認を受け task/IR を完了とした。

cancel の logical terminal と executor actual settle の境界を tests で分ける。S2 focused success を CLI/Pi/artifact integration や実provider成功とは呼ばない。

## Follow-ups

S3 は別 IR に記録し、S4 Worker binding/lease ownership と S5 CLI integration を継続する。canonical check deferred to coordinator merge run。
