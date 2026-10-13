---
id: "IMPL-034EWF6HsxdlNyPsQepBQ9"
type: "impl"
status: "completed"
title: "fixture DSL and degraded-run verification coverage"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/fixture-dsl-and-degraded-run-verification-coverage.md"}
      - {"type":"code","path":"crates/cheaper-cli/tests/regression/cases/run-tool-operator-degraded.json"}
      - {"type":"code","path":"crates/cheaper-cli/tests/regression/expected/run-tool-operator-degraded.json"}
    modified:
      - {"type":"code","path":"crates/cheaper-runtime/src/dev/fixture.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-cli/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-cli/tests/workflow_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/worker/validate.json"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/fixture-dsl-and-verification.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/fixture-dsl-and-verification.md"
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
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# fixture DSL and degraded-run verification coverage

## Summary

T4。`FixtureWorker.optional_tools`(snake_case DSL key)を追加し、
`to_markdown()` が非空時のみ `cheaper:` 配下へ frontmatter contract
key `optionalTools:` を emit するようにした — regression harness が
`worker_markdown()` を無条件で seed するため DSL knob が必須
(design §(i)、Risk #4)。その上で approved test-spec の行為保証を
固定する test を cli/runtime/workflow 各 dev-seam 層・regression
corpus・worker_validate golden へ追加した。admission
(`tools.allow` / `check_capabilities`) と call 経路の認可機構
(grant 解決・argv pattern・executable pin/検証)は一切緩めていない。

## Context

TASK-034EUvOR5z2gLPUgzBappw。issue #102 の tool-operator built-in
worker の第 4 task。T1(df540ad)が `optionalTools` vocabulary・
validate、T2(c9921f1)が degrade path + prompt surfacing、T3(7be4f41)
が builtin worker 定義・schema・`tool-operate` alias を着地済み。
本 task は test-spec `docs/test-specs/tool-operator-built-in-worker.md`
が pin する行為保証の検証層を実装する。

## Implementation

### Changes

- `crates/cheaper-runtime/src/dev/fixture.rs`:
  `FixtureWorker.optional_tools: Vec<String>` を struct 既存の
  `#[serde(default)]` に乗る field として追加(rename 不要 —
  他 field と同じ snake_case DSL key)。`to_markdown()` は非空時のみ
  `cheaper:` 配下へ `  optionalTools: [process, command]` を emit。
  unit test 2 件追加 — 非空 emit + `worker_definition` で
  `optional_tools` が `[Process, Command]` へ parse される roundtrip、
  空時の非 emit(既存 fixture markdown 不変)。
- `crates/cheaper-cli/tests/run_dev_seam.rs`: opt-in 縮退の dev-seam
  fixture test 9 件。
  - `run_fake_stream_opt_in_degraded_run_completes_with_blocked_result`
    — builtin default policy(tasks/grants 空)で opt-in tool が縮退し
    model session まで到達、`result.status == "blocked"` は
    run-level error ではなく worker result(AC4/AC8)。
  - `run_fake_stream_degraded_and_undeclared_calls_have_no_exec_path`
    — 縮退 `command`/`process` と未宣言 `write`/`edit` の scripted
    call は全て fail-closed(`meta.toolCalls` は実登録 `read` の
    1 件のみ、`x.txt` 非存在・`meta.changes` 空)。
  - `run_fake_stream_opt_in_command_grant_executes_and_marks_not_comparable`
    — `command_user_policy` で grants 非空なら opt-in tool も非
    opt-in と同一の authorization path(grant 解決 + argv pattern +
    executable pin)で実 spawn(marker file 到達)。`directlyComparable`
    false + `meta.changes` 空の honest metadata。
  - `run_fake_stream_opt_in_capability_denied_is_policy_violation`
    — user layer `tools.allow: [read]` で opt-in worker も
    `missing: [process, command]` 付き `POLICY_VIOLATION`(admission
    と content-gate の 2 層分離)。
  - `run_fake_stream_opt_in_mixed_policy_only_command_degrades` /
    `run_fake_stream_opt_in_mixed_policy_only_process_degrades`
    — mixed policy の両方向で failed-gate 側のみ縮退、他方は実
    spawn(project layer は `command` section を持てないため
    `process.tasks` 側は project policy seed で表現)。
  - `run_fake_stream_tool_operate_alias_completes_degraded`
    — `cheaper tool-operate` alias が builtin `tool-operator` へ
    解約し builtin default policy で縮退完走。`toolCalls == 0`、
    `meta.changes` 空、`directlyComparable` false、usage.jsonl に
    `kind:"run"` record(AC1/AC9/AC10)。
  - `run_fake_stream_tool_operator_non_success_statuses_are_results`
    — `partial` / `failed` / `blocked` は envelope `ok: true` の
    `result.status`(AC2/AC3)。
  - `run_fake_stream_tool_operator_returns_bounded_result_only`
    — 64KB read 出力を発生させても envelope `result` は宣言
    schema 適合の compact output のみ(生転載なし)、
    `sourceTokens > returnedTokens` の offload 実測(AC5)。
  - 既存 `run_fake_stream_command_worker_denied_without_grants` は
    無変更 green(非 opt-in fail-closed 不変 — Risk #6)。
- `crates/cheaper-runtime/tests/run_dev_seam.rs`: runtime 層 2 件。
  - `degraded_run_prompt_lists_only_the_failed_gate_tool`
    — mixed policy で "Available tools" に gate 通過の `process` が
    残り、利用不可注記は `command` のみ列挙(per-tool 縮退の
    prompt 反映)。
  - `degraded_and_undeclared_calls_fail_unknown_tool`
    — `RecordingSession`(`WorkerSession` wrapper が
    `RunEvent::ToolResult` の `output` を記録)経由で縮退・未宣言
    tool call の `{"error": "unknown or unregistered tool: <name>"}`
    を literal assert(envelope には載らないため tee-session seam)。
- `crates/cheaper-cli/tests/workflow_dev_seam.rs`:
  `workflow_run_tool_operator_degrades_like_run` — `kind: worker`
  job が builtin `tool-operator` を実行する経路でも `run` と同一の
  縮退挙動(job は POLICY_VIOLATION で死なず
  `result.status == "blocked"` を返す — test-spec workflow Rule)。
- `crates/cheaper-cli/tests/regression/{cases,expected}/`:
  `run-tool-operator-degraded.json` — DSL 経由の opt-in fixture
  worker + scripted `command`(degraded)と `read`(実登録)call。
  expected は `CHEAPER_REGRESSION_CAPTURE=1` で生成し diff review
  済み — `command` call は metering に到達せず
  `toolCalls == 1`(`read` のみ)・`directlyComparable: false` を
  固定する degraded run。
- `crates/cheaper-core/tests/golden/fixtures/worker/validate.json`:
  `optionalTools` accept 1 case(declaration order・`dump_worker_def`
  の conditional emit)+ reject 7 case — validator の全 reason
  (`invalid` / `empty` / `unknown-tool` / `undeclared` /
  `not-gated` / `duplicate` / `not-proper-subset`)を reason 固定で
  網羅。
- `docs/tasks/fixture-dsl-and-verification.md` — status を done へ、
  implemented-by・changes・Verification checklist を実績で更新。

### Decisions During Implementation

- regression case の初回 capture が `POLICY_VIOLATION
  (missing: ["command"])` になった。調査の結果、production binary
  の embedded builtin resource は `<temp>/cheaper-builtin-<version>/`
  へ `.ready` marker 付きで materialize される version-keyed cache
  であり、本機に残っていた `command` 追加前の stale 抽出
  (`tools.allow` に `command` が無い policy.yaml + `tool-operator.md`
  無し)が再利用されていた。cache を削除して再 capture し、
  縮退 run の期待 snapshot(exit 0・`toolCalls == 1`・
  `directlyComparable: false`・`meta.changes` 空)を得た。code
  変更ではなく環境起因の問題であり、in-process dev-seam test は
  `CliHooks.builtin_dir = resources/` を直接参照するため影響外。
- named worker 呼出しは canonical `run <name>` argv(`f.run(&["run",
  "opt-worker"])`)。`run opt-worker` は project/user/builtin に
  `opt-worker` が無いため `WORKER_NOT_FOUND` → fixture `worker` 節
  fallback(`resolve_named_worker` の dev-seam seam)で fixture 定義を
  使う。`run tool-operator` / `tool-operate` は builtin resource が
  先に解決し実 frontmatter を通るため、builtin worker 側の assert
  は fixture worker 節を持たない fixture で行う。
- `UNKNOWN_TOOL` の literal assert は runtime 層へ配置 — envelope
  `meta` には reject 理由が載らず、`ToolResult.output.error` を
  観測するには session seam が要るため `RecordingSession` wrapper
  を導入した(cli 層では `toolCalls`/file 非存在の fail-closed 側で
  代替 assert)。
- workflow `kind: worker` の縮退同一挙動は builtin `tool-operator`
  を job で呼ぶ形で表現 — DSL worker 節不要で実 builtin
  frontmatter の opt-in 宣言を通る。

## Related Experiments

### Adopted

なし。task/design が確定済みで追加 experiment は起こさなかった。

### Rejected

なし。

## Validation

- `cargo fmt --all -- --check` — exit 0。
- `cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam`
  — 全 target pass。`run_dev_seam` は 21/21(新規 2 件含む)。
- `cargo test -p cheaper-cli --test run_dev_seam --features
  cheaper-cli/dev-seam` — 19/19 pass(新規 9 件 + 非 opt-in 既存
  test の無変更 green)。
- `cargo test -p cheaper-cli --features cheaper-cli/dev-seam --test
  workflow_dev_seam` — 2/2 pass(新規 workflow degraded 1 件含む)。
- `cargo test -p cheaper-core --test golden -- worker_validate` —
  pass(accept 1 + reject 7 case 込み)。
- `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features
  cheaper-cli/dev-seam --test regression` — capture 生成後 diff
  review 済み。非 capture `cargo test -p cheaper-cli --features
  cheaper-cli/dev-seam --test regression` も pass。
- `cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0(初回 `assert_eq!(marker.exists(), true)` の
  bool_assert_comparison 指摘を `assert!` へ修正済み)。

## Risks

- `<temp>/cheaper-builtin-<version>/` の version-keyed materialize
  cache は、同一 version で resource 内容が変わる dev 環境では
  stale 化し得る(`.ready` marker 経路)。regression replay 等の実
  binary 経路で builtin resource 変更直後に capture する際は、
  再 capture 前に cache 削除が必要。CI 環境では発生しない
  local-only の観測差分。
- `RecordingSession` tee は `RunEvent::ToolResult.output` を記録する
  test seam であり production 経路には影響しない。

## Follow-ups

- T5: 公開面 docs + APM skill(AC12 — `docs/tasks/` の T5 task)。
