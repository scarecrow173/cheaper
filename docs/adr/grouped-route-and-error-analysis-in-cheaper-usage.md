---
id: "ADR-034M0QWL9LflaeyWc8vho9"
type: "adr"
status: "accepted"
title: "Grouped route and error analysis in cheaper usage"
created: "2026-10-25"
updated: "2026-10-25"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
    - "../discovery/usage-analysis-audit-issue-184.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/usage-analysis-scope-issue-184.md"
  implemented-by:
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/usage-analysis-scope-issue-184.md"
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/usage-analysis-audit-issue-184.md"
    - "../specs/usage-analysis-scope-issue-184.md"
  derived-by:
    - "../designs/usage-grouped-route-error-analysis-issue-184.md"
  verifies: []
  verified-by: []
  references:
    - "../adr/usage-jsonl-contract-canonical.md"
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
    - "../adr/usage-error-record-field-contract.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../specs/usage-and-savings-telemetry-aggregation.md"
    - "../specs/run-error-record-observability.md"
    - "docs/guides/usage.md"
    - "crates/cheaper-cli/src/usage_report.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Grouped route and error analysis in cheaper usage

## Context and Problem Statement

issue #184 は `cheaper usage` の集計拡張を要求する。
`docs/discovery/usage-analysis-audit-issue-184.md` の監査で確定した
前提:

- `build_usage_report`(usage_report.rs:551-556)が
  `route_bucket(&all)` を `filter_run_records` **より先**に全量へ
  掛けるため、`--since` が `route`/`route-refetch` bucket に効かない
  (既知 bug)。
- route record には per-route 分析に必要な field(`route`・
  `commandHash`・`originalTokens`/`returnedTokens`/`reductionRatio`・
  `durationMs{exec,analyze,total}`・`logRef`・`postprocessFailures`・
  `exitCode`・`ok`)が、run error record には `error.code` と
  `execution` echo が既に揃っている — **writer/schema 変更なしに
  read-only 集計の拡張で完結する**。
- 反面、`executed.argv` は平文 secret 含有可、route record の
  `worker` は object(run の worker 名文字列と別意味)、
  `parse_duration` は `ms|s|m|h` のみ受理、`--format text` は route
  bucket を描画しない現行 parity gap、明示 `null` usage/savings での
  report 全体 fail(JS-parity 契約)という制約が残る。

## Decision Drivers

- issue の受入条件を壊さず満たすこと(期間・route 名・error 分類の
  集計、二重計上なし、欠損耐性、text/JSON 契約、sensitive field の
  非漏出)。
- 既存 `cheaper usage` の report field・filter 意味論・JS parity
  契約(null fail-hard・NaN/Infinity → `null`)の後方互換。
- additive-only 原則 — record schema・report field ともに新規追加
  は retract 困難な durable contract になるため命名を慎重に。
- KISS/YAGNI — 現実的な usage.jsonl 規模(append-only telemetry)
  に見合う最小の構造。

## Considered Options

- O1: `--since` fix のみ — `route_bucket` へ期間 filter 済み集合を
  渡す最小修正。group-by・per-route・error 分析は行わない。
- O2: `--since` fix + additive `--group-by route|error` +
  `--route <name>` filter + sectioned text/JSON output — record
  schema 無変更で集計・flag・text section を additive に拡張。
- O3: O2 + streaming/bounded aggregation への再設計 — 全量
  `Vec<Value>` 展開を逐次集計へ置き換える。

## Decision Outcome

Chosen option: "O2"。理由:

- O1 は issue の scope 2-4(per-route 分析・error.code 分析・
  `--group-by`・text parity)を満たさず、issue を close できない。
- O2 は監査で確認した通り record 側に必要 field が揃っており、
  writer 無変更・read-only 集計の additive 拡張で全 scope を
  満たせる。既存 field・filter の後方互換を保ち、二重計上は
  kind 分離 + refetch の `logRef` join 単一計数源規則で防げる。
- O3 の streaming 化は現実的 file 規模(1 行/record の
  append-only telemetry)に対し YAGNI で、JS 意味論
  (`plus`/`truthy`/`or_zero`)の再実装による parity drift 面を
  増やすだけ。issue 自身も「測定し、必要なら段階的に」と条件付き
  — 移行トリガの実測閾値を spec に残して defer する。

### Consequences

- 良い: `--since` が route/refetch に一貫して効く。route 名・
  error.code 軸の分析が CLI で可能になり、#141 型の失敗調査が
  恒常化する。`executed.argv` 非漏出・二重計上なしが contract で
  固定される。
- 決定済みの意味論(正本は spec「出力 contract」節):
  `--route` は route-scope 集計(`byRoute` entry と既存
  `result.route` global bucket の両方)のみを絞り、run 集計と
  `byError` には効かない(run/route record は集計 key を共有
  しない。`--route` + `--group-by error` は合法)。`--group-by`
  は単値 last-wins で `route,error` 等の複合値・未知値は
  INVALID_REQUEST。`error.code` 欠損/null/非文字列・`error`
  object 欠損は `"unknown"` bucket。`logRef` 空/欠損の route
  record は join 対象外、同様の `route-refetch` は
  `result.route.orphanRefetches`(window 全体で判定)。計測不能は
  `unmeasurable{tokens, durations}` の metric family 別計数で、
  `worker:{}`(analyze 未実行の正常形)は計上しない。新 `result`
  field は全て flag-gated emit で既存 regression golden を変えず、
  `--format text` は route section を常時描画する。
- 悪い/許容: `parse_duration` は `d`/`w` を受理しないまま
  (policy schema 共有の `DURATION_PATTERN` に波及させないため、
  guide で `168h` 表記を案内。先行 spec
  `usage-and-savings-telemetry-aggregation.md` の `7d` 記述は
  未実装の既知不正確記述)。`--since` の route 適用は既存 field
  `result.route` の**値を変える意味論変更**(従来無視されていた
  filter が効くようになる)で、純粋な field 追加ではない —
  schema 互換は保つが挙動差分として guide に明記する。
  `--worker`/`--provider`/`--model` は run scoped のままで、route
  の analyze model 絞り込みは後追い flag に委ねる。新 report
  field 名は durable contract になるため spec「出力 contract」節
  で固定する。
- 既存の明示 `null` usage/savings での report fail-hard は run 集計
  に限定して維持し、grouped section では malformed entry を
  二段ルール(構造的 invalid は skip+`unkeyedRecords` 等で計数、
  otherwise-valid record 内の malformed scalar は `or_zero` 加算+
  `unmeasurable` family 計数)で扱う(既存契約と fixed test を
  壊さない)。

### Confirmation

- `crates/cheaper-cli/tests/integration/usage_report.rs` の fixture
  test で `--since` 反映・per-route/error 集計・二重計上なし・
  境界値を固定し、`mise run check` pass を確認する。

## Implementation Plan

- Affected paths: `crates/cheaper-cli/src/usage_report.rs`
  (route bucket の期間 filter・per-route/error 集計)、
  `crates/cheaper-cli/src/args.rs`(`--group-by`/`--route` option・
  UsageCliArgs・usage-only ガード編入)、
  `crates/cheaper-cli/src/text_format.rs`(新 section の text
  rendering)、`crates/cheaper-cli/src/cli.rs`(help 行)、
  `crates/cheaper-cli/tests/integration/usage_report.rs`、
  `docs/guides/usage.md`。
- Patterns to follow: `filter_run_records` の ts/`date_parse` 規約を
  route/refetch にも適用、`is_record` gate の kind 分離を維持、
  serde_json `Value` の additive field、`Map` insertion order を
  report literal 順に合わせる、best-effort 非破壊の read-only 集計。
- Patterns to avoid: run 集計への route token 混入、route record の
  `refetches` snapshot field を計数源にすること、`executed.argv`・
  生 prompt/errorMessage の出力・group key 化、`DURATION_PATTERN`
  の共有 pattern への独自 suffix 追加、`read_usage_records` の
  破損行 skip 規約の変更。
- Dependencies or configuration: 新規依存なし。record writer・
  schema は無変更。

## Verification

- [ ] `--since` が route/route-refetch 集計に効く(fixture test)。
- [ ] `--group-by route`/`--group-by error`/`--route` が集計を分岐
  し、未知値・他 command での使用は INVALID_REQUEST。
- [ ] 新旧 schema 混在・0 件・refetch のみ・失敗のみ・orphan
  refetch の fixture が正しい表示を返す。
- [ ] `--format text` と JSON で同一の集計結果・既存行の書式維持。
- [ ] `mise run check` pass。

## Pros and Cons of the Options

### O1: `--since` fix のみ

- Good, because headline bug を最小 diff で直し既存 test への影響が
  狭い。
- Bad, because issue scope の大半(per-route/error 分析・group-by・
  text parity)を満たせず issue を close できない。

### O2: additive grouped analysis(採択)

- Good, because issue scope 1-4・6 を record 無変更で網羅し、後方
  互換と additive-only 原則を保つ。実装面が既存の filter/aggregate
  構造の素直な延長になる。
- Bad, because aggregator + parser + text + test + guide の中規模 diff
  で、新 report field 名が durable contract になる点に設計慎重さが
  要る。

### O3: streaming/bounded aggregation 含む再設計

- Good, because 長期 log のメモリ上限を構造的に保証できる。
- Bad, because usage.jsonl は現実的に小規模で全量展開が問題にならず、
  JS 意味論の streaming 再実装が parity drift を招く(YAGNI)。
  実測閾値を spec に残して defer する。

## More Information

- 監査・採択空間の詳細: `docs/discovery/usage-analysis-audit-issue-184.md`。
- 要件・受入条件: `docs/specs/usage-analysis-scope-issue-184.md`。
- 関連する先行 contract(`references`): usage.jsonl contract・
  best-effort append・error record allowlist(#141)・route record
  (#149/#166)。
