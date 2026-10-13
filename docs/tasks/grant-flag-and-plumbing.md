---
id: "TASK-034Fpq9sfBaQAX6pFPjWJr"
type: "task"
status: "todo"
title: "T2: grant-flag-and-plumbing — repeatable --grant flag・build_invocation_layer・run/workflow 配線（RunWorkerDeps propagation 所有）"
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
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
    - "docs/tasks/invocation-policy-layer.md"
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
    - "docs/test-specs/invocation-scoped-command-grants.md"
  references: []
  defers: []
  deferred-by: []
---

# T2: grant-flag-and-plumbing — repeatable --grant flag・build_invocation_layer・run/workflow 配線（RunWorkerDeps propagation 所有）

## Scope

`cheaper run`・全 worker alias・`cheaper workflow run` が受ける
repeatable `--grant <json>` flag の CLI 表面、共有 builder
`build_invocation_layer`、run / workflow run 経路への `invocation` 接続、
runtime 側の `RunWorkerDeps.invocation_grants` + `grants.invocation`
span field、stderr `grant:` diagnostic を実装する（design §(b)(c)(f)(i)）。
`RunWorkerDeps` の `pub` field 追加は **6 literal site** を compile
error にするため全構築 site を本 task が同梱所有し、**T2 完了時点でも
workspace build（dev-seam feature 付き `--all-targets`）が green** を
維持する。`ParsedCli::Policy` variant は T3 に集約する（variant 追加は
`format()`/`quiet()`/`command_name`/dispatch の exhaustive match を全て
compile-forced にし、dispatch 無しの stub を生むため — T2 時点の
`cheaper policy ... --grant` は `unknown command: policy` の
`InvalidRequest` で fail し、silent drop は発生しない）。

- In: args.rs の `--grant` parse 表面（`STRING_OPTIONS`・`Values.grants`
  ・`set_string` push arm・`RunCliArgs`/`WorkflowRunCliArgs` grants
  field・64KiB bound・非 run arm の `!values.grants.is_empty()` reject・
  workflow arm allowlist の run 側のみ受理・規約 comment）、新規
  `src/grant.rs` の `build_invocation_layer`、run.rs /
  workflow_context.rs / workflow_run.rs の plumbing、stderr
  diagnostic・非露出 warning、runtime `RunWorkerDeps.invocation_grants`
  + `grants.invocation` span field、cli.rs HELP、tests/args.rs parse
  table + grant.rs unit test。
- Out: `policy` command 本体（T3 — `ParsedCli::Policy` variant・
  dispatch・parse surface を一体実装）、`grantRequests` output
  contract（T4）、結合検証（T5）、docs/APM（T6）。

### 所有ファイル

- `crates/cheaper-cli/src/args.rs` — `STRING_OPTIONS`（:228-247、
  `"format"` 直後・`"log"` ブロック前）へ `"grant"`、`Values.grants:
  Vec<String>`、`set_string` push arm（last-wins 規約からの deliberate
  deviation 明記）、`RunCliArgs`（:71-86）/`WorkflowRunCliArgs`
  （:118-127）に `grants` field + populate（:903-912・:527-538）、64KiB
  bound（populate 段で enforce — 非 run command では reject が先に
  発火）、非 run 系 arm（usage:552-589・count-tokens:598-653・
  models:655-726・auth:728-827）の `!values.grants.is_empty()` reject、
  workflow arm allowlist（:501-510）の run 側のみ `"grant"` 追加、
  「非 run command は grants 非空検査を持つ」規約 comment。
- `crates/cheaper-cli/src/grant.rs`（新規）—
  `build_invocation_layer(grants) -> Result<(Option<Map>, Vec<String>),
  CheaperError>`: `serde_json::from_str` → object/`name` 検査
  （`^[a-z][a-z0-9_-]{0,63}$`）→ `name` strip（残すと grant spec の
  `additionalProperties: false` で必ず失敗する不変条件）→
  intra-invocation duplicate 検査 → `{"version":1,"command":
  {"grants":{...}}}` 組立 + 宣言順 names。構造的失敗は `InvalidRequest`
  + `details.layer="invocation"`（name 関与時 `details.grant`）。
  `grants` 空なら `(None, vec![])`。error attribution 規約の doc
  comment は load.rs と同一文言。
- `crates/cheaper-cli/src/lib.rs` — `pub mod grant;`。
- `crates/cheaper-cli/src/run.rs` — `build_invocation_layer` 呼出し →
  `load_policy`（:143-147）の `invocation` 接続、`grant: <names>`
  stderr diagnostic（宣言順、`--quiet` 抑制、`model:` line 規約に一致）、
  非露出 warning（worker が `command` 非宣言時のみ — `tools.allow`
  除外・pin 失敗は hard `POLICY_VIOLATION` のまま、warning へ変換
  しない）、`RunWorkerDeps.invocation_grants` 配線（:234）。
- `crates/cheaper-cli/src/workflow_context.rs` —
  `prepare_workflow_context`（:267-299）に `invocation:
  Option<&Map<String, Value>>` param + `load_policy` literal
  （:277-281）接続。
- `crates/cheaper-cli/src/workflow_run.rs` — `execute_run_inner`（:701）
  で layer 組立 + `grant:` stderr diagnostic（`kind: tool` job は span
  を通らないため workflow 経路では stderr 行が唯一の audit surface）、
  `Shared` へ names 保持、`WorkerJobExecutor` の `RunWorkerDeps`
  （:358-376）へ `invocation_grants`、`execute_interface_inner`（:665）
  は `None` 固定。
- `crates/cheaper-runtime/src/run/boundary.rs` —
  `RunWorkerDeps.invocation_grants: &'a [String]` field（:49-72）。
- `crates/cheaper-runtime/src/run/mod.rs` — `cheaper.run.tool_scope`
  span（:417-420）へ `grants.invocation = field::Empty` 宣言 +
  `tools.degraded` record（:476-482）と同じ sync 区間で宣言順 join を
  record。
- `crates/cheaper-runtime/tests/{run_dev_seam.rs:270,:1046,
  run_fixture_test.rs:116, ollama_e2e.rs:296}` — 各 `RunWorkerDeps`
  literal に `invocation_grants: &[]`（compile-forced）。
- `crates/cheaper-cli/src/cli.rs` — HELP `Options:` に `--grant <json>`
  （repeatable 明記）1 行。
- `crates/cheaper-cli/tests/args.rs` — repeatable `--grant` parse table
  （`--grant <v>` / `--grant=<v>` 両形・複数指定の宣言順保持・worker
  alias 受理・非 run 系 command の `InvalidRequest` reject — `policy`
  以外は T2、`policy` arm は T3・64KiB reject）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T2）
- `docs/designs/invocation-scoped-command-grants.md` — §(b)(c)(f)(i) の
  設計判断
- `docs/specs/invocation-scoped-command-grants.md` — AC8・AC10・AC13・
  AC15・AC17
- `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
  — flag 値形式（`name` 内包 JSON + strip）・warn-and-continue 範囲限定・
  非 run reject・audit surface
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （parse 面・`build_invocation_layer` 規約・diagnostic の行為保証）

## Dependencies

- `docs/tasks/invocation-policy-layer.md`（T1 —
  `LoadPolicyOptions.invocation` field + merge 意味論が前提）

## Steps

1. args.rs に repeatable `--grant` を実装する（`STRING_OPTIONS`・
   `Values.grants`・push arm・`RunCliArgs`/`WorkflowRunCliArgs`
   populate・64KiB bound・非 run arm reject・workflow allowlist・規約
   comment）。
2. `src/grant.rs` に `build_invocation_layer` を実装し `lib.rs` に
   `pub mod grant;` を追加する（`name` strip・duplicate 検査の unit
   test を同梱 — Risk #2 の pin）。
3. run.rs / workflow_context.rs / workflow_run.rs に `invocation`
   接続・stderr diagnostic・非露出 warning を実装する。
4. runtime 側に `RunWorkerDeps.invocation_grants` field と
   `grants.invocation` span field を追加し、6 literal site 全てを
   同梱更新する。
5. cli.rs HELP と tests/args.rs の parse table を追加し、workspace
   build + focused test が green であることを確認する。

## Done When

- [ ] `run`・全 worker alias・`workflow run` が `--grant` を両受理形・
  宣言順保持で受け、`usage`/`count-tokens`/`models`/`auth`/
  `workflow interface` が `InvalidRequest` で reject し、64KiB 超が
  reject される（`policy` arm は T3 — T2 時点は `unknown command` で
  `InvalidRequest`）
- [ ] `build_invocation_layer` の unit test（`name` strip・duplicate・
  malformed・empty → `None`）が pass し、`{"version":1,"command":
  {"grants":{...}}}` shape と宣言順 names を返す
- [ ] run / workflow run 経路が invocation layer を `load_policy` へ
  接続し、stderr `grant: <names>` 行と `grants.invocation` span
  field を emit する（`--quiet` で stderr 行抑制）
- [ ] `command` 非宣言 worker への `--grant` が warn-and-continue で、
  `tools.allow` 除外・pin 失敗は hard `POLICY_VIOLATION` のまま
- [ ] 全 6 `RunWorkerDeps` literal site が更新され workspace build
  （dev-seam feature 付き `--all-targets`）が green

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --test args` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli --lib` exits 0
  （`build_invocation_layer` unit test）
- [ ] `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam` exits 0
- [ ] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
