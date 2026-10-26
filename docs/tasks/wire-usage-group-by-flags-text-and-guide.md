---
id: "TASK-034MbTqck0r2tjYDCYQUM7"
type: "task"
status: "done"
title: "Wire usage --group-by/--route flags, help, text sections, and usage guide (issue #184 T4)"
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
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/text_format.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "docs/guides/usage.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
  implemented-by:
    - "../impl/ir/wire-usage-group-by-flags-text-and-guide-issue-184-t4.md"
  depends-on:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/tasks/implement-usage-route-window-and-by-route.md"
    - "docs/tasks/implement-usage-by-error-aggregation.md"
  blocks:
    - "docs/tasks/measure-usage-jsonl-peak-memory.md"
    - "docs/tasks/verify-usage-analysis-chain-and-audit.md"
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
    - "docs/guides/usage.md"
  defers: []
  deferred-by: []
---

# Wire usage --group-by/--route flags, help, text sections, and usage guide (issue #184 T4)

## Work

T2/T3 の集計を CLI 面へ配線し、text parity と guide を着地させる
(design §4-§6・§8)。本 task で残りの pin test(args 系・
`format_text_parity`)を全て green にし、T6 の canonical
verification へ渡せる状態を作る。

1. `crates/cheaper-cli/src/args.rs`:
   - `STRING_OPTIONS`(:343-378)に `"group-by"`・`"route"` を
     `"compare-model"` の直後(usage option cluster)へ追加。
   - `Values`(:387-420)に `group_by: Option<String>`・
     `route: Option<String>` field + `Default` + `set_string`
     (:456-488)の arm を既存 last-wins 規約で追加。
   - `UsageCliArgs`(:241-252)に `pub group_by: Option<String>`・
     `pub route: Option<String>` を追加し、usage arm(:894-941)の
     組立てへ反映(args 層は生文字列を保持 — 値検証は cli 層)。
   - `usage_only_option_used`(:759)に `group_by`・`route` を編入
     し、guard message(:945)を
     `"--since/--compare-model/--group-by/--route are only valid
     for \`usage\`"` へ更新(列挙形式を維持)。guard 発火位置
     (:943-948)は変えない — `route resolve --route x` は既存
     per-action allowlist が `"--route is not valid for \`route
     resolve\`"` で reject する(silent drop 防止規約の維持)。
   - 想定内更新:既存 test `since_only_valid_for_usage`
     (`tests/integration/args.rs`:379-385)の expected message
     を新列挙(`--group-by`/`--route` 込み)へ更新する — guard
     message 変更に伴う churn。
2. `crates/cheaper-cli/src/usage_report.rs` `execute_usage_inner`
   (:673-741):`--since` parse の直後に `--group-by` 検証を追加
   — `Some("route")`/`Some("error")` を `UsageGroupBy` へ変換し、
   それ以外の値(`route,error` 等の複合値・未知値)は
   `invalid_request(format!("invalid --group-by: {v} (expected
   route|error)"), "groupBy", v)`。`UsageFilters` 組立てに
   `route: args.route.clone()` を追加し、`group_by` を
   `build_usage_report` へ渡す。
3. `crates/cheaper-cli/src/cli.rs`:`--compare-model` 行の後ろ
   (:595-603 付近)に help 2 行を追加(design §4 の固定文面 —
   `--group-by <route|error>`・`--route <name>`)。
4. `crates/cheaper-cli/src/text_format.rs` `format_usage_report`
   (:341-416):
   - 既存行は全て維持し、「avoided parent context」行の直後・
     Estimated block の前に route section を常時描画(固定 2 行
     format — `route:`/`route tokens:`。値は既存行と同じ
     `num(get(&route,"..."))` rendering)。
   - `result.route.orphanRefetches` が存在する場合のみ
     `route scope:` 行を追加。
   - `result.byRoute`/`result.byError` key 存在時は presence-driven
     に section を描画 — header は entry 0 件でも無条件に出し
     (JSON の空 array emit 規約と同一)、行 format は design §5
     の固定形。空の inner map(workers/models/reasons)は detail
     行を省略し、`execution` の 3 subfield が全て空なら行自体を
     省略する。
   - detection shim(`utility_text` :195-205)は変更不要 —
     `result.records` number + `result.measured` object の検出条件
     は変わらない。
5. `usage_command_format_text_writes_report` の expected text を
   更新する(route section 常時描画による想定内 churn — spec
   Req 6。同 test の inline `expected` 文字列を新 2 行込みへ)。
6. `docs/guides/usage.md` — design §8 doc-sync 一覧を反映:
   「集計対象は `kind:"run"` record のみ」の drift 修正・route
   bucket 説明・`--group-by route|error`・`--route` の用法・
   `--since` が `result.route` にも効く意味論変更・`--group-by`
   が単値で `route,error` は INVALID_REQUEST・期間表記は
   `168h`/`24h`・`shareOfRuns`/`shareOfFailures`/`measured`/
   `comparable` の母数の違いと actual/measured/estimated の区別・
   `orphanRefetches`/`unkeyedRecords`/`unmeasurable` の用語。
7. T1 の残 pin test を green にする:
   - `route_flag_rejected_for_route_resolve`・
     `args_usage_only_guard_messages`(`tests/integration/args.rs`
     の parse-level 側 — `route resolve --route x` の allowlist
     reject・`count-tokens --route x`/`run --group-by route` の
     usage-only guard 列挙 message)。
   - `invalid_group_by_value_rejected_at_dispatch`・
     `route_flag_scopes_route_at_dispatch`・
     `route_filter_leaves_by_error_unscoped`・
     `group_by_last_wins_emits_by_error_only`(dispatch 経路側 —
     `run_usage`/`CliIo` harness。`invalid --group-by: {v}
     (expected route|error)`・`args.route → filters.route`
     handoff・`--route`+`--group-by error` 合法組合せ・
     `--group-by` last-wins をそれぞれ pin)。
   - `format_text_parity`・`format_text_preserves_equals_in_execution_keys`。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-usage-route-error-analysis-issue-184.md (T4 slice)
- docs/designs/usage-grouped-route-error-analysis-issue-184.md (§4 CLI・§5 text format・§6 後方互換・§8 doc-sync)
- docs/specs/usage-analysis-scope-issue-184.md (Req 5-6・出力 contract flag-gating 節)

## Dependencies

- docs/tasks/implement-usage-route-window-and-by-route.md (T2 — `byRoute` rows・scoped route bucket)
- docs/tasks/implement-usage-by-error-aggregation.md (T3 — `byError` rows)

## Affected Files

- `crates/cheaper-cli/src/args.rs`
- `crates/cheaper-cli/src/cli.rs`
- `crates/cheaper-cli/src/usage_report.rs`(`execute_usage_inner` の
  み — 集計側は T2/T3 で完了)
- `crates/cheaper-cli/src/text_format.rs`
- `crates/cheaper-cli/tests/integration/args.rs`
- `crates/cheaper-cli/tests/integration/usage_report.rs`
  (`usage_command_format_text_writes_report` の expected 更新を含む)
- `docs/guides/usage.md`

対象外: 集計実装本体(T2/T3 で完了済み)、`tests/integration/
usage_log.rs`(T1 の caller 追従で完了済み)、writer/record schema
(不変 — spec Out of Scope)。

## Done When

- [x] `--group-by`/`--route` が args 登録・usage-only guard 編入・
      `execute_usage_inner` 検証(`route|error` 以外は
      INVALID_REQUEST の exact message)まで配線される
- [x] help 2 行が design §4 の固定文面で追加される
- [x] route section が text へ常時描画され、`byRoute`/`byError` が
      presence-driven section(0 件でも header 描画・空 map 行省略)
      で出る — `format_text_parity` green
- [x] `route_flag_rejected_for_route_resolve`・
      `args_usage_only_guard_messages`・
      `invalid_group_by_value_rejected_at_dispatch`・
      `route_flag_scopes_route_at_dispatch`・
      `route_filter_leaves_by_error_unscoped`・
      `group_by_last_wins_emits_by_error_only` が green
- [x] `since_only_valid_for_usage` の expected が新列挙 message
      へ更新される(想定内 churn)
- [x] flag 無し JSON が regression golden `cli-usage-*.json` と byte
      不変(flag-gated additive の証明)
- [x] `usage_command_format_text_writes_report` の expected が新
      route section 込みへ更新される(想定内 churn)
- [x] `docs/guides/usage.md` が design §8 の sync 一覧を反映する

## Verification

- [x] `usage_report::`: 65 passed / 0 failed
- [x] `args::`: 47 passed / 0 failed
- [x] `usage_log::`: 21 passed / 0 failed
- [x] `mise exec -- cargo fmt --all -- --check` exits 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0
- [x] regression golden 61/61 pass、golden directory に diff なし
- [x] `format_text_parity` と `format_text_preserves_equals_in_execution_keys` pass。PR review で指摘された execution key 内 `=` の改変を direct map formatting で修正。
