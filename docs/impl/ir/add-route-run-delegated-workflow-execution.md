---
id: "IMPL-034Jk79Ki8cC6wYBZZ5VII"
type: "impl"
status: "completed"
title: "Add route run delegated workflow execution"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
        note: "payload revalidation + RouteExecution + execute(D1)"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
        note: "routed exec integration tests(14 cases)"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
        note: "run module 追加"
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
        note: "new_run_id pub(crate)化 + writer tmp_path accessor"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
        note: "record_route が append 成否を bool で返す"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_context.rs"
        note: "from_policy + load_workflow_from(宣言層 file root)"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
        note: "execute_compiled 抽出 + execute_routed + route_exec 注入"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
        note: "command details schema に logRef/outputTokens/outputTokensTruncated"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
        note: "mod route_run 宣言"
      - type: "file"
        path: "docs/tasks/add-route-run-delegated-exec.md"
        note: "status in-progress + Verification command 追記"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-run-delegated-exec.md"
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

# Add route run delegated workflow execution

## Summary

D1 — `route::run::execute` が payload を decode + fail-closed
revalidate(`prepare`)し、宣言層 dir を file root に workflow を
load、projected policy で in-process `run_workflow` を実行する。
`RouteExecution` が tee writer・postprocess failure stage・capture
した exec canonical result を保持し、exec 完了直後に log finish・
token 計測・details enrich(logRef/outputTokens/
outputTokensTruncated)を行う。完走後は capture 済み real
execResult から exitCode を mirror し、bounded text・footer・
envelope・route usage record を返す。CLI arm の dispatch 配線は
D4、D2 系の worker model 検証は次 increment。

## Context

docs/tasks/add-route-run-delegated-exec.md /
docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md
Phase D1 / design §`route run` (R14・C3・C9)。

## Implementation

### Changes

- `crates/cheaper-cli/src/route/run.rs` — `PreparedRoute{supply,
  resolution, route, extra_env}` と `prepare(payload, cwd, env,
  builtin_dir)`。`supply::load` を invocation cwd で解決し、
  canonicalize した `payload.cwd` が `supply.root` と一致することを
  要求(cross-project policy replay の拒否)。`tokenize_shell` で
  単一 nonempty safe segment を要求し、`split_env_prefix` で再導出
  した prefix を除く token 列が `payload.argv`(nonempty・
  env-prefix 不含)と完全一致することを検証。`resolve::resolve` を
  `payload.argv` + canonical payload cwd + `supply.command_view()`
  で再評価し `matched && Runnable` を要求。matched route は
  command-prefix 最長一致 + 解決済み route 名で特定し、
  `payload.env` ⊆ `route.passthrough_env` を再検証。`extra_env` は
  ambient `payload.env` → env-prefix の順で merge(後勝ち)。
  `project_grant` で resolved command policy を route grant 1 件へ
  投影。`payload.executable`/`args` は実行判断に一切使わない。
  全 revalidation 失敗は `InvalidRequest`(payload 由来)/
  `PolicyViolation`(resolution 由来)の generic message + static
  `reason` detail で、command/env/値を diagnostics に含めない。
- `crates/cheaper-cli/src/route/mod.rs` — `pub mod run;`。
- `crates/cheaper-cli/src/route/run.rs`(D1 next)—
  `RouteRunOutput{envelope,text,exit_code}` と `pub async fn
  execute(token, io, hooks, started_at)`。`payload::decode` →
  `prepare` → `compose::resolve_workspace_layout` で宣言層 file
  root を選択(Builtin=`builtin_dir/workflows`・User=
  `user_config_dir`・Project=`project_dir`、canonicalize 済み)→
  `WorkflowContext::from_policy`(root は `supply.root` のまま・
  projected policy に `execution.timeout_ms = grant timeout +
  120_000` のみ変更)→ `RouteExecution` 構築(log sweep/begin
  失敗は stage 記録のみ)→ inputs は `{grant,executable,argv,args,
  execArgs,cwd,tokenThreshold,runId}`(re-resolution 由来・
  `command`/`env` は workflow inputs に入れない)→
  `execute_routed` → `finish` で exit mirror・fallback/result
  text・footer(`[cheaper route log: <id-or-unavailable>;
  truncated: <bool>]`)・`postprocessFailures`・envelope・
  `record_route` 呼出し。`RouteExecution` は tee(Write::write_all・
  失敗は 1 度だけ `log-write`)・count cap・expected exec input・
  captured result・logRef を保持し、`complete` で flush →
  `take(cap+1)` bounded read → `count_text_tokens` → `finish` →
  details enrich → enriched clone 保存の順に処理する。
- `crates/cheaper-cli/src/workflow_context.rs` —
  `from_policy(root, dirs, policy)` constructor(prepare_workflow
  _context の最終構築を委譲)と `load_workflow_from(path,
  file_root)`(`load_workflow` は self.root で delegate。denylist・
  inference root は不変)。
- `crates/cheaper-cli/src/workflow_run.rs` — Shared 構築・signal
  relay・`run_workflow`・cleanup finally を `execute_compiled` へ
  抽出(`execute_run_inner` の振る舞いは不変)。`execute_routed` は宣言層
  `load_workflow_from` → routed convention 検証(`jobs.exec` が
  kind tool/tool command・`when` 無し・`outputs.execResult` が
  `{$from:"exec"}` whole-result binding)を job 実行前に行い
  compile → `execute_compiled`。`Shared.route_exec` 経由で `exec`
  job の command tool のみ route context(extra_env・exec_cwd・
  accept_any_exit_code・log tee)を注入し、materialized input が
  `{name: grant, args: execArgs}` と完全一致することを要求。
  ArtifactPorts.reader は composed reader(runId 形 ref →
  RouteLogReader、他 → store reader)で worker scope へも継承。
- `crates/cheaper-tools/src/tools/command/mod.rs` —
  `command_details_schema` に optional `logRef`・`outputTokens`・
  `outputTokensTruncated`(required・additionalProperties 不変)。
- `crates/cheaper-cli/src/route/log_store.rs` — `new_run_id` を
  `pub(crate)` 化、writer に `tmp_path` accessor。
- `crates/cheaper-cli/src/usage_log.rs` — `record_route` が
  append 成否を bool で返す(stderr 行動は不変)。
- `docs/tasks/add-route-run-delegated-exec.md` — status in-progress
  + `route_run::` integration test command を Verification へ追記。

### Decisions During Implementation

- `#[allow(dead_code)]` は `execute` が `prepare` を使うため除去済み
  (CLI arm の dispatch 配線自体は D4)。
- error code の棲み分け: payload 構造・内容の不正(cwd mismatch・
  compound/unsafe command・argv 不一致・allowlist 外 env)は
  `InvalidRequest`、route resolution 状態(no-match・非 runnable・
  grant 不在・route 不整合)は `PolicyViolation`。
- unit test は real temp builtin/user/project file + pinned test
  executable(`std::env::current_exe`)の真の `supply::load` を
  使い、policy check を mock しない。
- PreparedRoute は Debug を derive しない(RouteSupply 非 Debug —
  既存 struct への derive 追加を避け、test は `.err().unwrap()`
  を使用)。
- `outputs.execResult` の whole-result binding は schema が
  `$from`+`path` を必須とするため `{"$from":"exec","path":""}`
  (path 空 = whole result)を受理 — path 投影を持つ binding は
  convention 違反(`execResult-output`)として拒否する。
- failure stage 名は固定文字列 + `job-failed:<id>`・
  `output-error:<name>`(job 名・output 名は workflow 内部で
  非 sensitive)。`usage-record` は append 失敗時のみ。
- `execute_routed`/`finish`/`CompiledRun`/`RoutedRun`/`FinishArgs`
  は params struct + lifetime 分割で `too_many_arguments` 回避。
- fixture child は `route_run::command_fixture --exact --nocapture`
  を current_exe pin で spawn(no-op unless
  `CHEAPER_ROUTE_CHILD_MODE`)。ambient env の global 変更不要。

## Related Experiments

### Adopted

### Rejected

## Validation

- D1 first increment red: `unimplemented!()` stub で 10/10 fail
  (`tmp/handoff-d1/red.txt`)。
- D1 first increment green: `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib
  route::run` → 10/10 pass(`tmp/handoff-d1/green.txt`)。
- D1 next increment: `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration route_run::` → 12/12 pass
  (`tmp/handoff-d1-next/route-run-green.txt`)。
- regression: `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test workflow`
  → 18/18 pass(`tmp/handoff-d1-next/workflow.txt`)。
- review-fix red: finish() の engine-Err drop・infra-failure exit
  mirror 検証が新規 test で fail
  (`tmp/handoff-d1-next/red2-lib.txt` 4 fail /
  `tmp/handoff-d1-next/red2-integration.txt` 1 fail)。
- review-fix green: `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration route_run::` → 14/14 pass
  (`tmp/handoff-d1-next/route-run-green2.txt`)。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::run`
  → 15/15 pass(`tmp/handoff-d1-next/lib-route-run2.txt`)。
- `mise exec -- cargo fmt --all -- --check` → clean
  (`tmp/handoff-d1-next/fmt2.txt`)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` → warning 0
  (`tmp/handoff-d1-next/clippy2.txt`)。

## Risks

- ~~SDP deterministic gate fail~~ → 解消済み(2026-10-21)。
  `.sdp/skill-reference-inferences.json` 欠落を `sdp infer init` +
  `infer apply --ops .sdp/infer-ops.jsonl` で再生成したが、
  `infer-ops.jsonl`(37 ops)が 8 新規 skill(cheaper-bulk-read /
  cheaper-code-write / cheaper-doc-write / cheaper-usage /
  doc-dashboard / doc-maintenance / review-protocol /
  test-spec-doc)を覆わず `sdp profile` が pending review で拒否。
  lead 提供 metadata で `infer set-skill` 8 件 upsert →
  `infer check` → `profile` → `validate` 全 4 gate pass
  (`.sdp/implementation-flow-default/validation-report.json`)。
  `.sdp/` は gitignored のため tracked artifact への影響なし。
- `worker_parity_and_route_log_read` は worker 起動時の policy
  parity と、`{$input:runId}` 動的 binding の `kind:tool` read
  job による現 run log の読み出しを検証する。worker 自身も
  seeded log の read を試みるが、fake-stream は tool error 後も
  固定結果を返せるため、この test だけでは worker 内 read の
  成功までは証明しない。ArtifactPorts の継承はコードレビューで
  確認し、worker 内の成功観測は E の統合検証で補う。

## Follow-ups

- D1 の Done When は検証済み。worker 内 log read の成功観測は
  E の統合検証で補う。
- D2: builtin routed-exec workflow resource 等の宣言 resource。
- D4: `route run --payload` CLI arm の dispatch 配線。
