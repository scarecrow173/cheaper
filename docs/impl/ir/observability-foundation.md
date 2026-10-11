---
id: "IMPL-034EcqaOLmLoT2kL3ztbwk"
type: "impl"
status: "completed"
title: "Observability foundation"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "dir"
        path: "crates/cheaper-observability/"
      - type: "file"
        path: "crates/cheaper-cli/tests/observability.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/observability_dev_seam.rs"
      - type: "file"
        path: "benchmarks/results/4eda3caf-5e19-4a4a-a6f3-fb7d3e04d0db.json"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "benchmarks/lib/cli.mjs"
      - type: "file"
        path: "benchmarks/lib/run-scenario.mjs"
      - type: "file"
        path: "benchmarks/lib/scenarios.mjs"
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
      - type: "file"
        path: "crates/cheaper-cli/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/auth.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/models.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/cli.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/models.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-core/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-core/src/model/config.rs"
      - type: "file"
        path: "crates/cheaper-core/src/model/resolve.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/src/worker/resolve.rs"
      - type: "file"
        path: "crates/cheaper-provider/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-provider/src/devin/cli.rs"
      - type: "file"
        path: "crates/cheaper-provider/src/devin/credentials.rs"
      - type: "file"
        path: "crates/cheaper-provider/src/devin/rig_provider.rs"
      - type: "file"
        path: "crates/cheaper-provider/src/devin/stream.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/agent.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/run_stream.rs"
      - type: "file"
        path: "crates/cheaper-runtime/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-runtime/src/auth/service.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/auth/store.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/catalog/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/grep_eval.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/tree.rs"
      - type: "file"
        path: "crates/cheaper-workflow/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-workflow/src/engine.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/create-cheaper-observability-crate.md"
    - "docs/tasks/wire-cli-log-flags-and-observability-init.md"
    - "docs/tasks/instrument-cli-dispatch-and-run-pipeline.md"
    - "docs/tasks/instrument-tools-workflow-provider-and-adapter.md"
    - "docs/tasks/instrument-hook-auth-catalog-and-core.md"
    - "docs/tasks/observability-contract-tests-and-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-observability-foundation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/observability-foundation-logging-and-instrumentation-wiring.md"
    - "docs/test-specs/observability-foundation-contract.md"
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

# Observability foundation

## Summary

`crates/cheaper-observability` を新設し、flag/env 駆動の dual-sink
(file `<workspace>/.cheaper/logs/` + stderr)logging 基盤を導入した。
`cheaper-cli` の `run_cli` 先頭で `--log`/`--log-format` flag と
`CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` env から `Activation` を解釈して
init し、hook 経路では stderr sink を構造的に無効化する。
instrumentation は 3 wave で配置: wave 1 が cli dispatch/command span
と run pipeline の 8 stage 子 span(`cheaper.run.*`)、wave 2 が
tools/workflow/provider/rig-adapter、wave 3 が hook/auth/catalog/core。
stage timing は `FmtSpan::CLOSE` の close event(`time.busy`)で得る。
contract test(`tests/observability.rs` + dev-seam gated
`tests/observability_dev_seam.rs`)を subprocess isolation で追加し、
benchmark harness に `CHEAPER_LOG*` env 注入の受け口と logging arm 3 本を
追加した。

## Context

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T1–T6 stream に従う。公開面・
配置表・metadata-only 規約は
`docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
(DESIGN-034DV9ypzmxdAobGSJ6Cm6)、activation/dual-sink contract は
`docs/adr/logging-output-and-activation-contract.md` と
`docs/adr/consolidate-observability-behind-a-dedicated-crate.md`、
acceptance criteria は `docs/specs/observability-foundation-scope.md`
と test-spec `docs/test-specs/observability-foundation-contract.md`
(TSPEC-034DWcP8v4SUuuui8hvhnT、approved)が正。

## Implementation

### Changes

- `crates/cheaper-observability`(T1)— activation 解釈
  (`config.rs`: flag > env > default、invalid 値は warnings に積み
  level OFF / format text 扱いの fail-open)、sink 構築(`sink.rs`:
  `Rotation::DAILY`・`filename_prefix("cheaper.log")`・
  `max_log_files(7)` の file roller + non_blocking、stderr writer は
  `with_ansi(false)`、`stderr_enabled=false` 時は layer 自体を install
  しない)、subscriber install(`init.rs`: `Registry` + file/stderr 2
  `fmt::Layer`(`Layer::boxed()` で format 型分岐を統一)+
  `with_span_events(FmtSpan::CLOSE)` + `set_global_default`。
  repeat-init は first subscriber wins で `(None, warnings)` を返す)。
  workspace 内 crate への依存は 0。公開面は `Activation`・
  `ObservabilityGuard`・`init` のみに narrow(review 後 fixup)。
- `cheaper-cli` activation 配線(T2)— `args.rs` の `STRING_OPTIONS` に
  `"log"`/`"log-format"` を追加し、lenient scan
  (`scan_string_flag` 共有実体経由の `global_log_flags`・
  `global_string_flag`・`global_flag_present`、`--` terminator・
  last-wins)を追加。`cli.rs` の `run_cli` 先頭で
  `resolve_log_root`(`--cwd` が command 意味を持つ 5 経路のみ argv
  scan、fail-open で `io.cwd` fallback)+ `stderr_enabled = !is_hook`
  で `cheaper_observability::init` を呼び、`_log_guard` を run_cli
  ローカルに束縛。hook 経路の init 警告は install 済み subscriber へ
  `tracing::warn!` で畳み、非 hook は `!quiet` 時のみ `io.stderr` へ
  `warning:` 行。HELP literal に 2 flag を追記。
- instrumentation wave 1(T3)— `cheaper.cli.dispatch` span +
  汎用 catch の `warn!`(`error.code`)、各 command module の
  `cheaper.cli.command` span、`cheaper-runtime` の `cheaper.run` +
  8 stage 子 span(`tool_runtime`/`model_resolve`/`tool_scope`/
  `session`/`prompt`/`extract`/`repair`/`meta`)、`on_event` の
  ToolCall/ToolResult/CompletionCall で metadata-only `debug!`。
- instrumentation wave 2(T4)— `cheaper.tool` span
  (`ToolRuntime::execute` 単一 instrumentation 点)+
  `cheaper.tool.result`/`cheaper.tool.process` event、
  `cheaper.workflow` span(`run.id`)+ node event、
  `cheaper.provider.devin` event(credential load・cli start/end・
  stream connect/end/error)、`cheaper.adapter.session` span +
  `cheaper.adapter.run` の RunEvent event(TextDelta は `text.len`
  のみ)。
- instrumentation wave 3(T5)— `cheaper.hook` span + 完了 `debug!`
  (`verdict`・`stdout.len`/`stderr.len`)、`cheaper.auth.store`/
  `cheaper.auth` span(`op` field で login/logout/status/resolve
  識別)、`cheaper.catalog` event(merge/persist)、`cheaper-core`
  4 file の `debug!`/`trace!`(resolved name/choice・`file.name`
  basename のみ)。
- contract test + benchmark 拡張(T6)— `tests/observability.rs`
  (non-seam、subprocess isolation で activation matrix・dual sink・
  format・`--quiet`・repeat-init・hook stderr contract・auth/catalog・
  retention m+1・`--cwd` anchoring・log root fail-open を assert)と
  `tests/observability_dev_seam.rs`(`required-features =
  ["dev-seam"]`、secrets 非記録 scan・stage timing・tool internals)。
  benchmark harness に `CHEAPER_LOG*` env passthrough・per-arm env 注入
  (`armBase`/`armEnv`)・3 logging arm を追加し、off-state overhead を
  `benchmarks/results/4eda3caf-…json` に記録。
- post-review fixup — `init` で subscriber install を `log` bridge
  (`LogTracer`)より先に行い bridge 失敗を warnings 化(4284cfc)、
  `EnvFilter` を `cheaper=<level>` に scope し third-party record の
  流出を遮断(ce2d3e4)、test の ambient `CHEAPER_LOG*` 継承を遮断
  (03e3f4f)、secrets scan の stdin marker を run pipeline 実経路へ
  通すよう実質化(57f9de5)。

### Commits

- `278035b` feat(observability): add cheaper-observability crate with dual-sink init
- `c12fbf6` feat(cli): wire observability init behind --log and CHEAPER_LOG
- `252c1a5` feat: instrument cli dispatch and run pipeline with tracing spans
- `f852a9d` feat: instrument tools, workflow, provider and adapter with tracing spans
- `35908ae` feat(observability): instrument hooks, auth, catalog and core boundaries
- `80e8eee` test(observability): add contract tests and benchmark logging arms
- `4284cfc` fix(observability): install subscriber before log bridge in init
- `57f9de5` test(observability): route stdin marker through run pipeline in secrets scan
- `3acbbbb` test: record observability off-state overhead measurement
- `ce2d3e4` fix(observability): scope log EnvFilter to cheaper targets
- `03e3f4f` test(cli): stop inheriting ambient CHEAPER_LOG in env lookups

### Decisions During Implementation

- **emit-only `tracing` dep の一元化**: leaf crate には
  `tracing.workspace` のみ追加し `cheaper-observability` への依存は
  `cheaper-cli` に限定(ADR-034DUFzrXJxOwfl1qveXWR の emit/subscriber
  分離)。runtime への `tracing` 追加は T3 が担い T5 の runtime file は
  同 dep を再利用。
- **`cheaper.tool` 単一 span + `tool.name` field**: per-tool 動的 span
  名は `span!` の `'static` 制約と相性が悪いため、`ToolRuntime::execute`
  を全 tool call の単一の正 instrumentation 点とし個別 tool file には
  散らさない。
- **in-span event の命名規約**: T3 は plain `debug!`/`warn!`(span
  context が subsystem 識別を担う)、T4/T5 は design 名を `target:` で
  名付ける形が混在 — 規約の不統一として記録済みで、contract test の
  `named()` は両形を受理する。
- **`op` field による op 識別**: `cheaper.auth`/`cheaper.auth.store`/
  `cheaper.catalog` は span・event 名が固定のため
  `op="login"/"logout"/"status"/"resolve"/"load"/"save"/"merge"/
  "persist"` field で op を識別。
- **`EnvFilter` の `cheaper=<level>` scoping**: bare global directive
  では third-party(reqwest/ureq/hyper/h2/rig-core、`log` bridge 経由
  含む)の record が sink へ流出していた。共通 prefix `cheaper` の
  prefix match 1 directive で全 cheaper emit site(module-path
  `cheaper_*::*` と明示 `cheaper.*`)をカバーし、unmatched target は
  default directive ゼロで disable — metadata-only contract を構造的に
  保った。
- **dev-seam 経路の assert 範囲**: `cheaper.adapter.*` は fake-stream
  fixture が session factory を差し替えるため seam 経路では emit されず、
  workflow/devin は deterministic run 構築コストが大きい — TSPEC の必須
  assert 対象(tool/auth/catalog/hook/cli/run)のみ固定した。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo build --workspace` / `--all-targets` — PASS。
- `mise exec -- cargo test --workspace` — PASS(61 test binary、dev-seam
  target は required-features で skip)。
- `mise exec -- cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — `--no-fail-fast` で
  `run_fixture_test::fixtures_replay_all` の `hang_grace` fixture 1 件の
  み fail(pre-existing の abort 分類 bug、issue #94)。同 test は単独
  再実行で全 pass、その他全 binary pass。本変更とは無関係。
- `mise exec -- cargo test -p cheaper-cli --test observability` —
  PASS(16/16)。`--test observability_dev_seam --features
  cheaper-cli/dev-seam` — PASS(2/2、secrets scan 実 leak 0 件)。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  PASS。`cargo fmt --all -- --check` — PASS。
- `node scripts/check-lockfile.mjs` — PASS。`node --test
  benchmarks/test/ scripts/test/` — PASS(42/42)。
- smoke — `cheaper --log debug models` で `.cheaper/logs/` 生成、flag
  無しで `.cheaper` 未作成、`--log bogus` で fail-open warning、
  hook 経路は stdout/stderr 完全沈黙 + file sink のみ。
- off-state overhead — current build(80e8eee)の release binary で
  off/file/debug 3 arm × 17 observation を計測(median: off 167.3ms /
  file 163.4ms / debug 164.0ms、base 159.5ms — p95 の散らばり内で
  logging 有効化の追加 overhead は観測不能)。
- `mise run check` — 構成コマンドは個別に全 pass 済みだが、上記
  `hang_grace` flake が dev-seam 付き workspace test で発火するため
  未実行のまま(通すと fail し得る)。issue #94 側の修正後に再実行要。

## Risks

- pre-instrumentation の benchmark baseline((a) instrumentation 導入前
  HEAD)は未取得 — off-state overhead は current build 間の比較のみで、
  (a)→(b) delta は未検証。
- `hang_grace` fixture が dev-seam 付き workspace test でほぼ確定的に
  fail する環境があり、`mise run check` が通らない。`setup_abort_error`
  が setup 区間で観測された `AbortReason::Signal` を INTERNAL_ERROR に
  wrap する pre-existing bug — issue #94 として起票済み・本 scope 外。
- `cheaper.workflow`/`cheaper.provider.devin`/`cheaper.adapter.*` の
  event は emit 配置済みだが contract test で固定されていない
  (TSPEC の必須 assert 対象外)。
- dispatch 汎用 catch の `warn!` は dispatch span close 後に emit され
  span context を持たない(`error.code` + message で識別可能 — cosmetic)。

## Follow-ups

- issue #94(hang_grace の abort 分類)の修正 — 修正後に `mise run
  check` の全経路実行と dev-seam 付き workspace test の再検証を行う。
- pre-instrumentation baseline が必要になった場合、pre-T1 HEAD の別
  checkout で (a) arm を計測して `benchmarks/results/` へ追記する。
