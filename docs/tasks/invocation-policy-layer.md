---
id: "TASK-034FppMiRb1TxdOmzgt11q"
type: "task"
status: "todo"
title: "T1: invocation-policy-layer — core load_policy への合成 invocation layer（compile-forced propagation 所有）"
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

# T1: invocation-policy-layer — core load_policy への合成 invocation layer（compile-forced propagation 所有）

## Scope

core `load_policy` に合成 `invocation` policy layer を実装する
（design §(a)(h)）。invoker 供給の grant map を user merge 直後に
`MergeMode::Replace` で畳み込み、当該 run の有効期間に限り user-policy
grant と同一の runtime 挙動（deny list・argv pattern・executable pin・
env/cwd 規則）を持たせる trusted layer とする（永続化しない）。
`LoadPolicyOptions.invocation` の追加は全 literal 構築点を compile error
にするため、**全 8 literal site の `invocation: None` 更新を本 task が
同梱所有し、完了時点で
`cargo build --workspace --all-targets`（dev-seam feature 付き）が
green** であることが必須条件（#101/#102 precedent —
`..Default::default()` は採用しない）。

- In: `LoadPolicyOptions.invocation` field、user merge 直後の injection
  block（grants-only key 制約 → `PolicySchemaMode::Partial` validate →
  `command.grants` collision 検査 → `MergeMode::Replace` merge →
  `policy.layer = "invocation"` debug event）、`details.layer` 畳み込み
  private helper、load-order doc comment 更新、golden harness の
  `opts.invocation` fixture key + `policy/load.json` invocation case、
  load.rs unit test、8 literal site 更新。
- Out: `--grant` flag 表面と plumbing（T2）、`read_policy_yaml` 公開と
  `policy grant` command（T3）、`grantRequests` contract（T4）、結合検証
  （T5）、docs/APM（T6）。

### 所有ファイル

- `crates/cheaper-core/src/policy/load.rs` —
  `LoadPolicyOptions.invocation: Option<&'a Map<String, Value>>` field、
  user merge 直後（:67-70 の gap）の injection block —
  (1) grants-only key 制約（top-level ⊆ `{version, command}` かつ
  `command` ⊆ `{grants}`、違反は `InvalidRequest` +
  `details.layer="invocation"`）、(2) `validate_policy_schema(Partial)`
  （内部で `resolve_command_policy` が deny list・`ArgPattern::compile`・
  env/cwd 規則を継承 — 失敗は inner code `InvalidRuntimePolicy` 保持 +
  `details.layer` 畳み込み）、(3) builtin+user 済み `command.grants` との
  key collision 検査（`InvalidRequest` + `details.grant` +
  `details.layer` — validate 通過後に行い malformed spec が collision
  error を装う順序を避ける）、(4) `merge_policy_layers(Replace)`、
  (5) `tracing::debug!(policy.layer = "invocation", ...)`（emit site は
  user :60・project :110 の sibling 同様 merge 呼出しの直前へ揃える）。
  shared validation 由来の失敗へ `details.layer` を畳み込む private
  helper（`e.details().cloned().unwrap_or_default()` + insert +
  `.with_details`）、load-order doc comment の 4-layer 更新（builtin →
  user → invocation → typed re-resolve → project、merged map でのみ発火
  する検査が layer 帰属を持たない attribution gap の accepted limitation
  明記）、unit test 4 件の literal 更新（:163/:193/:207/:222 —
  compile-forced）+ invocation 経路の新規 unit test（merge・key 制約
  reject・collision・deny 該当 spec が `InvalidRuntimePolicy` +
  `details.layer` で inner code 保持・project partial が invocation
  grant を override 不能）。
- `crates/cheaper-core/tests/golden/policy_load.rs` +
  `crates/cheaper-core/tests/golden/fixtures/policy/load.json` —
  `LoadPolicyOptions` literal の `invocation` field（:18 —
  compile-forced）、harness が `opts.invocation` fixture key を読む経路
  追加、invocation layer case（user grant への追加 accept・ordering・
  collision reject・grants 以外 key の reject・deny 該当 executable の
  `details.layer` を含む error shape）。既存 case の `expected` は不変。
- `crates/cheaper-cli/src/count.rs`（:69-73）・`src/run.rs`（:143-147）・
  `src/workflow_context.rs`（:277-281）— `LoadPolicyOptions` literal に
  `invocation: None`（compile-forced — T2 で実値へ置換）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T1）
- `docs/designs/invocation-scoped-command-grants.md` — §(a)(h) の設計判断
- `docs/specs/invocation-scoped-command-grants.md` — AC3・AC4
- `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
  — Option A 合成 trusted layer・collision 意味論・error attribution 規約
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （invocation layer 意味論・`policy_load` golden の行為保証）

## Dependencies

- なし（plan のみ。critical path の root — T2・T3・T5 を block する）

## Steps

1. `load.rs` に `LoadPolicyOptions.invocation` field を追加し、全 8
   literal site（run.rs:143・workflow_context.rs:277・count.rs:69・
   load.rs unit test :163/:193/:207/:222・golden `policy_load.rs:18`）に
   `invocation: None` を同梱する。
2. user merge 直後に injection block を実装する — grants-only key 制約
   → Partial validate → collision 検査 → Replace merge → debug event の
   順序を厳守する（collision 検査は validate 通過後 — malformed spec が
   collision error を装わない）。
3. `details.layer="invocation"` 畳み込み private helper と load-order
   doc comment（4-layer + attribution gap の accepted limitation）を
   追加する。
4. golden harness の `opts.invocation` fixture key 対応と
   `policy/load.json` の invocation case を追加する（既存 case の
   `expected` を変えない）。
5. load.rs unit test を整備し、workspace build + core test が green
   であることを確認する。

## Done When

- [ ] `LoadPolicyOptions.invocation` が存在し、grant map が user merge
  直後に `MergeMode::Replace` で畳み込まれる（project partial は
  `command` section を schema reject するため invocation grant を
  override 不能 — trusted boundary の不変条件）
- [ ] grants 以外の key・collision が `InvalidRequest` +
  `details.layer="invocation"`、deny 該当 spec が inner code
  `InvalidRuntimePolicy` 保持 + `details.layer` で fail する unit test
  と golden invocation case が pass する
- [ ] 全 8 literal site が `invocation: None` で更新され、
  `cargo build --workspace --all-targets`（dev-seam feature 付き）が
  green — compile-forced propagation 完遂の証明

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` exits 0（load.rs unit
  test + `--test golden` の `policy_load`/`policy_schema`/
  `policy_restrict`/`policy_merge` を含む）
- [ ] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
