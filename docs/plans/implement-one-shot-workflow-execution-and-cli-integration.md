---
id: "PLAN-034AGr55LYOkphfEZr5Pr2"
type: "plan"
status: "draft"
title: "Implement one-shot workflow execution and CLI integration"
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
  implements:
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
    - "../designs/one-shot-workflow-execution-and-cli-composition.md"
    - "../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "../adr/keep-artifact-storage-invocation-local-and-bounded.md"
  defers: []
  deferred-by: []
---

# Implement one-shot workflow execution and CLI integration

## Goal

compiled DAG を injected executors で一度実行し、invocation の resource/cancellation/artifact lifetime を維持して、`cheaper workflow run <path>` から宣言 outputs と全 job の terminal metadata を一つの envelope で返す。

## Scope

engine → canonical runtime binding → CLI composition が同じ critical path と release intent を持つため、一つの plan とする。core/workflow/tool-runtime/runtime-pi/cli を変更するが、各 slice は focused tests と論理変更ごとの commit で検証する。engine 単体の完了を CLI 統合の完了と扱わない。

issue #62 Triage 2026-10-04 と #69 slice B、承認済み spec/ADR/design を実装する。daemon、polling、開始後 DAG mutation、optional jobs、暗黙の artifact 化、新 provider、実モデルの費用比較は対象外。

新 dependency は不要。package manifests/tsconfigs/root config/scripts/.github、二つの policy.yaml、overview/shared ADR、tracked APM bundle は変更しない。CLI args.ts/cli.ts/envelope.ts は design gate で承認済みの map 内だけを変更する。追加 hot file、依存変更、契約変更は coordinator へ戻す。

## Upstream Documents and Design Inputs

- [承認済み spec](../specs/one-shot-workflow-engine-and-run-cli.md): AC1-AC11 と public outputs/privacy/CLI 互換性。
- [accepted local ADR](../adr/compose-one-shot-workflows-with-injected-executors-and-invocation-owned-resource.md): ports、lifetime、policy defaults、shared D2 restrict-key の artifact 四キー extension。
- [承認済み design](../designs/one-shot-workflow-execution-and-cli-composition.md): API、状態遷移、full option set、hot-file map。2026-10-05 coordinator verdict は P0/P1 なし。
- [shared ADR](../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md): D1-D7 を拘束条件として引用し、references のみに保持する。

## File Map

パスは repo root 相対。new のみ新規作成。既存 source/test の責務は承認済み design に対応する。

| Slice | Source files | Responsibility |
| --- | --- | --- |
| S1 contracts | `packages/core/src/policy/workflow.ts` (new), `policy/schema.ts`, `policy/load.ts`, `policy/merge.ts`, `packages/core/src/index.ts` | WorkflowPolicy、default 補完、safe integer schema、project min restriction を一箇所に置く |
| S1 contracts | `packages/core/src/worker/write-scope.ts` (new), `packages/core/src/index.ts` | runtime-pi の literal extractWriteScope を互換移設して public export |
| S1 contracts | `packages/workflow/src/types.ts`, `compiler.ts` | CompiledWorkflow.validateJobOutput が既存 JobValidation.output で validate/snapshot |
| S2 engine | `packages/workflow/src/engine.ts` (new), `index.ts` | ports/result/job diagnostics、ready queue、capacity、collect-all、outer cancellation、public output projection |
| S3 artifacts | `packages/tool-runtime/src/artifact.ts`, `read.ts`, `catalog.ts`, `registry.ts`, `invocation.ts`, `index.ts` | ArtifactPut/Publisher 正本、三つの read mode、owner binding、borrowed artifact ports |
| S3 artifacts | `packages/workflow/src/artifacts.ts` | ArtifactPut 互換 export、store publisher cap getter。既存 identity/caps/cleanup を再利用 |
| S4 binding | `packages/tool-runtime/src/access.ts`, `invocation.ts` | prepareWorkerManifest による literal permission narrowing、caller-owned lease の dispose 後 retention |
| S4 binding | `packages/runtime-pi/src/run.ts`, `tools/registry.ts`, `index.ts` | core helper と narrowed manifest、borrowed execution、createPiToolHandlers export、per-Worker budget/journal |
| S5 CLI | `packages/cli/src/workflow-context.ts`, `workflow-run.ts` (new) | interface/run の metadata context 共用、lazy runtime/store/model、engine executors/ledger の composition |
| S5 CLI hot files | `packages/cli/src/args.ts`, `cli.ts`, `envelope.ts` | workflow-run args、main dispatch、signal/model seam 注入、workflow:true envelope variant |
| S5 CLI | `packages/cli/src/text-format.ts`, `usage-log.ts` | failure も public projection を表示、buildUsageRecord の workflow guard |
| S6 records | own spec/design/plan/test-spec/tasks/IR と各 generated README index | 実装/承認/verification evidence、own chain + shared ADR scoped exit-audit |

| Slice | Test files | Responsibility |
| --- | --- | --- |
| S1 | `packages/core/test/policy/workflow.test.ts` (new), `schema.test.ts`, `load.test.ts`, `merge.test.ts`, `restrict.test.ts`; `packages/core/test/write-scope.test.ts` (new) | defaults/min layering/timeout narrowing、literal helper の互換性 |
| S1 | `packages/workflow/test/compiler.test.ts` | job output と public output schema の正本を compiler に保持 |
| S2 | `packages/workflow/test/engine.test.ts` (new), `no-pi-deps.test.ts` | mixed ports、bounded/dependencies/fan-in/conflicts/failure/cancel/late settle/privacy、source import guard |
| S3 | `packages/tool-runtime/test/artifact-read.test.ts` (new), `runtime.test.ts`; `packages/workflow/test/artifacts.test.ts` | read union/caps/foreign refs/deny/cleanup、既存 workspace read の互換性 |
| S4 | `packages/tool-runtime/test/invocation.test.ts`, `resources.test.ts`; `packages/runtime-pi/test/resource-leases.test.ts`, `run.test.ts`, `tools/read.test.ts`, `tools/registry.test.ts` | child permission、dispose/lease retention、Worker timeout isolation、single-worker/Pi schema |
| S5 | `packages/cli/test/workflow-run.test.ts` (new), `workflow-interface.test.ts`, `args.test.ts`, `input.test.ts`, `envelope.test.ts`, `text-format.test.ts`, `usage-log.test.ts`, `main.test.ts` | parsed.tokens/input、tool-only/lazy model、stdout/error/text/usage、既存 run/interface |
| S5 | `packages/cli/test/e2e/workflow-run.test.ts` (new), existing `e2e/helpers.ts` を使用 | built CLI bin/cheaper.js を spawn し一つの JSON と exit/privacy を検証。bundle 再生成不要 |

test fixtures は上記 test 内の deferred promises、temporary workspace/config、既存 workflow fixtures と fake streams を再利用する。helpers.ts の変更は予定しない。readStrictness・Windows canonical paths・deny/symlink の既存 test 条件を緩めない。

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| G0 approval/test-spec/tasks/IR | plan approval | S1 |
| S1 policy/helper/compiler contracts | G0 | S2, S3, S4 |
| S2 injected engine | S1 | S5 |
| S3 canonical artifact read/publisher | S1 | S4, S5 |
| S4 Worker binding/lease ownership | S1, S3 | S5 |
| S5 workflow CLI/envelope/compatibility | S2, S3, S4 | S6 |
| S6 verification/review/exit-audit | S5 | completion |

critical path は G0→S1→S3→S4→S5→S6。S2 は S3 と依存上は独立だが、common brief の nested worker 禁止に従い S1→S2→S3→S4→S5 の順で inline 実行する。engine ports の unit evidence は S4/S5 の physical lifetime/composition evidence を代行しない。

## Implementation Sequence and Task Breakdown

plan approval 後に次の六つの behavior 群の test-spec を作り、plan の verified-by へリンクする: contracts、engine scheduling/cancellation、artifact read、Worker lease/permission、CLI projection/compatibility、final verification。続いて S1-S6 に対応する task docs を作り、各 task の dependency/verified-by を固定する。approval 前には test-spec/tasks/IR/code を作らない。

G0 は承認 evidence を記録する、behavior test-spec を作る、dependency を持つ tasks を作る、graph returned audits を実行する、in-progress IR を開く、の個別 action とする。以下の RED は import/type failure だけで終わらせず、port/API が存在した後に違反している振る舞いを確認する。各 slice の R/G evidence を IR に残す。

### S1: policy/helper/compiler の正本

- [ ] S1.1 workflow defaults/min layering、writeScope literal helper、validateJobOutput の契約 test を書く。
- [ ] S1.2 V1 を実行して未導入の契約の失敗を確認する。
- [ ] S1.3 approved defaults/schema/min keys と helper 移設、既存 schema を呼ぶ compiler seam を実装する。
- [ ] S1.4 V1 を再実行して GREEN を確認する。
- [ ] S1.5 IR に focused evidence を記録する。
- [ ] S1.6 論理変更を commit する（policy と helper/compiler は独立 review 可能なら分割）。

defaults は maxJobs=1、artifactBytes=8388608、totalBytes=33554432、previewBytes=2048、readBytes=65536。maxJobs/read は正、他 caps は非負の safe integer。trusted builtin/user merge 後・project restriction 前に default 補完し、project は五キーを min restriction。user4/project8→4、user4/project2→2、未指定/project16→1。--timeout は既存 restrictPolicy の narrowing を使い上限超過を拒否する。

### S2: injected one-shot engine

- [ ] S2.1 deferred Worker/tool/admission と fake clock を用いた engine contract test を書く。
- [ ] S2.2 V2 を実行して scheduling/collect-all/cancel 契約の失敗を確認する。
- [ ] S2.3 approved ports と pending/admitting/ready/running/terminal 状態を実装する。
- [ ] S2.4 V2 を再実行して GREEN を確認する。
- [ ] S2.5 IR に focused evidence を記録する。
- [ ] S2.6 engine 論理変更を commit する。

compiled edges だけを使い、全 predecessor 成功後に input materialize。admission/ready は slot を使わず maxJobs は executor 呼出し〜settle/detach。dependency failure は affected descendants のみ skip、他 branch は継続。job output は compiler seam で validate、最後の public outputs は名前ごとに validate/materialize して独立成功を残す。ID/output 名は code-unit 昇順。private payload と details/cause は公開しない。diagnostic message は2048 UTF-8 bytes 以内。

preabort は admission/executor 0。signal/timeout は first cause と ports.abort 一回、取得済み未実行 lease と abort 後 grant は release。running は5000msの共通 grace を待ち、logical cancellation 後の late result が outcome を変えない。開始済み parent の release は actual executor finally に委ねる。全 continuations を観測し unhandled rejection と無期限 Promise.all を避ける。

DEPENDENCY_FAILED は新しい job-only diagnostic string。ERROR_TABLE/core ErrorCode は変更せず aggregate は TOOL_EXECUTION_FAILURE。overall priority は TIMEOUT/ABORTED→job aggregate→output-only INVALID_OUTPUT。job/overall execution は owner/invocation snapshot であり Worker snapshot の合算は禁止。

### S3: canonical artifact read/publisher

- [ ] S3.1 publication/consumer/workspace read と store 互換性の contract test を書く。
- [ ] S3.2 V3 を実行して未配線の modes と cap 拒否の失敗を確認する。
- [ ] S3.3 ArtifactPublisher と exclusive read union を canonical runtime/store に配線する。
- [ ] S3.4 V3 を再実行して GREEN を確認する。
- [ ] S3.5 IR に focused evidence を記録する。
- [ ] S3.6 artifact 論理変更を commit する。

ArtifactReader は既存、ArtifactPut を tool-runtime 正本へ移し workflow 互換 export。publisher producer.jobId は scope owner が固定。workspace publication は regular-file/deny/image/NUL/abort を維持して cap+1 の chunk read、overflow は put 前に拒否。artifact mode は reader の bounded byte slices と continuation を使い workspace resolver は呼ばず resources は explicit 空。mixed mode、ports 不在、foreign/tampered refs、zero caps、duplicate/cap mutation 前拒否を検証する。

### S4: Worker 子 permission と lease ownership

- [ ] S4.1 literal writeScope narrowing、borrowed scope、child token 有/無の dispose/late-settle regression を書く。
- [ ] S4.2 V4 を実行して既存 blanket release と permission mismatch を確認する。
- [ ] S4.3 ResourceAccess.prepareWorkerManifest、WorkerExecutionBinding と caller-owned lease cleanup を配線する。
- [ ] S4.4 V4 を再実行して GREEN を確認する。
- [ ] S4.5 IR に focused evidence を記録する。
- [ ] S4.6 Worker binding/lease 論理変更を commit する。

parent は compiled safety、child は canonical literal writeClaims を安全 manifest の各 writes/produces bucket に contained な exact claims へ narrow。mkdir ancestor を child permission として残さず、readStrictness/parent containment を既存正本に委ねる。helper operations observeAccess を同じ scope に結線。writeScope 無し/explicit empty を保守的正本どおり扱う。

InvocationToolRuntime.dispose の resources.releaseJobs() 一括呼出しを除去し、caller の releaseJobLease を dispose 後も可能にする。ResourceManager 自体は変更しない。scope.ready 自己取得 lease は既存 scope.dispose で release。single-worker cleanup、borrowed child timeout/budget が他 Worker/invocation に及ばないこと、operation tokens と externally owned parent の actual settle までの保持を検証する。

### S5: CLI と public envelope

- [ ] S5.1 args/input、hooks composition、envelope/text/usage、built CLI E2E test を書く。
- [ ] S5.2 V5a を実行して workflow-run の未導入動作を確認する。
- [ ] S5.3 workflow context/run と approved hot-file dispatch/variant を実装する。
- [ ] S5.4 V5a を再実行して GREEN を確認する。
- [ ] S5.5 pnpm build 後に V5b を実行する。
- [ ] S5.6 IR に focused/unit/E2E evidence を記録する。
- [ ] S5.7 CLI 論理変更を commit する。

full option set は --input、--cwd、--timeout、--format json|text、--quiet、--help、--version。token allowlist でその他を拒否。parsed.tokens で input omission={} と明示 '-'=stdin を区別する。file input は既存 JSON acquisition、definition は explicit path loader/deny 境界を使う。interface は共用 metadata helper に抽出して非実行のまま維持する。

runtime/store は初回 admission で一組だけ lazy 作成し preabort は constructor 0。modelConfig/createModelRuntime は最初の Worker start の shared promise、tool-only は初期化0。CliHooks.runWorker wrapper が RunWorkerDeps.streamFn を注入し、CliHooks 自体に streamFn field を作らない。cli.ts の既存 test-only CHEAPER_TEST_FAKE_STREAM loader を callback として composition へ渡す。new workflow-run module に private loader を複製しない。

direct tool と Worker は same invocation、jobId owner、parent lease と narrowed child manifest、per-Worker budget/journal を使う。tool executor は canonical execute、Pi helper handler は既存 public export を注入。runtime/store は finally dispose、bytes 無し reference ledger から availability:unavailable/includePreview:false を投影する。

workflow:true/worker:null variant は failure でも result/jobs/meta を保持。formatText は workflow failure を含む variant を generic errorText より先に dispatch。buildUsageRecord の最初に workflow guard、cli main の engine 前 error も workflow-run を usage writer へ渡さない。公開 output 名 tokens でも count record を作らない。main は stdout 一回、diagnostics は stderr、nonzero は既存 error exitCode。

### S6: verification、implementation review、exit-audit

- [ ] S6.1 V6 を実行して affected package focused suites と lint/typecheck/build を確認する。
- [ ] S6.2 関連 CLI test files を一つずつ実行して個別 evidence を記録する。
- [ ] S6.3 concrete commit/file/test evidence を coordinator の新 implementation review ask へ渡す。
- [ ] S6.4 review で必要な修正を行い、影響する focused check を再実行する。
- [ ] S6.5 scoped own chain + shared ADR exit-audit を実行する。
- [ ] S6.6 task/plan/IR と report に actual outcomes/deferred canonical check を記録する。
- [ ] S6.7 最終論理変更を commit する。

global 既存48 blocking findings は ownership 外で修正しない。indexes は doc-maintenance plan→apply、impl/ir unsupported 時は common brief の承認済み index helper 例外を記録して使う。graph signals は実 evidence 後だけに付与する。full mise run check は coordinator merge run に defer する。

## Verification Matrix

全 command は worktree root から `mise exec --` 経由。新規 test は当該 slice で作成した後に実行する。RED は意図する invariant の失敗を記録、GREEN は exit 0/該当 cases pass。実 provider/network は必要ない。

| ID / Slice | Command | GREEN pass condition |
| --- | --- | --- |
| V1 / S1 | `mise exec -- pnpm --filter @cheaper/core exec vitest run test/policy/workflow.test.ts test/policy/schema.test.ts test/policy/load.test.ts test/policy/merge.test.ts test/policy/restrict.test.ts test/write-scope.test.ts` | defaults/min/safe integer/timeout/helper regressions pass |
| V1 / S1 | `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/compiler.test.ts` | job output snapshot/rejection と既存 compile contracts pass |
| V2 / S2 | `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/engine.test.ts test/no-pi-deps.test.ts test/resource-inference.test.ts` | bounded scheduling/edges/failure/cancel/privacy と import guard pass |
| V3 / S3 | `mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/artifact-read.test.ts test/runtime.test.ts test/no-pi-deps.test.ts` | read modes/security/resource declarations と既存 runtime pass |
| V3 / S3 | `mise exec -- pnpm --filter @cheaper/workflow exec vitest run test/artifacts.test.ts` | store compatibility/identity/caps/cleanup pass |
| V4 / S4 | `mise exec -- pnpm --filter @cheaper/tool-runtime exec vitest run test/invocation.test.ts test/resources.test.ts` | caller-owned lease が dispose 後/child token 有無で保持、self-owned scope cleanup pass |
| V4 / S4 | `mise exec -- pnpm --filter @cheaper/runtime-pi exec vitest run test/resource-leases.test.ts test/run.test.ts test/tools/read.test.ts test/tools/registry.test.ts` | narrowed write permission、borrowed isolation、single-worker/Pi regressions pass |
| V5a / S5 | `mise exec -- pnpm --filter @cheaper/cli exec vitest run test/workflow-run.test.ts test/workflow-interface.test.ts test/args.test.ts test/input.test.ts test/envelope.test.ts test/text-format.test.ts test/usage-log.test.ts test/main.test.ts` | omission/stdin/path/timeout/lazy model、partial false/text/privacy/usage、既存 dispatch pass |
| V5b / S5 | `mise exec -- pnpm build` then `mise exec -- pnpm --filter @cheaper/cli exec vitest run test/e2e/workflow-run.test.ts test/e2e/workflow-interface.test.ts` | built Node CLI 一つの stdout JSON、exit/privacy と interface 非実行 pass |
| V6 / S6 | `mise exec -- pnpm lint`; `mise exec -- pnpm typecheck`; `mise exec -- pnpm build` | 各 exit 0。既存障害なら原因/範囲を report し coordinator 判断を受ける |
| V6 / S6 | V1-V5 の focused commands、S5 の CLI files を一 file ずつ `mise exec -- pnpm --filter @cheaper/cli exec vitest run <file>` | 対象 evidence GREEN、単独実行でも同じ契約 pass |
| V6 / S6 | `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` と own IR audit、graph returned scoped exit-audit | own chain findings 0、shared ADR blocking 0、completed records と relation/index が整合。shared ADR の既存 nonblocking reciprocal warnings 2件は変更せず report |

S*.1 は named contract tests を source/design と照合、S*.2/S*.4 は対応 V の RED/GREEN log、S*.3 は実装後の V と source import/ownership inspection、S*.5 は IR の command/result completeness、S*.6/S5.7/S6.7 は staged diff check と commit contents で確認する。G0 は approved plan/test-spec/task/IR の canonical relations と returned audits、S6.3 は coordinator verdict、S6.4 は再実行 evidence、S6.6 は actual results と status consistency が pass criterion。verification 無しの action を done にしない。

## Acceptance Coverage

| Spec AC | Slices / verification | Observable guarantee |
| --- | --- | --- |
| AC1 mixed/privacy | S2,S4,S5 / V2,V4,V5 | same runtime ports/owners、declared outputs のみ、private sentinel は JSON/text に出ない |
| AC2 concurrency | S1,S2,S5 / V1,V2,V5 | maxJobs1/2 の active count、admission は slot 非消費、maxOperations は別 accounting |
| AC3 dependencies/input | S1,S2,S5 / V1,V2,V5 | needs/$from/inferred edge と fan-in の全 producer 待ち、deferred schema validate |
| AC4 resource conflict | S2,S4,S5 / V2,V4,V5 | real ResourceManager conflict wait/bypass、compiled edges は不変 |
| AC5 collect-all | S2,S5 / V2,V5 | affected skip/independent continued、false envelope に成功 output を保持 |
| AC6 cancel | S2,S4,S5 / V2,V4,V5 | preabort、outer timeout/signal、grant race、grace/late settle、dispose retention、child isolation |
| AC7 output | S1,S2,S5 / V1,V2,V5 | schema/input/output source は compiler、独立 valid output を保持、INVALID_OUTPUT |
| AC8 artifacts | S3,S4,S5 / V3,V4,V5 | reference を Worker へ渡す明示 bounded read、foreign/overflow 拒否、cleanup unavailable、private preview 無し |
| AC9 CLI | S5 / V5 | input/defaults/stdin/file、definition deny、tool-only init0、one stdout、exit/timeout restriction |
| AC10 regressions | S1,S3,S4,S5 / V1,V3,V4,V5 | interface 非実行、run/alias、args/envelope/text、workflow usage 除外 |
| AC11 verification | S6 / V6 | focused + lint/typecheck/build + CLI 単独 + scoped audit、canonical merge defer |

## Dependencies and Risk Register

#57/#58/#59/#61/#63/#64/#65-B/#66/#69-A/#70 は dispatch baseline に merge 済みで再実装しない。fresh setup は完了済み。spec/ADR/design は承認済み、plan approval と implementation review は未承認。

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| data loss: blanket dispose が未 settle executor の parent を解放し競合 write を許可 | medium | high | S4 で caller ownership を正本化、child token 有/無・dispose 前後の retention regression |
| data/privacy: publication が cap 越え payload/preview/private diagnostics を保持・公開 | medium | high | bounded read before put、owner固定、metadata ledgerのみ、JSON/text sentinel/caps tests |
| breaking change: read union/CLI envelope narrowing が既存 Worker/interface/usage を変える | medium | high | exclusive schema modes、workflow discriminator先頭、related single-file regressions |
| policy: project default 未補完が maxJobs/caps を拡大 | medium | high | trusted merge 後 default、project min keys、load fixtures |
| deadlock: admission waiter が executor slot を占有 | medium | high | S2 slot 分離と real ResourceManager bypass composition test |
| external dependency availability: provider/catalog/network が未準備 | low | medium | fake RunWorkerDeps.streamFn/injected executors/offline isolated config、tool-only init0、実モデル成功を主張しない |
| late error: detached continuation/unhandled rejection/二重 cleanup | medium | high | first-cause、一回 abort、continuation 観測、actual finally release と immutable terminal tests |
| documentation: existing global findings と settled Dispatch が gate を隠す | medium | medium | own scope audit、新しい active Dispatch/ask、既存 findings は report のみ |

plan 作成時の audit baseline は own chain findings 0、global blocking 48、shared ADR の既存 nonblocking reciprocal warnings 2件（pi-execution-engine と workspace-filesystem-boundary-cheaper-sandboxed-tool への related）である。shared ADR は hot file のため変更しない。最終 audit では新規 drift が無いことを区別して報告する。

## Rollback Strategy

データ migration/永続 workflow store/credential 書込/新外部 service は導入しない。public API は additive（read の既存 mode、ArtifactPut export、single-worker binding 省略を維持）。既に許可された tool write の作業結果は Git revert では戻らないため、検証は disposable temporary workspace のみで行う。

trigger は既存 run/interface/read/usage の回帰、permission widening、lease 早期 release、bounded storage の違反。未公開の該当論理 commit を coordinator と確認した上で `mise exec -- git revert <recorded-commit-sha>` で戻し、S5 は S4/S3/S2 に依存するため dependent commits を新しい順に戻す。破壊的 reset/amend/history rewrite は使わない。rollback verification は V1-V5 の存在する既存 tests と pnpm typecheck/build、git status/diff による intended source removal を確認する。dependency/hot-file の追加が必要になった場合は実装を止め gate へ戻す。

## Open Questions / Gaps

実装契約の未決事項は無い。plan/test-spec/tasks の exact artifact IDs は承認後に creation scripts が発行するため今は slice key で依存を示す。plan gate には新しい active Dispatch が必要であり、旧閉鎖 ask IDs と lifecycle IDs を再使用しない。実装中に source seam が設計を満たせないことが判明した場合は原因と concrete revision を coordinator へ渡す。

## Self-Review

- [x] AC1-AC11 が各 slice/verification に対応する。
- [x] source/test paths、defaults、streamFn 所属、buildUsageRecord 名称、job-only diagnostics を current source/design に照合した。
- [x] generated template placeholders を除去し、shared ADR は references に保持した。
- [x] test 作成、RED 実行、実装、GREEN、record、commit を個別 action にした。
- [x] data-loss/breaking/external availability と rollback を記録した。
- [ ] coordinator が plan を承認する。
- [ ] approval 後に behavior test-spec と dependent task docs を作り verified-by/implemented-by を付ける。

## Implementation Handoff

inline task execution、各 slice の contract verification、独立 reviewer の checkpoint を必要能力とする。S2 と S3 は依存上分離できるが common dispatch brief が nested runs/sub-workers を禁止しているため追加 delegation は行わない。review は coordinator の新 ask へ送る。plan approval→test-spec/tasks→IR→implementation の順を保ち、run-until-yield graph の返す audits/delegate と commit gate を満たす。

## Review Handoff

reviewer は upstream spec/local ADR/design と shared D1-D7 に対して、S1 policy restriction、S2 admission/slot と cancellation、S4 caller-owned lease lifetime/child permission、S5 CLI privacy/usage/model lazy seam を確認する。design verdict の P3 四点は本 plan と design に反映済み。test-spec を approval 前に作らないため verified-by は現在空であり、approval 後に意味を持つ link を追加する。

本 plan は draft。plan approval が得られるまで実装しない。canonical check deferred to coordinator merge run。
