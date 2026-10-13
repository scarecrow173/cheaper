---
id: "IMPL-034EOYv2cBtPBDWY0EMSiv"
type: "impl"
status: "completed"
title: "command core policy vocabulary and propagation"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"code","path":"crates/cheaper-core/src/policy/command.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/command/mod.rs"}
      - {"type":"doc","path":"docs/impl/ir/command-core-policy-vocabulary-and-propagation.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-core/src/tools/vocabulary.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/mod.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/process.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/schema.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/load.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/capability.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/policy/restrict.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/worker/validate.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/tools/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/catalog.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/registry.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/invocation.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/src/run/mod.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/src/dev/fixture.rs"}
      - {"type":"code","path":"crates/cheaper-cli/src/workflow_run.rs"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/policy_schema.rs"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/worker_validate.rs"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/vocabulary.json"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/policy/schema.json"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/policy/load.json"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/fixtures/worker/validate.json"}
      - {"type":"code","path":"crates/cheaper-runtime/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/tests/ollama_e2e.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/process.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/artifacts.rs"}
      - {"type":"code","path":"crates/cheaper-rig-adapter/tests/adapter.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_runtime.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/characterization.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/differential.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_invocation.rs"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/command-core-policy-and-propagation.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/command-core-policy-and-propagation.md"
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

# command core policy vocabulary and propagation

## Summary

T1。`ToolName::Command` を closed vocabulary の 9 個目として追加し、
`policy/command.rs` に grant schema・`resolve_command_policy`・
`ResolvedCommandPolicy`/`CommandGrant`・`ArgPattern`・deny set・
resource manifest・workspace-resident executable helper を実装した。
`PolicySchemaMode::ProjectPartial` で project 層から `command` section
全体を除外する信頼境界を load に配線し、`RuntimePolicy`/
`ToolRuntimeOptions`/`InvocationToolRuntimeOptions` の `command` field
追加による compile-forced propagation を workspace 全体に適用した。
`command` は catalog `NAMES`/registered set に入れず dormant で、
`exposes_eight_schemas` が 8 個のままであることを確認済み。

## Context

TASK-034EOHDmemAJi6IbUQel4x / PLAN-034EO1kmKwJ8MAHSHWyCws。
issue #101 の policy-gated `command` tool の第 1 task で、
design §(a)(b)(d)(f)(h)(k)(l)・spec AC4 の core 側を実装する。
実行 glue・pin・nonempty grants gate は T3、managed-process 抽出は
T2、invocation resolved re-embed は T4 が所有する。

## Implementation

### Changes

- `policy/command.rs`(新規): `command_policy_properties`(grant は
  `{executable, argv?, cwd?, env?, timeout?, killGrace?, maxOutputBytes?}`)、
  `resolve_command_policy`(None/null → `{}` collapse、grant 単位で
  deny/argv compile/cwd relative/env 規則・override fold)、
  `resolve_command_grant`(`POLICY_VIOLATION` "unknown or unauthorized
  command grant")、`ArgPattern`(`*` rest marker・trailing-`*` prefix・
  interior `*` 拒否)、`command_denied_executable_re` +
  `is_denied_command_executable`(basename 小文字化 + `.exe`/`.com`
  strip、interpreter/exec wrapper/loader/LOLBin/editor/追加 shell)、
  `command_grant_resources`(root subtree reads+writes、`produces` なし)、
  `is_relative_executable_path`(workspace-resident pre-filter)。
  error prefix は `"invalid command policy"`(`command_invalid`)。
- `policy/process.rs`: `argv_shell_re` を `pub`、`relative`/`str_vec` を
  `pub(crate)`、`bounded_duration`/`relative` に section label 引数を追加
  ("invalid {label} policy" prefix を共有)。process 側 message 文字列は
  不変。`DEFAULT_*` const を `pub(crate)` で共有。
- `policy/schema.rs`: complete + user partial schema に `command`
  section(section required `["maxConcurrent","grants"]`)、
  `policy_project_partial_schema()` 新設(`command` 丸ごと除外)、
  `PolicySchemaMode::ProjectPartial` + 3-validator tuple、
  `RuntimePolicy.command`、`normalize_policy`/`validate_policy_schema`
  の resolve 配線。
- `policy/load.rs`: project call site を `ProjectPartial` へ切替
  (`Partial` は user 層に温存)、resolved command を `process` と同型に
  effective policy へ re-embed。`merge.rs` 無変更。
- `worker/validate.rs`: `is_mutating_tool(ToolName::Command)` → true。
- cheaper-tools: `tools/command/mod.rs`(新規)— `command_schema`
  (`{name,args}`、required both、余剰 key は schema 層 `INVALID_INPUT`)、
  `command_details_schema`(`{grant,argv,state,exitCode,signal,durationMs,
  stdout,stderr}`)、deny-stub `command_tool_definition`(execute は
  `POLICY_VIOLATION "command unavailable"`)。catalog の
  input/output/resources match に `Command` arm を実 schema/宣言で
  充足(NAMES 非追加)。registry に `ToolRuntimeOptions.command` field +
  `registered_definition` の `Command` deny-stub arm。
  invocation に `InvocationToolRuntimeOptions.command` field +
  `create_worker_scope` の passthrough。
- runtime/CLI: `run/mod.rs`・`workflow_run.rs` で
  `serde_json::to_value(&policy.command)` を機械的 serialize。
- literal 構築点: capability/restrict/schema.rs 内 helper、
  dev/fixture、run_dev_seam、ollama_e2e、workflow/rig-adapter/tools
  test options に `command` field を追加。
- golden: `policy_schema.rs` に `"project-partial"` mode mapping。
  `vocabulary.json` 9 要素化・`worker/validate.json` mutating map に
  `command: true`・`policy/{schema,load,restrict}.json` の expected
  policy に resolved `command` section を追加。focused case として
  `partial accepts command grants`・`project-partial rejects command
  section`・`project-partial rejects command authoring fields`・
  `project-partial accepts non-command sections`(schema.json)、
  `project command section rejected`・`user command grants accepted`
  (load.json)を追加。

### Decisions During Implementation

- project partial は `command` section 全体を property から除外する
  design の Option A を採用 — grant authoring key の個別列挙より
  fail-closed で、将来 grant field が増えても境界が閉じたまま。
- resolved grant の `argv` は pattern source string を保持し、
  `ArgPattern` は resolve 時に compile 検証のみ。`to_value` → 再
  resolve → `PartialEq` の parity 経路を unit test で固定した。
- `PolicySchemaMode::Partial` は user 層用に温存し `ProjectPartial`
  を新設 — user policy が command grants を authoring できる trusted
  境界を維持したまま project 層のみ閉じる。
- `RuntimePolicy`/`ToolRuntimeOptions` 等の literal は全て
  `command: ...` を `process` の隣に配置 — section 順を schema/doc
  上の並び(workflow → process → command → execution)に揃えた。
- `create_worker_scope` は T1 で `options.runtime.command`
  passthrough のみ(task 記載通り)。`create_tool_runtime` での command
  resolve は行わない — dormant stub に resolved policy を食わせる
  必要がなく、T3 が gate 配線と共に決める。

## Related Experiments

### Adopted

なし。task/design が確定済みで追加 experiment は起こさなかった。

### Rejected

なし。

## Validation

- `mise exec -- cargo check --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0
  (compile-forced propagation 完遂)。
- `mise exec -- cargo test -p cheaper-core` — 198 unit + 22 golden
  全 pass。新規 golden case(project-partial 拒否 2・project-partial
  受理・user-partial grant 受理・user command grants load)を含む。
- `mise exec -- cargo test -p cheaper-tools` — 247 unit + 全
  integration pass。`exposes_eight_schemas` が catalog 露出 8 個の
  まま(dormant)であることを確認。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。

## Risks

- `command` は dormant のため policy path 以外の runtime 影響はない。
  deny set の網羅性は design §(d) の known-bad fail-fast 範囲 —
  containment は argv pattern/pin が担う(T3)。
- golden fixture は手動生成(TS dump 由来の sibling とは別 update) —
  diff 規約(sorted keys・error cap shape)に合わせたが、将来 TS dump
  再生成があれば key 順差分が出得る。

## Follow-ups

- T2: managed-process 共通基盤(`proc` module)抽出。
- T3: `command` tool 実装 — NAMES/registered set 追加、grant 名
  埋め込み description、argv match・pin 再検証の execute glue、
  nonempty-grants gate・writeScope 拒否、`command_operation_declaration`。
- T4: invocation resolved policy への `command` 再注入確定。
