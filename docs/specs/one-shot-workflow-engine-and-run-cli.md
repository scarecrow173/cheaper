---
id: "SPEC-034AqzLDP7xm2lCmDoyKzg"
type: "spec"
status: "approved"
title: "One-shot workflow engine and run CLI"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/62", "https://github.com/scarecrow173/cheaper/issues/69"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: ["../plans/implement-one-shot-workflow-execution-and-cli-integration.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../discovery/one-shot-workflow-execution-boundaries.md"]
  derived-by: ["../designs/one-shot-workflow-execution-and-cli-composition.md", "../plans/implement-one-shot-workflow-execution-and-cli-integration.md"]
  verifies: []
  verified-by: ["../test-specs/workflow-execution-contract-boundaries.md", "../test-specs/workflow-one-shot-scheduling-and-cancellation.md", "../test-specs/workflow-artifact-publication-and-consumption.md", "../test-specs/workflow-worker-permissions-and-lease-lifetimes.md", "../test-specs/workflow-run-cli-public-projection.md", "../test-specs/workflow-execution-verification-boundaries.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "../adr/keep-artifact-storage-invocation-local-and-bounded.md"]
  defers: []
  deferred-by: []
---

# One-shot workflow engine and run CLI

## What Should Be Built

compiled workflow を injected tool/Worker executor に対して一度実行し、公開 outputs と全 job の terminal status/meta を一つの combined envelope にまとめる。CLI は cheaper workflow run <path> --input <file>|- を提供する。

## Why Now

compiler・resource inference・runtime・artifact store の導入が揃い、親 agent が fan-out/fan-in を複数 invocation と polling で組む必要を無くせる。

## Users and Value

親 Coding Agent が依存制御と中間 payload の往復を委譲し、deterministic tool と Worker の混在処理を一度で受け取る。

## Scope

### In Scope

- one-shot DAG ready queue、workflow concurrency、resource admission、collect-all、abort/timeout。
- compiler の deferred input/output schema validation、artifact reference の保持と明示 bounded consumer。
- invocation 単位の共有 runtime/store、per-Worker budget/journal、combined CLI envelope。
- stored definition loader と既存 compact interface・single-worker CLI の互換性。

### Out of Scope

- daemon/server、persistent status API、parent polling、Worker wake loop、開始後 DAG mutation、分散 scheduler。
- bare workflow/stdin definition、--values、optional jobs、implicit payload 型変換、新 provider、token savings の実測研究。

## Requirements

- shared ADR D1-D7 と issue Triage を維持する。engine は runtime-pi を import せず、CLI が injected executor と共有実行 context を構成する。
- 全 producer 成功後に downstream input を materialize/validate してから起動する。explicit needs、$from、inferred producer/read edge は compiler の compiled edge を使う。
- ready job は bounded maxJobs（default 1、明示 policy で増加）内で実行する。operation maxOperations は独立。resource conflict は wait であり semantic edge や job failure に変えない。
- failed job の affected downstream は dependency_failed skip、無関係 branch は継続する。全 job 必須なので failed/skip は overall ok:false と非0 exit。
- stdout は JSON envelope 一つ。result は宣言 outputs のみで materialize 可能な独立成功 output を保持する。jobs は job-id 昇順の status/meta/bounded diagnostics、private result 全文を含めない。
- overall execution は invocation の実測 snapshot。per-Worker usage/model は job meta に保持し、複数 model の usage を単一 model と偽装しない。
- outer signal/timeout は active と queued job を cancel。operation/job lease は実 settle と bounded detach を区別し、他 Worker の child timeout/budget で invocation を終了しない。
- workflow input 省略は {}、--input - は invocation 値だけを stdin から読む。definition path は canonical cwd の workspace/deny 境界を適用する。
- 大値は明示 artifact reference と preview を運べる。consumer は明示 bounded read で必要部分を読む。workspace resolver と artifact namespace を混在させず、終了後 metadata は unavailable とする。
- error code は D6 の INVALID_INPUT/INVALID_REQUEST、TIMEOUT/ABORTED、TOOL_EXECUTION_FAILURE、INVALID_OUTPUT を使い、job の元 code を保持する。

## Acceptance Criteria

- [ ] AC1: tool と Worker が同じ workflow/runtime で混在し、private result 本文は stdout に現れない。
- [ ] AC2: 独立 ready job は maxJobs 内で並列、maxJobs=1 は逐次、operation limit は別に保持する。
- [ ] AC3: $from/needs/inferred edge と fan-in が全 producer を待ち、job input を実行直前に schema validate する。
- [ ] AC4: conflicting resource は待機し、非 conflict job は実行でき、compiled edge は増えない。
- [ ] AC5: failed job の downstream だけ skip、無関係成功と公開 output は overall failure でも保持する。
- [ ] AC6: signal/preabort/timeout は active と queued job を cancel し、late executor settle が terminal outcome を書き換えない。
- [ ] AC7: 個別 output の materialization/schema failure は INVALID_OUTPUT、独立 valid output は保持する。
- [ ] AC8: 大きな cross-job value を artifact reference で Worker へ渡し、明示 bounded read、foreign reference/overflow 拒否、cleanup 後 unavailable を検証する。
- [ ] AC9: CLI --input 省略/defaults/file/stdin、definition path deny、tool-only model 未初期化、single JSON stdout と exit code を検証する。
- [ ] AC10: workflow interface、single-worker run/alias、CLI args/envelope/text の関連 regression を維持する。
- [ ] AC11: focused suites、pnpm lint/typecheck/build、関連 CLI test 単独、own chain + shared ADR scoped exit-audit を実行する。canonical check は coordinator merge run に defer する。

## Deferred Design Concerns

今回の非目標は issue #62 と shared ADR に明示された後続事項であり、新しい未承認 future artifact は作らない。

## Sources and Relations

canonical issue #62 Triage 2026-10-04 と dispatch brief が正本。shared ADR は references のみとし、他 chain と overview は変更しない。

## Approval Evidence

2026-10-05 coordinator の直接 verdict により spec + ADR intent は APPROVED。review-protocol は P0/P1 なし、issue AC 全件と D1 import 境界を確認済み。閉じた ask msg_3d72c1cb069d は再開せず、ADR 本文・design・plan・implementation はそれぞれ新しい承認ゲートを通す。
