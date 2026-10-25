---
id: "DISC-034M0QFbqunN8MzxeSmmDu"
type: "discovery"
status: "resolved"
title: "Usage report analysis audit — route/error breakdown and --since scope (issue #184)"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/args.rs"
    - "crates/cheaper-cli/src/text_format.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/route/logs.rs"
    - "crates/cheaper-cli/tests/integration/usage_report.rs"
    - "crates/cheaper-cli/tests/integration/usage_log.rs"
    - "docs/guides/usage.md"
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
  derives-from: []
  derived-by:
    - "../specs/usage-analysis-scope-issue-184.md"
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
  verifies: []
  verified-by: []
  references:
    - "../discovery/usage-and-savings-telemetry-for-delegation-reduction.md"
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../specs/delegation-usage-measurement-log.md"
    - "../specs/usage-log-recording-scope-docs.md"
    - "../adr/usage-jsonl-contract-canonical.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../discovery/worker-run-failure-rate-31pct-investigation.md"
    - "../specs/run-error-record-observability.md"
    - "../adr/usage-error-record-field-contract.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../specs/route-grant-set-projection.md"
    - "../impl/ir/add-route-and-route-refetch-usage-records.md"
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
---

# Usage report analysis audit — route/error breakdown and --since scope (issue #184)

## Exploration Goal

issue #184 は `cheaper usage` の集計拡張を要求する:

1. `--since` を `route` / `route-refetch` bucket にも適用する(現行は
   run record のみに効く — **既知 bug**)。
2. Route 別分析(実行数・成功/失敗・original/returned tokens・
   reductionRatio・postprocess failures・refetch 回数/bytes・
   exec/analyze/total durations・計測不能数)を route 名・期間単位で
   報告する(「期間単位」は `--since` による window 絞り込みを
   意味し、日次/週次等の期間 bucket 行出力を要求しない — issue
   文面の曖昧さに対する本 chain の解釈として固定)。
3. Worker 失敗分析 — run error の `error.code`(TIMEOUT・
   MAX_TURNS_EXCEEDED・INVALID_OUTPUT・PROVIDER_FAILURE 等)別件数/
   比率と、worker・provider/model・期間別の内訳。
4. `--group-by route|error` 等の新 flag と `--format text` / JSON
   envelope の parity。
5. run/route token の二重計上防止、`executed.argv` 等の secret 含有
   可能 field を group key・出力へ漏出させないこと、malformed/legacy
   record 耐性。

本 doc は現行実装の監査結果(record schema inventory・bug 位置・
filter surface)と採択空間を固定する。要件化は
`docs/specs/usage-analysis-scope-issue-184.md`、方式採択は
`docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md` が
担う。

## Key Issues and Assumptions

- **`route_bucket(&all)` は filter 前に全 record へ掛かる** —
  `build_usage_report`(usage_report.rs:551-553)が
  `let routes = route_bucket(&all);`(:552)を
  `filter_run_records(all, filters, now)`(:553)より先に評価する。
  `--since`/`--worker`/`--provider`/`--model` は route bucket に
  一切効かない。route record は `ts` field を持つ
  (usage_log.rs:380)ため `--since` 適用自体は可能。
- **filter の意味論は run record 専用に設計されている** —
  `filter_run_records`(:246-287)は `kind:"run"` gate の後に
  `worker` は文字列 field 比較、`provider`/`model` は
  `model.{provider,id}` 参照。route record の `worker` は
  **object**(`{model, usage}` — analyze/condense worker meta、
  route/run.rs:602-612)であり、run record の `worker` 文字列
  (worker 名)とは別概念。`--worker bulk-reader` を route record へ
  素直に適用すると意味をなさない。
- **`executed.argv` は平文の引数値** — route record の
  `executed.{grant, argv}`(#166 で additive 追加、
  usage_log.rs:388-398)は argv を平文で保持し、secret を含み得る
  (docs/guides/usage.md:302-310 に caveat 明記済み)。group key・
  出力には `route` 名と `commandHash`(DefaultHasher hex、
  usage_log.rs:384-387)のみ使える。
- **`parse_duration` は `ms|s|m|h` のみ受理** —
  cheaper-core/src/duration.rs:40(SUFFIXES)・:7
  (`DURATION_PATTERN` は policy JSON schema —
  policy/command.rs:62-92 等 — と共有)。issue 例示の
  `cheaper usage --since 7d` の `d` suffix は **現行では
  `invalid --since duration: 7d` で reject される**。なお先行 spec
  `docs/specs/usage-and-savings-telemetry-aggregation.md`(:148)は
  `--since 7d` を受理するかのように記述するが、`d` suffix は
  実装されたことがない既知の不正確な記述であり、issue 例示は
  `168h` で充足する。`DURATION_PATTERN` を拡張すると policy
  schema の受理面にも波及するため、本 chain では `d`/`w` を追加
  しない(当該 doc は他 chain の approved doc で改訂せず、分岐は
  本 chain 側に記録する — spec の Out of Scope 参照)。
- **`--format text` に route section が存在しない** —
  `format_usage_report`(text_format.rs:341-416)は
  records/usage/cost/turns/measured/estimated のみ描画し、JSON の
  `result.route` bucket を一切出さない。text/JSON parity は
  現時点で既に欠けている。
- **error.code の集計経路は未実装** — #141 で error record の
  `error` object(`code` + allowlist `reason`/`message`/
  `schemaErrors`/`errorMessage`/`checkpoint`、usage_log.rs:242-327)
  が記録されるようになったが、`aggregate`/`route_bucket` のどこにも
  code 別集計はない。`code` は欠損・明示 `null`(:246-250)・非
  文字列を取り得、`error` object 自体を欠く run error record も
  あり得る — これらを畳む `"unknown"` bucket 規約を spec で固定
  する。
- **refetch の真値は 2 面構成** — route record の `refetches`
  field は write 時点の snapshot(通常 0、usage_log.rs:411)で、
  `route logs <id>` の **read 成功ごと**の `route-refetch`
  record(別行、logRef join 用 — 読み取り失敗は `?` 早期 return
  で record されない、route/logs.rs:59)が後続観測。現行 bucket は
  refetch record のみ数え snapshot field は読まない(:522-525) —
  per-route 化では `logRef` → route record への join が必要で、
  双方向の加入で二重計上しない規則を定義する。なお route record
  の `logRef` は `Option` で、absent 時は `""` が書かれる
  (route/run.rs:630 `unwrap_or_default`)— 空・欠損の `logRef` は
  join 不能であり、join 規則への明記が要る。
- **route record の `worker` は `{}` が正常形** — `worker` field
  は analyze job meta の `{model,usage}` で、analyze 未実行時は
  空 object になる(route/run.rs:602-612 `unwrap_or`)。
  `worker:{}` を malformed 判定や計測不能計数に含めてはならない。
  「計測不能」の定義(どの field 欠損がどの計数へ入るか)は spec
  で metric family 別に固定する。
- **明示 `null` の usage/savings は report 全体を INTERNAL_ERROR
  にする** — `aggregate`(:364-397)は JS `Cannot read properties
  of null` parity として CheaperError を返す。test
  `explicit_null_usage_fails_like_js_type_error` 等で固定済みの
  既存契約 — 新 section が同じ fail-hard に従うか、malformed entry を
  skip+計数するかは spec の判断点。
- **usage.jsonl は全量メモリ展開** — `read_usage_records`(:232-242)
  が `fs::read` → `from_utf8_lossy` → per-line parse で
  `Vec<Value>` を返す。append-only・1 行/record の telemetry file で
  あり、streaming 化の要否は実測次第(issue scope 5 も「測定し、
  必要なら段階的に」と条件付き)。

## 現行実装の監査

### usage.jsonl record inventory(writer: usage_log.rs)

| kind | writer | 固定 field | optional/条件付き field |
| --- | --- | --- | --- |
| `run`(成功) | `build_usage_record` :138-203 — 呼出 cli.rs:477 | `schemaVersion:1`・`ts`(ISO8601 `Z` 付き)・`root`・`durationMs`(meta 由来・null 許容)・`kind`・`worker`(worker 名文字列)・`ok:true`・`version` | `model`{provider,id,tier,selection,tierSource,…}・`usage`{input,output,cacheRead,cacheWrite,totalTokens,costUsd}・`turns`・`toolCalls`・`savings`{sourceTokens,returnedTokens,avoidedContextTokens,directlyComparable}・`repairs`(non-null のみ)・`execution`{timeoutMs,maxTurns(null 可),maxToolCalls}(echo 供給時のみ) |
| `run`(usage-bearing failure) | 同上 :206-233 — 呼出 cli.rs:266(汎用 catch) | 上記共通 + `ok:false` + `error` object | `error`{`code`(SCREAMING_SNAKE ErrorCode、errors.rs:54-80)、`reason`(abort 語彙 timeout/signal/maxTurns/maxToolCalls のみ)、`message`≤300字、`schemaErrors`≤5件{instancePath≤200,message≤120}、`errorMessage`≤300字、`checkpoint`{path,observationsCount}} — allowlist 外(rawOutput/exitCode/retryable/cause 等)は記録されない |
| `count` | 同上 :161-183 — 呼出 cli.rs:379 | 共通 + `kind:"count"`・`worker:null`・`ok:true` | `result`{path?,tokens,bytes?} |
| `route` | `build_route_record` :369-422 — 呼出 route/run.rs:620 | 共通 + `kind:"route"`・`route`(宣言名)・`commandHash`・`executed`{grant,argv(**平文**)}・`originalTokens`・`returnedTokens`・`reductionRatio`・`worker`{model?,usage?}(analyze job meta)・`durationMs`{exec,analyze,total}(**object**)・`logRef`・`refetches`(write 時 snapshot)・`postprocessFailures`・`exitCode`・`ok` | `suppressed` |
| `route-refetch` | `build_route_refetch_record` :428-442 — 呼出 route/logs.rs:60-68(**read 成功時のみ**:59 の `?` で失敗時は早期 return) | 共通 + `kind:"route-refetch"`・`logRef`・`bytesRead` | — |

注意点: run record の `durationMs` は **数値**、route record の
`durationMs` は **object** `{exec,analyze,total}` — 同名で型が違う。
`route` record の `reductionRatio` は record 単位の比率で、集計時は
`1 - Σreturned/Σoriginal` の weighted 形が正しい(単純平均は歪む)。

### 集計経路(usage_report.rs)

| 関数 | 位置 | 振る舞い |
| --- | --- | --- |
| `read_usage_records` | :232-242 | `<userConfigDir>/usage.jsonl` を `fs::read` → `from_utf8_lossy` → 行毎 `serde_json` parse → `is_record` gate。破損行・未知 kind・file 不在を skip/0 件化。**全量 Vec<Value>** |
| `is_record` | :219-225 | kind ∈ {run,count,route,route-refetch} の object のみ通す |
| `filter_run_records` | :246-287 | `kind:"run"` のみ + `--since`(`ts` を `date_parse`、ISO subset)・`--worker`(文字列一致)・`--provider`(`model.provider`)・`--model`(`provider`+`id`) |
| `aggregate` | :356-401 | run 集計 — ok/errors・usage.*・turns/toolCalls/durationMs・measured(savings 持ち)・comparable(`directlyComparable===true`)。明示 null の usage/savings で InternalError |
| `route_bucket` | :503-538 | route/route-refetch の **全期間 global 集計** — executions/failures/refetches/refetchBytes/originalTokens/returnedTokens/postprocessFailures。route 名別・期間別・logRef join・duration 集計 **なし** |
| `build_usage_report` | :544-597 | `route_bucket(&all)`(:552)→ `filter_run_records`(:553)→ `aggregate` → `report_map` + `report["route"]`(:556)→ `--compare-model` 時のみ estimated。CLI 入口 |
| `execute_usage_inner` | :673-741 | `--since` を `parse_duration`(ms\|s\|m\|h のみ)→ invalid は INVALID_REQUEST。`--model`/`--compare-model` を `<provider>/<id>` 検証。utility envelope で `result` に report を埋める |

### CLI surface(args.rs / cli.rs / text_format.rs)

- `UsageCliArgs`(args.rs:241-252): `since`・`worker`・`provider`・
  `model`・`compare_model`・`format`・`quiet`。**`--group-by`・
  `--route`・`--kind` は存在しない**(`--group-by` は repo 内で
  未使用、`--route` は `route` command の positional 名と衝突しない
  — option は `--` 接頭で別 namespace)。
- usage command arm: args.rs:894-941。`usage_only_option_used`
  ガード(:759, :943-948 — error message :945 が対象 flag 名を
  列挙する形式 `"--since/--compare-model are only valid for
  \`usage\`"`)が非 usage command での `--since`/`--compare-model`
  を reject — 新 flag を追加する場合は同ガードへ編入し、message
  の列挙にも追加する必要がある(他 command での silent drop 防止
  規約)。
- option 登録は hand-rolled: `STRING_OPTIONS`(:343-378)+
  `Values` field + `set_string` arm(:456-488)+ `UsageCliArgs` +
  usage arm の写像。
- dispatch: cli.rs:390-396 → `execute_usage` → `write_output` →
  `--format text` は `format_text` → `utility_text` が report 形状を
  検出して `format_usage_report` へ(text_format.rs:195-205)。
  **route bucket は text 出力に現れない**(:341-416)。
- help: cli.rs:595-603 に `--since`/`--worker`/`--compare-model` 等の
  usage 行があり、新 flag はここにも追加が要る。

### 既存 test 面

- `tests/integration/usage_report.rs`(834 行): filter・aggregate・
  pricing/estimated・envelope・route bucket(:788-834)を固定。
  `now` は注入された固定時計、fixture JSONL は `write_log` helper。
  route bucket の test は `--since` 非適用の現行挙動を固定している。
- `tests/integration/usage_log.rs`(719 行): record builder の
  shape・allowlist・route/route-refetch record(:296-360 付近)。
- `docs/guides/usage.md` の `usage` 節(:360-429)は `route` bucket
  自体を記述していない — 「集計対象は `kind:"run"` record のみ」
  (:375)とあるが `result.route` は実際に出力される。新設計と
  一緒に document drift を直す対象。

## Scope 項目の事実分解

| issue scope | 検証済み事実 | gap |
| --- | --- | --- |
| 1. 集計スキーマと filter 意味論 | `--since` が route に効かない bug は usage_report.rs:552 に確定。run/route/refetch は kind で厳密分離済み(is_record:219、filter_run_records:255、route_bucket:512)。run 集計へ route token が混ざる経路は存在しない(route の `worker.usage` は nested object で `r.usage` 未該当) | `--since` の route 適用。`--route`/`--kind`/`--group-by` flag が無い。`--worker`/`--provider`/`--model` の route 適用は意味論が違い未設計 |
| 2. Route 別分析 | record には route・reductionRatio・durationMs{exec,analyze,total}・logRef・postprocessFailures・exitCode・ok・suppressed が既にある | 集計は global bucket のみ。per-route 名・期間・logRef join(refetch 帰属)・duration 集計・計測不能数の報告が無い。`executed.argv` は secret 含有可で出力非対象 |
| 3. Worker 失敗分析 | run error record の `error.code` は ErrorCode 全型を取り得る(#141 導入済み)。`reason`(abort 語彙)・`execution`{timeoutMs,maxTurns,maxToolCalls}・`durationMs` も record 済み | code 別件数/比率・worker/provider/model/期間内訳の集計・出力が一切無い |
| 4. CLI / UI | parser は table 駆動で flag 追加は机械的手続き。envelope は additive field を許す(Value 構築順維持) | `--group-by`/`--route` flag・route/error section の text parity・guide が無い |
| 5. 規模・欠損耐性 | 破損行 skip・file 不在 0 件・legacy field 欠損(or_zero/plus/truthy)は実装済み。fake provider record も kind gate を通る通常 record として集計される | 明示 `null` usage/savings は report 全体 fail(JS parity で固定済み) — 新 section でも fail-hard か skip+計数か要判断。refetch orphan(logRef 未達)の扱い未定義。全量メモリ展開の実測閾値未設定 |
| 6. test・guide | usage_report.rs/usage_log.rs に充実した fixture 基盤と injectable `now` がある | `--since` 反映 route・per-route 集計・error breakdown・text parity・0 件/refetch-only/failure-only の境界 test が無い。guide に route bucket・新 flag の記述が無い |

## Alternatives and Comparison

| Option | 内容 | 利点 | 欠点 | 傾き |
| --- | --- | --- | --- | --- |
| O1: `--since` fix のみ | `route_bucket` を `--since` 適用済み record 集合へ変更(他 filter は run 専用のまま) | headline bug を最小 diff で直す。既存 test への影響が狭い | issue scope 2-4(per-route・error 分析・group-by・text parity)を全く満たさない — bug fix だけでは issue を close できない | no(単独では不足。O2 の部分集合としては必須) |
| O2: `--since` fix + additive `--group-by route\|error` + `--route` filter + sectioned output | route/refetch を `--since`(および `--route <name>`)で filter してから集計。`--group-by route` で `result.byRoute[]` の per-route section(executions・succeeded/failed・original/returned/reductionRatio・postprocessFailures・refetch join・durations・計測不能数)、`--group-by error` で `result.byError[]` の code 別 section(count・share・worker/model 内訳・durationMs・execution echo 傾向)。report は additive field・text も section を追加(field 名は spec「出力 contract」節で固定) | issue scope 1-4・6 を網羅。record schema 無変更(read-only 集計のまま)。既存 field・filter の後方互換を保つ。二重計上は kind 分離 + logRef join 規則で定義可能 | diff は中規模(aggregator + parser + text + test + guide)。新 report field の shape を一度決めると additive-only 原則上 retract しづらい | **yes** |
| O3: streaming/bounded aggregation 含む再設計 | O2 + `read_usage_records` の streaming 化・上限付き集計 | 長期 log のメモリ上限を構造的に保証 | usage.jsonl は append-only の小規模 telemetry。Vec<Value> の全量展開は現実的サイズで問題にならず、JS 意味論(plus/truthy/NaN)の streaming 再実装は parity drift 面を増やすだけ(YAGNI)。issue 自身も「測定し、必要なら」と条件付き | no(計測閾値を spec に残して defer) |

### 従属設計論点(O2 内部の sub-decision)

- **`--worker`/`--provider`/`--model` を route record に適用するか**
  — route の `worker` は analyze worker の `{model,usage}` object で
  意味が違う。推奨: 既存 filter は run scoped のままとし、route 軸の
  絞り込みは `--route <name>` のみ(`--provider`/`--model` を route
  の `worker.model` に読み替える選択肢は spec の open question として
  残すが、混同事故を避け scope から外すのが簡潔)。
- **refetch join の規則** — `route-refetch.logRef` → 同 window 内の
  route record `logRef` へ join。join 先が window 外/不存在の
  refetch は `result.route.orphanRefetches` に分離して route に混入
  させない。空・欠損・非文字列の `logRef` を持つ route record は
  join 対象にならず、同様の `logRef` を持つ `route-refetch` は常に
  orphan(absent logRef は `""` で書かれる — route/run.rs:630)。
  route record の `refetches` snapshot field は集計 source にしない
  (route-refetch record 側を唯一の計数源に = 二重計上防止)。
- **error group の次元数** — `code` 主軸 + 副内訳(worker 別・
  model ref 別)は additive object で十分。period 内訳は `--since`
  の filter として表現(group key に期間を畳まない)。
- **`d`/`w` duration suffix** — `parse_duration`/`DURATION_PATTERN`
  は policy JSON schema と共有。usage 専用の拡張は共有 pattern の
  分岐を生む。**決定: 本 issue では拡張せず `168h`/`24h` 表記を
  guide に書く**(issue 例示 `7d` は `168h` で充足。先行 spec
  `usage-and-savings-telemetry-aggregation.md` の `7d` 記述は
  未実装の既知不正確記述として spec の Out of Scope に明記)。
  `d`/`w` の受理は `DURATION_PATTERN` を policy schema から分離
  する将来判断に委ねる。
- **明示 null usage/savings の fail-hard** — 既存 run 集計の
  JS-parity 契約は維持し、新しい grouped section では malformed entry
  を skip し `unmeasurable`/`invalid` 計数で可視化する(既存契約を
  緩めると fixed test を壊し parity を損なう)。

## Tentative Conclusions and Hypotheses

- **H1(confirmed)**: `--since` が route/route-refetch に効かない
  原因は `route_bucket(&all)` が `filter_run_records` より先に全量へ
  掛かる構造(usage_report.rs:552-553)。修正は filter 済み集合を
  渡すか、bucket 側で `ts` を評価する形で一点修正可能。なおこの
  修正で **`--since` 指定時の既存 field `result.route` の値が
  変わる**(これまで無視されていた filter が効くようになる) —
  field 追加ではなく既存値の意味論変更であり、guide での明記が
  要る。
- **H2(confirmed)**: issue 要求の per-route・per-error 分析に必要な
  field は **全て record に既存**(route/…/error.code/execution echo)
  — writer 変更・record schema 変更は不要。read-only 集計の拡張で
  完結する。
- **H3(confirmed)**: text/JSON parity は現時点で欠けている
  (route bucket は text に出ない)。`--format text` は section 追加が
  必要で、guide の「集計対象は run record のみ」記述も drift。
- **H4(confirmed)**: `--worker` は run 専用の意味論 — route record
  の `worker` は object であり、同一 flag を両 kind に掛けると
  「worker 名一致が常に false」になるだけ。route 軸は `--route` で
  分離する方が contract が明確。
- **H5(confirmed)**: `executed.argv` は平文 secret 含有可のため
  group key・既定出力から除外し、安全な identity は `route` +
  `commandHash` に限る(issue 指定と一致)。
- **H6(hypothesis → ADR で採択)**: O2(`--since` fix + additive
  `--group-by route|error` + `--route` filter + sectioned text/JSON
  output)が適切。O1 は issue scope を満たさず、O3 の streaming は
  現実的 file 規模に対し YAGNI。
- **H7(confirmed)**: run/route/refetch の token 二重計上は現行でも
  構造上起きていない(kind 分離)。新設計でも route の
  `originalTokens`/`returnedTokens` は route section に閉じ、run の
  `usage`/`measured` 母数に入れない規則を明示すれば維持できる。

## Open Questions

- `--provider`/`--model` を route record の `worker.model` へ拡張
  適用するか — spec で scope 外とする提案だが、利用者が route の
  analyze model を絞りたい需要が出れば additive flag(例:
  `--route-model`)として別 issue 化の余地。
- grouped 出力の field 名・shape(`routes[]` vs `byRoute{}` 等) —
  additive-only のため初回命名が durable contract になる。**解決:
  spec「出力 contract」節で `byRoute[]`/`byError[]`/
  `result.route.orphanRefetches`/`"unknown"` bucket に固定済み**。
- `--group-by` 複数指定(`,` 区切り/反復 flag)の要否 — **解決:
  単値 last-wins**。`--group-by route,error` のような複合値・未知
  値は INVALID_REQUEST。必要なら後追いでも additive に可能。
- 長期 log のメモリ実測 — 代表的環境での file サイズ分布が取れれば
  spec に streaming 移行トリガの閾値を明記できる。
- `--since` に route-refetch を含める際の orphan 表示名・位置 —
  **解決: `result.route.orphanRefetches`**(`--group-by route`
  指定時のみ emit。window 全体で判定し `--route` 絞り込みでは
  隠れない)。

## Promotion Candidates

- [x] spec-doc needed: 集計意味論・flag・出力 contract・受入条件 —
  `docs/specs/usage-analysis-scope-issue-184.md`
- [x] adr-doc needed: O1/O2/O3 の方式採択と filter 意味論 —
  `docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md`
- [ ] design-doc: grouped report の field shape・refetch join・
  text section layout の細部 — 実装着手前に検討(高確度で必要)

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/184>
- 監査対象: `crates/cheaper-cli/src/usage_report.rs`(全行)・
  `crates/cheaper-cli/src/usage_log.rs`(record builder 全行)・
  `crates/cheaper-cli/src/args.rs`(UsageCliArgs:241-252・
  STRING_OPTIONS:343-378・usage arm:894-941・usage-only ガード
  :759,943-948)・`crates/cheaper-cli/src/cli.rs`(dispatch:390-396・
  record_usage 呼出:266,379,477・help:595-603)・
  `crates/cheaper-cli/src/text_format.rs`(:195-205,341-416)・
  `crates/cheaper-cli/src/route/run.rs`:602-642(`worker` object・
  `durationMs{exec,analyze,total}` 組立て・record 呼出・空
  `logRef`)・`crates/cheaper-cli/src/route/logs.rs`:55-68(refetch
  record 呼出 — read 成功時のみ)・
  `crates/cheaper-core/src/duration.rs`(SUFFIXES:40・
  DURATION_PATTERN:7)・`crates/cheaper-core/src/errors.rs`
  (ErrorCode:54-80)・
  `crates/cheaper-cli/tests/integration/usage_report.rs`(route bucket
  test:788-834)・`docs/guides/usage.md`(計測 log・usage 節)
- 関連 chain(`references` のみ — 別 issue 由来のため chain 内の
  `derives-from`/`refines` には混ぜない): usage telemetry chain
  (`usage-and-savings-telemetry-*`・`delegation-usage-measurement-log`・
  `usage-jsonl-*` ADR)・#141 chain(`worker-run-failure-rate-31pct-*`・
  `run-error-record-observability`・`usage-error-record-field-contract`)
  ・#149 chain(`pretooluse-*` route resolver 一式)・#166 chain
  (`route-grant-set-projection-*` — `executed` attribution)・
  `docs/impl/ir/add-route-and-route-refetch-usage-records.md`。
