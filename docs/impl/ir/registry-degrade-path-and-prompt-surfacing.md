---
id: "IMPL-034E5b824duUI8BjKw5jx1"
type: "impl"
status: "completed"
title: "registry degrade path and prompt surfacing"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"docs/impl/ir/registry-degrade-path-and-prompt-surfacing.md"}
      - {"type":"code","path":"crates/cheaper-tools/tests/proc_degrade.rs"}
    modified:
      - {"type":"code","path":"crates/cheaper-tools/src/registry.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/src/run/mod.rs"}
      - {"type":"code","path":"crates/cheaper-runtime/tests/run_dev_seam.rs"}
      - {"type":"code","path":"crates/cheaper-rig-adapter/src/agent.rs"}
      - {"type":"code","path":"crates/cheaper-rig-adapter/tests/adapter.rs"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/registry-degrade-and-prompt-surfacing.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/registry-degrade-and-prompt-surfacing.md"
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

# registry degrade path and prompt surfacing

## Summary

T2。`create_tool_runtime` の gate 分岐を effective-names 方式へ拡張し、
opt-in(`options.degradable` membership)した `process` / `command` が
empty `tasks` / `grants` の content-gate 不成立時に `POLICY_VIOLATION`
ではなく未登録へ縮退する degrade path を実装した(design §(e))。
`RuntimeInner.degraded` + `ToolRuntime::degraded_names()` で縮退名を
`options.names` 宣言順で報告し、run 層は `scope.definitions()` の実
登録集合から "Available tools" を組み立て、`build_system_prompt` 第
4 引数の `unavailable_tools` として利用不可注記 + `status` enum
条件付き blocked 誘導行を emit する(design §(f))。
`cheaper.run.tool_scope` span に `tools.degraded` field を追加した。

## Context

TASK-034EUvFhVkFFo9cY6aX6LB。issue #102 の tool-operator built-in
worker の第 2 task で、design §(e)(f)・spec AC4/AC8・
ADR-034ESBcMTWxNCMDI9obdb8(Option C1)の registry/run/adapter 側
挙動を実装する。T1(df540ad)が `ToolRuntimeOptions.degradable`・
`WorkerCheaper.optional_tools`・`is_scope_gated_tool` を着地済み。

## Implementation

### Changes

- `crates/cheaper-tools/src/registry.rs`:
  `create_tool_runtime` 冒頭で `options.degradable` を
  `is_scope_gated_tool` で clamp し、effective `names` を決定。
  process empty-tasks gate と command empty-grants gate は
  `degradable` membership 時に `names.retain` + `degraded.push`、
  それ以外は従来の `POLICY_VIOLATION`。grants 非空時の
  writeScope × `command` reject・pin 失敗・`resolve_*_policy`
  error は opt-in tool に対しても hard のまま。degradation 後の
  有効 names が空なら `POLICY_VIOLATION` の zero-effective-names
  guard を配置(validate bypass の直接 caller 向け防御)。
  `degraded` は `options.names` 宣言順に整列して
  `RuntimeInner.degraded` へ格納し、`pub fn degraded_names()` を
  公開。pin 判定・`ScopeAccessOptions.names`・registered loop・
  `RuntimeInner.names` の全 consumer を effective names へ切替え。
- `crates/cheaper-runtime/src/run/mod.rs`: `create_session` の
  `tool_names` を `self.effective_tools` ではなく
  `scope.definitions()` の実登録集合から組み立て、
  `scope.degraded_names()` を `build_system_prompt` 第 4 引数
  `unavailable_tools` へ。`cheaper.run.tool_scope` span に
  `tools.degraded = field::Empty` を宣言し、`create_worker_scope`
  return 後・`scope.ready().instrument(tool_scope_stage)` move 前
  の sync 区間で宣言順 join を record(metadata-only 規約)。
- `crates/cheaper-rig-adapter/src/agent.rs`: `build_system_prompt`
  に第 4 引数 `unavailable_tools: &[String]` を追加。非空時は
  "Available tools" 直後に schema-neutral の列挙行
  `- Declared but unavailable under current policy: {join}` を
  emit し、`output_schema` の `properties.status.enum` に
  `"blocked"` を含む場合のみ blocked 誘導行を追加
  (`output_schema_accepts_blocked` helper)。既存行・文言は不変。
- `crates/cheaper-rig-adapter/tests/adapter.rs`: 既存 4 call site
  を signature change へ追従 + 注記文言・blocked 誘導行
  conditional の新規 unit test 2 件。
- `crates/cheaper-tools/tests/proc_degrade.rs`(新規): degrade path
  の gate test — opt-in 縮退(宣言順 `degraded_names()`・実登録
  集合・縮退 tool の `UNKNOWN_TOOL` 実行経路不在)、mixed policy
  per-tool 縮退、非 opt-in hard 維持、pin 失敗 / writeScope ×
  `command` / resolve error の hard 維持、zero-effective-names
  guard、非 gated `degradable` entry の clamp、非縮退時の
  `degraded_names()` 空。
- `crates/cheaper-runtime/tests/run_dev_seam.rs`: `FakeSessionFactory`
  の `RecordedSpec.system_prompt` で prompt 注記の文言と
  `"Available tools"` の実登録集合組み立てを固定する test 2 件
  (`status` enum が `"blocked"` を受理する schema / 受理しない
  schema の両側 — conditional emit の両分岐を run 層で検証)。

### Decisions During Implementation

- zero-effective-names guard は「degradation が空にしたか」ではなく
  「gate 通過後の有効 names が空か」で判定する単純な fail-closed
  guard とした(直接 caller が最初から空 `names` を渡す degenerate
  入力も同じ `POLICY_VIOLATION` に畳まれる。worker 経路では
  validate の `tools` non-empty + not-proper-subset が防ぐため
  観測可能な差はない)。
- `tools.degraded` span field は空縮退時も空文字列で record する
  (field 宣言は常時、値は join 結果 — span field の有無を
  opt-in の有無で変えない方が sink 側の観測が stable)。
- clamp は `options.degradable.clone().retain(is_scope_gated_tool)`
  で実装 — `options` 本体は後段の field-by-field 消費を変えない。
- `degraded` の宣言順整列は gate 分岐の push 結果を
  `options.names` 順で filter する形 — gated 集合の増加に
  対して順序ロジックを追加せず宣言順が常に得られる。

## Related Experiments

### Adopted

なし。task/design が確定済みで追加 experiment は起こさなかった。

### Rejected

なし。

## Validation

- `mise exec -- cargo fmt --all -- --check` — exit 0。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo test -p cheaper-tools -p cheaper-rig-adapter
  -p cheaper-runtime --features cheaper-runtime/dev-seam` — exit 0。
  `proc_degrade` 9 test・adapter 2 test・runtime `run_dev_seam` の
  prompt 注記 2 test を含む全 pass。
  (task doc Verification の `cheaper-cli/dev-seam` 併記は
  cheaper-cli 非選択時に受理されないため `cheaper-runtime/dev-seam`
  で実行 — T1 と同じ feature 解決。)
- `mise exec -- cargo test -p cheaper-cli --test run_dev_seam
  --features cheaper-cli/dev-seam` — 10/10 pass。
  `run_fake_stream_command_worker_denied_without_grants` は無変更で
  green(非 opt-in worker の fail-closed 意味論不変の証明)。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0。

## Risks

- 一度だけ adapter の `same_turn_tool_calls_run_in_call_order` が
  `results[1]` 不一致で flake した(再実行・続く full suite では
  全 pass)。temp file 読み順の既存 flake であり本変更とは無関係。
- `names` が最初から空の直接 caller は従来 0-tool runtime が
  生成できたが、zero-effective-names guard により
  `POLICY_VIOLATION` になる。worker/invocation 経路では
  `names` 空は validate で到達不能のため観測可能な回帰なし。

## Follow-ups

- T3: `resources/workers/tool-operator.md` + input/output schema +
  CLI alias `tool-operate`。
- T4: fixture DSL `FixtureWorker.optionalTools` knob + dev-seam/
  regression case(degraded / granted / capability-denied / mixed /
  scripted `UNKNOWN_TOOL` / blocked status)。
- T5: 公開面 docs + APM skill。
