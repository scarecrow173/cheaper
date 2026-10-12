---
id: "IMPL-0BrKmzUw8KyfSdgUn1UtPX"
type: "impl"
status: "completed"
title: "command tool test coverage"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_command.rs"}
      - {"type":"doc","path":"docs/impl/ir/command-tool-test-coverage.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-cli/tests/workflow.rs"}
      - {"type":"code","path":"crates/cheaper-cli/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/resource_inference.rs"}
      - {"type":"doc","path":"docs/tasks/command-tests.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/test-specs/policy-gated-command-tool.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/command-tests.md"
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

# command tool test coverage

## Summary

T5。`command` tool の包括的 test coverage を追加し、issue #101 の AC1–AC6 を
行為レベルで固定した。新規 `crates/cheaper-tools/tests/proc_command.rs`
(25 tests)は `proc_process.rs` と同じ fixture-child re-exec パターンで実
subprocess を spawn し、grant 実行・認可拒否・lifecycle 全系・独自 pool・
Exclusive claim を網羅。加えて workflow `kind: tool` e2e(`workflow.rs`
3 件)、worker 経路 fake-stream e2e(`run_dev_seam.rs` 2 件)、workflow
resource inference の command manifest/underclaim 固定(`resource_inference.rs`
1 件)を追加。既存 `process` test・golden は無変更のまま全 pass。

## Context

TASK-034EOHY1ZKj6PcEy0zVz2P / PLAN-034EO1kmKwJ8MAHSHWyCws /
TSPEC-034EOCRqkrPUFcQLHVN5I3。T1–T4(b706188・2cb6965・9445a64・4cf0e2a)で
実装は完了済み。T5 の責務は行為保証の固定:信頼境界(trusted layer grants の
みが executable を authorize)、fail-closed 経路、managed-process lifecycle、
resource scheduling/usage 計上の継続。

## Implementation

### Changes

- `crates/cheaper-tools/tests/proc_command.rs`(新規、25 tests —
  Windows 24 pass、`pinned_resolved_basename_deny_rejects` は `#[cfg(unix)]`
  で POSIX CI が実行)。fixture child `command_fixture_child` は
  `CHEAPER_PROC_CHILD_MODE`/`CHEAPER_PROC_CHILD_ARG` を grant `env` 経由で
  受け取る(command spawn は minimal env — 親 env 非継承)。mode 群:
  `inspect`(cwd/args/mode を `@@` sentinel JSON で出力)、`output`(N bytes
  stdout + stderr)、`fail`(exit 7)、`hold`(gate file 待ち — pool cap 用
  release gate)、`probe`(marker file)、`stubborn`(pid 書いて park —
  kill 対象)、`tree`/`child`(process-tree cleanup 対象)。coverage 対応:
  - registration gate: `nonempty_grants_gate_and_dormant_runtime`(空
    grants → `POLICY_VIOLATION`、`Command` 非含有 runtime は dormant、
    pin 不能 grant も dormant なら構築可)、`write_scope_rejects_command`。
  - deny 3 適用点: `declared_denied_executables_fail_policy_resolution`
    (interpreter・exec wrapper/loader・shell/shim の declared basename →
    `INVALID_RUNTIME_POLICY`)、`pinned_resolved_basename_deny_rejects`
    (symlink で allowed declared name → denied resolved basename →
    `POLICY_VIOLATION`)、`spawn_time_revalidation_detects_swapped_executable`
    (pin 後の内容差替え → spawn 前再検証 `POLICY_VIOLATION`)。
  - workspace-resident: relative declared path(pre-filter)と realpath 後
    inside-workspace 検査の両方。
  - model input: schema が `{name,args}` のみ公開・`executable`/`cwd`/`env`
    余剰 key・型違反・args 超過 → `INVALID_INPUT`、name pattern 違反 →
    `INVALID_INPUT`、unknown grant → `POLICY_VIOLATION`(details `{grant}`)。
  - argv: literal 不一致 → `POLICY_VIOLATION`、prefix(`--ex*`)/trailing
    rest(`*`)の適合実行、metachar 8 種 → `POLICY_VIOLATION`(rest marker
    適合前の独立 check)。
  - lifecycle: 成功 details `{grant,argv,state,exitCode,signal,durationMs,
    stdout,stderr}` + `command {grant}:` display prefix、grant `cwd` 解決、
    非零終了 `TOOL_EXECUTION_FAILURE` + failed details、per-grant timeout
    → `TIMEOUT` + pid 実 kill、call-scope abort → reason message 伝播 +
    kill、abort で parent+descendant 両方 kill(process-tree cleanup)、
    `maxOutputBytes` combined cap(`capturedBytes` 合計 = cap・`truncated`)。
  - scheduling: 独自 pool `"command"`(`maxConcurrent: 1`)が 2 個目の
    command op を `Queued` に留める、`command` の Exclusive root claim が
    mutating sibling(`write`)を `Queued` に留める。
- `crates/cheaper-cli/tests/workflow.rs`(+4): fixture child
  `workflow_command_fixture_child` + 3 tests。
  - `workflow_run_command_grant_end_to_end` — user layer `policy.yaml` に
    self-exec grant(argv literal pin + env)→ `kind: tool` job が
    `tools.allow` admission → `create_worker_scope`(re-injected grants)
    → 実 subprocess で marker file 生成。`$from /details/state` =
    "completed"、`/details/exitCode` = 0 を output binding で固定。
    job `meta.changes.files == []`(command は journal しない)。
  - `workflow_run_command_denied_without_grants` — builtin 空 grants で
    scope 生成 `POLICY_VIOLATION` → job failed、envelope
    `TOOL_EXECUTION_FAILURE`(exit 6)。
  - `workflow_run_command_tool_not_allowed` — user policy `tools.allow:
    [read]` で admission `POLICY_VIOLATION "tool not allowed: command"`。
- `crates/cheaper-cli/tests/run_dev_seam.rs`(+3): fixture child
  `run_dev_seam_command_fixture_child` + 2 tests。
  - `run_fake_stream_command_grant_executes_and_marks_not_comparable` —
    `tools: [command]` worker file + user grant + fake-stream
    `tool_calls` step で worker 経路の実 subprocess。marker 到達 +
    `meta.savings.directlyComparable == false` +
    `meta.changes.files == []`(child の実 marker は audited change と
    して主張しない spec 要件)+ `toolCalls == 1`。
  - `run_fake_stream_command_worker_denied_without_grants` — grants 無しで
    scope 生成 `POLICY_VIOLATION`(exit 4)、session 開始前 fail-closed。
- `crates/cheaper-workflow/tests/resource_inference.rs`(+1):
  `catalog_command_conservative_manifest_and_underclaim` — generic tool
  分岐が `command_grant_resources()`(root subtree Shared read +
  Exclusive write、`produces` 非宣言)を返す、`resources` omitted は
  generic fallback で同一形へ収束("resources-omitted" diagnostic)、
  declared `reads:[]`/`writes:[exact]` が `declared {bucket} underclaims
  known access`(`INVALID_REQUEST`)で fail-closed。
- `docs/tasks/command-tests.md`: status → done、Done When/Verification
  全 check、coverage notes(test ↔ guarantee 対応表 + 自動化不可の明記)
  を追記。
- `docs/test-specs/policy-gated-command-tool.md`: Guarantees が本 task の
  test 群で実装されたため status → approved。

### Verified-no-change(確認のみで変更なし)

- project-layer `command` 拒否(Option A 信頼境界)— T1 golden
  (`crates/cheaper-core/tests/golden/fixtures/policy/schema.json` の
  "project-partial rejects command section"/"project-partial rejects
  command authoring fields"、同 `load.json` "project command section
  rejected")で既に固定。
- pool `maxConcurrent` default 1 — `policy/command.rs`
  `defaults_when_absent`。
- deny set 語彙・`ArgPattern` compile/match・serde round-trip parity —
  `policy/command.rs` mod tests。
- `is_mutating_tool(Command)`・`is_directly_comparable_worker` — core
  `worker/validate.rs` tests(mutating set に Command を含む実装済み;
  dev-seam run test が end-to-end で `directlyComparable == false` を固定)。
- `process` tool — 全既存 test(`proc_process.rs`・`process.rs` 等)無変更
  で pass(AC1)。

### Decisions During Implementation

- **fixture path 引数は `env` 経由**: `process` fixture は argv で path を
  渡すが、`command` は grant `argv` を literal pin する設計のため、mode
  依存 path(gate/marker/pid file)は `CHEAPER_PROC_CHILD_ARG` env で渡す。
  結果として grant env が minimal env 経由で子へ届くこと自体も検証される。
- **fixture re-exec argv は `[<test名>, --exact, --nocapture]`**: libtest
  harness のため子 argv には `--nocapture` が残る — `inspect` mode の
  `args` 検証は `["--nocapture"]` を期待値にした(harness noise を
  正しく扱う形)。
- **resolved-basename deny test は `#[cfg(unix)]`**: declared≠resolved
  basename を作れるのは symlink のみで Windows symlink は権限依存。
  deny check の適用点自体は declared basename test が両 OS で固定。
- **`output_cap` は harness noise に耐える形**: libtest が子 stdout に
  ノイズを混ぜるため、raw bytes は `>=` 意図値、cap 検証は
  `capturedBytes` 合計 == cap + `truncated` flag で固定。
- **user-layer `policy.yaml` には `version: 1` が必須**: partial schema
  でも `version` required。初回 run で `INVALID_RUNTIME_POLICY` となり
  両 fixture helper に追加済み。

## Related Experiments

### Adopted

なし。

### Rejected

- workflow e2e を `workflow_dev_seam.rs` に置く案 — `kind: tool` job は
  model を呼ばないため非 gated `workflow.rs` で十分(dev-seam 依存を
  増やさない)。worker 経路のみ model 解決が必要なので dev-seam gated
  `run_dev_seam.rs` に配置。

## Validation

- `mise exec -- cargo test -p cheaper-tools --test proc_command` —
  24/24 pass(Windows;25 個目は `#[cfg(unix)]` で POSIX CI 実行)。
- `mise exec -- cargo test -p cheaper-cli --features dev-seam --test
  workflow` — command 関連 4/4 pass(全体 18/18 は workspace run で確認)。
- `mise exec -- cargo test -p cheaper-cli --features dev-seam --test
  run_dev_seam` — 10/10 pass。
- `mise exec -- cargo test -p cheaper-workflow --test resource_inference`
  — command test 1/1 pass(全体 22/22 は workspace run で確認)。
- `mise exec -- cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0(全 68 test
  target `test result: ok`、0 failed。`proc_command` 24、`workflow` 18、
  `run_dev_seam`(cli)10、`resource_inference` 22 を含む)。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` — exit 0。
- `mise exec -- cargo fmt --all -- --check` — clean(`cargo fmt --all` で
  初回差分を適用後)。

## Risks

- `pinned_resolved_basename_deny_rejects` は Windows では compile されない
  (cfg(unix))— Windows CI のみの環境では POSIX 側の symlink 経路未検証の
  まま。deny 適用点の語彙は declared basename test が両 OS で固定済み。
- fixture child は libtest harness 依存 — `--nocapture` 残余 argv や
  harness 出力ノイズを test が織り込み済みだが、将来 libtest の出力形式が
  変わると `inspect`/`output` mode の assertion が影響を受け得る。
- `alive()` の Windows 実装は `tasklist` 外部依存 — CI の Windows 環境で
  tasklist が無い場合 timeout/abort kill 検証が落ちる(現行 green)。

## Follow-ups

- T6: `docs/guides/configuration.md` builtin `policy.yaml` embed 同期・
  user-facing command guide(trust/sandbox 限界)。
- project 側 narrowing field は spec 通り deferred — 将来の workspace
  trust 設計待ち。
