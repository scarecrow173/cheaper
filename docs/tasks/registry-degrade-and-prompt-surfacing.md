---
id: "TASK-034EUvFhVkFFo9cY6aX6LB"
type: "task"
status: "todo"
title: "T2: registry degrade path と prompt surfacing"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
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
    - "../plans/implement-tool-operator-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-tool-operator-built-in-worker.md"
    - "docs/tasks/optional-tools-core-machinery.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/tool-operator-built-in-worker.md"
  references: []
  defers: []
  deferred-by: []
---

# T2: registry degrade path と prompt surfacing

## Scope

`create_tool_runtime` の gate 分岐を effective-names 方式へ拡張し、opt-in
（`options.degradable` membership）した `process` / `command` が empty
`tasks` / `grants` の content-gate 不成立時に `POLICY_VIOLATION` ではなく
未登録へ縮退する degrade path を実装する（design §(e)）。縮退した tool 名を
宣言順で報告する `ToolRuntime::degraded_names()` accessor と、
`cheaper.run.tool_scope` span の `tools.degraded` field、および system
prompt への surfacing（"Available tools" を `scope.definitions()` の実登録
集合から組み立て + 利用不可注記 + `status` enum 条件付き blocked 誘導行）
を実装する（design §(f)）。

- In: effective names の冒頭決定と gate 分岐、declaration-order degraded
  list + `degraded_names()`、zero-effective-names `POLICY_VIOLATION` guard、
  `options.degradable` の `is_scope_gated_tool` clamp、pin 判定・
  `ScopeAccessOptions.names`・registered loop・`RuntimeInner.names` の全
  consumer の effective names 化、`tool_names` の `scope.definitions()`
  組み立て、`build_system_prompt` 第 4 引数 + 注記 + conditional blocked
  誘導行、`tools.degraded` span field（sync 区間 record）。
- Out: `optionalTools` の parse/validate・`degradable` field 本体（T1）、
  worker 定義（T3）、fixture DSL・dev-seam/regression test（T4 — 本 task は
  unit test で挙動を固定し、fixture test は T4）、docs（T5）。
- Hard のまま維持（縮退対象外 — design Risks の accepted 決定）: pin 失敗・
  `resolve_*_policy` error・grants 非空時の writeScope × `command` reject・
  `tools.allow` admission（`check_capabilities`）。

### 所有ファイル

- `crates/cheaper-tools/src/registry.rs` — degrade path:
  `create_tool_runtime`（gate 分岐 :1063-1095）で effective names を冒頭
  決定。`options.degradable` membership 時は `names.retain` +
  `degraded.push`、それ以外は従来の `POLICY_VIOLATION`。grants 非空時の
  writeScope × `command` reject は hard のまま。冒頭の
  `options.degradable.retain(is_scope_gated_tool)` clamp（optional
  defensive）と degradation 後の有効 names 空なら `POLICY_VIOLATION` の
  guard。`degraded` を `options.names` 宣言順に整列。pin 判定（:1139）・
  `ScopeAccessOptions.names`（:1174）・registered loop（:1246）・
  `RuntimeInner.names`（:1257）の全 consumer を effective names へ。
  `RuntimeInner.degraded` + `pub fn ToolRuntime::degraded_names()`。
- `crates/cheaper-runtime/src/run/mod.rs` — `create_session`（:474-516）の
  `tool_names` を `self.effective_tools` ではなく `scope.definitions()` の
  実登録集合から組み立て、`scope.degraded_names()` を `unavailable_tools`
  として `build_system_prompt` 第 4 引数へ。`cheaper.run.tool_scope` span
  （:417）に `tools.degraded = field::Empty` を宣言し、inner block 閉鎖
  （:462 — `_stage_guard` はここで drop 済み）と
  `scope.ready().instrument(tool_scope_stage)`（:463-465 — `instrument`
  が span を move するため、これが record の真の期限）の間で record
  （宣言順 join、metadata-only 規約）。
- `crates/cheaper-rig-adapter/src/agent.rs` — `build_system_prompt`
  （:225-264）へ第 4 引数 `unavailable_tools: &[String]` + 非空時の利用
  不可注記行（`- Declared but unavailable under current policy: {join}`）
  + `output_schema` の `properties.status.enum` に `"blocked"` を含む
  場合のみ emit する blocked 誘導行。unit test で文言固定。
- `crates/cheaper-rig-adapter/tests/adapter.rs:364,560,572,742` —
  `build_system_prompt` 既存 4 call site を第 4 引数追加の signature
  change へ追従（compile-forced 波及）+ 注記文言・blocked 誘導行
  conditional の新規 unit test。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-tool-operator-built-in-worker.md` — 実装対象 plan（T2）
- `docs/designs/tool-operator-built-in-worker.md` — §(e) degrade path・
  §(f) prompt surfacing・§(i)(j) file map / back-compat
- `docs/specs/tool-operator-built-in-worker.md` — AC4（挙動機構）・AC8
- `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`
  — Option C1・admission vs content-gate の 2 層分離
- `docs/test-specs/tool-operator-built-in-worker.md` — 検証 spec
  （effective names・宣言順・hard 維持の行為保証）

## Dependencies

- `docs/tasks/optional-tools-core-machinery.md`（T1 —
  `ToolRuntimeOptions.degradable` field・`WorkerCheaper.optional_tools`・
  `is_scope_gated_tool`）

## Steps

1. `registry.rs` で effective names を冒頭決定し、content-gate 分岐を
   `options.degradable` membership で縮退 / それ以外は `POLICY_VIOLATION`
   へ分岐させる。`degradable` の `is_scope_gated_tool` clamp と
   zero-effective-names guard を入れる。
2. `degraded` を `options.names` 宣言順に整列し、`RuntimeInner.degraded`
   + `ToolRuntime::degraded_names()` を追加、pin 判定・
   `ScopeAccessOptions.names`・registered loop・`RuntimeInner.names` の
   全 consumer を effective names へ切替える。
3. `run/mod.rs` で `tool_names` を `scope.definitions()` から組み立て、
   `degraded_names()` を `build_system_prompt` 第 4 引数へ渡し、
   `tools.degraded` span field を sync 区間（`create_worker_scope` return
   後・`instrument` 前）で record する。
4. `agent.rs` の `build_system_prompt` に第 4 引数・利用不可注記・
   `status` enum 条件付き blocked 誘導行を追加し、adapter.rs 既存 4 call
   site を追従させ、注記文言・conditional 分岐の unit test を書く。
5. 非 opt-in reject test `run_fake_stream_command_worker_denied_without_grants`
   （`crates/cheaper-cli/tests/run_dev_seam.rs:294`）が無変更で green で
   あることを確認する（後方互換 regression の証明 — Risk #6）。

## Done When

- [ ] degrade path の unit test が pass — effective names・宣言順
  `degraded_names()`・zero-effective-names guard・`degradable` clamp・
  非 opt-in / pin 失敗 / writeScope × `command` の hard 維持
- [ ] `build_system_prompt` の注記 emit / blocked 誘導行 conditional
  （`status` enum に `"blocked"` を含む schema のみ emit）の unit test が
  pass — schema が受理しない終了状態を prompt が促さない
- [ ] `run_fake_stream_command_worker_denied_without_grants` が無変更で
  green — 非 opt-in worker の fail-closed 意味論不変

## Rollback

- T1 の Rollback を参照。T1+T2 は T3 land 前なら unit として revert 可能。
  T3 以降に revert する場合は `resources/workers/tool-operator.md`（と
  schema file、必要なら alias）の除去を必ず伴わせる。
- revert 後に
  `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  と `mise exec -- cargo test -p cheaper-core -p cheaper-tools` が pass
  すること。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-tools -p cheaper-rig-adapter -p cheaper-runtime --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --test run_dev_seam --features cheaper-cli/dev-seam` exits 0（`run_fake_stream_command_worker_denied_without_grants` 無変更 green を含む）
