---
id: "SPEC-034MEaZYrURZ1MPlywunIi"
type: "spec"
status: "approved"
title: "Execution History for Issue 192"
created: "2026-10-26"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/192"
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
  related:
    - "usage-analysis-scope-issue-184.md"
    - "../adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/execution-history-issue-192.md"
  derived-by:
    - "../adr/record-per-execution-history-with-bounded-best-effort-storage.md"
  verifies: []
  verified-by: []
  references:
    - "run-error-record-observability.md"
  defers: []
  deferred-by: []
---

# Execution History for Issue 192

## What Should Be Built

個別実行とcheaperが下した意味のある処理判断を、ユーザーの `<userConfigDir>` 内の構造化 Execution History と `cheaper history` CLI で調査できるようにする。History は実行ID・親子/trace相関・分類済み状態/理由・開始/終了・既存ログ参照を記録し、usage集計、診断tracing、Routeログ、checkpointの責務はそれぞれの既存契約に残す。

## Why Now

`usage.jsonl` はusageを持つRunやRouteの集計が目的であり、pre-usage失敗やskip、委譲助言、未実行Jobを網羅しない。Route logs/checkpointsは大容量出力と限定再開用で、History queryに適さない。利用者が、どの処理が実際に開始され、何が未実行で、何の観測済み理由で判断されたかを後から区別するために必要である。

## Users and Value

cheaperを親Agentから使う開発者・運用者が、実行元のCLI出力やdebugログを推測でつなげずに、特定run/trace/route/jobと処理判断の事実を確認できる。利用者はHookの助言をWorker実起動と誤認せず、失敗Workflowの実行位置やRoute logへ辿れる。

## Scope

### In Scope

- Worker standalone Run、Workflow executionと各Job、Route evaluation/runおよびRouteに含まれる実処理の状態と親子関係。
- Hook read/search/shell等で実際に評価した意味ある結果と、Route viability/no-match/rewrite/fallback等の結果。全hook受信の監査は行わない。
- Startedからcompleted/failed/cancelled/timeout等の終端、または終端欠落をincomplete/unknownとして表示する契約。
- 各executionに独自`executionId`、実際の包含関係にだけ使う`parentExecutionId`、相関可能な系列を示す`traceId`を持たせる。外部別プロセスの無伝搬runは推測で結合しない。
- 適用可能な新規usage `run`/`route`/`route-refetch`記録へのexecution IDs追加とHistoryから対応Usageを辿る導線。IDの無い過去レコードも既存どおり読む。
- Cross-process trace/parent context carrierを用意する。`executionId`は常に現在のcheaper processが発行する。in-process lifecycleが確認した包含関係だけを`parentExecutionId`として記録する。外部から受け取るtrace/parent値は別の`traceId`/`parentExecutionHintId`相関hintとして記録し、必ず`inherited-unverified` provenanceで表示する。carrierが無い/不正なら独立rootとし、Hook native stdout/verdictへIDを追加しない。
- list/show/traceのtext/JSON出力、filter・件数上限/ページング、保持/削除、保存場所・機密最小化・運用方法の文書。
- User-visible reason codeと必要な限定スナップショット（例: `measuredTokens: null`, `fileBytes`, `threshold`）を区別し、未測定値を捏造しない。
- Record schemaは有限の`kind/status/event/reasonCode/verdict/viability/errorCode`語彙と、長さ・文字種が制限された識別子/数値を使う。provider/modelはraw inputやendpointでなくresolved catalog identityからIDのみ投影し、公式ID文字列は損なわない。診断に必要なworker/provider/model IDと実行metricのみ保存し、command textとfile pathは既定で保存しない。

### Out of Scope

- Hook event受信回数の完全監査、上流agentや別プロセスの未観測行動の推定、全Tool Call transcriptの保存。
- `usage.jsonl` のcost/token/savings/error集計定義変更、Usage単位とHistory eventの数を混同した二重集計、強制backfill。
- Policy/grant/approval/verdict/native stdout/exit contractの変更、外部サービスへのログ送信、新しいAPI呼び出し。
- `CHEAPER_LOG`診断tracing、Route logs、budget checkpointの置換。
- Issue #162 の対象（明示的に範囲外）。#185/#189 は別PRの後続作業として扱う。

## Requirements

- Historyはschema version、必須`recordedAt`、execution/trace/optional parent IDs、kind、event/status、分類済みreason codeを機械可読に保存する。execution summaryの`recordedAt`は開始時刻、decision-only recordでは判断時刻とし、list/trace orderingの共通時刻にする。
- `executionId`は実行単位ごとに一意。`traceId`はrootで生成され、cheaper管理下のnested boundaryでは明示的に伝搬する。IDsは入力文字列由来でなく、grant/policyへ影響しない。
- Workerを表すrecordには、許可されたworker/model/provider ID、開始/終了時刻とduration、分類済みerror code、turn/toolCall数、対応usage record referenceを、該当値が観測可能な場合に保存する。pre-usage failureではworker/model/provider等の未解決値を推測しない。
- 次のexecution cardinalityとusage ownershipを保つ。

  | Observed unit | History identity / parent | `usage.jsonl` owner |
  | --- | --- | --- |
  | Standalone Worker invocation | 1 Worker execution ID; optional propagated parent | Applicable `kind:"run"` uses the same ID |
  | Workflow invocation | 1 Workflow execution ID; parent only when nested under a Route/Workflow | No independent `kind:"run"` row for the workflow envelope |
  | Workflow Job | 1 Job execution ID, parent is Workflow; Worker fields belong on this Job record | No standalone Worker `kind:"run"` row for the same job work; workflow metrics remain outside usage aggregation |
  | Route run | 1 Route execution ID; routed Workflow/Jobs may have child IDs; preserve Route `runId`/`logRef` separately | Applicable `kind:"route"` uses the Route execution ID; route-owned exec/postprocess usage remains owned by the Route row |
  | Route log refetch | 1 independent refetch execution ID; reference source Route by `logRef` | Applicable `kind:"route-refetch"` uses the refetch ID; #184 joins only by its existing in-window route+`logRef` rule |
  | Hook/Route decision | 1 decision execution ID, with propagated trace/parent only when explicitly supplied | No usage row |
  | `count-tokens` utility | Not an Issue #192 History execution | Existing `kind:"count"` record stays unchanged |

  Same `traceId` never deduplicates usage rows. History absence never creates or backfills a usage row. If a `logRef` has no matching History/Route record, show the unresolved ref rather than infer an owner.
- Workflow Job内のWorker workはJob identityで表現し、独立Worker History/usage Runを二重作成しない。別に起動された`cheaper run`には新しいIDを発行し、明示伝搬された場合だけparent hintを付ける。
- Routeの既存`runId`/`logRef`は保持し、Route executionとの対応をHistoryで示す。`route logs` refetchは独立execution IDを持ち元Routeを参照する。
- Hook `Deny`はdelegation-advised decisionで記録し、Worker started/skipped/not-startedと断定しない。Rewrite発行とroute runの開始/終了は別に記録する。
- statusが終端しない時はイベント欠落を許容し、CLIはincomplete/unknownと示す。存在しない成功を補完しない。
- History保存はbest-effort。Historyの不在・破損・部分書込・append失敗は元のverdict、command exit/stdout、Run結果、policyを変えない。
- History query/list/show/traceはHistory storeを作成せず、Usage記録・History書込・worker起動を行わない。
- 保存項目はallowlistとし、prompt、stdin/input JSON、tool引数・出力、raw argv/env、API key、生のエラー本文、credentialを保存しない。パス等は最小化または明示ポリシーに従う。
- 複数processからのappend、crash/partial tail、長期・大量履歴、retention、Windows/POSIX/custom config dirを扱う。Hook hot pathで全件fsync/LLM/高価なlock待ちをしない。
- History eventごとにusageの二重計上を防ぎ、Route内部workerと独立Worker `run` の集計境界を維持する。

## Acceptance Criteria

- [ ] standalone Workerのusage付き成功/失敗とpre-usage失敗を別execution ID/状態/reason/timeでlist/showできる（History started writeが成功した場合）。pre-usage failureにはusage rowを捏造せず、存在する時だけHistoryから同じexecution IDのusage recordを辿れる。History write failureはbest-effortとして欠落を許容し、command結果は維持する。
- [ ] Worker showでworker/model/provider ID、開始/終了/duration、分類済みerror code、turn/toolCall数、Usage referenceを表示し、pre-usage failureでは未確定値を省略する。
- [ ] Workflow詳細は親executionと各Jobの依存/状態/skip・blocked理由/失敗位置を示し、並列Jobを相関IDから追える。Job内Workerのcostが独立runとして二重加算されない。
- [ ] Route evaluation/match/viability/rewrite/fallbackと実route run、exec/postprocess workerの実行有無・理由・exit/errorを区別し、route logRefへ辿れる。refetchは元Route参照を保つ独立execution IDとなる。
- [ ] Read allow/threshold skipとlarge-read delegation adviceで判定reason、計測方法/thresholdを示す。未計測token数はnullで、byte等から推算しない。
- [ ] Delegation adviceとWorker起動を別イベントとして表示し、別CLI processに明示相関情報がない場合はtraceを推定結合しない。Hook recordは evaluator outcome と native verdict を区別し、unsupported Rewrite→fallback Denyも表す。外部hostが後でRewriteを実行したかは推測しない。実Route runは独立executionで、明示伝搬時だけ相関し、それ以外は未確認/不明と表示する。
- [ ] 対象の新規usage `run`/`route`/`route-refetch`レコードとHistory execution IDが一致し、過去IDなしレコードと#184 aggregation/refetch semanticsは維持される。同一trace内の別runは両方集計し、refetchは独立recordとして`logRef` owner-mapのみでjoinする（execution/parent/trace joinやwindow外backfillなし）。
- [ ] Workflow/Job/Route/Worker event ownership tableとUsage mapping tableのfixtureで、同一処理のdouble usage rowがなく、usage append失敗時もHistory linkage欠落を推測で補わない。
- [ ] Store不在/破損/途中書込/強制終了/大量record/並行processに対し読み込みが堅牢で、有効な先行recordを保ち、欠損terminalはincomplete/unknownとして表示する。保存失敗は元の結果を変えない。
- [ ] Readersはvalid records before malformed/truncated entryを保ち、破損entryのみを除外し、scan budget到達を`truncated/incomplete`と明示する。terminal status conflictはfirst-writer-winsまたはconflictとして扱い、successを合成しない。
- [ ] History queryはstore不在時にファイルを作らず、History/Usageを書かず、実行/判断を再実行しない。Permission/disk-full/write-before-start failureではHistory欠落を許容し、実在するrecordのみ表示する。
- [ ] List/show/traceはstable keyset ordering、explicit default/max page size、scan count/time budgetを持つ。全recordに`recordedAt`を必須とし、decision-onlyを含め`recordedAt`降順、executionId降順でpagingする。scanが完全な場合のみcursorを返す。budget到達時は`scanComplete:false`としcursorを返さず、不完全結果から全件取得できるように見せない。retention対象内でbudget内に完全走査できたdatasetについてpaginationを保証する。Concurrent-write時cursor semanticsを文書化し、query memoryをpage size plus bounded path metadataへ制限する。
- [ ] Hookのstdout/stderr/exit codeとAllow/Deny/Rewrite、grant/policy判定、Worker/Workflow/Routeの既存結果に差がないことを契約テストで固定する。
- [ ] 保存データfixture/auditでraw prompt, input, tool payload, argv/env, credential, error bodyが含まれない。`reason/status/errorCode/verdict/viability`はfinite enum、identifier文字列はfieldごとの文字種/最大長、metricsは非負/上限付き、provider/modelはresolved catalog IDのみ、Route `logRef`は既存strict format、full path/basename/arbitrary labelsは既定で保存しない。負値・小数・非finite・上限超過metricは省略し、signed `exitCode`は別の有限整数型で保持する。retention/保存先/CLI usageを文書化する。
- [ ] Concurrent process writesはrecord単位で競合せず、retention/queryもboundedで、Hook hot-path overheadを計測する。configured storage budgetの上限保証とbest-effort overflow境界を区別し、`mise run check`を含む正準gateがpassする。

## Deferred Design Concerns

Issue #192 の acceptance に含まれるため、本specでは追加機能のdeferを作らない。#185 の副作用なしdiagnostic utilitiesおよび#189は別PRに残す。

- 最終的なstorage schema、file/segment organization、retention limits、およびCLI exact flagsはproposed ADR と design/plan で決める。要求から外れて削らない。

## Sources and Relations

- Primary requirement source: [Issue #192](https://github.com/scarecrow173/cheaper/issues/192).
- Upstream exploration: `../discovery/execution-history-issue-192.md`.
- Usage compatibility: `usage-analysis-scope-issue-184.md` and `../plans/implement-usage-route-error-analysis-issue-184.md`.
- Existing worker error projection: `run-error-record-observability.md`.
