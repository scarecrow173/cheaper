---
id: "IMPL-034JmoINr6JYbqaA7G0Ex7"
type: "impl"
status: "completed"
title: "Add route integration tests"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/add-route-integration-tests.md"
        note: "本 IR"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
        note: "R40 — routes_sig へ content hash(PolicySig::Contents + route_file_sig)"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
        note: "R37 — devin Rewrite を公式 nested hookSpecificOutput shape へ(helper 共有)"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
        note: "R7 — RouteRecordInput.suppressed + wire emit"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
        note: "suppressed provenance 伝播 + observable worker-read tests"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
        note: "CompiledRun/execute_compiled/Shared を pub(crate) へ(crate-local test のため)"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
        note: "e2e child mode + hook→CLI・bounded・stale-grant・stdin・suppressed・devin tests"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
        note: "suppressed Some/None wire assert"
      - type: "file"
        path: "crates/cheaper-workflow/tests/compile_contracts/compiler.rs"
        note: "numeric-when regression"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
        note: "viability は選択 grant を pin_command_grant で pre-pin して options.pins へ載せる"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
        note: "check_command_execution が pins 必須へ戻る(no pinned executable)+ 既存 comment text 訂正"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
        note: "pin map 無し PolicyViolation + 選択 grant pre-pin の実行経路"
      - type: "file"
        path: "scripts/test/apm-install-targets.test.mjs"
        note: "codex の期待 command を committed --target codex registration へ整合"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
        note: "PreToolUse 自動 route section(日本語)"
      - type: "file"
        path: "docs/tasks/add-route-integration-tests.md"
        note: "E task 完了クローズ"
      - type: "file"
        path: "docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md"
        note: "plan 完了・OpenQuestions 解決記録"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
        note: "AcceptanceCriteria checkbox 反映"
      - type: "file"
        path: "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
        note: "canonical design 修正(lead-authored)"
      - type: "file"
        path: "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
        note: "Verification box 反映(lead-authored 部分含む)"
      - type: "file"
        path: "docs/test-specs/hook-dedup-route-fingerprint.md"
        note: "test-spec 反映"
      - type: "file"
        path: "docs/test-specs/hook-route-e2e.md"
        note: "test-spec 反映"
      - type: "file"
        path: "docs/test-specs/hook-route-rewrite.md"
        note: "test-spec 反映"
      - type: "file"
        path: "docs/impl/ir/README.md"
        note: "managed index row"
      - type: "file"
        path: "docs/tasks/README.md"
        note: "確定済み task 状態への managed index 同期"
      - type: "file"
        path: "docs/plans/README.md"
        note: "plan 完了状態への managed index 同期"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-integration-tests.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://docs.devin.ai/cli/extensibility/hooks/overview"
    - "https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add route integration tests

## Summary

E — PreToolUse 自動 route の integration 証拠追加。3 件の
source-owner 修正(R7 suppressed provenance・R37 devin official
nested shape・R40 routes content fingerprint)と hook→CLI
E2E・bounded・stale grant・stdin・observable worker-read・
numeric-when・builtin no-grant・guide を実装した。初回の全体 gate
で検出された scope pin 境界の回帰(d18df96 が共通 checker へ入れた
pin 即時解決 fallback)を strict pin 契約へ戻し、新 E tests を
変更なしの production reference workflow 経路へ更新。canonical
`mise run check` は最終実行で全 gate green(CHECK_EXIT: 0)。

## Context

- task: `docs/tasks/add-route-integration-tests.md`
- upstream: `docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md` Phase E
- spec: `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
- design: `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
- ADR: `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
- scope pin の既存契約: `docs/designs/policy-gated-command-tool.md`

## Implementation

### Changes

- R7: `RouteRecordInput.suppressed: Option<String>` +
  `build_route_record` が Some のとき wire `suppressed` を追加
  (None で既存 shape 不変)。`run::finish` が
  `resolution.suppressed` を渡す。authoritative Option を
  流用し provenance 型は新設しない。
- R37: `hook_specific_output_rewrite(command)` helper に
  Claude nested rewrite を抽出し pretooluse/devin 両 render が
  共有。devin は Deny/Allow 不変、Rewrite のみ公式 wire
  `{hookSpecificOutput:{hookEventName:PreToolUse,updatedInput:
  {command}}}` へ(decision/permissionDecision 無し)。
  参照: https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks
- R40: `PolicySig::Contents(u64)` + `route_file_sig`(8KiB
  stream DefaultHasher、absent→Absent、read error→None で
  prepare_dedup fallback)。Shell variant の builtin/user/project
  routes signature を stat から content hash へ。非 Shell・
  builtin-None(Embedded)は不変。
- E tests(route_run.rs): `e2e` child mode(marker append・
  envdump+STDIN_BYTES・head/300x/tail・stderr・exit7)、hook
  pretooluse→rsplit_once extraction→route run(env prefix/
  captured/forbidden・bounded 128・stale grant・refetch・
  `routed_binary_does_not_forward_parent_stdin`)、
  `suppressed_project_provenance_reaches_route_record`、
  `devin_hook_rewrite_emits_nested_updated_input`。
- Observable worker-read(crate-local、dev-seam):
  `worker_reads_route_log_through_inherited_reader` が condense
  worker の `savings.sourceTokens == count_text_tokens(seed)`
  を assert(正の制御)、`worker_route_log_read_denied_by_policy_cap`
  が cap1 → sourceTokens 0(負の制御)。fake summary は固定値で
  read 成功の証拠には使わない。`CompiledRun`/`execute_compiled`/
  `Shared` を pub(crate) へ変更(crate-internal、public API 無し)。
- numeric-when: `numeric_when_left_path_and_right_input` —
  `$from` path `/details/outputTokens`・right `$input:threshold`
  の compile 受理 + edge 推論(no invocation/threshold0 両方)。
- scope pin strict 化: `check_command_execution` は
  `options.pins.get(name)` 必須へ戻る(無しは
  `no pinned executable` の PolicyViolation)。`route::resolve`
  viability は選択 grant のみを既存 public `pin_command_grant`
  で pre-pin して `options.pins` へ載せ、checker が
  argv/cwd/verify の同一 authority のままとなる。unrelated grant
  の全 pin 構築はしない。`verify_pinned` の未構築 map 経路は
  `current_exe` canonical + `args=["--list"]` の決定的な missing-pin
  test とし、PolicyViolation 期待は維持。既存 comment 3 箇所の
  陳腐化した文面を strict pin 契約へ訂正(行数不変)。
- `apm-install-targets.test.mjs` の codex 期待 command を
  committed `--target codex` registration へ整合(8f6cd12 の
  codex target split が authoritative、test の pretooluse 期待が
  stale)。
- guide: `parent-agent-integration.md` へ PreToolUse 自動 route
  section を追加(層・opt-in・approval 保全・payload opaque・
  raw stdout・route logs・env/計測の事実のみ)。

### Decisions During Implementation

- worker-read test は `execute_compiled` 直接呼出しにした —
  `execute_routed` の routed convention(exec tool command job +
  execResult output)は test の責務外のため visibility のみ
  pub(crate) 化して isolation を保った。
- devin test で `decision`/`updatedInput` top-level 不在を assert
  — 公式 nested shape 以外の emit を拒否する。
- e2e mode は STDIN_BYTES を envdump file へ書き、stdout/log へ
  env 値を出さない(in-process mock では証明不能な実 subprocess
  stdin contract のため)。
- e2e_stale_grant は payload 生成後に policy.yaml を grant 無しへ
  上書きし fail-closed を確認。
- `run_cli_raw` helper は JSON parse しない raw stdout を返す
  (D4 CLI test と共有)。

## Related Experiments

### Adopted

(候補スケッチや検討経路は作らず、既知の recipe から直接実装したため
experiment は無し)

### Rejected

(同上 — experiment は無し)

## Validation

初回 focused 検証の記録に続き、scope pin 境界の修正と
production workflow への置換後の最終 canonical gate を末尾に記す。

- `--lib hook::eval_cache` → 11/11 pass(`tmp/handoff-e/eval-cache.txt` — R40 `Contents(1)!=Contents(2)` + `routes_content_change_with_preserved_stat_changes_fingerprint`)。
- `--lib route::` → 34/34 pass(`tmp/handoff-e/lib-route.txt` — worker-read positive/negative 含む)。
- `--test integration route_run::` → 24/24(`tmp/handoff-e/integration.txt` — e2e・bounded・stale・stdin・suppressed・devin 全て)。
- `--test integration route::`(5)・`route_logs::`(8)・`args::`(45)・`cli::`(7)・`usage_log::`(10)全 pass(同 file)。
- `--test integration hook::` → 26/26(`tmp/handoff-e/hook.txt` — R37 devin change で既存 pass)。
- `--test integration usage_log::` → 10/10(`tmp/handoff-e/usage-log.txt` — Some/None wire)。
- `--test compile_contracts numeric_when` → 1/1(`tmp/handoff-e/numeric-when.txt`)。
- `cargo fmt --all -- --check` → clean(`tmp/handoff-e/fmt.txt`)。
- `clippy --all-targets` → 0 warnings(`tmp/handoff-e/clippy.txt`)。
- 初回 `mise run check` → **FAILED**(`tmp/handoff-e/check.txt`)。
  `tools::command::tests::verify_pinned_rejects_missing_swapped_or_workspace_paths`
  で実行時の pin 必須契約が破れていた。同一 feature stream の先行
  commit `d18df96` が dry-run 用の即時 pin 解決を共通 checker に
  入れた結果、実行も未構築 pin map を受け入れていた。既存設計は
  per-scope pin を expose 前に要求するため、テストの期待は正しい。
  テストを緩めず、checker を厳密に戻し、resolver が既存 helper で
  選択 grant を pre-pin してから同じ checker を呼ぶ形に修正した。
- 最終 focused: `--test integration route_run::` → 25/25
  (`tmp/handoff-e/route-run-final.txt` — production workflow・
  hook token・builtin no-grant・stdin subprocess env_clear を含む)。
  pin 修正の focused 実行(`--lib tools::command::` /
  `--test proc_command`)は実行時に各スイート全 pass を観測、
  canonical gate が同一 suite を包括するため最終記録は下記の
  canonical 実行を根拠とする。
- `cargo fmt --all -- --check` → clean(`tmp/handoff-e/fmt-final.txt`)、
  comment-only の最終 pass は `tmp/handoff-e/fmt-comment.txt`。
- `clippy --all-targets` → 0 warnings(`tmp/handoff-e/clippy-final.txt`)。
- 2 回目 gate は `node --test scripts/test/` の
  `apm-install-targets.test.mjs` で codex 期待
  `pre-tool-use --target pretooluse` との不一致 — 8f6cd12 の
  codex target split で登録された実 package は `--target codex` で
  あり、test 側の期待が stale。committed registration に合わせて
  期待を `codex` へ修正。
- **最終 `mise run check` → 全 gate green、CHECK_EXIT: 0**
  (`tmp/handoff-e/check-final.txt`)。optional dedup integration・
  real provider・native permission UI は実行していない。

### Spec / ADR verification map

表は実装・テストの所在を確認した対応表であり、各 test-spec の意図を
置き換えるものではない。実行結果は上記 Validation と最終 gate の記録を
参照する。`別名::関数名` の出典は次のとおり。

| 別名 | テストの出典（repo 相対 path） |
| --- | --- |
| loader | `crates/cheaper-cli/src/route/routes.rs` |
| resolver | `crates/cheaper-cli/src/route/resolve.rs` |
| payload | `crates/cheaper-cli/src/route/payload.rs` |
| prepare | `crates/cheaper-cli/src/route/run.rs` |
| route | `crates/cheaper-cli/tests/integration/route.rs` |
| run | `crates/cheaper-cli/tests/integration/route_run.rs` |
| logs | `crates/cheaper-cli/tests/integration/route_logs.rs` |
| store | `crates/cheaper-cli/src/route/log_store.rs` |
| embedded | `crates/cheaper-cli/src/embedded.rs` |
| context | `crates/cheaper-cli/src/workflow_context.rs` |
| args | `crates/cheaper-cli/tests/integration/args.rs` |
| hookPolicy | `crates/cheaper-cli/src/hook/routing_policy.rs` |
| targets | `crates/cheaper-cli/src/hook/targets.rs` |
| hook | `crates/cheaper-cli/tests/integration/hook.rs` |
| cache | `crates/cheaper-cli/src/hook/eval_cache.rs` |
| command | `crates/cheaper-tools/tests/proc_command.rs` |
| proc | `crates/cheaper-tools/src/tools/proc/tests.rs` |
| compiler | `crates/cheaper-workflow/tests/compile_contracts/compiler.rs` |
| engine | `crates/cheaper-workflow/tests/engine.rs` |
| usage | `crates/cheaper-cli/tests/integration/usage_log.rs` |
| report | `crates/cheaper-cli/tests/integration/usage_report.rs` |

| Spec | ADR | テスト識別子（出典別名::関数名） |
| --- | --- | --- |
| R1 | — | `loader::loads_valid_layer_file` / `loader::malformed_and_unknown_fields_are_config_errors` |
| R2 | — | `loader::malformed_and_unknown_fields_are_config_errors` |
| R3 | D2, D7, D13 | `loader::user_shadows_builtin_and_project_needs_trusted` / `embedded::builtin_resources_extracts_routes_workflows_and_condense` / `context::builtin_routed_exec_workflow_compiles` / `run::builtin_cargo_test_route_registered_but_no_grant` |
| R4 | — | `resolver::longest_match_wins` / `route::resolve_matched_runnable_reports_r11_shape` |
| R5 | — | `resolver::longest_match_wins` |
| R6 | D7 | `loader::same_layer_duplicates_are_quarantined_as_invalid` / `resolver::viability_order_and_branches` |
| R7 | D2, D7 | `loader::user_shadows_builtin_and_project_needs_trusted` / `resolver::viability_order_and_branches` / `run::suppressed_project_provenance_reaches_route_record` |
| R8 | — | `prepare::prepare_rejects_compound_and_unsafe_command` / `resolver::viability_order_and_branches` / `hookPolicy::tokenize_unsafe_chars_and_windows_path` |
| R9 | — | `resolver::self_bypass_and_env_prefix_strip` |
| R10 | — | `args::route_resolve_requires_exactly_one_input` / `route::resolve_matched_runnable_reports_r11_shape` / `run::e2e_hook_rewrite_to_cli_route_run` |
| R11 | — | `route::resolve_matched_runnable_reports_r11_shape` / `route::resolve_no_match_omits_route_fields` |
| R12 | — | `resolver::viability_order_and_branches` |
| R13 | — | `payload::round_trip_and_allowlist` / `payload::decode_rejects_malformed` / `run::e2e_hook_rewrite_to_cli_route_run` |
| R14 | D2, D11 | `run::revalidation_failures_reject_without_executing` / `prepare::prepare_ignores_payload_executable_and_args` / `run::e2e_stale_grant_fails_closed_after_rewrite` |
| R15 | D11 | `run::exit_mirrored_and_log_written` / `run::cli_run_emits_raw_text_exit_and_route_record` / `run::exec_infra_failure_returns_error` / `run::cli_run_cwd_override_uses_foreign_root` |
| R16 | D11 | `run::post_job_failure_keeps_exit_and_records_stage` / `run::output_schema_failure_still_mirrors_real_result` / `prepare::finish_mirrors_captured_exit_and_records_workflow_error` / `run::tiny_count_cap_partial_count` / `run::e2e_hook_rewrite_to_cli_route_run` |
| R17 | D1, D12 | `context::builtin_routed_exec_workflow_compiles` / `run::skipped_analyze_keeps_fallback_text` / `run::worker_parity_and_route_log_read` / `prepare::worker_reads_route_log_through_inherited_reader` |
| R18 | D8, D12 | `run::env_prefix_beats_ambient_beats_grant` / `run::cli_run_cwd_override_uses_foreign_root` / `command::route_context_extra_env_overrides_grant_env` / `command::route_context_exec_cwd_overrides_grant_cwd` / `command::timeout_kills_and_reports` / `command::abort_kills_managed_parent_and_descendant` / `prepare::deadline_is_grant_timeout_plus_120s` / `run::routed_binary_does_not_forward_parent_stdin` |
| R19 | D5 | `command::declared_exitcodes_report_completed_result` / `command::undeclared_nonzero_exit_remains_failure` / `command::invalid_exitcodes_rejected_at_policy_resolution` |
| R20 | D5 | `command::timeout_kills_and_reports` / `command::abort_kills_and_preserves_reason` / `run::exec_infra_failure_returns_error` / `proc::spawn_failure_async_shape_follows_node` |
| R21 | D4 | `engine::when_true_runs_job` / `engine::when_numeric_operators_safely_evaluate` |
| R22 | D4 | `engine::when_false_skips_without_failure_and_materializes_null` |
| R23 | D4 | `engine::when_false_skips_without_failure_and_materializes_null` |
| R24 | D4 | `compiler::when_schema_and_dependency_inference` / `engine::when_from_creates_dependency_edge` / `engine::boundary_errors_skip_dependents_and_keep_independent_outputs` |
| R25 | D4 | `compiler::when_schema_and_dependency_inference` / `engine::when_false_skips_without_failure_and_materializes_null` / `run::skipped_analyze_keeps_fallback_text` |
| R26 | D6 | `command::log_tee_captures_full_output_beyond_cap` / `run::bounded_text_full_log` |
| R27 | D6 | `store::run_id_format_and_validation` / `store::begin_writes_tmp_then_finish_renames` |
| R28 | D6 | `store::sweep_enforces_ttl_count_size_and_skips_tmp` |
| R29 | D6 | `run::log_begin_failure_preserves_exit_without_logref` / `command::log_tee_sink_failure_does_not_break_result` |
| R30 | D6 | `logs::logs_reads_paged_and_eof` / `logs::logs_default_max_obeys_policy_cap` / `logs::logs_offsets_are_bytes_not_chars` / `store::reader_enforces_policy_read_cap` / `prepare::worker_reads_route_log_through_inherited_reader` / `prepare::worker_route_log_read_denied_by_policy_cap` |
| R31 | D6 | `store::artifact_reader_shares_run_id_validation` / `logs::logs_rejects_bad_offsets_and_ids` / `loader::target_path_must_be_layer_relative` |
| R32 | D8 | `payload::round_trip_and_allowlist` / `run::env_prefix_beats_ambient_beats_grant` / `prepare::prepare_rejects_unallowlisted_env_without_leak` / `run::e2e_hook_rewrite_to_cli_route_run` |
| R33 | D8 | `resolver::viability_order_and_branches` / `run::routed_binary_does_not_forward_parent_stdin` |
| R34 | D9 | `usage::route_record_shape` / `usage::route_refetch_record_shape` / `report::route_records_aggregate_into_independent_bucket` / `logs::logs_reads_paged_and_eof` |
| R35 | D9 | `run::cli_run_quiet_suppresses_usage_failure_diagnostic` / `logs::logs_usage_append_failure_keeps_success` / `report::route_records_aggregate_into_independent_bucket` |
| R36 | D3 | `hookPolicy::rewrite_serde_round_trip` / `hookPolicy::runnable_route_produces_rewrite` / `hookPolicy::rewrite_preserves_fallback_deny` |
| R37 | D3 | `targets::pretooluse_rewrite_emits_updated_input_without_permission` / `targets::devin_rewrite_emits_updated_input_without_permission` / `run::devin_hook_rewrite_emits_nested_updated_input` |
| R38 | D3 | `targets::codex_rewrite_renders_fallback_only` / `targets::windsurf_rewrite_renders_fallback` / `targets::cursor_and_copilot_rewrite_render_fallback` / `hook::routed_shell_per_target_render_contract` |
| R39 | D3 | `hookPolicy::non_runnable_falls_through` / `hook::shell_route_arm_engages_via_firing_load` |
| R40 | D10 | `cache::fingerprint_includes_routes_signature` / `cache::prepare_dedup_scopes_routes_sig_to_shell_variant` / `cache::routes_content_change_with_preserved_stat_changes_fingerprint` / `hook::shell_route_arm_engages_via_firing_load` |

対応表の範囲に関する注意:

- R16 の failure isolation は既存 post-processing / `finish` test で確認する。
  新しい happy-path marker の一回性を、fault injection の証拠と混同しない。
- R20 の spawn 失敗は共通 proc の platform 分類 test と、exit-code 判定へ
  進む前に Err を返す実装で確認する。新たな routed spawn fault test を
  実施したと主張するものではない。
- R30 の read 成功は実 tool result の `sourceTokens` と投入 text の
  canonical token 数の一致で確認する。cap 拒否の負の制御では実 tool call が
  一回、sourceTokens は 0 である。固定 fake summary 自体は証拠にしない。
- 公開 `count_tokens` の runId / artifact URI 入力への一般化は本設計の
  scope 外であり、この対応表で実装済みと扱わない。

## Risks

- `run_worker_read` が `RouteLogStore::new`/`RouteExecution::new`/
  `CompiledRun` 等 crate-internal に依存 — crate 境界外の public
  API 変更は意図的に避けた。
- native Coding Agent 実体の permission UI / upstream の silent-ignore
  問題 / 実 provider session はこの deterministic suite の対象外。
  Claude #81340 と非 Rewrite target の #150 は承認済みの scope 外に残る。

## Follow-ups

- 承認済み scope 内の新規 task は起こさない。`#150`(非 Rewrite
  target の approval-bypass 検証)と `#81340`(Claude の
  upstream silent-ignore)は deferred のまま。
