---
id: "TASK-034MbSQgdIQfx2nMRZ0RCz"
type: "task"
status: "todo"
title: "Pin usage route/error contract tests — compile seam, fixture builders, args/report/caller tests (issue #184 T1)"
created: "2026-10-25"
updated: "2026-10-26"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/184"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  blocks:
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  references:
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
    - "docs/specs/usage-analysis-scope-issue-184.md"
  defers: []
  deferred-by: []
---

# Pin usage route/error contract tests — compile seam, fixture builders, args/report/caller tests (issue #184 T1)

## Work

spec「出力 contract」節と design §7 test outline の全 pinned test を、
実装に先立って `tests/integration/usage_report.rs`・`args.rs`・
`usage_log.rs` へ追加する TDD slice。contract test が compile できる
最小限の seam(新 API 面)だけを production code に入れ、動作は全て
未実装のまま残す — 本 task 終了時点で新規 pin test は expected-red、
既存 test 全体と caller 追従は green を維持する。

1. `crates/cheaper-cli/src/usage_report.rs` の compile seam(動作
   変更なし):
   - `pub enum UsageGroupBy { Route, Error }` を追加(design §1)。
   - `UsageFilters` に `pub route: Option<String>` を追加。
   - `build_usage_report` に `group_by: Option<UsageGroupBy>` 引数を
     追加するが、本 task では emit・filter へ一切使用しない
     (signature のみ先行 — 未使用は `_` prefix か同等で明示し
     clippy を通す)。
   - src 側の既存 caller(`execute_usage_inner` 等)を新 signature
     へ追従させ `None` を渡す — flag 無し出力の shape・値は不変。
2. `tests/integration/usage_report.rs`:
   - fixture builder `route_record()`・`refetch_record()`・
     `run_error_record(code)` を既存 `write_log`/`run_record`/
     `count_record`/`now`/`f64_at` と同じ style で追加する。
   - `report()` helper を始め既存 caller を `group_by: None` 追従へ
     機械的に更新する。
   - design §7 表の pin test を追加する(名称は概名の厳密実装を
     期待)。所有 task 別の割当は次の通り:
     - T2(route-scope):`route_bucket_applies_since`・
       `route_window_invalid_timestamps`・
       `group_by_route_emits_per_route_rows`・
       `route_filter_scopes_route_sections_only`(主張範囲は
       route-scope と run 集計の分離のみ — `byError` 無影響の
       句は T4 の `route_filter_leaves_by_error_unscoped` へ
       分離)・`route_group_ignores_worker_filter`・
       `route_filter_without_group_by`(scoped bucket に加えて
       flag-gated key — `orphanRefetches`/`unkeyedRecords`/
       `byRoute` — が `--group-by` 無指定で emit されないことも
       pin)・`unkeyed_route_records_counted`・
       `orphan_and_joined_refetches`・`unmeasurable_metric_families`・
       `no_double_counting`・`refetch_only_log`・
       `fake_records_aggregate`(fake route/`route-refetch`
       record のみ — run error fake は T3 の
       `fake_run_error_records_aggregate` へ)・
       `legacy_records_use_defaults`。
     - T3(byError):`group_by_error_emits_code_rows`・
       `empty_error_code_is_preserved`・
       `execution_number_keys_use_js_canonical_format`・
       `nonfinite_aggregates_serialize_as_null`・
       `failure_only_log_emits_by_error_rows`・
       `empty_log_emits_empty_grouped_sections`(`byRoute:[]`・
       `byError:[]` 両方 — T2 の `byRoute` emit が先に landed
       している前提)・`fake_run_error_records_aggregate`・
       `argv_and_error_detail_not_leaked`。
     - T4(CLI/dispatch/text):`route_flag_rejected_for_route_resolve`・
       `args_usage_only_guard_messages`(args.rs — 後述)・
       `invalid_group_by_value_rejected_at_dispatch`・
       `route_filter_leaves_by_error_unscoped`・
       `route_flag_scopes_route_at_dispatch`・
       `group_by_last_wins_emits_by_error_only`・
       `format_text_parity`。dispatch 系は既存 `run_usage`/
       `CliIo` harness(`usage_report.rs` test 内)を使う —
       `invalid_group_by_value_rejected_at_dispatch` は args 層が
       生文字列を保持して `parse_cli_args` が `Ok` を返すため
       args.rs では観測できず、dispatch 経路でのみ検証できる。
   - fixture JSONL shape は spec の fixture matrix(valid・
     mixed-legacy・malformed・fake・empty・refetch-only・
     failure-only)を網羅する — malformed 行・欠損/非文字列 `ts`・
     `logRef:""`・route 側 `logRef:""`・unkeyed route record を
     参照する refetch・同一 `logRef` の複数 owner・
     `executed.argv` 内 secret 文字列・`error.message`/
     `schemaErrors` 含行を含める。
3. `tests/integration/args.rs`:
   - `route_flag_rejected_for_route_resolve` — `route resolve
     --route x` → per-action allowlist の `"--route is not valid
     for \`route resolve\`"`。
   - `args_usage_only_guard_messages` — `count-tokens --route x`・
     `run --group-by route` の usage-only guard exact message
     (`"--since/--compare-model/--group-by/--route are only valid
     for \`usage\`"` の列挙形式)。
   - これらは T4 の `STRING_OPTIONS` 登録前は exact message 不一致
     ではなく `Unknown option '--route'`/`'--group-by'` の parse
     error で red(option 未登録が失敗理由の観測 — T4 で flag
     登録・guard 編入が入って初めて期待 message へ到達する)。
4. `tests/integration/usage_log.rs`: `usage_log_report_callers` を
   追加し、`build_usage_report` を呼ぶ既存 caller を棚卸しして、
   `group_by` 引数追従後も各 caller の既存出力契約が保たれること
   を検証する(この test は T1 時点で green になる)。
5. `tests/integration/main.rs`: 既存 module(usage_report/args/
   usage_log)への追加を原則とし、新 module が必要な場合のみ宣言
   する。

## #192 consumer tolerance 追加（2026-10-26、改訂レビュー・承認済み）

T1 の既存 Usage fixture/harness を用いて次を pin する（test-spec の同名
Rules が詳細契約）。新しい History schema/writer や ID generator は作らない。

- `usage_optional_ids_preserve_report`: optional executionId/
  parentExecutionId/traceId の有無で aggregate/group/filter/refetch と
  JSON/text 出力が同一（flag 無し、route/error group、既存 filter を含む）。
- `usage_same_trace_distinct_runs_counted`: 同一 trace の別 run は両方計数。
  success/failure 各 1 の recorded-run 母数と失敗割合を固定。
- `usage_refetch_distinct_execution_id_joins_log_ref`: 元 Route と別 ID の
  refetch が window 内の logRef で join し、kind 間件数は統合しない。
- `usage_window_orphan_not_backfilled_from_history`: window 外 owner は
  対応 History 相当の外部情報があっても orphan のまま。
- `usage_report_history_independent`: History 不在で report 成功、
  外部情報の追加でも不変。History records は集計入力ではない。

後半 2 test は既存 temp directory 内に必要なら inert な無関係の sentinel
を置き、将来の History file path/schema を仮定しない。これは #192 の ID
妥当性/発行/伝搬/必須 Usage 連携 test ではなく consumer tolerance だけ。
T1 で pin し T2/T3 の集計で関連保証を満たし、T4 の CLI/text 完成後に
全組合せを green にする。既存挙動で既に満たす tolerance test は T1 から
green でよい。新契約全体の expected-red window は T4 まで維持し、
6 task の直列順・新規依存なし・T5 Usage-only 計測を変更しない。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T1 slice)
- docs/designs/usage-grouped-route-error-analysis-issue-184.md (§1 seam 構造・§7 test outline)
- docs/specs/usage-analysis-scope-issue-184.md (出力 contract・Req 5・Req 7)
- docs/test-specs/usage-route-error-analysis-contract-issue-184.md (Rules の網羅対象)

## Dependencies

- docs/plans/implement-usage-route-error-analysis-issue-184.md

## Affected Files

- `crates/cheaper-cli/src/usage_report.rs`(seam のみ —
  `UsageGroupBy`・`UsageFilters.route`・`build_usage_report`
  signature + caller 追従。集計動作は変更しない)
- `crates/cheaper-cli/tests/integration/usage_report.rs`
- `crates/cheaper-cli/tests/integration/args.rs`
- `crates/cheaper-cli/tests/integration/usage_log.rs`
- `crates/cheaper-cli/tests/integration/main.rs`(新 module を追加
  する場合のみ)

対象外: `route_window`/`build_route_report`/`build_by_error` 本体
(T2/T3)、`args.rs`/`cli.rs`/`text_format.rs` の flag・handoff・
format 実装(T4)、`docs/guides/usage.md`(T4)。

## Done When

- [ ] consumer tolerance 5 test が追加され、T1 時点の green/expected-red と
      その理由を記録。History schema/writer・ID validation/mint/linkage
      を実装・要求していない

- [ ] `UsageGroupBy` enum・`UsageFilters.route`・`build_usage_report`
      の `group_by` 引数が compile-only seam として追加され、src/
      test の全既存 caller が `group_by: None` へ追従している
- [ ] fixture builder 3 種と design §7 表の pin test が追加され、
      未実装動作を要求する test が期待どおり red になる(失敗名と
      理由を本 task に記録する)
- [ ] `usage_log_report_callers` と既存 test 群(filter・
      aggregate・estimated・regression golden `cli-usage-*.json`)
      が無変更で green のまま
- [ ] `route_window`/`build_route_report`/`build_by_error`・args
      登録・text format の実装が本 task に含まれない

## Verification

- [ ] `mise exec -- cargo build -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0(`usage_log_report_callers` green)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` の出力で新規 pin test 名が FAILED として列挙され、既存 test が全て pass する(expected red — 失敗名と理由を本 task へ記録。`invalid_group_by_value_rejected_at_dispatch`・`route_flag_scopes_route_at_dispatch`・`group_by_last_wins_emits_by_error_only` は `Unknown option '--group-by'`/`'--route'` の parse error、残りは未実装集計・emit の assertion failure)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` の出力で新規 pin(`route_flag_rejected_for_route_resolve`・`args_usage_only_guard_messages`)が `Unknown option '--route'`/`'--group-by'` の parse error で FAILED として列挙され、既存 test(`since_only_valid_for_usage` を含む)が全て pass する(expected red — 失敗名と理由を本 task へ記録)
- [ ] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0(未使用 seam 引数の warning なし)
