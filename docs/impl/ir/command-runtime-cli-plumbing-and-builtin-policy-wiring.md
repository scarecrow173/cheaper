---
id: "IMPL-034EvP2s9FEfHuZw09PT5J"
type: "impl"
status: "completed"
title: "command runtime cli plumbing and builtin policy wiring"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/command-runtime-cli-plumbing-and-builtin-policy-wiring.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-tools/src/invocation.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_invocation.rs"}
      - {"type":"code","path":"crates/cheaper-cli/resources/policy.yaml"}
      - {"type":"doc","path":"docs/tasks/runtime-cli-plumbing.md"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/runtime-cli-plumbing.md"
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

# command runtime cli plumbing and builtin policy wiring

## Summary

T4。`create_worker_scope` の `runtime_options.command` を T3 が残した
`options.runtime.command` passthrough から `inner.command`
(invocation-resolved grants)の serialize 再注入へ確定し、builtin
`policy.yaml` に `command` section(`{maxConcurrent, timeout,
maxOutputBytes, killGrace, heartbeat, grants}` — `process` section と同じ
default 群)+ `tools.allow` の `command` を追加した。これにより Worker
経路(tools 宣言 ∩ tools.allow ∩ grants)と Workflow `kind: tool` 経路
(tools.allow admission + nonempty-grants gate)の双方で `command` が
同一 canonical 実装へ到達する。`run/mod.rs`・`workflow_run.rs`・
dev fixture の `command` 配線は T1/T3 で既に正しく敷設済みで、検証の
結果追加分の機械変更は不要だった。

## Context

TASK-034EOHPDg1Gg2LSKLw8WRT / PLAN-034EO1kmKwJ8MAHSHWyCws。issue #101
の policy-gated `command` tool の第 4 task。T3(9445a64)は
`InvocationInner.command` 格納と declared-shape parity check
(`resolve_command_policy` → `PartialEq`)を実装したが、scope への
`runtime_options.command` は caller 指定 raw の passthrough のまま
残した(T4 で確定予定と明記)。T1(b706188)は runtime/CLI 側の全
literal site に `command: serde_json::to_value(&policy.command)` を
機械的に配線済み。design §(g) registry plumbing・§(i) builtin
policy・Data and Control Flow の両経路が対象セクション。

## Implementation

### Changes

- `crates/cheaper-tools/src/invocation.rs`: `create_worker_scope` の
  `runtime_options` literal で `command` を
  `Some(serde_json::to_value(&inner.command))` へ確定 —
  `process`(:262-265)と同型。caller 指定 raw は直上の parity check
  が `inner.command` との declared-shape 一致を検証済みであり、
  `command: None` を渡す scope(Workflow `ToolJobExecutor` の形状)は
  invocation-resolved grants を受け取る。`None` がそのまま scope へ
  漏れて empty grants 化する経路は存在しない。
- `crates/cheaper-cli/resources/policy.yaml`: `command` section を
  `process` と同一の lifecycle default 群で追加
  (`maxConcurrent: 1`、`timeout: 2m`、`maxOutputBytes: 65536`、
  `killGrace: 2s`、`heartbeat.interval: 30s`、`grants: {}`)。
  complete schema の required key `["maxConcurrent", "grants"]` を
  充足し、`grants: {}` の既定は execution 権限実質ゼロの
  fail-closed。`tools.allow` に `command` を追加(design §(i) —
  権限境界は nonempty-grants gate に集約し、builtin allow は
  exposure toggle)。
- `crates/cheaper-tools/tests/proc_invocation.rs`: T4 挙動固定 test
  3 件を追加。
  - `worker_scope_receives_invocation_resolved_command_grants` —
    invocation に `self` grant(executable = この test binary の
    absolute path、argv literal pin)を持たせ、scope options は
    `command: None`(`ToolJobExecutor` と同型)で `names: [Command]`。
    scope 構築成功 + `definitions()` に `command` + `execute` が
    `exitCode: 0` で完了することを検証 — passthrough のままなら
    empty grants gate で `POLICY_VIOLATION` になるため再注入を直接
    固定する。
  - `command_scope_fails_closed_without_grants` — grants 空の
    invocation で `names: [Command]` の scope 生成が
    `POLICY_VIOLATION`("nonempty authorized grants")で fail-closed。
    Worker 経路・workflow 経路共通の registration gate。
  - `worker_scope_rejects_command_policy_mismatch` — scope が
    `inner.command` と異なる declared-shape raw を渡すと
    `invalid execution binding: command policy mismatch` で reject
    (T3 parity check の挙動固定)。

### Verified-no-change(確認のみで変更なし)

- `crates/cheaper-runtime/src/run/mod.rs`: `prepare_invocation`
  (:284)と `create_tool_scope`(:433)の `command:
  Some(serde_json::to_value(&policy.command))` は T1 配線済みで、
  Worker 経路が resolved policy を正しく運ぶことを確認。追加分の
  機械変更は不要。
- `crates/cheaper-cli/src/workflow_run.rs`: `ensure_invocation`
  (:242-246)が `shared.policy.command` を serialize して invocation
  へ渡す配線済み。`ToolJobExecutor` の `command: None`(:468)は
  re-injection で `inner.command` に置換されるため正しい形状。
  `tools.allow` admission(:553-568)は string 比較で `command` を
  自動受理(builtin policy 追加で有効化)。
- dev/test fixture(`dev/fixture.rs` `default_policy()`、
  `tests/run_dev_seam.rs:166`、`tests/ollama_e2e.rs:239`)の
  `RuntimePolicy.command` literal は `process` と同じ lifecycle
  field を持つ正しい resolved shape(空 grants = dormant)で、stub
  ではないことを確認。
- `command` を含む project `.cheaper/policy.yaml` の load 時
  `InvalidRuntimePolicy` 拒否は T1 の `PolicySchemaMode::ProjectPartial`
  + golden fixture(`policy/schema.json` の "project-partial rejects
  command section" 等)で既に担保済み。

### Decisions During Implementation

- **`policy.yaml` の `command` section は全 default を明示記述**:
  task doc/design §(i) の最小形は `{maxConcurrent: 1, grants: {}}` だが、
  builtin `process` section が lifecycle default 群を全て明示する
  既存 style に揃えた。resolve 結果は省略時と同一のため挙動差はない。
- **T4 の test は `proc_invocation.rs` に配置**: 両実行経路
  (worker scope / workflow tool job)は `create_worker_scope` →
  `create_tool_runtime` に収束するため、re-injection と gate の
  挙動固定はこの層の test が最小かつ直接。CLI 側の `tools.allow`
  admission は string 比較で新規 code path を持たず、builtin
  policy の schema 整合は `load_policy` の `Complete` validation
  (load.rs:56)が全 dev-seam CLI test 経路で副作用検証する。
  `proc_command.rs` の広範な integration suite(grant 実行 E2E・
  lifecycle drain 等)は T5 の所有範囲。

## Related Experiments

### Adopted

なし。design §(g)/(i)・task doc が確定済みで追加 experiment は
起こさなかった。

### Rejected

- `workflow_run.rs` 側に `command` の追加 gate を置く案 — admission
  は既存の `tools.allow` string check が担い、authorization は
  `create_tool_runtime` の nonempty-grants gate に集約済みのため、
  二重の防御的 check は design の単一権限境界をぼかす。

## Validation

- `mise exec -- cargo test -p cheaper-runtime -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0
  (全 test target pass。`ollama_e2e` 3 件は gated 実 provider
  検証として実行・pass)。
- `mise exec -- cargo test -p cheaper-tools --test proc_invocation` —
  13/13 pass(T4 追加 3 件含む)。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo fmt --all -- --check` — clean。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0。

## Risks

- `command` は builtin `tools.allow` 入りしたが `grants: {}` の既定は
  nonempty-grants gate で fail-closed のまま — exposure と
  authorization の分離(design §(i))に依存し、user-scope policy が
  grant を宣言しない限り実行権限はゼロ。
- `docs/guides/configuration.md` の builtin `policy.yaml` verbatim
  embed(:93-135)は本 commit では未同期 — plan file map 上 T6 の
  所有 file であり、T6 で `command` section・`tools.allow` を含めて
  同期する。

## Follow-ups

- T5: `tests/proc_command.rs` integration suite(grant 実行 E2E・
  deny/pin/metachar/argv match・lifecycle drain・parity の拡張
  検証)と `resource_inference` 挙動固定。
- T6: `configuration.md` builtin embed 同期・`workers.md` 三重
  ゲート記述・command guide(trust / sandbox 限界)。
