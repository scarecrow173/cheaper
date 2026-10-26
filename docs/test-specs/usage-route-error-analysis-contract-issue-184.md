---
id: "TSPEC-034MbSDd2gRpJFzX5FgSQq"
type: "test-spec"
status: "draft"
title: "cheaper usage route/error grouped analysis contract — --since route window, byRoute/byError sections, scoping and privacy invariants (issue #184)"
created: "2026-10-25"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  derived-by: []
  verifies:
    - "docs/specs/usage-analysis-scope-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
  verified-by:
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
  references:
    - "docs/discovery/usage-analysis-audit-issue-184.md"
    - "docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
  defers: []
  deferred-by: []
---

# cheaper usage route/error grouped analysis contract — --since route window, byRoute/byError sections, scoping and privacy invariants (issue #184)

## Purpose

`cheaper usage` の route-scope 集計(global `result.route` bucket と
flag-gated `byRoute`)と error.code 集計(flag-gated `byError`)が、
spec(SPEC-034M0QMBJ1LQRGDdB2wnKl)の「出力 contract」節と
design(DESIGN-034M1By0TmSs1tD6TIDBdK)§1-§6 の確定契約を維持して
いることを pin する検証契約。`--since` window が `kind:"route"` /
`"route-refetch"` record に適用される確定 bug の fix、`--route`
scoping、logRef join・orphan・unkeyed の window-wide 診断、
error code bucket・share 分母・canonical execution key・非有限 →
`null`・text/JSON parity・flag-gated additive emit・privacy
allowlist を固定する。

本 test spec は `crates/cheaper-cli/src/usage_report.rs`・`args.rs`・
`cli.rs`・`text_format.rs`・`tests/integration/{usage_report,args,
usage_log}.rs` を書き換える将来の変更が、確定済みの不変条件を維持
しているかを判定するための契約である。廃止・supersede の検討
タイミングは、usage report の出力 schema が再設計されて
`result.route`/`byRoute`/`byError` の shape・emit 規則が変わった
とき、または集計責務が `build_usage_report` 以外の層へ移ったとき。

## Feature

`cheaper usage` は usage.jsonl の `kind:"run"`・`"route"`・
`"route-refetch"` record を read-only・best-effort に集計する。
`--since` は route-scope にも効き、`--group-by route|error` で
deterministic な per-route/per-error section を flag-gated additive
に emit し、`--format text` と JSON で同一の集計結果を出す。
`--route` は route-scope のみを絞り、run 集計・`byError`・
orphan/unkeyed の window-wide 診断には効かない。group key・出力に
は allowlist 外の sensitive field(`executed.argv`・平文 command・
error detail)を出さない。

## Rules

### #192 consumer tolerance（`tests/integration/usage_report.rs`）

2026-10-26 追加分は独立 review・承認済み（plan の approval checkpoint）。
test-spec 自体は従来規約どおり draft を維持する。#184 は writer/schema を変更せず、
optional `executionId`/`parentExecutionId`/`traceId` を受理して無視する。
以下は consumer tolerance fixture であって、#192 の ID 妥当性・発行・伝搬・
Usage/History 連携の検証ではない。

- `usage_optional_ids_preserve_report`: 同じ run/route/refetch fixture に
  上記 3 field を付けた版と欠損版を用意する。flag 無しと
  `--group-by route`/`error`、既存 `--since`/`--worker`/`--provider`/
  `--model` および `--route` の適用範囲を代表する組合せで、aggregate・
  group・filter・refetch を含む JSON 結果と text 出力がそれぞれ同一。
  ID の有無を gate/filter/dedup に使わない。
- `usage_same_trace_distinct_runs_counted`: 同じ traceId・異なる
  executionId の run 2 件（success と usage-bearing failure 各 1）を
  両方計数し、`records:2`、該当 code の `shareOfRuns:0.5`/
  `shareOfFailures:1`。History の usage 前 failure/skip は入力でなく、
  全実行数を母数とする assertion は追加しない。
- `usage_refetch_distinct_execution_id_joins_log_ref`: window 内 Route と
  refetch に別々の executionId、同じ非空 logRef を付け、join と
  refetch count/bytes が ID 欠損版と同一。kind 別件数は混ぜない。
- `usage_window_orphan_not_backfilled_from_history`: Route owner を
  window 外、同じ logRef を持つ refetch を内側に置く。対応 History が
  存在するという状況を模した inert 外部情報があっても、
  `orphanRefetches` に残り、`byRoute` に owner を復活させない。
- `usage_report_history_independent`: Usage fixture のみで report が成功し、
  History 相当の無関係な外部ファイルを追加しても JSON/text・失敗率母数は
  不変。History event/record は Usage JSONL へ投入しない。

全 test は既存 `write_log`/`report`/`run_usage`/`CliIo` と temp directory
harness を再利用する。後半 2 test の外部情報には、必要なら無関係な
sentinel ファイルと opaque な内容を置く（同じ logRef の文字列を含めても
History record として parse しない）。**将来の History filename/schema を
仮定しない**。この fixture は History を認識する test ではなく、
Usage のみを読む境界の保証であり、実 History writer/store は追加しない。

### Route window と `--since` 適用(`tests/integration/usage_report.rs`)

- `route_bucket_applies_since`: `kind:"route"`・`"route-refetch"`
  record は `ts >= now - sinceMs` を満たすもののみが
  `result.route`(global bucket・refetch 計数)へ反映される —
  window 外 record は除外される(確定 bug の fix、spec Req 1)。
- `route_window_invalid_timestamps`: `--since` 指定時、`ts` 欠損・
  非文字列・parse 不能な route/refetch record は window に含めない。
  `--since` 無指定なら従来通り含める(run filter と同一規約、
  `date_parse` 共有)。
- `route_group_ignores_worker_filter`: `--worker`/`--provider`/
  `--model` は route-scope へ一切適用しない — `--group-by route
  --worker W` で run 集計のみが絞られ、route bucket・`byRoute` は
  route window 全体を集計する(route record の `worker` は analyze
  meta object で意味が違う — spec Out of Scope、監査 H4)。
- run/route/refetch は kind で厳密分離され、route record の
  `worker.usage`・token field は `records`/`usage`/`measured`/
  `comparable` 母数へ混入しない(`no_double_counting`)。

### `byRoute` 集計と join/orphan/unkeyed

- `group_by_route_emits_per_route_rows`: `--group-by route` で route
  名昇順(`BTreeMap` による構造保証)に `{route, executions,
  succeeded, failed, originalTokens, returnedTokens, reductionRatio,
  refetches{count,bytes}, durationsMs{exec,analyze,total},
  postprocessFailures, unmeasurable{tokens,durations}}` の field 順で
  emit。`reductionRatio = Σoriginal > 0 ? 1 - Σreturned/Σoriginal :
  0.0`(record writer と同一規約値、`num()` 経由で非有限 →
  `null`)。`durationsMs` は 3 subfield それぞれの和(mean は
  導出値で emit しない)。
- `unkeyed_route_records_counted`: `route` field 欠損/非文字列の
  route record は group fold から除外し `unkeyedRecords` に計数
  する。計数は route window 全体で行い `--route` で減らさない。
  global `executions` 等へは `--route` 無指定時のみ従来通り計上
  する(`--route` 指定時は scoped 集合外のため global にも入ら
  ない)。
- `orphan_and_joined_refetches`: join owner になれるのは `route`
  が文字列かつ `logRef` が非空文字列の route record のみ。refetch
  の `logRef` が非空文字列かつ owner map に存在 → joined;
  空(`""` で書かれる absent logRef 含む)・欠損・非文字列・
  owner 不在 → `orphanRefetches{count,bytes}`。orphan 判定は
  window 全体で行い `--route` 絞り込みでは隠れない。同一 `logRef`
  の複数 owner は file 順 first-wins(二重帰属を構造で防止)。
- refetch 計数の唯一の計数源は `route-refetch` record の `logRef`
  join — route record の `refetches` snapshot field は集計しない
  (`no_double_counting` が pin)。
- `unmeasurable_metric_families`: `originalTokens`/`returnedTokens`
  のいずれかが欠損/非数値 → `unmeasurable.tokens += 1`、
  `durationMs` が欠損/非 object、または `exec`/`analyze`/`total`
  のいずれか欠損/非数値 → `unmeasurable.durations += 1`(いずれも
  record 毎 1 回の family 別計数)。値自体は `or_zero` で和に
  残る(欠損 subfield は当該和へ 0 加算 — spec「計測不能」節)。
  route record の `worker:{}` は analyze 未実行の正常形で
  unmeasurable に計上しない。
- group 0 件でも `--group-by route` 指定済みなら `byRoute: []` を
  emit する(key 省略しない)。

### `byError` 集計と canonical key

- `group_by_error_emits_code_rows`: 入力は `filter_run_records` 済み
  run 集合の `!truthy(ok)` subset(errors 母数と同一集合)。
  `count` 降順・同数 `code` 昇順に sort して `{code, count,
  shareOfRuns, shareOfFailures, workers{…}, models{…}, durationMs,
  reasons{…}, execution{timeoutMs{…}, maxTurns{…},
  maxToolCalls{…}}}` の field 順で emit。`shareOfRuns = count /
  filter 後 run record 数`、`shareOfFailures = count / filter 後
  error 数` — 分母 0 は非有限 → `null` の既存規約。
- `empty_error_code_is_preserved`: `error.code:""` は空文字列 key
  として維持し、`error` object 欠損・`code` 欠損・明示 `null`・
  非文字列を畳む `"unknown"` bucket へ誤統合しない(spec の畳み
  込み条件は欠損/null/非文字列のみ — 厳密適用)。
- `execution_number_keys_use_js_canonical_format`: `execution` echo
  の distinct value key — Number は
  `cheaper_core::json::number::format_number` 経由で `3` と `3.0`
  が同じ `"3"` key に集約、Null→`"null"`、Bool/String/array/object
  は writer contract 外の値としてその subfield entry を skip。
  `execution` は常に 3 subfield を持つ(空でも `{}`)。
- `workers`/`models`/`reasons` の inner map は key 昇順。該当
  field(`worker` 文字列・`model.provider` + `model.id` の両方
  文字列・`error.reason` が abort 語彙 `timeout|signal|maxTurns|
  maxToolCalls` のいずれか)を持たない record は当該内訳にのみ
  含めず、`count` 本体には入る。未知 reason 文字列は出力せず、保存済み
  JSONL に任意 error detail が混入しても privacy allowlist を維持する。
- `execution` inner map は `null` と JSON number のみ含める。数値は
  canonical JavaScript number key、`null` は `"null"` key とする。
  boolean/string/array/object は writer が生成しない型のため skip し、
  arbitrary string の出力・集計を許さない。
- error 0 件でも `--group-by error` 指定済みなら `byError: []` を
  emit する。
- `argv_and_error_detail_not_leaked`: `error.message`・
  `schemaErrors`・`checkpoint`・`errorMessage`・`root`/path 系・
  `executed.argv`・平文 command・allowlist 外 `error.reason`・不正な
  `execution` string value を集計 key・出力に出さない —
  secret 文字列を含む fixture record を置き、report 文字列化に
  現れないことを pin する(spec Req 4 allowlist)。

### `--route` scoping と filter 分離

- `route_filter_scopes_route_sections_only`: `--route X` は
  `result.route` global bucket と `byRoute` entry を X に限定する
  が、run 集計(records/usage/measured/comparable)には一切効か
  ない。本 test の主張範囲は route-scope と run 集計の分離まで —
  `byError` が `--route` の影響を受けないことは別名 test
  `route_filter_leaves_by_error_unscoped` が pin する(T4 所有、
  flag 組合せが dispatch で到達可能になってから green)。
- `route_filter_leaves_by_error_unscoped`: `--route X --group-by
  error` の組合せは合法で、`byError` は無 scoped のまま filter
  済み run 集合全体を集計する(route bucket のみ scoped — spec
  Req 5 の組合せ規約)。
- `route_flag_scopes_route_at_dispatch`: `usage --route X` を
  dispatch 経路(`run_cli`/`CliIo` harness)で e2e 検証する —
  `args.route` → `UsageFilters.route` の handoff が効き、
  envelope の `result.route` が X に scoped される。
  `build_usage_report` 直撃の pin だけでは CLI 側の
  `filters.route` 配線を保証できないため、本 test がその結合を
  固定する。
- `route_filter_without_group_by`: standalone `--route X` でも
  global bucket を X に限定するが、flag-gated key は emit しない
  — `byRoute` は `--group-by route` がない限り出ず、
  `orphanRefetches`/`unkeyedRecords` も `--group-by` 無指定では
  `result.route` に載らない(flag-gated additive の証明)。
- scoped 集合の refetch は対象 route の非空 `logRef` に join した
  もののみ(`joined[n]`)。`--route` 無指定時の global
  `refetches`/`refetchBytes` は window 内全 refetch(joined +
  orphan — 従来 field 意味論の維持)。
- `--route` の値は空文字列を含め検証しない(一致 route が無ければ
  scoped 集合が空になるだけ)。

### CLI flag とガード(`tests/integration/args.rs` + dispatch)

- `route_flag_rejected_for_route_resolve`: `route resolve --route x`
  は `route` arm の per-action allowlist が `"--route is not valid
  for \`route resolve\`"` で reject する(silent drop 防止規約の
  維持 — `tests/integration/args.rs` の parse-level assertion)。
- `args_usage_only_guard_messages`: `count-tokens --route x`・
  `run --group-by route` 等の他 command は usage-only guard が
  `"--since/--compare-model/--group-by/--route are only valid for
  \`usage\`"` の列挙形式 exact message で reject
  (`tests/integration/args.rs` — `parse_cli_args` で観測可能)。
- `invalid_group_by_value_rejected_at_dispatch`: `--group-by` は
  単値の `route|error` のみ受理(既存 string option の last-wins
  規約)。`route,error` のような複合値・未知値は `INVALID_REQUEST`
  — `"invalid --group-by: {v} (expected route|error)"`。args 層は
  生文字列を保持して `parse_cli_args` が `Ok` を返すため、この
  値検証は `tests/integration/args.rs` では観測できず、
  `tests/integration/usage_report.rs` の `run_usage`/`CliIo`
  dispatch harness 経由で検証する。
- `group_by_last_wins_emits_by_error_only`: `usage --group-by route
  --group-by error` は last-wins で `"error"` が採用され、
  `byError` section のみ emit される(`byRoute` は出ない)—
  dispatch 経路で検証。
- `usage_log_report_callers`: `integration/usage_log.rs` の
  `build_usage_report` 全 caller が新 `group_by`/`route` 引数へ
  追従し、各 caller の既存出力契約を保つ。
- 想定内更新(anticipated churn): `since_only_valid_for_usage`
  (`tests/integration/args.rs`:379-385)は、guard message が
  `--group-by`/`--route` を含む新列挙へ変わるため T4 で expected
  message を更新する。`usage_command_format_text_writes_report`
  の更新は `format_text_parity` を参照。

### 出力 contract・text parity・flag-gating

- 新規 `result` key は全て flag-gated additive — `--group-by` 指定
  時のみ emit。key 挿入位置は `route` 直後・`estimated` の前で
  `byRoute` → `byError` の順(単値 flag のため同時には出ないが
  順序は固定)。
- global `result.route` は既存 7 key 同順を維持し、`--group-by
  route` 時のみ末尾へ `orphanRefetches{count,bytes}`・
  `unkeyedRecords` を additive 追加する。
- `format_text_parity`: text 側は `result.route` 相当の route
  section を常時描画(現行 parity gap の解消 —
  `usage_command_format_text_writes_report` の想定内更新)。
  `route.orphanRefetches` 存在時のみ `route scope` 行、
  `byRoute`/`byError` key 存在時は entry 0 件でも section header
  を無条件に描画(JSON の空 array emit 規約と同一)。空の inner
  map(workers/models/reasons)は detail 行を省略し、`execution`
  の 3 subfield が全て空なら行自体を省略する。
- `format_text_preserves_equals_in_execution_keys`: count-detail text は
  map key を加工せず表示し、key `a=b` を `a=b:1` として保持する。
- `nonfinite_aggregates_serialize_as_null`: 非有限 aggregate 値は
  JSON `null` になり、JSON/text ともに `NaN`/`Infinity` 表記を
  出さない(既存 JS number 意味論 `num()`/`plus`/`or_zero`/
  `truthy` の踏襲 — spec Req 8)。

### Fixture 網羅と後方互換

- fixture matrix — valid/mixed-legacy/malformed/fake/empty/
  refetch-only/failure-only を固定 JSONL で検証する:
  - `fake_records_aggregate`: 構造的に valid な fake route/
    `route-refetch` record(`is_record` 合格)が通常 record として
    route-scope 集計される(route-record fake のみを主張 — T2)。
  - `fake_run_error_records_aggregate`: 構造的に valid な fake
    run error record が `byError` rows に通常集計される(T3)。
  - `legacy_records_use_defaults`: optional field 欠損の旧 schema
    record は `or_zero` 既定値 + 対応 `unmeasurable` 計数で
    処理される。
  - `refetch_only_log`: route record 0 件で refetch のみ →
    `executions:0`・global `refetches` 計上・全件 orphan・
    `byRoute:[]`。
  - `failure_only_log_emits_by_error_rows`: error record のみ →
    `byError` rows が立つ(T3)。
  - `empty_log_emits_empty_grouped_sections`: record 0 件 +
    `--group-by route` → `byRoute:[]`、`--group-by error` →
    `byError:[]` が emit される(T3 — `byRoute:[]` の emit 検証が
    T2 の `byRoute` emit 実装に依存するため、T2 完了後にのみ
    green になり得る)。
  - malformed JSONL 行・未知 kind・非 object record は
    `read_usage_records`/`is_record` gate で除外済み(集計側で
    再判定しない)。
- `existing_contract_unchanged`: 既存 test 群(filter・aggregate
  の明示 null fail-hard・estimated・regression golden)が無変更で
  green — flag 無し出力の shape/値不変。`aggregate` の fail-hard
  契約は run 集計に限定したまま grouped section へ持ち込まない。
- `no_double_counting`: route record の token・refetch snapshot が
  run 母数へ混入せず、route-refetch の `logRef` join が唯一の
  refetch 計数源である。

### 大規模入力の sanity

- 代表的サイズの bounded JSONL fixture(run/route/route-refetch
  混在・実運用スケール相当の行数)で `build_usage_report` が完結
  し、集計結果が小規模 fixture と整合する sanity を pin する。
  peak memory の数値計測自体は本 spec の保証対象外(T5 task の
  計測証跡 — Non-goals 参照)。

## Examples

- window 内外の route/refetch を混ぜた fixture に `--since 24h` を
  付けると、window 内 record のみが `result.route`・`byRoute`・
  refetch 計数に反映される。
- `--group-by route` で route-a・route-b の 2 entry が route 名昇順に
  出て、join 済み refetch の `refetches{count,bytes}` が entry 毎に
  付き、owner 不在の refetch は global `orphanRefetches` に入る。
- `--route route-a --group-by error` で `result.route` は route-a のみ
  に scoped されつつ `byError` は filter 後 run 集合全体を集計し、
  `orphanRefetches`/`unkeyedRecords` は window-wide の値を維持する。
- `error.code` が `"SCHEMA_VIOLATION"`・`""`・欠損・`null`・数値の
  error record が混在すると、`byError` は `""` と `"unknown"` を別
  bucket に出し、`""` は `unknown` に畳まれない。
- `execution.timeoutMs` が `3` と `3.0` の 2 record は同じ `"3"`
  key に集約され、`null` 値は key `"null"` になり、array 値の
  subfield entry は skip される。
- `executed.argv` に secret 文字列・`error.message` に平文を含む
  record を置いても、JSON report の stringify 結果・text 出力の
  どちらにも該当文字列が現れない。
- flag 無しの `cheaper usage` JSON は新 key を一切持たず、regression
  golden `cli-usage-*.json` が byte 不変で green。
- `count-tokens --route bulk-reader` は usage-only guard の列挙
  message で INVALID_REQUEST になり、`route resolve --route x` は
  per-action allowlist message で reject される。

## Guarantees

- optional 相関 ID による aggregate/group/filter/dedup の変化がなく、
  独立 refetch ID でも window-bound logRef join を維持する。
  History 不在で成立し、History 情報で orphan/失敗率を補完しない。

- `tests/integration/usage_report.rs`・`args.rs`・`usage_log.rs` の
  pin test が、`--since` の route 適用・scoping・join/orphan/
  unkeyed・`byRoute`/`byError` emit 規則・share 分母・canonical
  execution key・非有限 → `null`・flag-gated additive・text/JSON
  parity・privacy allowlist・fixture matrix を deterministic に
  検証する。
- flag 無しの JSON 出力は shape・値ともに不変で、既存 filter・
  aggregate・regression golden test が無変更で green を維持する。
- grouped section は fail-soft(構造的 invalid → skip+計数、
  malformed scalar → `or_zero` + `unmeasurable`)で、`aggregate`
  の fail-hard 経路を侵食しない。
- run/route/refetch は kind で厳密分離され、token・件数の二重計上
  がない。

## Non-goals

- #192 の ID 表現・発行・伝搬・妥当性・History store および適用可能な
  新規 Usage record の必須 ID 連携。その acceptance は #192 が所有し、
  本 consumer tolerance test は代替しない。

- usage.jsonl writer・record schema の正しさ — read 側のみの変更
  であり、writer 契約は既存 chain(usage telemetry/error record
  系 test-spec)の管轄。
- ピークメモリ・parse 時間の数値閾値検証 — T5 task が再現可能
  command で証跡を残す計測であり、`cargo test` の保証対象外。
- streaming/bounded aggregation 移行後の正しさ — 本 spec は現行の
  全量 `Vec<Value>` 展開前提の契約。
- `--worker`/`--provider`/`--model` の route record(`worker.model`)
  への拡張適用・`d`/`w` duration suffix・期間 bucket 行出力・
  workflow 由来の usage 観測 — spec/design の Out of Scope・
  Deferred Design Concerns。
- `usage_only_option_used` 以外の option guard 体系・`route`
  command の per-action allowlist 全体 — 新 flag に関わる分のみを
  pin する。
- guide の文面・doc 品質 — `docs/guides/usage.md` の更新内容は
  T4 task の review 対象であって test 保証対象外。

## Risk

- `--since` route 適用 fix が pin されないと、`route_bucket` が再び
  window 外の全量集計へ戻り、期間分析の値が静かに誤る。
- scoping・window-wide 診断の規則が緩むと、`--route` で orphan/
  unkeyed が隠れたり run 集計へ route record が混入したりして、
  「二重計上しない」契約が壊れる。
- flag-gating・key 順・sort 規則が drift すると regression golden
  が理由なく更新され、text/JSON parity gap が再発する。
- allowlist が緩むと `executed.argv`・error detail が report へ
  漏出し、telemetry file の privacy 前提が崩れる。
- canonical execution key が `format_number` を通らないと `3`/
  `3.0` が別 bucket に分裂し、execution echo 集計が使い物になら
  ない。
