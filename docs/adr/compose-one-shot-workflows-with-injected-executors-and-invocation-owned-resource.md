---
id: "ADR-034AtVcQU6kREPV6w4aHs4"
type: "adr"
status: "accepted"
title: "Compose one-shot workflows with injected executors and invocation-owned resources"
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
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: ["../discovery/one-shot-workflow-execution-boundaries.md"]
  derived-by: ["../designs/one-shot-workflow-execution-and-cli-composition.md"]
  verifies: []
  verified-by: []
  references: ["../specs/one-shot-workflow-engine-and-run-cli.md", "./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md", "./keep-artifact-storage-invocation-local-and-bounded.md"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: ["coordinator"]
    informed: []
---

# Compose one-shot workflows with injected executors and invocation-owned resources

## Context and Problem Statement

compiler・resource inference・InvocationToolRuntime・RunArtifactStore は存在するが、compiled DAG の一度の実行と CLI composition は未実装である。親 agent が複数 invocation と中間 payload の往復を管理する必要がある。shared ADR D1-D7 を admission、lifetime、artifact consumer の実装契約へ具体化する。

2026-10-05 coordinator が spec + 本 ADR intent に続いて、本 ADR 本文（ADR-034AtVcQU6kREPV6w4aHs4）を直接 verdict で承認した。review-protocol は P0/P1 なし。終了済み Dispatch の ask msg_2fa9655ca645 は再開せず、この verdict を acceptance evidence とする。設計・plan・実装 review は別ゲートとする。

## Decision Drivers

- D1: workflow は core/tool-runtime のみに依存し、Pi は injected executor の背後に置く。
- D2/D4: semantic DAG と resource wait を分離し、admission 待ちは workflow slot を消費しない。
- D3/D7: capacity/resources/cancellation/artifacts は invocation 単一、Worker budget/journal は子の所有。
- D5/D6: binding/schema の正本は compiler。公開 data は declared outputs のみ。
- #66: reference の provenance と上限を検証し、workspace path と artifact namespace を分離する。

## Considered Options

- engine が Pi/session/runtime を生成する。
- injected executors に対して engine が DAG を管理し、CLI が invocation context を構成する。
- resource waiter が maxJobs slot を保持する queue。
- resource admission と executor capacity を分離する queue。
- 大値をサイズで自動 reference 置換する。
- canonical read の明示 publication と bounded artifact consumer を提供する。

## Decision Outcome

採用するのは injected executors、admission/capacity 分離、明示 artifact read/publication の組合せである。

### Engine and admission

engine は compiled workflow、WorkerExecutor/tool executor、job admission port、owner metrics port を受け取る。Pi/session/provider を import しない。全 producer 成功後に compiler.materializeJobInput で input を確定して admission へ進む。resource wait は executor slot を占めない。admission 済み job を stable ID 順 ready queue から maxJobs 内で start する。terminal/lease grant のイベントだけで queue を進め、polling と Worker wake は行わない。

parent lease は compiled safety manifest を使う。Worker child permission manifest は authoritative writeScope へ narrowed した宣言を使い、mkdir 安全性用の祖先/root-exclusive claim を write permission と混同しない。writeScope 抽出は core へ移して single-worker と workflow で共用する。子の読み取り・明示 access 制限を保持し、ResourceManager が parent containment を最終検証する。process resource/pool 宣言も canonical runtime を通す。

全 job は required。producer failure は affected downstream を dependency_failed skip とし独立 branch は継続する。tool/Worker output schema 不一致は当該 job の INVALID_OUTPUT。全 job terminal 後に compiler.materializeOutput を output 名ごとに呼び、独立 valid outputs を overall failure でも保持する。

### Timeout and physical settlement

outer timeout は既存 policy.execution.timeoutMs。workflow run --timeout は既存 restrict 関数で上限を短縮する。新 timeout policy field は追加しない。計測区間は engine start から finalization（definition/input load と compile は前段）。engine API は timeoutMs を明示受け取り、outer signal と first-cause abort を合成する。

outer abort/timeout は invocation と active/queued job を cancel。preabort は executor を呼ばない。Worker 固有 timeout/budget は子だけを停止する。abort 後は default 5000ms grace まで cooperative executor settle を待つ。超過 job は cancelled terminal として envelope を確定するが、実 promise settle 前に parent lease を release しない。late settlement は結果を更新しない。queued/admitted 未実行 lease は即時解放、開始済み lease は実 settle と child runtime drain を経て解放する。

### Policy, ownership, metrics and usage

workflow.maxJobs を policy schema/normalization に明示追加する。正の safe integer、default 1、trusted user override、project min restriction。operation maxOperations とは独立。

workflow.artifacts は optional bounded cap 設定。defaults は maxArtifactBytes=8388608、maxTotalBytes=33554432、maxPreviewBytes=2048、maxReadBytes=65536。store と同じ validation（safe integer、前三者は非負、read は正）、trusted user override、project は各 cap を min restriction。policy.yaml 二つは編集せず normalization で default を補完する。

workflow.artifacts.* の min restriction keys は shared ADR D2 の閉じた restrict-key 集合（execution.concurrency.maxOperations / workflow.maxJobs / process.maxConcurrent）を拡張する。shared ADR の Consequences に従う互換的な tightening として、既存キーの意味を変えず本 ADR で追加を記録する。

CLI が runtime と store を一つずつ所有し全 executor が借りる。finally で runtime dispose/drain と store dispose を行う。外側 abort 時の store clear は operation の物理 settle と独立。per-job meta.execution は runtime.executionMetrics(jobId) の owner snapshot、overall は runtime.executionMetrics() の invocation snapshot とし Worker snapshots を合算しない。Worker usage/model/changes は job meta に保持する。workflow は既存 single-model usage.jsonl writer の対象に含めない。

### Explicit artifact consumer

canonical read の既存 path/offset/limit/byteInLine は維持する。明示 publication は {path, artifact: <name>}、line slice option と併用不可。sandbox/deny・画像/NUL の既存 read 制約内で source bytes を cap+1 まで bounded に読み、超過は保存前に拒否する。結果は content: [] と details.artifact: ArtifactReference。producer は owning job ID と指定 name であり input から job ID を上書きできない。

consumer は {source: {kind: "artifact", reference}, offset: <byte>, maxBytes: <positive>}。canonical read は reader port のみに dispatch し workspace resolver を呼ばない。返却 text と byte continuation（offset/nextOffset/eof）は bounded。catalog input/output schema と inference が union を認識し artifact read は filesystem access を持たない。publisher port のみ tool-runtime に追加し、reader は既存 ArtifactReader（artifact.ts）を再利用する。workflow store が両 port を実装する。

任意 output 全体を自動 reference 置換しない。`$from` は reference JSON を運び、Worker prompt は reference/preview のみを持つ。final envelope の artifacts は unavailable metadata として projection し cleanup 後の再取得可能性を宣言しない。

### CLI and public envelope

workflow run <path> の全 option は --input <file>|-、--cwd、--timeout、--format json|text、--quiet、--help、--version。それ以外（model/provider/worker/worker-file/tier/auth/usage option）は拒否する。definition は explicit file のみ。parse token の input option 存在を見て、省略は {}、明示 - のみ stdin を消費する。

workflow interface <path> は現行 option/非実行契約を維持。stdout は envelope 一つ、text も同じ public projection を使う。result は declared outputs のみ、jobs は ID 昇順 status/meta/最大2048 UTF-8 bytes の error message を持つ。error details/cause に private input/result をコピーしない。required failed/skip/cancelled または output validation failure は ok:false/非0 exit。outer first-cause TIMEOUT/ABORTED と job の元 code を保つ。

### Consequences

- 良い点: fake executor で DAG/failure/abort を検証でき、tool-only invocation は model runtime を初期化しない。
- 良い点: resource/store の所有権が明確になり、子 timeout で他 branch を停止しない。
- 負担: admission 待ち、論理 terminal、物理 settle を区別する必要がある。
- 非目標: daemon、persistent status、cross-invocation artifact API、optional jobs、開始後 DAG refinement、新 provider、費用の empirical study。

### Confirmation

design に API/file map/state transitions を固定し、plan acceptance matrix と coordinator implementation review で検証する。shared ADR または single-worker の互換を保てない場合は再検討する。

## Implementation Plan

- Affected paths: 以下の engine/core/runtime/CLI と対応 tests に限定する。
- Patterns to follow: compiler materialization、canonical ResourceManager/OperationManager、borrowed scope、既存 loader を再利用する。
- Patterns avoid: Pi の workflow import、budget/journal 複製、implicit payload 置換、semantic edge の追加、private result の公開。
- engine: packages/workflow/src/engine.ts、types.ts、index.ts と engine/dependency guard tests。
- policy/writeScope: packages/core/src/policy/schema.ts、merge.ts、worker/ 共用 helper/public export と policy/helper tests。manifest/policy.yaml は編集しない。
- artifacts/runtime: packages/tool-runtime/src/artifact.ts、read.ts、catalog.ts、registry.ts、invocation.ts、access.ts、index.ts と packages/workflow/src/artifacts.ts、対応 tests。
- Worker binding: packages/runtime-pi/src/run.ts、tools/registry.ts、index.ts と borrowing/writeScope regression tests。budget/journal を再実装しない。
- CLI composition: packages/cli/src/workflow-run.ts、args.ts、cli.ts、envelope.ts と関連 CLI unit/e2e tests。hot file 具体 map は design gate で承認を取る。
- dependencies: 既存 workspace packages のみ。追加しない。input/path/compiler は既存 loader/policy/root を使う。
- docs: own design/plan/test-spec/tasks/ir と generated indexes のみ。overview/共有 ADR は変更しない。

## Verification

- [ ] engine focused tests で mixed/fan-out/fan-in/maxJobs/inferred edge/resource wait/collect-all/output validation を検証する。
- [ ] signal/preabort/timeout/grace/late settlement tests で terminal outcome と physical lease retention を検証する。
- [ ] artifact publication/read/foreign ref/cap overflow/cleanup と Worker borrowing/writeScope を検証する。
- [ ] 関連 CLI test file を単独実行し input omission/file/stdin、deny、tool-only no-model、single stdout、exit code、interface/single-worker regression を検証する。
- [ ] pnpm lint、pnpm typecheck、pnpm build、runtime-pi import ban guard を実行する。
- [ ] own chain + shared ADR scoped exit-audit を通し canonical check は coordinator merge run へ defer と記録する。

## Pros and Cons of the Options

### Engine-owned Pi versus injected executors

前者は composition が短いが D1/D7 違反。後者は ports が必要だが Worker runtime 借用と lazy model 初期化ができるため採用。

### Slot-held wait versus separate admission

前者は簡単だが conflicting waiter が unrelated ready job を妨げる。後者は granted lease cleanup が必要だが resource wait を semantic DAG と分離できるため採用。

### Implicit replacement versus explicit artifacts

前者は便利だが schema と `$from` の値を変える。後者は explicit option が必要だが型の意味と private result を保持できるため採用。

## More Information

正本は issue #62 Triage 2026-10-04、spec、shared ADR D1-D7。admission/lifetime は ResourceManager/OperationManager、store 内部と filesystem safety は既存実装を再利用する。
