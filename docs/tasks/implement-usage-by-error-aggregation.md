---
id: "TASK-034MbTk4xoYlrywa6aUQja"
type: "task"
status: "done"
title: "Implement usage byError aggregation and canonical execution value keys (issue #184 T3)"
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
    - "docs/impl/ir/implement-usage-byerror-aggregation-issue-184-t3.md"
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
  blocks:
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

# Implement usage byError aggregation and canonical execution value keys (issue #184 T3)

## Work

T1 の error 系 pin test を green にする `byError` 集計を
`usage_report.rs` に実装する — `ErrorAgg`・`build_by_error`・
`build_usage_report` への `byError` emit 配線(design §1・§2.3・§3)。
`build_by_error` 自体は `build_route_report` と独立だが、
`build_usage_report` の emit 配線(design §3 — `route` 直後の
`byRoute` → `byError` 固定順)と `tests/integration/
usage_report.rs` の pin test 編集は T2 と同一箇所に当たるため、
本 task は T2 完了後に開始する(並行作業は merge conflict を
確実に引く — plan の T1→T2→T3→T4→T5→T6 chain 参照)。
module doc-comment への `build_by_error` 追記は本 task で行う。

1. `build_by_error(records: &[Value]) -> Vec<Map<String, Value>>`
   (design §2.3):
   - 入力は `filter_run_records` 済み run 集合(`aggregate` と同じ
     `records` を共有 — share 分母の一貫性)。`--route` は効かない
     (run filter に `route` を含めない)。
   - `errors = records.iter().filter(|r| !truthy(get(r,"ok")))` を
     母数とし、`error.code` で group 化:
     - `r.error` が object かつ `error.code` が文字列 → その値
       (空文字列 `""` はそのまま bucket 名 — spec の畳み込み条件は
       欠損/null/非文字列のみで空文字列を含まない厳密適用)。
     - それ以外(error 欠損/非 object・code 欠損/null/非文字列)
       → `"unknown"` bucket。
   - per-record fold: `count += 1`・`duration_ms += or_zero(r
     .durationMs)`・`r.worker` が文字列 → `workers[worker] += 1`・
     `model.provider` と `model.id` が両方文字列 → `models["{p}/
     {id}"] += 1`・`error.reason` が文字列 → `reasons[reason] += 1`・
     `execution` が object → 3 subfield 固定で存在する各値を
     canonical key 化して `exec_*[key] += 1`。
   - canonical key: Null→`"null"` / Number→
     `cheaper_core::json::number::format_number` による canonical
     JavaScript number text(`3` と `3.0` が `"3"` に集約)/
     Bool→`"true"`|`"false"` / String→その文字列 / その他
     (array/object)→ その subfield entry を skip。
   - 内訳 map は `BTreeMap` で key 昇順を構造保証する。
2. emit: `count` 降順・同数 `code` 昇順に sort してから rows を
   組み立てる(sort は emit 直前)。row は spec 固定 field 順
   `{code, count, shareOfRuns, shareOfFailures, workers, models,
   durationMs, reasons, execution}`。
   - `shareOfRuns = count / records.len()`、`shareOfFailures =
     count / errors.len()` — `num()` 経由で分母 0・非有限 →
     `null` を uniform に適用。
   - `execution` は常に 3 subfield を持つ(空でも `{}`)。
   - error 0 件でも `byError: []` を emit(flag 指定済み前提)。
   - `error.message`/`schemaErrors`/`checkpoint`/`errorMessage`・
     path 系・`executed.argv`・生 prompt は集計・出力しない
     (spec Req 4 allowlist — 読む field は design §2.3 のみ)。
3. `build_usage_report` で `group_by == Some(Error)` のとき
   `byError` rows を `route` 直後・`estimated` の前へ insert
   (design §3 の key 順 — `byRoute` → `byError` 順を固定。
   単値 flag のため同時には出ない)。
4. T1 の error 系 pin test を green にする。対象:
   `group_by_error_emits_code_rows`・`empty_error_code_is_preserved`・
   `execution_number_keys_use_js_canonical_format`・
   `nonfinite_aggregates_serialize_as_null`・
   `failure_only_log_emits_by_error_rows`・
   `empty_log_emits_empty_grouped_sections`(`byRoute:[]` 半分は
   T2 landed 済みの emit を検証)・`fake_run_error_records_aggregate`・
   `argv_and_error_detail_not_leaked`。args/text/dispatch 系の残
   pin test(`route_flag_rejected_for_route_resolve`・
   `args_usage_only_guard_messages`・
   `invalid_group_by_value_rejected_at_dispatch`・
   `route_filter_leaves_by_error_unscoped`・
   `route_flag_scopes_route_at_dispatch`・
   `group_by_last_wins_emits_by_error_only`・
   `format_text_parity`)は T4 まで red のまま許容する。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T3 slice)
- docs/designs/usage-grouped-route-error-analysis-issue-184.md (§1 ErrorAgg/struct・§2.3 集計・§3 emit 順・Non-Functional Constraints)
- docs/specs/usage-analysis-scope-issue-184.md (Req 4・7-8、出力 contract `--group-by error` 節)

## Dependencies

- docs/tasks/pin-usage-route-error-contract-tests.md (T1 の seam・fixture builder・pin test)
- docs/tasks/implement-usage-route-window-and-by-route.md (T2 —
  `build_usage_report` の emit 配線 tail と同一 test file を共有
  するため serialized — `empty_log_emits_empty_grouped_sections`
  の `byRoute:[]` 半分も T2 の emit に依存)

## Affected Files

- `crates/cheaper-cli/src/usage_report.rs`
- `crates/cheaper-cli/tests/integration/usage_report.rs`(pin test の red→green 追従のみ)

対象外: `route_window`/`build_route_report`(T2)、`args.rs`/
`cli.rs`/`text_format.rs`・`execute_usage_inner` の group-by 検証
(T4)、`docs/guides/usage.md`(T4)。

## Done When

- [x] `build_by_error` が `!truthy(ok)` run subset を `error.code`
      別に集計し、spec 固定 field 順・`count` 降順/`code` 昇順
      sort・share 分母規約・canonical execution key・`"unknown"`
      畳み込み・`""` 保持を実装する
- [x] `byError` rows が `route` 直後・`estimated` の前へ emit され、
      error 0 件でも `byError: []` が出る
- [x] privacy allowlist 外の field(`error.message`・`schemaErrors`・
      `checkpoint`・`errorMessage`・`executed.argv` 等)が集計 key・
      出力に使われない
- [x] T1 の error 系 pin test が全て green
- [x] flag 無し出力の shape/値が不変(既存 regression golden 無変更
      で green)

## Verification

- [x] T3 所有 pins 8件と `usage_same_trace_distinct_runs_counted` を個別実行し、
      全て 1 passed / 0 failed
- [x] module 全体は 60 passed / 5 failed。残りは T4 所有の
      `invalid_group_by_value_rejected_at_dispatch`,
      `route_filter_leaves_by_error_unscoped`,
      `route_flag_scopes_route_at_dispatch`,
      `group_by_last_wins_emits_by_error_only`, `format_text_parity`
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] regression golden 61/61 pass、regression directory に diff なし
- [x] independent cooperative/adversarial review: NO CANDIDATES
