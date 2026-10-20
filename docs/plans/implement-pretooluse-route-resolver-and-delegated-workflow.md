---
id: "PLAN-034Jcl2O612UZMGTK2IN1n"
type: "plan"
status: "completed"
title: "Implement PreToolUse route resolver and delegated workflow execution"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/test-specs/route-run-delegated-exec.md"
    - "docs/test-specs/command-exitcodes-and-options.md"
    - "docs/test-specs/workflow-when-condition.md"
    - "docs/test-specs/route-logs.md"
    - "docs/test-specs/hook-dedup-route-fingerprint.md"
    - "docs/test-specs/route-usage-records.md"
    - "docs/test-specs/builtin-route-resources.md"
    - "docs/test-specs/hook-route-e2e.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement PreToolUse route resolver and delegated workflow execution Implementation Plan

## Goal

`PreToolUse` hook で発火した shell command を、`routes.yaml` に宣言された
route 経由で登録 workflow(`routed-exec`)へ rewrite し、原 command の
exit code・cwd・env 互換を維持したまま bounded output + route log + usage
record を返す自動ルーティングを実装する。

## Scope

1 つの実装ストリーム — route 解決・hook verdict・実行経路・log/usage は
同じ critical path(route resolve → Rewrite → route run → log/usage)に
乗り、分割すると feature 単体の検証境界を失う。workflow `when`・command
`exitCodes`・proc tee は本 feature の前提 primitive として同一 plan で
先行実装する(それぞれ独立検証可能だが、本 feature なしに出荷意味を
持たない)。

Non-goals: Codex/Cursor/Copilot への Rewrite 発行(approval-bypass 検証
待ち issue #150)、複合 shell command routing、artifact URI 統合、worker
からの recursive delegation、汎用 `kind:exec` executor。

## Upstream Documents

- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`(R1-R40)
- `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`(D1-D13)
- `docs/discovery/pretooluse-auto-routing-to-tasks-and-workflows.md`

## Design Inputs

- `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
  (DESIGN-034JbqoCWm85pKC033fgfd、approved) — 全構造決定の canonical
  source。route module 配置・`Verdict::Rewrite{command,fallback}`・
  `args`/`exec_args` 区別・payload revalidation・単一 grant 投影・
  `execResult` 基準 exit mirror・`RouteLogReader`・`when` 演算子意味論・
  dedup 非対象化は design の記述どおり実装する。

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `crates/cheaper-cli/src/route/mod.rs` | `route` module 集約・CLI arm dispatch | 新規 |
| `crates/cheaper-cli/src/route/routes.rs` | routes.yaml schema 検証・3 層 load・trust/shadow・重複 `invalid-route` 化 | 新規 |
| `crates/cheaper-cli/src/route/resolve.rs` | argv → match → viability の共通 resolve(`RouteResolution`)・`route resolve` dry-run | 新規・3 caller 共有 |
| `crates/cheaper-cli/src/route/run.rs` | `route run` — payload decode・revalidate・route context 合成・in-process workflow 起動・exit mirror | 新規 |
| `crates/cheaper-cli/src/route/logs.rs` | `route logs <id>` — runId 検証 + bounded read + `route-refetch` 記録 | 新規 |
| `crates/cheaper-cli/src/route/log_store.rs` | `<userConfigDir>/route-logs/` temp write + rename・`0600`・retention sweep・runId 採番・`RouteLogReader` | 新規 |
| `crates/cheaper-cli/src/route/payload.rs` | payload schema・base64url codec・allowlist subsetting の単一 owner(encoder/decoder skew 防止) | 新規 |
| `crates/cheaper-cli/resources/routes.yaml` | builtin route 宣言 | 新規 |
| `crates/cheaper-cli/resources/workflows/routed-exec.workflow.json` | routed 実行の参照 workflow(exec + conditional analyze) | 新規 |
| `crates/cheaper-cli/resources/workers/condense.md` + `schemas/condense.{input,output}.schema.json` | routed 出力縮約用の新規 builtin worker(現行 4 worker に縮約 worker は非存在) | 新規 |
| `crates/cheaper-cli/src/lib.rs` | `route` module 宣言(dispatch 本体は cli.rs) | 変更 |
| `crates/cheaper-cli/src/hook/mod.rs` | hook entry で effective routes + command policy を 1 firing 1 回 load、`HookDeps`/`EvaluateOptions` 経由で eval へ供給 | 変更 |
| `crates/cheaper-cli/src/hook/routing_policy.rs` | `Verdict::Rewrite{command,fallback}`・`evaluate_shell` route arm | 変更 |
| `crates/cheaper-cli/src/hook/targets.rs` | `HookTarget::Codex` variant・claude/devin `updatedInput` render・codex 含む他 target の fallback render | 変更 |
| `packages/apm-hook-codex/.apm/hooks/hooks.json` | codex 登録 command を `--target pretooluse` → `--target codex` へ分離 | 変更 |
| `crates/cheaper-cli/src/hook/eval_cache.rs` | `VariantInputs::Shell` routes hash・Rewrite 非 cache | 変更 |
| `crates/cheaper-cli/src/usage_log.rs` | `kind:"route"`/`"route-refetch"` record builder | 変更 |
| `crates/cheaper-cli/src/usage_report.rs` | route kind 独立集計 bucket | 変更 |
| `crates/cheaper-cli/src/workflow_run.rs` | `route run` からの in-process 起動経路共有 | 変更 |
| `crates/cheaper-cli/src/args.rs` / `cli.rs` | `route` subcommand wiring・`resolve_log_root`/`command_name` 対象追加 | 変更 |
| `crates/cheaper-cli/src/embedded.rs` | builtin routes.yaml・routed-exec workflow embed | 変更 |
| `crates/cheaper-core/src/policy/command.rs` | `command.grants.<name>.exitCodes` schema(default `[0]`) | 変更 |
| `crates/cheaper-tools/src/invocation.rs` | route context scope option(`extra_env`/`exec_cwd`/`accept_any_exit_code`/`log_tee`)・`RouteLogReader` port 継承 | 変更 |
| `crates/cheaper-tools/src/tools/command/mod.rs` | `exitCodes` result 化・route option 適用・route mode `outputTokens`/`logRef` details + **`pub fn check_command_execution`(仮)** — grant 解決・pin・denied・cwd 事前 check を `command::execute` と同一 predicate で行う公開 seam(viability 共有用、`pub(crate)` 関数群の複製を防ぐ) | 変更 |
| `crates/cheaper-tools/src/registry.rs` | `ctx.command()` への route option 注入経路 | 変更 |
| `crates/cheaper-tools/src/tools/proc/mod.rs`・`monitor.rs` | 全量 output tee sink(optional `ProcTee`) | 変更 |
| `crates/cheaper-workflow/src/schema.rs` | job `when` predicate schema | 変更 |
| `crates/cheaper-workflow/src/types.rs` | `WorkflowJobState.condition_skipped`・`when` eval 型 | 変更 |
| `crates/cheaper-workflow/src/compiler.rs` | `when` の `references()` 参加・`$from` path 検証 | 変更 |
| `crates/cheaper-workflow/src/engine.rs` | `pump()` の `when` 評価・`CONDITION_SKIPPED`・dep 伝播除外・error 集約除外 | 変更 |
| `crates/cheaper-workflow/src/bindings.rs` | condition-skipped producer `$from` → `null` materialization | 変更 |
| `crates/cheaper-cli/tests/integration/route.rs` + `main.rs` | CLI contract test(`mod route;` 宣言追加) | 新規/変更 |
| `crates/cheaper-workflow/tests/compile_contracts/` | `when` schema/dep-edge contract(`main.rs` に `mod` 宣言) | 新規 module |
| `crates/cheaper-workflow/tests/engine.rs` | `CONDITION_SKIPPED` の pump/materialize/error 集約 engine test | 変更(独立 target) |
| `crates/cheaper-core/tests/golden/` | `exitCodes` schema 追加による golden fixture 再生成(必要時) | 変更可能性 |
| `docs/guides/routing.md`(仮) + `docs/README.md` | routes.yaml 記法・trust 境界・hook env ≠ shell env の user doc + index 更新 | 新規/変更 |

## Implementation Sequence

Phase 依存: A(foundation)→ B(route core)→ C(hook 側)→ D(実行)→ E(統合)。

### Phase A — 前提 primitive(並列可)

- [x] A1: `command.grants.<name>.exitCodes` schema + `command::execute` の
      集合内非 0 exit result 化(`state`/`exitCode`/`spawnCode` の実
      shape 判定)+ contract test。pass: 集合内非 0 が result data・
      集合外/spawn 失敗が Err の test が pass(+ 必要時 golden
      fixture 再生成)
- [x] A2: proc tee sink — `proc::run` の optional `ProcTee` byte sink +
      `CommandToolOptions.log_tee` + tee write 失敗の postprocess 縮退。
      pass: tee された全量 byte と bounded result の整合 test が pass
- [x] A3: route context scope option — `InvocationToolRuntimeOptions` 系
      へ `extra_env`/`exec_cwd`/`accept_any_exit_code` を追加し
      registry → `ctx.command()` へ注入 + **resolved
      `RuntimePolicy.command.grants` の単一 grant 投影**(Shared/
      invocation 構築前 — `load_policy` invocation 層・options raw
      JSON 注入では parity check が崩れるため、resolved policy 側を
      投影)。pass: scope option が command tool へ届く contract
      test + 投影外 grant を `input.name` で参照した job が失敗
      する test + worker job の parity check が通る test が pass
- [x] A4: workflow `when` — schema + compile `references()` 参加 +
      `pump()` 評価 + `CONDITION_SKIPPED` + materialization null +
      error 集約除外 + compile_contracts test。pass: when-false の
      skip が dependent へ伝播せず workflow failure に計上されない
      engine test が pass

### Phase B — route core

- [x] B1: `route::routes` — routes.yaml schema・3 層 load・`trusted`
      shadow・同一 `match.command` 重複 `invalid-route`・
      `target.path` 層起点 + traversal 拒否・`suppressed` provenance
      field。pass: unit test で 3 層 merge・shadow・重複・traversal
      拒否が全 pass
- [x] B2: `route::resolve` — `ResolveInput{argv, cwd, routes,
      command_policy}` の純粋評価(caller が load 責務を持つ)・
      self-bypass・prefix/longest match・`args`/`exec_args` 区別・
      viability 固定優先順・env-prefix skip・`INTERACTIVE_COMMANDS`
      curated list・`route resolve` dry-run CLI arm + cheaper-tools
      の `pub fn check_command_execution`(仮)公開 seam — `pub(crate)`
      な `pin_command_grants`/`prepare_cwd`/`minimal_env`/
      `native_executable` を複製せず `command::execute` と同一
      predicate で viability を判定する共有入口。pass:
      `route resolve --command/--argv` の contract test で
      viability 各分岐が pass + `check_command_execution` が
      `command::execute` の受理判定と一致する test が pass
- [x] B3: `route::log_store` — runId 採番・`<runId>.log.tmp` streaming +
      rename・`0600`・TTL/count/size sweep(`*.tmp` 除外)・
      `RouteLogReader`(`ArtifactReader` impl、ref 検証共有)。
      pass: unit test で write/rename・sweep・ref 検証拒否が pass
- [x] B4: `usage_log` `kind:"route"`/`"route-refetch"` builder +
      `usage_report` 独立 bucket 集計。pass: record shape の
      unit test + usage 集計で route record が
      `directlyComparable` 母数に混入しないこと

### Phase C — hook 側

- [x] C1: `Verdict::Rewrite{command,fallback}` + `evaluate_shell` 先頭
      route arm(env-prefix skip・`passthroughEnv` capture・
      `current_exe` anchor・base64url payload・Rewrite 非 cache)+
      `hook/mod.rs` で firing ごとの effective routes +
      command policy load(`HookDeps`/`EvaluateOptions` 拡張)。
      pass: hook eval の unit/contract test で Rewrite 発行・
      non-runnable fallthrough・fallback 保持が pass
- [x] C2: target 分離 + render — `HookTarget::Codex` variant 追加・
      `apm-hook-codex` hooks.json を `--target codex` へ変更・
      `pretooluse_render`(claude)`updatedInput`・`devin_render`
      `updatedInput`・`codex_render` + windsurf/cursor/copilot は
      `fallback` render・`verdict_name` `"rewrite"`。pass:
      per-target render contract test で codex が updatedInput を
      受け取らないことを pin
- [x] C3: `eval_cache` — `VariantInputs::Shell` routes hash 追加 +
      Rewrite verdict の publish/serve 除外。pass: dedup test で
      routes 変更時 invalidation と Rewrite 非 persist が pass

### Phase D — route run + resources

- [x] D1: `route::run` — `route::payload` で decode・re-resolve/
      revalidate・env-prefix 再導出・route context 合成(resolved
      `RuntimePolicy` 単一 grant 投影・`extra_env`/`exec_cwd`/
      `accept_any_exit_code`/`log_tee`)・in-process
      `run_workflow`(workflow file は **宣言層 dir を load
      root** とし、resource inference の workspace_root は
      payload cwd — builtin 層 target が workspace escape で
      load 不能にならないよう 2 root を区別)・run timeout =
      `grant.timeout` + post-processing headroom(+120s、R18
      clamp)・`execResult.details.exitCode` exit mirror・
      post-processing failure 分離・stdout compact result。
      pass: integration test で exit-code mirror・
      revalidation fail-closed・postprocessFailures 記録が pass
- [x] D2: builtin `routes.yaml` + `routed-exec.workflow.json`
      (analyze `when` = `{$from: exec,
      path:/details/outputTokens}` — `CanonicalToolResult.details`
      階層の pointer)+ `condense` worker 新設(md + input/
      output schema)+ `embedded.rs` materialize +
      `embedded_fingerprint` の resource 列挙を routes.yaml/
      workflows/ へ拡張(列挙漏れで .ready dir が新規 file を
      欠く staleness 防止)。pass: embedded materialize
      contract test + routed-exec workflow compile が pass +
      builtin_workers assertion 更新が pass
- [x] D3: `route::logs` CLI arm + runId 検証 + bounded read +
      `route-refetch` 記録。pass: `route logs` の offset/eof/
      invalid-id 拒否 contract test が pass
- [x] D4: `args.rs`/`cli.rs`/`lib.rs` wiring — `route` subcommand
      dispatch・`route run` は generic envelope/`write_output`
      経路を bypass し bounded command output を stdout へ生
      emit(`--format json` は direct CLI 呼出専用)・
      `record_usage` 非対象・`resolve_log_root`/`command_name`
      更新・HELP text 更新。pass: `cheaper route --help`・
      dispatch smoke test が pass

### Phase E — 統合・文書

- [x] E1: `tests/integration/route.rs` — `route resolve`/`route run`/
      `route logs` CLI contract + hook 経由同一 contract 固定
- [x] E2: user-facing doc(routes.yaml 記法・routing≠authorization・
      trust 境界・hook env ≠ shell env・exit-code domain・fallback
      挙動)
- [x] E3: `mise run check` 全体 green + usage 集計 join の統合確認

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `when` の engine 意味論変更が既存 workflow を regress させる | med | high | `CONDITION_SKIPPED` を diagnostic code で分離し DEPENDENCY_FAILED 経路不変、compile_contracts + engine test で pin |
| 2 | `current_exe` anchor が dev build で `target/debug/cheaper` を rewrite 先にする | low | low | contract test で pin、失敗時 bare name fallback(design deferred) |
| 3 | Codex/approval-bypass 未検証で rewrite を誤発行 | med | high | `HookTarget::Codex` 分離で claude pretooluse のみ発行、codex は `fallback` render(構造防止) |
| 4 | payload 改竄・env 密輸 | med | high | payload env ⊆ allowlist 再確認 + env-prefix 再導出 + 単一 grant 投影 + revalidation 失敗時 raw 実行禁止 |
| 5 | route log の任意 file read | low | high | runId regex 検証を `route logs` と `RouteLogReader` で共有、`target.path` traversal 拒否 |
| 6 | `exitCodes`/`accept_any_exit_code` で非 0 の error/result 境界を誤る | med | med | monitor.rs の実 shape(`state`/`exitCode`/`spawnCode`)を判定条件として test pin |
| 7 | 外部依存 — なし | — | — | 新規 crate 依存は設計に無し |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step(マイグレーション・外部 format 固定)は無い — route log/
usage record は best-effort append で旧 record の解釈変更をしない。

## Task Breakdown

Implementation Sequence の A1-E3 が task doc 候補 — 各項目は単一 action
集合(schema + engine 改修のような複合項目は task-doc 側で test 先行・
実装・検証へ分割する)。依存: A1-A4 並列可。critical path は
B1→B2→C1→D1。追加依存: A1→A3(`accept_any_exit_code` は A1 の
result 化機構の上に成立)・A1→D1(exit mirror 同)、A2→D1
(`log_tee` 注入)・A3→D1(scope options+投影)・A4→D2(`when`
が routed-exec 必須)・B1→D2(builtin routes.yaml schema)・
B3→D1(`log_tee`/runId/`RouteLogReader` 装着)・B3→D3、
B4→D1(route record)・B4→D3(route-refetch)、C1→C2/C3
(Rewrite variant 先行)、D2→D1(D1 の integration pass は
builtin resource の load を前提)。D4 は D1-D3 統合、E1-E3 は
最終。`route::payload` は C1(encoder)・D1(decoder)の共有前提
— B2 に隣接して先に作る。

## Verification

- [x] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 全 green
- [x] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      route::` で route CLI contract 全 pass
- [x] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      compile_contracts` で `when` schema/dep-edge 全 pass
- [x] `mise run check` 全 green(biome/fmt/clippy/version/lockfile/
      payload)
- [x] spec R1-R40・ADR D1-D13 の Acceptance Criteria が全て task/test
      spec の検証条件へ写像されていること
- [x] hook eval 内の resolve が `route resolve` CLI の出力と同一で
      あり、hook 発行の `route run` payload が direct CLI の
      `route run --payload` と同一 contract で処理されること
      (`route logs` は hook 経路を持たない CLI 専用)

R1-R40 / ADR D1-D13 の検証対応の詳細は
[IR verification map](../impl/ir/add-route-integration-tests.md#spec--adr-verification-map)
を参照。最終 canonical gate の抜粋・両 log digest・merge 実績・
再実行手順は
[IR Final canonical verification / Reproducibility](../impl/ir/add-route-integration-tests.md#final-canonical-verification)
を、graph/audit evidence は
[IR Graph / audit evidence](../impl/ir/add-route-integration-tests.md#graph--audit-evidence)
をそれぞれ参照できる — tmp/ の raw log や JSON projection は
参照前提ではなく、当時の結果・再実行手順を参照できる。

## Open Questions / Gaps(実装時に解決済み)

- Devin `updatedInput` field 名の正式 contract — 公式 docs で
  `{hookSpecificOutput:{hookEventName:"PreToolUse",updatedInput:
  {command:"..."}}}` の nested schema が確認され、Claude と共通の
  `hook_specific_output_rewrite` helper で実装。unit + integration
  test が nested shape・decision/permissionDecision 不在を pin 済み。
  参照: https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks
- `condense` worker の定義内容 — builtin resource として
  `condense.md` + input/output schema を embedded 済みで read-only。
  prompt の変更は承認済み scope 外。
- routed-exec の `tokenThreshold` default 値 — builtin routes.yaml の
  `output: {tokenThreshold: 8192}` で確定。

## Self-Review

- [x] Every spec requirement maps to a task.(R1-R40 が Phase A-E のいずれ
      かに割当済み — R1-R8→B1/B2、R9→B2、R10-R12→B2、R13-R16→D1、
      R17→D1、R18→D1/A3、R19-R20→A1、R21-R25→A4、R26→A2/B3/D1、
      R27-R31→B3/D3、R32→C1/D1、R33→B2/D1、R34-R35→B4/D1、
      R36-R39→C1/C2、R40→C3)
- [x] No placeholders remain.(Gap は Open Questions へ明示)
- [x] File paths, relations, status values, and terminology are
      consistent.(design manifest と同一 path 集合)

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation —
  Phase A の 4 項目(A1-A4)は相互独立で並列実装可能、B1-B3 も並列可。
- Independent work streams: A1, A2, A3, A4(並列); B1→B2, B3, B4
  (B1 後に B2/B3/B4 並列); C1-C3 は B2 後、D1-D4 は C 後、E は最終。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 焦点: Phase A-D の分割が design の component 境界(route::*/hook/*
  /tools/*/workflow/*)と一致するか、依存順序が design の前提
  (resolve 共有→Rewrite→run)と矛盾しないか。
- 明示仮定: `when` は本 feature 内でのみ使用する新規 engine 機能
  (既存 workflow への後方互換は diagnostic code 分離で維持)。
- 未解決依存: Devin hook response schema(issue #148)は D 段階までに
  block しない — render 層の field 名のみに影響。
- 上流: SPEC-034J7SxWvGL3oSbeBpo8a3 / ADR-034J8eZHNPqARi25HLFOL6 /
  DESIGN-034JbqoCWm85pKC033fgfd。
