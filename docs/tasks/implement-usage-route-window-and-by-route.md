---
id: "TASK-034MbTdI8lEI1dRpWvsRQQ"
type: "task"
status: "done"
title: "Implement usage route window, --route scoping, logRef join, orphan/unkeyed diagnostics, and byRoute rows (issue #184 T2)"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  implemented-by:
    - "../impl/ir/implement-usage-route-window-and-byroute-aggregation-issue-184-t2.md"
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
  blocks:
    - "docs/tasks/implement-usage-by-error-aggregation.md"
    - "docs/tasks/wire-usage-group-by-flags-text-and-guide.md"
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

# Implement usage route window, --route scoping, logRef join, orphan/unkeyed diagnostics, and byRoute rows (issue #184 T2)

## Work

T1 の route-scope pin test を green にする集計実装を
`usage_report.rs` に入れる — `route_window`・`RouteAgg`・
`RouteReport`・`build_route_report`(`route_bucket` の後継)・
`build_usage_report` への組込み(design §1-§3)。T3 の `byError`・
T4 の CLI/text・guide は本 task に含めない。

1. `route_window(records, since_ms, now) -> RouteWindow` — `kind:
   "route"`・`"route-refetch"` を `--since` のみで絞る専用経路
   (design §1 の kind 分離責務、`filter_run_records` は拡張しない)。
   `ts` 欠損/非文字列/parse 不能は since 指定時のみ除外し、
   `date_parse` を run record と共有して同一 window 規約に揃える
   (design §2.1)。worker/provider/model filter は一切適用しない。
2. `build_route_report(window, route_name, group_by_route) ->
   RouteReport`(design §2.2 の手順をそのまま実装する):
   - `logRef` owner map を window 全体で構築 — `route` が文字列かつ
     `logRef` が非空文字列の record のみ(`HashMap<logRef,
     route名>`)。同一 `logRef` の複数 owner は file 順 first-wins
     (`.entry().or_insert()` — 二重帰属防止)。
   - refetch の join/orphan 振り分けは window 全体で判定し
     `--route` 絞り込みでは隠れない(非空 `logRef` かつ owner 在
     → `joined[owner]` に `count+=1`・`bytes+=or_zero(bytesRead)`、
     それ以外 → `orphan_count`/`orphan_bytes`)。
   - scoped route 集合(`route_name=Some(n)` なら `route` field が
     `Some(n)` に一致する record のみ — unkeyed は名前一致でき
     ないため常に scoped 外)。
   - per-record fold は `BTreeMap<String, RouteAgg>`(route 名昇順
     の構造保証)へ — `executions += 1`・`truthy(ok)` で
     succeeded/failed・`originalTokens`/`returnedTokens` は `or_zero`
     で和へ(いずれか欠損/非数値なら `unmeasurable_tokens += 1`)、
     `postprocessFailures` は `or_zero`、`durationMs` は 3 subfield
     それぞれ `or_zero` で `dur_*` へ(非 object・subfield 欠損/
     非数値なら `unmeasurable_durations += 1`)、`worker` field は
     一切読まない(`{}` は analyze 未実行の正常形)。
   - `unkeyedRecords` は window 全体の route record を別走査して
     計数(`--route` の有無で値を変えない)。
   - global `result.route` は既存 7 key 同順を維持し、
     `group_by_route=true` のときのみ末尾へ
     `orphanRefetches{count,bytes}`・`unkeyedRecords` を additive
     追加。`route_name=None` の global `refetches`/`refetchBytes`
     は window 内全 refetch、`Some(n)` では `joined[n]` のみ。
   - `byRoute` rows は route 名昇順・spec 固定 field 順・
     `reductionRatio = Σoriginal > 0 ? 1 - Σreturned/Σoriginal :
     0.0`(`num()` 経由で非有限 → `null`)・group 0 件でも
     `byRoute: []` emit。
3. `route_bucket`(:503-538)を `build_route_report` へ置き換え、
   `build_usage_report` を design §3 の実行順へ組み替える —
   `route_window(&all)` → `build_route_report` →
   `filter_run_records(all)` → `aggregate` → `report_map` →
   `route` insert → `group_by == Route` なら `byRoute` insert
   (`route` 直後・`estimated` の前)。`route_window` が `&all` を
   借りるため、`all` を move する `filter_run_records` より先に
   route-scope を完了させる borrow 順序を守る。
   - module doc-comment(`usage_report.rs` 冒頭の `//!` 関数
     inventory)を新構造へ更新する — `route_bucket` への言及を
     残さず、`route_window`/`build_route_report` の route-scope
     経路を記載する(T3 が `build_by_error` を追記するのは
     T3 の責務)。
4. T1 の route-scope pin test を green にする。対象:
   `route_bucket_applies_since`・`route_window_invalid_timestamps`・
   `group_by_route_emits_per_route_rows`・
   `route_filter_scopes_route_sections_only`(主張範囲は
   route-scope + run 集計分離のみ — `byError` 無影響の句は
   T4 の `route_filter_leaves_by_error_unscoped`)・
   `route_group_ignores_worker_filter`・
   `route_filter_without_group_by`(scoped bucket に加えて
   `orphanRefetches`/`unkeyedRecords`/`byRoute` key が
   `--group-by` 無指定で emit されない flag-gated 証明を含む)・
   `unkeyed_route_records_counted`・`orphan_and_joined_refetches`・
   `unmeasurable_metric_families`・`no_double_counting`・
   `refetch_only_log`・`fake_records_aggregate`(fake route/
   `route-refetch` record のみ — run error fake は T3 の
   `fake_run_error_records_aggregate`)・
   `legacy_records_use_defaults`。byError 系・args/text/dispatch 系
   の残 pin test(`group_by_error_emits_code_rows`・
   `empty_error_code_is_preserved`・
   `execution_number_keys_use_js_canonical_format`・
   `nonfinite_aggregates_serialize_as_null`・
   `failure_only_log_emits_by_error_rows`・
   `empty_log_emits_empty_grouped_sections`・
   `fake_run_error_records_aggregate`・
   `argv_and_error_detail_not_leaked`・
   `route_flag_rejected_for_route_resolve`・
   `args_usage_only_guard_messages`・
   `invalid_group_by_value_rejected_at_dispatch`・
   `route_filter_leaves_by_error_unscoped`・
   `route_flag_scopes_route_at_dispatch`・
   `group_by_last_wins_emits_by_error_only`・
   `format_text_parity`)は T3/T4 まで red のまま許容する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T2 slice)
- docs/designs/usage-grouped-route-error-analysis-issue-184.md (§1 module 構造・§2.1-2.2・§3 実行順)
- docs/specs/usage-analysis-scope-issue-184.md (Req 1-3・5・7-8、出力 contract `--group-by route` 節・計測不能・`--route` scoping 節)

## Dependencies

- docs/tasks/pin-usage-route-error-contract-tests.md (T1 の seam・fixture builder・pin test)

## Affected Files

- `crates/cheaper-cli/src/usage_report.rs`
- `crates/cheaper-cli/tests/integration/usage_report.rs`(pin test の red→green 追従のみ)

対象外: `build_by_error`(T3)、`args.rs`/`cli.rs`/`text_format.rs`・
`execute_usage_inner` の group-by 検証(T4)、`docs/guides/usage.md`
(T4)。

## Done When

- [x] `route_window` が kind 分離の専用経路として route/refetch を
      `--since` のみで絞る(run filter を拡張しない)
- [x] `build_route_report` が global bucket(7 key 同順 + flag 時の
      orphan/unkeyed 末尾追加)・`byRoute` rows(昇順・固定 field
      順・`[]` emit)・`logRef` join(first-wins)・orphan/unkeyed
      の window-wide 計数を実装する
- [x] `route_bucket` が `build_route_report` に置き換わり、
      `build_usage_report` が design §3 の順序で route-scope →
      run filter → aggregate を組み立てる
- [x] T1 の route-scope pin test が全て green
- [x] flag 無し出力の shape/値が不変(既存 regression golden 無変更
      で green)

## Verification

- [x] T2 所有 pins 13件と #192 join/window tolerance 2件を
      module filter 別に個別実行し、全て 1 passed / 0 failed
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::`
      で既存/T2 test が green、残り14失敗が T3/T4 pin のみであることを確認
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] regression golden 61/61 pass、`crates/cheaper-cli/tests/regression`
      に diff がない

## T2 implementation evidence

- route/window pins 13件と T1 の #192 join/window pins 2件を個別実行し、
  全て成功。format と clippy は warning-free。
- module report は 51 passed / 14 failed / 0 ignored。残りは T3 所有9件
  (`group_by_error_emits_code_rows`, `empty_error_code_is_preserved`,
  `execution_number_keys_use_js_canonical_format`,
  `nonfinite_aggregates_serialize_as_null`,
  `failure_only_log_emits_by_error_rows`,
  `empty_log_emits_empty_grouped_sections`,
  `fake_run_error_records_aggregate`, `argv_and_error_detail_not_leaked`,
  `usage_same_trace_distinct_runs_counted`) と T4 所有5件
  (`invalid_group_by_value_rejected_at_dispatch`,
  `route_filter_leaves_by_error_unscoped`,
  `route_flag_scopes_route_at_dispatch`,
  `group_by_last_wins_emits_by_error_only`, `format_text_parity`)。
- regression golden: 61/61 pass。capture 未設定時に golden diff なし。
- 独立 cooperative/adversarial review とも NO CANDIDATES。
