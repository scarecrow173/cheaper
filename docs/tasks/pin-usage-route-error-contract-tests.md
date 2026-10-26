---
id: "TASK-034MbSQgdIQfx2nMRZ0RCz"
type: "task"
status: "done"
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

- [x] consumer tolerance 5 test が追加され、T1 時点の green/expected-red と
      その理由を記録。History schema/writer・ID validation/mint/linkage
      を実装・要求していない

- [x] `UsageGroupBy` enum・`UsageFilters.route`・`build_usage_report`
      の `group_by` 引数が compile-only seam として追加され、src/
      test の全既存 caller が `group_by: None` へ追従している
- [x] fixture builder 3 種と design §7 表の pin test が追加され、
      未実装動作を要求する test が期待どおり red になる(失敗名と
      理由を本 task に記録する)
- [x] `usage_log_report_callers` と既存 test 群(filter・
      aggregate・estimated・regression golden `cli-usage-*.json`)
      が無変更で green のまま
- [x] `route_window`/`build_route_report`/`build_by_error`・args
      登録・text format の実装が本 task に含まれない

## Verification

- [x] `mise exec -- cargo build -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` exits 0(`usage_log_report_callers` green)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` の出力で新規 pin test 名が FAILED として列挙され、既存 test が全て pass する(expected red — 失敗名と理由を本 task へ記録。`invalid_group_by_value_rejected_at_dispatch`・`route_flag_scopes_route_at_dispatch`・`group_by_last_wins_emits_by_error_only` は `Unknown option '--group-by'`/`'--route'` の parse error、残りは未実装集計・emit の assertion failure)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` の出力で新規 pin(`route_flag_rejected_for_route_resolve`・`args_usage_only_guard_messages`)が `Unknown option '--route'`/`'--group-by'` の parse error で FAILED として列挙され、既存 test(`since_only_valid_for_usage` を含む)が全て pass する(expected red — 失敗名と理由を本 task へ記録)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0(未使用 seam 引数の warning なし)

## T1 implementation evidence

Implementation Record:
`docs/impl/ir/pin-usage-route-error-contract-tests-issue-184-t1.md`。
上の check は **T1 の expected-red 観測を含む完了条件を確認した**印であり、
新規契約の green・`mise run check` green を意味しない。
T2/T3/T4 の本体、CLI flag 登録、writer、History/ID 実装、Cargo/dependency、
既存 assertion と regression golden は変更していない。

### Routing / status

- 開始 HEAD `a3144f7`、branch `feature/usage-analysis-issue-184`、clean。
- 最初の route command は script の所在を `implementation-flow/scripts`
  と誤認し exit 1（MODULE_NOT_FOUND）。下記の正しい所在で再実行:
  `mise exec -- node .agents/skills/doc-driven-dev-graph/scripts/route_graph.js
  --current task-graph --focus docs/plans/implement-usage-route-error-analysis-issue-184.md
  --task-dir docs/tasks --cwd . --json` → exit 0。
  `edgeId: task-graph-to-implementation` / `condition: tasks-runnable` /
  `next: implementation`、issues/blockers は空、runnable は T1 のみ。
- Task は `in-progress`。この子エージェントの tool surface に built-in
  reviewer dispatch が無いため、独立レビューを親へ依頼する。
  自己点検を独立レビューとは扱わず、レビューゲートと最終 commit/closeout
  は親に引き継ぐ。T2 は開始しない。

### Actual verification

以下は全て `mise exec --` 経由、build/clippy/test は
`--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付き。

| command (`mise exec --` に続く部分) | exit | actual result |
| --- | --- | --- |
| `cargo fmt --all` | 0 | format 実施 |
| `cargo fmt --all --check` | 0 | format check |
| `cargo build -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | 0 | build 成功 |
| `cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` | 0 | warning なし |
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_log::` | 0 | 21 passed / 0 failed / 0 ignored / 315 filtered（既存20 + 新規1） |
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration usage_report::` | 101 | 36 passed / 29 failed / 0 ignored / 271 filtered（既存34全pass、新規2pass・29 expected-red） |
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration args::` | 101 | 45 passed / 2 failed / 0 ignored / 289 filtered（既存45全pass、新規2 expected-red） |
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test regression usage` | 0 | 0 tests / 1 filtered — 証明には使用しない |
| `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test regression -- --nocapture` | 0 | `regression_golden` 1 passed、cases=61 pass=61 fail=0 |

`CHEAPER_REGRESSION_CAPTURE` 未設定を確認して replay し、
`git diff --exit-code -- crates/cheaper-cli/tests/regression` も exit 0。
golden は再生成していない。`usage_command_format_text_writes_report` と
`since_only_valid_for_usage` の既存 assertion もこの T1 では変更せず pass。
`mise run check` / workspace 全体の test は実行していない。

文書・差分検証:

- `mise exec -- node .agents/skills/impl-doc/scripts/audit_impl_record.js
  --json`: exit 0、181 files / findings 0。
- `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type
  task --json`: exit 0、401 files / 143 non-blocking warnings /
  blocking 0。本 T1 への findings は 0（他 task の reciprocal relation 警告）。
- `mise exec -- node scripts/check-doc-ids.mjs`: exit 0、
  `doc id check OK: 1238 markdown files scanned`。
- `git diff --check`: exit 0。
- `git diff --quiet -- Cargo.toml Cargo.lock crates/cheaper-cli/Cargo.toml`:
  exit 0。

### Exact expected-red list (31 tests)

`usage_report::` の 29 件は下記。Null は absent section/row に対する
`Value` indexing の観測値であり、期待値を捏造して green にしていない。
owner は後続で green にする責任 task（T3 の一部は先行 T2 も必要）。

| owner | test | 最初の失敗 assertion / 原因 |
| --- | --- | --- |
| T2 | `route_bucket_applies_since` | executions 2 != 1、旧 bucket が window 外 route を含む |
| T2 | `route_window_invalid_timestamps` | executions 5 != 1、不正/欠損 ts の window 除外が未実装 |
| T2 | `group_by_route_emits_per_route_rows` | byRoute Null != 期待2 rows、emit 未実装 |
| T2 | `route_filter_scopes_route_sections_only` | executions 2 != 1、route filter 未使用 |
| T2 | `route_group_ignores_worker_filter` | byRoute[0].executions Null != 1、emit 未実装（run filter は通過） |
| T2 | `route_filter_without_group_by` | executions 2 != 1、standalone route filter 未実装 |
| T2 | `unkeyed_route_records_counted` | unkeyedRecords Null != 2、window-wide 診断未実装 |
| T2 | `orphan_and_joined_refetches` | orphanRefetches Null != {count:4,bytes:36}、join/orphan 未実装 |
| T2 | `unmeasurable_metric_families` | unmeasurable Null != {tokens:1,durations:1}、family 集計未実装 |
| T2 | `no_double_counting` | byRoute[0].refetches Null != {count:1,bytes:9}、run 母数/既存 snapshot 非混入 assertion は pass |
| T2 | `refetch_only_log` | orphanRefetches Null != {count:1,bytes:9}、global の既存計数 assertion は pass |
| T2 | `fake_records_aggregate` | byRoute[0].route Null != "invented"、reader の malformed/未知kind/非object除外 assertion は pass |
| T2 | `legacy_records_use_defaults` | byRoute[0].originalTokens Null != 0、row/default/unmeasurable 未実装 |
| T2 | `usage_refetch_distinct_execution_id_joins_log_ref` | byRoute[0].refetches Null != {count:1,bytes:9}、kind別件数 assertion は pass |
| T2 | `usage_window_orphan_not_backfilled_from_history` | executions 1 != 0、window 外 owner を除外しない（sentinel 前後の同一性は pass） |
| T3 | `group_by_error_emits_code_rows` | byError.as_array() の `expect("byError emitted")`、emit 未実装 |
| T3 | `empty_error_code_is_preserved` | byError[0].code Null != "unknown"、unknown/空文字の別 bucket 未実装 |
| T3 | `execution_number_keys_use_js_canonical_format` | timeoutMs Null != 期待 distinct-value map、canonical echo 集計未実装 |
| T3 | `nonfinite_aggregates_serialize_as_null` | grouped route row の is_object false、先行T2とT3の非有限契約が未実装 |
| T3 | `failure_only_log_emits_by_error_rows` | byError[0].count Null != 2、error rows 未実装 |
| T3 | `empty_log_emits_empty_grouped_sections` | byRoute Null != []、先行T2の empty emit と T3 の byError emit が未実装 |
| T3 | `fake_run_error_records_aggregate` | byError.as_array() が None、fake run error rows 未実装 |
| T3 | `argv_and_error_detail_not_leaked` | grouped section の is_array false、先行T2/T3の emit が未実装（漏出を観測した失敗ではない） |
| T3 | `usage_same_trace_distinct_runs_counted` | byError[0].count Null != 1、既存の records=2/ok=1/errors=1 は pass、share=1/2・1 は T3 待ち |
| T4 | `invalid_group_by_value_rejected_at_dispatch` | error.message が Unknown option '--group-by'、期待 invalid --group-by: bogus (expected route\|error) ではない |
| T4 | `route_filter_leaves_by_error_unscoped` | exit 2 != 0、Unknown option '--route'（flag 未登録） |
| T4 | `route_flag_scopes_route_at_dispatch` | exit 2 != 0、Unknown option '--route'（flag 未登録） |
| T4 | `group_by_last_wins_emits_by_error_only` | exit 2 != 0、Unknown option '--group-by'（flag 未登録） |
| T4 | `format_text_parity` | route executions=1 等の常時 route 行が無い（formatter 未実装） |

`args::` の残り 2 件（T4 所有）:

| test | actual / expected |
| --- | --- |
| `route_flag_rejected_for_route_resolve` | `invalid arguments: Unknown option '--route'...` / `` --route is not valid for `route resolve` `` |
| `args_usage_only_guard_messages` | 最初の count-tokens case が `invalid arguments: Unknown option '--route'...` / `` --since/--compare-model/--group-by/--route are only valid for `usage` ``（次の run --group-by case は最初の panic により未到達） |

### Consumer tolerance 5 件の状態

- green: `usage_optional_ids_preserve_report` — ID無し/有りの JSON/text が
  flag 無し・route/error group selector・既存filterの組合せで同一。
  この同一性 test 単体は section emit を保証しない。emit は別 pin が red。
- green: `usage_report_history_independent` — History 不在で成功し、無関係の
  inert sentinel の追加でも不変。recorded-run 数2/error数1を維持。
- expected-red / T3: `usage_same_trace_distinct_runs_counted`（上表）。
- expected-red / T2: `usage_refetch_distinct_execution_id_joins_log_ref`（上表）。
- expected-red / T2: `usage_window_orphan_not_backfilled_from_history`（上表）。

fixture は optional opaque ID を付けるだけで形式妥当性を要求せず、
sentinel は将来の History path/schema を仮定しない。

### Review handoff

- 自己点検: production diff は enum/filter/signature/`None` caller のみ。
  新規34 tests（report31/args2/log1）は既存 module 内、
  `#[ignore]`・無効化・期待値の実装への迎合は無し。
- [x] 独立レビューで blocking issue candidate を検証し、refetch join fixture の
      parentExecutionId を owner と不一致に修正。route-filter の assertion
      強化候補は本 test の明示した route/run 分離範囲外の P2 として記録。
- [x] レビュー承認後の `test(usage): ...` logical commit と status 完了
