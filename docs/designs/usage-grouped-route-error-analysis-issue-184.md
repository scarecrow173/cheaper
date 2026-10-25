---
id: "DESIGN-034M1By0TmSs1tD6TIDBdK"
type: "design"
status: "approved"
title: "Usage grouped route/error analysis — --group-by・--route・per-route/byError sections (issue #184)"
created: "2026-10-25"
updated: "2026-10-26"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
    - "https://github.com/scarecrow173/cheaper/issues/192"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-cli/src/args.rs"
    - "crates/cheaper-cli/src/text_format.rs"
    - "crates/cheaper-cli/src/cli.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/usage-analysis-scope-issue-184.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/usage-analysis-scope-issue-184.md"
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/usage-analysis-scope-issue-184.md"
    - "../adr/grouped-route-and-error-analysis-in-cheaper-usage.md"
    - "../discovery/usage-analysis-audit-issue-184.md"
  derived-by:
    - "../plans/implement-usage-route-error-analysis-issue-184.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/run-error-record-observability.md"
    - "../designs/delegation-usage-measurement-log.md"
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../specs/route-grant-set-projection.md"
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
metadata: {}
---

# Usage grouped route/error analysis — --group-by・--route・per-route/byError sections (issue #184)

## Context

2026-10-26: #192 境界の amendment は独立 review と Judge 判定を経て承認済み
（記録は plan の approval checkpoint）。先行版の承認と ADR の O2 は維持する。
実装の検証完了は意味しない。

`docs/specs/usage-analysis-scope-issue-184.md`(SPEC-034M0QMBJ1LQRGDdB2wnKl)
と `docs/adr/grouped-route-and-error-analysis-in-cheaper-usage.md`
(ADR-034M0QWL9LflaeyWc8vho9、O2 採択)の実装設計。
`docs/discovery/usage-analysis-audit-issue-184.md` の監査で確定した構造:

- `build_usage_report`(usage_report.rs:551-556)が `route_bucket(&all)` を
  `filter_run_records` より先に全量へ掛けるため、`--since` が
  `kind:"route"`/`"route-refetch"` bucket に効かない(確定 bug — 本 design
  の修正対象)。
- per-route 分析に必要な record field(`route`・`logRef`・
  `originalTokens`/`returnedTokens`・`durationMs{exec,analyze,total}`・
  `postprocessFailures`・`ok`)と run error record の
  `error.code`/`execution` echo は **writer 側に全て揃っている**
  (usage_log.rs:369-442・:242-327)— writer/schema 変更なしの
  read-only 集計拡張で完結する。
- spec「出力 contract」節が新規 `result` field の**名前・shape・emit
  規則**(flag-gated additive、`route` key 直後に `byRoute` → `byError`
  の順)を固定済み。本 design はその構造と code path を埋める。field
  名・意味論は変えない。

設計の原則: 既存の JS parity 意味論(`plus`/`or_zero`/`truthy`/非有限 →
`null` の `num()`)と read-only・best-effort 集計の責務を維持し、
`aggregate` の明示 `null` fail-hard 契約は run 集計に限定したまま触れ
ない。新規 grouped section は spec Req 7 の二段ルール(構造的 invalid は
skip+計数、otherwise-valid record 内の malformed scalar は `or_zero`
加算 + `unmeasurable` family 計数)で扱う。

## Scope

### In Scope

- `crates/cheaper-cli/src/usage_report.rs`:
  - route/route-refetch record への `--since` window 適用
    (route 専用の filter 経路を新設)。
  - `route_bucket` の再設計 — global bucket の維持 +
    `--route` scoping + `logRef` join + orphan/unkeyed 計数 +
    per-route 集計(`byRoute` rows)。
  - run error record の `error.code` 別集計(`byError` rows)。
  - `build_usage_report` の signature 拡張(`group_by` 引数)と
    `UsageFilters.route` 追加。
- `crates/cheaper-cli/src/args.rs`: `--group-by`/`--route` string option
  登録・`UsageCliArgs` 拡張・`usage_only_option_used` ガード編入 +
  error message 更新。
- `crates/cheaper-cli/src/usage_report.rs` の `execute_usage_inner`:
  `--group-by` 値検証(`route|error` 以外は INVALID_REQUEST)と
  `filters`/`group_by` の `build_usage_report` への受け渡し。
- `crates/cheaper-cli/src/cli.rs`: help 行 2 行追加。
- `crates/cheaper-cli/src/text_format.rs`: `format_usage_report` への
  route section 常時描画 + flag-gated `byRoute`/`byError` section 追加。
- `crates/cheaper-cli/tests/integration/usage_report.rs`: 固定 JSONL
  fixture test 群(§7)。
- `crates/cheaper-cli/tests/integration/args.rs`: `count-tokens --route`
  と `run --group-by` の usage-only guard exact-message test。
- `crates/cheaper-cli/tests/integration/usage_log.rs`: report builder を
  呼ぶ既存 caller を棚卸しし、引数追従と出力契約を検証。
- `docs/guides/usage.md`: route bucket・新 flag・`--since` 意味論変更・
  母数/actual/measured/estimated の区別の追記(§8)。

### Out of Scope

- usage.jsonl の writer 変更・record schema 変更(additive-only schema を
  維持 — read 側のみの変更)。
- streaming/bounded aggregation への移行(全量 `Vec<Value>` 展開を維持。
  移行判断前に代表的な大規模 JSONL でピークメモリを bounded measurement
  し、その結果を記録する。移行 trigger は §「Deferred Design Concerns」)。
- `parse_duration`/`DURATION_PATTERN` の `d`/`w` suffix 拡張(policy
  JSON schema と共有の pattern — spec Out of Scope)。
- `--worker`/`--provider`/`--model` の route record への拡張適用
  (route の `worker` は analyze meta object で意味が違う — spec Out of
  Scope)。
- 日次/週次等の期間 bucket 行出力(`--since` window 絞り込みのみ —
  spec Req 1 の固定解釈)。
- `error.message`・`schemaErrors`・`checkpoint`・`errorMessage`・
  `root`/path 系・`executed.argv`・平文 command の集計・出力
  (spec Req 4 の allowlist)。
- workflow 由来の usage 観測(usage.jsonl に記録されない現行契約)。

## Components and Boundaries

### 1. Module structure — `usage_report.rs`

route-scope filtering は `filter_run_records` を拡張せず、**route 専用の
別経路**とする。理由: 適用される filter が kind 毎に異なる
(route 系は `--since` のみ、`--worker`/`--provider`/`--model` は
run 専用意味論 — 監査 H4/spec Req 5)。1 関数に両 kind の filter を畳むと
「どの filter がどの kind に効くか」が signature から読めなくなり、
`--worker` を route record に誤適用する事故面が残る。責務境界を kind
分離のまま保つ方が contract が明確(KISS)。

```rust
/// `--group-by` の値域 — spec Req 5 の単値 contract。
/// args 層は生文字列を保持し、cli 層(`execute_usage_inner`)で
/// この enum へ検証変換する(`--since`/`--model` と同じ分担)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UsageGroupBy {
    Route,
    Error,
}
```

```rust
pub struct UsageFilters {
    pub since_ms: Option<f64>,
    pub worker: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    /// `--route <name>` — route-scope 集計(global `result.route` bucket
    /// と `byRoute`)のみを絞る。`filter_run_records` は参照しない
    /// (run record 集計・`byError` には効かない — spec「--route の
    /// scoping」)。
    pub route: Option<String>,
}
```

```rust
/// `--since` window 済みの route-scope record 集合(kind 別に分離)。
struct RouteWindow<'a> {
    routes: Vec<&'a Value>,
    refetches: Vec<&'a Value>,
}

/// `kind:"route"`/`"route-refetch"` を `--since` window で絞る。
/// `ts` 欠損・parse 不能な record は `since_ms` 指定時のみ除外
/// (run record と同一規約 — spec Req 1)。`since_ms` が None なら
/// 全件通す。worker/provider/model filter は一切適用しない。
fn route_window<'a>(
    records: &'a [Value],
    since_ms: Option<f64>,
    now: &dyn Fn() -> f64,
) -> RouteWindow<'a>
```

```rust
/// per-route 集計の accumulator(byRoute entry と同じ metric set)。
/// global bucket の計算にも一部 field を再利用する。
#[derive(Debug, Default)]
struct RouteAgg {
    executions: f64,
    succeeded: f64,
    failed: f64,
    original_tokens: f64,
    returned_tokens: f64,
    postprocess_failures: f64,
    refetch_count: f64,
    refetch_bytes: f64,
    dur_exec: f64,
    dur_analyze: f64,
    dur_total: f64,
    unmeasurable_tokens: f64,
    unmeasurable_durations: f64,
}

/// route-scope 集計の結果。`global` は `result.route` の値(Map)。
/// `by_route` は `--group-by route` 指定時のみ rows を持つ。
struct RouteReport {
    global: Map<String, Value>,
    by_route: Vec<Map<String, Value>>,
}

/// `route_bucket`(:503-538)の後継。window 済み集合 + `--route` 名 filter
/// + grouped emit flag を受け、global bucket・(あれば)per-route rows・
/// orphan/unkeyed 計数を 1 回の構造で組み立てる。
/// `route_name=None, group_by_route=false` で従来の global-only 挙動と
/// 同一の 7-field map を返す(後方互換)。
fn build_route_report(
    window: &RouteWindow,
    route_name: Option<&str>,
    group_by_route: bool,
) -> RouteReport
```

```rust
/// error.code 別 accumulator。内訳 map は `BTreeMap` で key 昇順を
/// 構造保証する(serde_json `Map` の挿入順 = 出力順のため、
/// deterministic な golden に必要)。
#[derive(Debug, Default)]
struct ErrorAgg {
    count: f64,
    duration_ms: f64,
    workers: BTreeMap<String, f64>,
    models: BTreeMap<String, f64>,
    reasons: BTreeMap<String, f64>,
    exec_timeout_ms: BTreeMap<String, f64>,
    exec_max_turns: BTreeMap<String, f64>,
    exec_max_tool_calls: BTreeMap<String, f64>,
}

/// filter 済み run record 集合から `byError` rows を組み立てる。
/// 入力は `aggregate` と同じ `records` を共有する(share 分母の一貫性 —
/// spec Req 4 の「`errors` 母数と同一集合」)。
fn build_by_error(records: &[Value]) -> Vec<Map<String, Value>>
```

`build_usage_report` の signature は `group_by` 引数を 1 つ追加する形に
留める(7 引数化を避けるため `route` filter は `UsageFilters` 側へ):

```rust
pub fn build_usage_report(
    user_config_dir: &Path,
    filters: &UsageFilters,
    group_by: Option<UsageGroupBy>,
    compare_model: Option<&str>,
    compare_catalog_model: Option<ModelInfo>,
    now: &dyn Fn() -> f64,
) -> Result<Map<String, Value>, CheaperError>
```

`route_bucket` は `build_route_report` に置き換わる(旧名・旧 signature
の wrapper は残さない — caller は `build_usage_report` 一箇所のみで、
test 側は新構造へ追従する)。

### 2. Aggregation algorithms

#### 2.1 `route_window` — route/refetch の期間 filter

```
for r in records:
    kind ∈ {"route","route-refetch"} 以外 → skip(is_record gate 済み)
    if since_ms.is_some():
        ts = str_field(r,"ts").and_then(date_parse)
        ts が Some(t) かつ t >= now() - since_ms 以外 → 除外
    kind=="route" → routes.push(r) / kind=="route-refetch" → refetches.push(r)
```

`date_parse`(:97)をそのまま共有し、run record(:258-264)と同一の
window 規約に揃える。

#### 2.2 `build_route_report` — global bucket + per-route + join

手順(全て window 済み集合の上で):

1. **`logRef` owner map を window 全体で構築** — window 内 route
   record の `route` が文字列であり、かつ `logRef` が非空文字列の
   場合だけを `HashMap<&str,&str>`(logRef → route 名)へ入れる。
   `route` field 欠損/非文字列の record は owner を主張できないので map に入らず、
   `logRef` 空・欠損・非文字列の record も join 対象にならない
   (spec「--route の scoping」)。同一 logRef を複数 route record が
   持つ degenerate ケースは **file 順 first-wins**(`.entry().or_insert()`)
   — refetch の二重帰属を構造で防ぎ deterministic にする(本 design の
   refine — spec は「非空 `logRef` のどれかに一致」までで、複数 owner
   の帰属先を未指定)。
2. **refetch の join/orphan 振り分け** — window 内 refetch record 毎に:
   - `logRef` が非空文字列かつ owner map にある → `joined[owner]`
     の `{count,bytes}` へ `count+=1`、`bytes+=or_zero(bytesRead)`。
   - それ以外(空・欠損・非文字列・owner 不在)→ `orphan_count`/`orphan_bytes`
     へ同様に加算。**window 全体で判定**し、`--route` 絞り込みでは
     隠れない(spec固定 — orphan はどの route にも帰属しない)。
3. **scoped route 集合** — `route_name=Some(n)` なら `route` field が
   `Some(n)` に一致する record のみ、None なら全 route record。
   `route` field 欠損/非文字列の record は名前一致できないため
   `--route` 指定時は常に scoped 集合から外れる(unkeyed 計数は 4)。
4. **per-record fold** — scoped 集合の route record 毎に:
   - `route` field が文字列 → group accumulator(`BTreeMap<String,RouteAgg>`
     で route 名昇順を構造保証)へ fold。**`--route` 指定時は group も
     scoped 集合から作るため、byRoute entry も対象 route に限定される**
     (spec Req 5)。
   - `route` field 欠損/非文字列 → group fold から除外する。
     別途、window 全体の route record を走査して `unkeyedRecords` を数える
     (`--route` の有無で値は変えない)。unkeyed は特定 route に帰属しない
     診断数であり、`--route` 指定時も可視性を維持する(spec 明示)。
   - fold 内容(per-record):
     - `executions += 1`、`truthy(ok)` → `succeeded += 1` else
       `failed += 1`。
     - `originalTokens`/`returnedTokens`: いずれかが欠損/非数値なら
       `unmeasurable_tokens += 1`(record 毎に 1 回)。値はそれぞれ
       `or_zero` で和へ。
     - `postprocessFailures`: `or_zero` で和へ。
     - `durationMs`: object でない・欠損、または `exec`/`analyze`/`total`
       のいずれかが欠損/非数値なら `unmeasurable_durations += 1`
       (record 毎に 1 回)。各 subfield は `or_zero` で `dur_*` の和へ
       (spec「計測不能」節 — 欠損 subfield は当該和へ 0 加算)。
     - `worker` field は一切読まない(`{}` は analyze 未実行の正常形 —
       計上しない。spec固定)。
   - group fold と並行して、scoped 集合の `executions`/`failed`/
     `originalTokens`/`returnedTokens`/`postprocessFailures` の和を
     global 用に畳む(unkeyed record も `--route` 無指定時は global
     へ従来通り計上 — spec:「global `executions` 等には従来通り計上」。
     `--route` 指定時は scoped 集合に含まれないため global にも入らない)。
5. **refetch 総数の決定** —
   - `route_name=None`: global `refetches`/`refetchBytes` = window 内
     **全** refetch record(joined + orphan。従来 field 意味論を維持し
     window のみ効かせる — additive 原則)。
   - `route_name=Some(n)`: `joined[n]` のみ(対象 route の非空 `logRef`
     に join したもののみ — spec固定)。
   - per-group `refetches{count,bytes}` = `joined[route名]`。
6. **出力組立て** — global `result.route` は既存 7 key
   (`executions`/`failures`/`refetches`/`refetchBytes`/`originalTokens`/
   `returnedTokens`/`postprocessFailures`)を**同順**で出し、
   `group_by_route=true` のときのみ末尾へ `orphanRefetches{count,bytes}`、
   `unkeyedRecords` を additive 追加。
   `by_route` rows は `BTreeMap` の昇順で
   `{route, executions, succeeded, failed, originalTokens,
   returnedTokens, reductionRatio, refetches{count,bytes},
   durationsMs{exec,analyze,total}, postprocessFailures,
   unmeasurable{tokens,durations}}` の field 順に組み立てる。
   `reductionRatio = Σoriginal > 0 ? 1 - Σreturned/Σoriginal : 0.0`
   (record writer と同一規約値 — spec Req 3。`num()` 経由で非有限 →
   `null`)。
   group 0 件でも `group_by_route=true` なら `byRoute: []` を emit。

#### 2.3 `build_by_error` — error.code 別集計

入力: `filter_run_records` 済み run 集合(`--route` は効かない — run
filter に `route` を含めない)。処理:

```
errors = records.iter().filter(|r| !truthy(get(r,"ok")))
for r in errors:
    code = r.error が object かつ error.code が文字列 → その値
           それ以外(error 欠損/非 object・code 欠損/null/非文字列) → "unknown"
           ※ 空文字列 "" は文字列としてそのまま bucket 名になる
             (spec の畳み込み条件は欠損/null/非文字列のみ — 厳密適用)
    agg = groups.entry(code)
    agg.count += 1
    agg.duration_ms += or_zero(r.durationMs)
    r.worker が文字列 → agg.workers[worker] += 1
    r.model.provider と r.model.id が両方文字列 → agg.models["{p}/{id}"] += 1
    r.error.reason が文字列 → agg.reasons[reason] += 1
    r.execution が object → 3 subfield 固定で、存在する各値 v について
        key(v) → agg.exec_*[key] += 1
        key(v): Null→"null" / Number→canonical JavaScript number text /
                Bool→"true"|"false" / String→その文字列 /
                その他(array/object) → その subfield entry を skip
        Number の正規化には公開 API
        `cheaper_core::json::number::format_number(f64)` を使う。
        これにより `3` と `3.0` は同じ key `"3"` になる(writer は
        number|null のみ書く — usage_log.rs:230 insert_execution_echo。
        それ以外の型は defensive skip)
```

emit: `count` 降順・同数は `code` 昇順に sort してから rows を組み立てる
(sort は emit 直前に行い、`BTreeMap` 群の deterministic 性と分ける)。
各 row は spec固定 field 順:

```
{code, count, shareOfRuns, shareOfFailures, workers{…}, models{…},
 durationMs, reasons{…}, execution{timeoutMs{…}, maxTurns{…},
 maxToolCalls{…}}}
```

- `shareOfRuns = count / records.len()`(filter 後 run record 総数)、
  `shareOfFailures = count / errors.len()`(同一集合の error 件数)。
  `num(count/denominator)` で非有限 → `null`(分母 0 規約 — entry が
  ある以上 error 分母は ≥1 だが uniform に適用)。
- `workers`/`models`/`reasons`/`execution` の inner map は key 昇順
  (`BTreeMap` → `Map::from_iter`)。`execution` は常に 3 subfield を
  持つ(空でも `{}` — spec「3 subfield 固定」)。
- `error`/`code` 欠損・明示 `null`・非文字列は `"unknown"` bucket。
- `error.message`/`schemaErrors`/`checkpoint`/`errorMessage`・生
  prompt/path 系は集計・出力しない(spec Req 4 allowlist)。
- error 0 件でも `--group-by error` 指定済みなら `byError: []` を emit。

### 3. Filter flow — 実行順序

`build_usage_report` 内の順序(borrow 上もこの順でなければならない —
`route_window` は `&all` を借りるため、`all` を move する
`filter_run_records` より先に `build_route_report` まで完了させる):

```
1. let all = read_usage_records(user_config_dir)
      — per-line parse + is_record gate(破損行・未知 kind・
        非 object はここで除外済み。集計側で再判定しない)
2. let window = route_window(&all, filters.since_ms, now)
      — route/route-refetch を --since のみで絞る
3. let route_rep = build_route_report(&window, filters.route.as_deref(),
                                      group_by == Some(Route))
      — scoped route 集合 + join + per-route + orphan/unkeyed
4. let records = filter_run_records(all, filters, now)
      — run record を since/worker/provider/model で絞る(従来通り。
        filters.route は読まない)
5. let agg = aggregate(&records)?  — 従来通り。明示 null fail-hard 維持
6. let mut report = report_map(records.len(), &agg)
   report.insert("route", route_rep.global)
   if group_by == Route  → report.insert("byRoute", rows)   // route 直後
   if group_by == Error  → report.insert("byError", build_by_error(&records))
7. estimated ブロック(従来通り — 最後に insert するため key 順は
   …route, byRoute|byError, estimated となり spec「route 直後・estimated
   の前・byRoute → byError 順」を満たす。`--group-by` は単値のため両者が
   同時に出ることはないが、順序は将来の additive 拡張に備えて固定)
```

`--route` が効くのは step 2→3 の route-scope のみ — run 集計(step 4-5)
と `byError`(step 6)には一切効かない。`--group-by error` は step 6 の
`build_by_error` のみを有効化し、route-scope には効かない
(`--route` + `--group-by error` は合法で、route bucket は scoped、
byError は無 scoped のまま — spec Req 5)。

### 4. CLI — `args.rs` / `cli.rs` / `execute_usage_inner`

- `STRING_OPTIONS`(:343-378): `"group-by"`・`"route"` を `"compare-model"`
  の直後(usage option cluster)へ追加。
- `Values`(:387-420): `group_by: Option<String>`・`route: Option<String>`
  field + `Default` + `set_string`(:456-488)の arm
  (`"group-by" => self.group_by = Some(value)`、`"route" => self.route =
  Some(value)` — 既存の last-wins 規約に従う)。
- `UsageCliArgs`(:241-252):
  - `pub group_by: Option<String>` — 生文字列。値検証は cli 層
    (`--since`/`--model` と同じ分担。args 層は enum を持たない)。
  - `pub route: Option<String>` — route 名。空文字列を含め検証しない
    (一致 route が無ければ scoped 集合が空になるだけ)。
- `usage_only_option_used`(:759):
  `values.since.is_some() || values.compare_model.is_some() ||
   values.group_by.is_some() || values.route.is_some()`。
  error message(:945)を `"--since/--compare-model/--group-by/--route
  are only valid for \`usage\`"` へ更新(列挙形式を維持)。
- guard の発火位置(:943-948)は変えない — `route` command arm(:771-885)
  が先に評価されるため、`cheaper route resolve --route x` は既存の
  per-action allowlist が `"--route is not valid for \`route resolve\`"`
  で reject し、`run`/`count-tokens`/`models`/`auth` 等では usage-only
  ガードが reject する(silent drop 防止規約の維持)。
- usage arm(:894-941): `UsageCliArgs` 組立てに `group_by`・`route` を
  追加するのみ(他 command への新 flag の reject は上記ガードが担う)。
- `execute_usage_inner`(:673-741): `--since` parse の直後に
  `--group-by` 検証を追加 —

  ```rust
  let group_by = match args.group_by.as_deref() {
      Some("route") => Some(UsageGroupBy::Route),
      Some("error") => Some(UsageGroupBy::Error),
      Some(v) => {
          return Err(invalid_request(
              format!("invalid --group-by: {v} (expected route|error)"),
              "groupBy",
              v,
          ))
      }
      None => None,
  };
  ```

  `route,error` のような複合値・未知値はここで INVALID_REQUEST になる
  (spec Req 5 — guide にも明記)。`UsageFilters` 組立てに
  `route: args.route.clone()` を追加し、`build_usage_report` へ
  `group_by` を渡す。
- `cli.rs` help(:595-603 付近、`--compare-model` 行の後ろ)に 2 行追加:

  ```
  --group-by <route|error>  usage: emit per-route / per-error-code sections
  --route <name>            usage: scope route aggregation to this route
  ```

### 5. Text format — `text_format.rs`

- `format_usage_report`(:341-416)の既存行は全て維持し、「avoided parent
  context」行の**直後**・`Estimated` block の前に route block を挿入する。
- **route section は常時描画**(spec Req 6 — 現行 parity gap の解消。
  `usage_command_format_text_writes_report` snapshot の更新を伴う想定内
  churn)。行 format(固定):

  ```
  route:         executions={e} failures={f} refetches={r} refetchBytes={b}
  route tokens:  original={o} returned={rt} postprocessFailures={pf}
  ```

  値は `num(get(&route, "..."))` で既存行と同じ number rendering。
- `result.route.orphanRefetches` が存在する場合(`--group-by route`
  指定時のみ JSON に載る)は追加行:

  ```
  route scope:   orphanRefetches={c} orphanRefetchBytes={b} unkeyedRecords={u}
  ```

- `result.byRoute` が存在する場合(`--group-by route` 指定時のみ —
  JSON が空 array を emit する規約に合わせ、**entry 0 件でも section
  header は無条件に描画**):

  ```
  byRoute:
    {route}: executions={e} ok={s} failed={f} tokens={o}->{rt} reduction={ratio} postprocessFailures={pf}
      refetches={rc} bytes={rb} exec/analyze/total={de}/{da}/{dt}ms unmeasurable={ut}/{ud}
  ```

- `result.byError` が存在する場合(同じく header は無条件):

  ```
  byError:
    {code}: count={c} shareOfRuns={s} shareOfFailures={sf} durationMs={d}
      workers: {name}={n} ...
      models: {p}/{id}={n} ...
      reasons: {reason}={n} ...
      execution: timeoutMs={{{v}:{n} ...}} maxTurns={...} maxToolCalls={...}
  ```

  空の inner map(workers/models/reasons)はその detail 行を省略し、
  `execution` は 3 subfield 全て空のとき行自体を省略する(text parity
  は「同じ集計結果が見える」こと — 空 map の表示は値を増やさない)。
- detection shim(`utility_text` :195-205)は**変更不要** —
  `result.records` number + `result.measured` object の検出条件は新
  field で変わらず、引き続き `format_usage_report` へ分岐する。
- JSON flag-gating との整合: byRoute/byError は `--group-by` 無しでは
  JSON key 自体が存在しないため、text 側は「key 存在 → 描画」の
  presence-driven 分岐で足り、CLI flag の再検査は不要(envelope の
  `result` だけを見る現行構造を維持)。

### 6. Backward compatibility

#### #192 consumer tolerance（writer/history 非依存）

- 将来の optional `executionId`/`parentExecutionId`/`traceId` は未知 field
  として受理して無視する。aggregate/group/filter/dedup の key にせず、
  同一 trace の別 run も両方計数する。run/route/refetch の cardinality は
  kind ごとに独立のまま、親子/trace を用いて統合しない。
- refetch が元 Route と異なる executionId を持っても、join は window 内
  の既存 `logRef` owner map のみを使う。first-wins と orphan 判定を維持し、
  window 外 owner を History から backfill しない。
- `build_usage_report` は History を探索/読取しない。History が無くても
  成立し、その event/skip/usage 前 failure を run 母数へ足さない。
  失敗率は recorded run に関する値であって全実行の失敗率ではない。
- `execution` は実効 policy echo
  `{timeoutMs,maxTurns,maxToolCalls}` であり `executionId` ではない。
  未知 optional ID や caller 提供 trace は相関ヒントでしかなく、
  grant/trust/実行権限など security policy の証明には決して使わない。
- ID 発行・伝搬・History store・適用可能な新規 Usage record の必須 ID
  連携は [#192](https://github.com/scarecrow173/cheaper/issues/192) が所有する。
  この read-side 設計では ID generator/形式/store を導入・選択しない。
  `usage --execution-id` も #192 の optional candidate であり実装しない。

- **`--since` が `result.route` に効くようになる意味論変更** — 既存
  field の値が変わる(filter が従来無視されていた bug fix)。field
  shape・key 順は不変。guide に「`--since` は route bucket にも適用さ
  れる」と明記する。
- **新規 key は全て flag-gated additive** — `byRoute`/`byError`/
  `route.orphanRefetches`/`route.unkeyedRecords` は `--group-by` 指定時
  のみ emit。flag 無しの JSON は shape・値ともに不変で、regression
  golden `cli-usage-*.json` は再生成不要のまま green。
- text 側は route section 常時描画により
  `usage_command_format_text_writes_report` の snapshot が変わる
  (想定内の更新 — spec Req 6)。
- writer/record schema は無変更。`aggregate` の明示 `null` fail-hard・
  `read_usage_records` の破損行 skip・`is_record` gate・
  JS number 意味論(`plus`/`or_zero`/`truthy`/非有限 → `null`)は
  そのまま。
- **二重計上なし** — run/route/refetch は kind で厳密分離され、route
  record の `worker.usage`・token field は run 母数に入らない(既存構造
  の維持)。refetch 計数は `route-refetch` record の `logRef` join が
  唯一の計数源で、route record の `refetches` snapshot field は
  集計しない(従来通り)。同一 logRef の複数 owner は first-wins で
  1 route に帰属する。

### 7. Test outline — `tests/integration/usage_report.rs`

#192 境界の consumer tolerance は test-spec「#192 consumer tolerance」の
5 test (`usage_optional_ids_preserve_report`・
`usage_same_trace_distinct_runs_counted`・
`usage_refetch_distinct_execution_id_joins_log_ref`・
`usage_window_orphan_not_backfilled_from_history`・
`usage_report_history_independent`) を追加する。既存 harness の Usage fixture
のみを操作し、History schema/writer や ID 発行/妥当性/連携を検証しない。

既存の `write_log`/`run_record`/`count_record`/`now`/`f64_at` helper に
`route_record()`・`refetch_record()`・`run_error_record(code)` fixture
builder を追加する。`build_usage_report` の signature 変更に伴う既存
caller(`report()` helper・各 test)の更新は機械的追従
(`group_by: None` を挿す)。

| test(概名) | 固定する acceptance criterion |
| --- | --- |
| `route_bucket_applies_since` | `--since` window 内外の route/refetch を混ぜ、window 内のみが `result.route` に反映される(AC:「--since で route 集計が window 内のみ」) |
| `route_window_invalid_timestamps` | `--since` 指定時の route/refetch の欠損・非文字列・parse不能 `ts` は window に含めない。`--since` 無指定なら従来通り含める(既存 run filter と同じ window 規約) |
| `group_by_route_emits_per_route_rows` | 2 route 名 × 成否・token・durations・postprocess・join 済み refetch の各値と、route 名昇順・weighted `reductionRatio` を固定(AC:「--group-by route で route 名別に…出る」) |
| `group_by_error_emits_code_rows` | 複数 code + `error` 欠損/null code/非文字列 code の `"unknown"` 畳み込み、count 降順・code 昇順、`shareOfRuns`/`shareOfFailures`、workers/models/reasons/execution distinct-value maps(AC:「--group-by error で code 別件数/比率と worker/provider/model 別内訳」) |
| `empty_error_code_is_preserved` | `error.code:""` は仕様どおり空文字列 key として維持し、欠損/null/非文字列の `"unknown"` へ誤統合しない |
| `execution_number_keys_use_js_canonical_format` | `execution.timeoutMs:3` と `3.0` が同じ `"3"` key に集約され、null/boolean/string の key 表現も固定される。数値 key は `cheaper_core::json::number::format_number` を通る |
| `nonfinite_aggregates_serialize_as_null` | 非有限 aggregate 値は JSON `null` になり、JSON/text report で `NaN`/`Infinity` 表記を出さない |
| `route_filter_scopes_route_sections_only` | `--route X` で `result.route` が X のみ・`byRoute` が X のみ・run 集計(records/usage)が無影響・`--group-by error` の `byError` が無影響(AC:「--route で route 集計が限定される」) |
| `route_group_ignores_worker_filter` | `--group-by route --worker W` の run 集計は worker W に絞るが、route bucket と byRoute は `--worker` によらず route window 全体を集計する(spec Out of Scope: route record への worker filter 非適用) |
| `route_filter_without_group_by` | standalone `--route X` でも global route bucket を X に限定し、byRoute key は `--group-by route` がない限り emit しない |
| `unkeyed_route_records_counted` | `route` field 欠損/非文字列の record が unkeyed count に計数され、byRoute entry を作らない。global `executions` 等には --route 無指定時に計上し、`--route` 時は対象 route totals から除外。unkeyed count はどちらも window-wide |
| `orphan_and_joined_refetches` | join 成功 refetch・owner 不在・`logRef:""`・route 側 `logRef:""`・unkeyed route record の非空 logRef を参照する refetch を含む fixture で `orphanRefetches`/`refetches`/`byRoute[].refetches` が正しい |
| `unmeasurable_metric_families` | token 欠損 → `unmeasurable.tokens`、durationMs 非 object/subfield 欠損 → `unmeasurable.durations`、`worker:{}` は無計上。値は `or_zero` で和に残る(spec「計測不能」節) |
| `no_double_counting` | route token が `records`/`usage`/`measured` に混ざらない + route record の `refetches` snapshot field(例 99)を無視して record 行のみ計数(AC:「二重計上しない」) |
| `refetch_only_log` | route record 0 件で refetch のみ → `executions:0`、global `refetches` 計上、全件 orphan、`byRoute:[]` emit(AC:「refetch のみ」) |
| `failure_only_and_empty_log` | error record のみ → `byError` rows;record 0 件/flag 指定 → `byRoute:[]`/`byError:[]` が emit される(AC:「0 件・失敗のみ」) |
| `fake_records_aggregate` | fixture で捏造した `is_record` 合格 record(route・run error)が通常集計される(AC:「fake record で正しい表示」) |
| `legacy_records_use_defaults` | optional field 欠損の旧 schema record で `or_zero` 既定値 + 対応する `unmeasurable` 計数(AC:「missing/legacy」) |
| `group_by_and_route_flag_validation` | `--group-by route,error`/`bogus` → `INVALID_REQUEST`("invalid --group-by: …")、`count-tokens --route x`・`run --group-by route` → usage-only ガードの message、`route resolve --route x` → `"--route is not valid for \`route resolve\`"`(AC/Req 5) |
| `args_usage_only_guard_messages` | `count-tokens --route x` と `run --group-by route` の各失敗について `integration/args.rs` で exact guard message を固定する |
| `usage_log_report_callers` | `integration/usage_log.rs` 内の usage report builder 全 caller を inventory し、新しい group/route 引数を正しく渡して各 caller の既存出力契約を保つ |
| `format_text_parity` | `--format text` で route section が常時出る + `--group-by` で byRoute/byError section が同じ値で出る + `usage_command_format_text_writes_report` snapshot 更新(AC:「text/JSON parity」) |
| `argv_and_error_detail_not_leaked` | `executed.argv` に secret 文字列・`error.message`/`schemaErrors` を含む record を置き、report 文字列化にそれらが現れない(AC:「sensitive な argv/prompt/error detail を漏出させない」) |
| `existing_contract_unchanged` | 既存 test 群(filter・aggregate・null fail-hard・estimated・golden)が無変更で green — flag 無し出力の shape/値不変(AC:「既存フィルタの後方互換・golden 無変更」) |

### 8. Doc-sync list

| file | 変更内容 |
| --- | --- |
| `docs/guides/usage.md` | 「集計対象は `kind:"run"` record のみ」(:375)の drift 修正 + `route` bucket の説明、`--group-by route\|error`・`--route` の用法、`--since` が `result.route` にも効く意味論変更、`--group-by` は単値(`route,error` は INVALID_REQUEST)、期間表記は `168h`/`24h`、`shareOfRuns`/`shareOfFailures`/`measured`/`comparable` の母数の違いと actual/measured/estimated の区別、`orphanRefetches`/`unkeyedRecords`/`unmeasurable` の用語 |

## Data and Control Flow

1. `cheaper usage [--since D] [--worker W] [--provider P] [--model M]
   [--route N] [--group-by route|error] [--compare-model M2]`
   → `parse_cli` が `UsageCliArgs`(新 field 含む)を構築。
2. `execute_usage_inner` → `--since`/`--model`/`--compare-model`/
   `--group-by` を検証(不正は INVALID_REQUEST)→ config dir 解決 →
   `UsageFilters` + `group_by` を `build_usage_report` へ。
3. `build_usage_report` → §3 の 7 step で `result` Map を組み立て、
   `utility_success_envelope` が `result` に埋める。`--format text` は
   dispatch の `write_output` → `utility_text` → `format_usage_report`
   で section を描画。

## Non-Functional Constraints

- 全量 `Vec<Value>` 展開を維持(append-only telemetry の現実的規模で
  streaming は YAGNI — ADR O3 不採択と同じ判断)。
- serde_json `Map` の挿入順 = JSON key 順の規約に従い、新規 key は
  固定位置(`route` 直後)・固定 subfield 順・`BTreeMap` 由来の昇順で
  deterministic にする(golden/snapshot 安定)。
- `build_by_error`・`build_route_report` は `CheaperError` を返さない
  — grouped section は fail-soft(構造的 invalid → skip+計数、
  malformed scalar → `or_zero`+`unmeasurable`)で、`aggregate` の
  fail-hard 経路を新 section に持ち込まない(spec Req 7)。
- `executed.argv`・`error.message` 等の sensitive field を group key・
  出力へ出さない — 読む field は §2 の allowlist で固定。

## Risks and Trade-offs

- **`result.route` の値が `--since` 指定時に変わる** — bug fix だが
  observable な意味論変更。対策: guide 明記 + flag-gated field のみで
  regression golden を壊さない(spec固定)。
- **`--route` 指定時の unkeyed/orphan の取り扱い** — spec は
  orphanRefetches と unkeyedRecords の両方を window-wide と明示する。
  scoped 集合から unkeyed を数えると `--route` 指定時に常 0 となるため、
  unkeyed は別走査で route window 全体から計数する(§2.2-4)。
- **同一 `logRef` の複数 owner** — degenerate だが fake record で起き
  得る。first-wins で帰属先を一意にして二重計上を構造で防ぐ
  (「どちらの route にも count しない」案は join 済み refetch を
  orphan に見せかけ、真の観測を歪めるため不採用)。
- **空文字列 `error.code`** — spec の畳み込み条件に空文字列がないため
  `""` bucket として残る。writer は空 code を書かず、fake record でしか
  発生しない — spec を amend せず厳密適用を選ぶ。
- **text 行 format は本 design で固定** — spec は section 存在と値の
  parity までで行書式を未指定。`byRoute`/`byError` の入れ子構造は
  header + インデント detail 行で可読性を優先し、空 map 行の省略で
  冗長さを抑える。

## Deferred Design Concerns

- streaming/bounded aggregation への移行 | usage.jsonl が現実的に
  小規模(append-only telemetry)で全量 `Vec<Value>` 展開が問題になら
  ない | 運用実測で file サイズ・parse 時間が閾値を超えたとき |
  長期運用で集計応答・メモリが劣化しうる。
- `--worker`/`--provider`/`--model` の route record(`worker.model`)
  への拡張適用 | run と route で `worker` の意味が違い混同を避ける |
  route の analyze model を絞りたい需要の確認 | route を model 軸で
  分析できない(別軸 flag 追加で後追い可能)。
- `d`/`w` duration suffix の受理 | `DURATION_PATTERN` は policy JSON
  schema と共有で波及面が広い | `--since 7d` のような自然な期間表記の
  要求が上がったとき | 利用者が `168h` への換算を強いられる。
- `byRoute`/`byError` 同時 emit(`--group-by` 複数値) | spec は単値を
  固定し需要未確認 | `route,error` 等の複合値指定の需要確認 | 現状は
  INVALID_REQUEST(後追いで additive 拡張可能)。
- `--route` で scoped な `unkeyedRecords` の副計数(scoped 集合内の
  unkeyed)を出すか | window-wide 計数で現要件を満たす | scoped 内訳の
  需要確認 | unkeyed record が route 絞り込み下で見えにくい。

## Verification Notes

- [ ] `cheaper usage --since <期間>` で route/route-refetch 集計が
  window 内 record のみを反映する(fixture test)。
- [ ] `--group-by route`/`--group-by error`/`--route` が spec「出力
  contract」節の shape・emit 規則・sort 規則で出る(fixture test)。
- [ ] malformed 行・旧 schema・0 件・refetch のみ・失敗のみ・orphan
  refetch・fake record の各 fixture が正しい表示を返す。
- [ ] `--format text` と JSON で同一の集計結果が出る。flag 無しの
  `cli-usage-*.json` golden が無変更で green、text snapshot は想定内
  更新。
- [ ] group key・出力に `executed.argv`・平文 command・生の
  errorMessage/message が現れない。
- [ ] `mise run check` が pass する。
