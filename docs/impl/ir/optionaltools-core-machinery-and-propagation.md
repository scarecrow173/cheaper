---
id: "IMPL-034EUQHxLuXN3I40r1pNs8"
type: "impl"
status: "completed"
title: "optionalTools core machinery and propagation"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/optionaltools-core-machinery-and-propagation.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-core/src/tools/vocabulary.rs"}
      - {"type":"code","path":"crates/cheaper-core/src/worker/validate.rs"}
      - {"type":"code","path":"crates/cheaper-core/tests/golden/common/mod.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/registry.rs"}
      - {"type":"code","path":"crates/cheaper-tools/src/invocation.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/src/run/mod.rs"}
      - {"type":"code","path":"crates/cheaper-cli/src/workflow_run.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_command.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_invocation.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_runtime.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/characterization.rs"}
      - {"type":"code","path":"crates/cheaper-tools/tests/differential.rs"}
      - {"type":"code","path":"crates/cheaper-rig-adapter/tests/adapter.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/artifacts.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/process.rs"}
      - {"type":"code","path":"crates/cheaper-workflow/tests/common/mod.rs"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/optional-tools-core-machinery.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/optional-tools-core-machinery.md"
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

# optionalTools core machinery and propagation

## Summary

T1。`cheaper.optionalTools` frontmatter key の parse・validate 機構と、
縮退許可集合を registry まで運ぶ `ToolRuntimeOptions.degradable` の
compile-forced propagation を実装した。`ToolName::resolve`
(canonical 名 → alias table の single-sourced resolver)と
`is_scope_gated_tool`(`Process`/`Command` の一権威 predicate)を
vocabulary に集約し、`normalize_tool_names` の per-element path も
`resolve` 呼び出しへ refactor した。design §(b) の 7 validate rule
を全て `InvalidWorkerDefinition` で実装し、非 opt-in worker の
意味論は不変。registry degrade path・`degraded_names()`・prompt
surfacing・`tools.degraded` span field は T2、worker 定義は T3、
fixture DSL knob は T4 が所有する。

## Context

TASK-034EUvBkRkmTjWxeq3iG0h。issue #102 の tool-operator built-in
worker の第 1 task で、design §(b)(c)(d)(i)(j)・spec AC4/AC8・
ADR-034ESBcMTWxNCMDI9obdb8(Option C1 per-tool opt-in 縮退)の core
機構を実装する。#101 precedent に従い `WorkerCheaper.optional_tools`
と `ToolRuntimeOptions.degradable` の必須 field 追加による全 literal
構築点の compile-forced propagation を本 task が同梱した。

## Implementation

### Changes

- `crates/cheaper-core/src/tools/vocabulary.rs`:
  `pub fn ToolName::resolve`(`from_canonical().or_else(tool_alias)` を
  集約 — `tools` と `optionalTools` で名解決規則が drift しない
  一権威化)、`pub fn is_scope_gated_tool`(`Process`/`Command` のみ
  true — gated 集合の二重定義を生まない)、`normalize_tool_names` の
  per-element path を `resolve` 呼び出しへ refactor(既存の error
  reason/behavior は不変)。resolve/predicate の unit test 追加。
- `crates/cheaper-core/src/worker/validate.rs`: `CHEAPER_KEYS` に
  `"optionalTools"` 追加、`WorkerCheaper.optional_tools: Vec<ToolName>`
  (未指定時は空 vec)。design §(b) の 7 rule を全て `invalid()` 経由
  `InvalidWorkerDefinition` で実装 — `optionalTools-invalid`(非配列・
  非 string 要素)/`optionalTools-empty`(空配列)/
  `optionalTools-unknown-tool`(resolve 不能)/`optionalTools-undeclared`
  (`tools` 未宣言)/`optionalTools-not-gated`(非 scope-gated)/
  `optionalTools-duplicate`(正規化後重複)/
  `optionalTools-not-proper-subset`(`optionalTools ⊇ tools`)。各
  reject reason + accept+normalize + 未指定時空の unit test 追加。
- `crates/cheaper-tools/src/registry.rs`:
  `ToolRuntimeOptions.degradable: Vec<ToolName>` field 追加
  (`names` の隣 — 同じく scope 単位の宣言)。degrade path 本体は T2
  のため現時点では inert。
- `crates/cheaper-tools/src/invocation.rs`: `create_worker_scope` の
  `ToolRuntimeOptions` 再構成に `degradable` pass-through 1 行。
  `WorkerScopeOptions`/`InvocationToolRuntimeOptions` は shape 不変。
- `crates/cheaper-runtime/src/run/mod.rs`: `create_tool_scope` の
  literal に `degradable: worker.cheaper.optional_tools.clone()`。
- `crates/cheaper-cli/src/workflow_run.rs`: `ToolJobExecutor` の
  literal に `degradable: Vec::new()`(`kind: tool` job は
  frontmatter を持たず opt-in 不能。`kind: worker` job は run 経路で
  自動的に流れる)。
- compile-forced literal 波及: cheaper-tools tests
  `proc_command.rs`/`proc_invocation.rs`/`proc_runtime.rs`/
  `characterization.rs`/`differential.rs`、rig-adapter
  `adapter.rs`、workflow `artifacts.rs`/`process.rs` に
  `degradable: Vec::new()`、workflow `common/mod.rs` の
  `WorkerCheaper` literal に `optional_tools: Vec::new()`。
- `crates/cheaper-core/tests/golden/common/mod.rs`:
  `dump_worker_def` に `optionalTools` の conditional emit(非空時のみ
  `cheaper.optionalTools` を出力 — `input`/`writeScope` と同じ
  pattern)。既存 expected file への churn なし。

### Decisions During Implementation

- validate rule の検査順は design §(b) の表順(invalid → empty →
  unknown → undeclared → not-gated → duplicate → not-proper-subset)
  に固定した。`resolved ⊆ tools` は undeclared/duplicate 検査済みで
  保証されるため、not-proper-subset は `tools` を全て覆うかの集合
  判定で表現した。
- 全 reject は detail に tool 名等を含めず `invalid()` の
  `{sourcePath, reason}` のみ — design の reason 表が固定する
  contract に合わせた。
- `optionalTools` の名解決に alias input を許容するのは `resolve`
  共有の副産物。現行 alias table は非 gated tool のみ持つため
  alias 入力は必ず not-gated で reject されるが、規則の drift を
  防ぐ一権威化を優先した(test で `Read` → not-gated を固定)。
- golden dump の `optionalTools` emit 位置は frontmatter 宣言順
  (design §(a) の `cheaper:` 配下先頭)に合わせ `model` の前に
  置いた。

## Related Experiments

### Adopted

なし。task/design が確定済みで追加 experiment は起こさなかった。

### Rejected

なし。

## Validation

- `mise exec -- cargo fmt --all -- --check` — exit 0。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0
  (compile-forced propagation 完遂の証明)。
- `mise exec -- cargo test -p cheaper-core` — 209 unit + 22 golden
  全 pass。`optionalTools` 7 reject + accept + default-empty と
  vocabulary `resolve`/`is_scope_gated_tool` の新規 test を含む。
  (`--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` は
  `-p cheaper-core` 単体指定では feature 不在で受理されないため、
  task doc の Verification 記載どおり素の `cargo test -p
  cheaper-core` で実行。)
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0。

## Risks

- `degradable` は degrade path 本体(T2)未実装のため現時点では
  inert — scope 構築の gate 挙動は一切変わらず、非 opt-in worker の
  意味論不変は compile+test で確認済み。
- alias input を受け付ける `optionalTools` は現状 not-gated で必ず
  reject されるため観測可能な挙動差はないが、将来 gated tool の
  alias が追加された場合はそのまま受理される(resolve 共有の意図
  された帰結)。

## Follow-ups

- T2: registry degrade path(`create_tool_runtime` の gate 分岐拡張・
  `RuntimeInner.degraded`・`degraded_names()`)、prompt surfacing
  (実登録集合 + 利用不可注記 + blocked 誘導行の conditional emit)、
  `tools.degraded` span field。
- T3: `resources/workers/tool-operator.md` + input/output schema +
  CLI alias `tool-operate`。
- T4: fixture DSL `FixtureWorker.optionalTools` knob + dev-seam/
  regression case。
- T5: 公開面 docs + APM skill。
