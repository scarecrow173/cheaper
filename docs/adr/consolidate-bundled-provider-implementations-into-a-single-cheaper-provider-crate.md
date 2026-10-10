---
id: "ADR-034DyGzba1FVqBR7PZCchF"
type: "adr"
status: "accepted"
title: "Consolidate bundled provider implementations into a single cheaper-provider crate"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/post-migration-structural-inventory-and-cleanup-policy.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/post-migration-structural-cleanup-scope.md"
    - "../designs/post-migration-structural-cleanup-reorganization.md"
    - "../plans/execute-post-migration-structural-cleanup.md"
    - "../discovery/post-migration-structural-inventory-and-cleanup-policy.md"
    - "./gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "./devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
    - "./provider-only-registration-via-explicit-bundled-plugin-contract.md"
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
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Consolidate bundled provider implementations into a single cheaper-provider crate

## Context and Problem Statement

bundled provider 実装の配置規約が未確立(DISCO の F8)。現在の唯一の
bundled provider である devin は `cheaper-provider-devin` という単独
crate として workspace 直下に置かれており、今後 provider を追加するたび
`cheaper-provider-<name>` が flat に増殖する形になる。

provider plugin contract(`CheaperProviderPlugin`)は `cheaper-runtime`
所有で既に分離済みであり、問題は「実装側の置き場」の規約だけ。
`cheaper-provider-devin` は Rig 接触面の一つ(隔離原則の意図的例外:
custom `Wire`/`Decoder`/`Transport`)でもあり、移動してもその役割は維持
する必要がある。

## Decision Drivers

* 「bundled provider 実装の置き場」が 1 箇所に集約され、追加時の規約
  迷いが消えること
* 現行 provider は devin の 1 つ — YAGNI に反する先行の階層化を避ける
* plugin contract(runtime 所有)・composition root(cli による登録)の
  依存方向を変えないこと
* Rig 型の隔離原則(adapter + provider 実装の 2 接触面)を維持すること
* 将来、cheaper の依存モデルと合わない異質依存を持つ provider が出た
  場合の退避先を残すこと

## Considered Options

* PV-A: 現状(`cheaper-provider-<name>` を flat に増やす)
* PV-B: 単一 `cheaper-provider` crate に provider 別 module として集約
* PV-C: `crates/provider/` 配下に provider 別 crate(workspace members へ
  `crates/provider/*` を追加する入れ子構成)

## Decision Outcome

Chosen option: "PV-B: 単一 `cheaper-provider` crate + provider 別 module"、
because 1 provider の現状で最も単純かつ規約を明示でき、将来の依存分離
要求には PV-C への移行トリガーを明記して退避先を確保できる。

### Consequences

* Good, because bundled provider 実装の置き場が `cheaper-provider` に
  集約され、新規 provider は `src/<name>/` module 追加で規約に従える。
* Good, because workspace 直下の crate 増殖が止まる。
* Bad, because 将来 provider が異質な依存(異なる rig 系・独自 transport
  スタック)を持つと 1 crate に deps が合流する — その時点で PV-C へ
  移行する。
* Bad, because crate rename は依存宣言・import path・関連 doc の一括
  書き換えを要する(機械的だが差分が広い)。

### Confirmation

`SPEC-034DyDAOW1my2ihNQIVNfN` の acceptance criteria により検証される
(`cheaper-provider` crate が `devin` module を持ち全テスト pass)。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-provider-devin/` → `crates/cheaper-provider/` へ
    git rename(package name も `cheaper-provider` へ)
  - `src/` 内の devin 実装を `src/devin/` module 配下へ移動
    (cli/context_map/credentials/jwt/metadata/models/plugin/
    rig_provider/stream/wire)
  - `src/lib.rs` は `pub mod devin;` + 従来の re-export を
    `pub use devin::*` 系で維持
  - `src/tests/` → `tests/` へ分離(F7 の実施と同じ着地点)
  - `Cargo.toml` workspace members は `crates/*` のまま変更不要。
    `cheaper-cli` の依存宣言 `cheaper-provider-devin` → `cheaper-provider`
    (path も更新)
  - `cheaper-cli` の import `cheaper_provider_devin::` →
    `cheaper_provider::devin::`(または `cheaper_provider::`)
  - crate 名・構成を記述する doc(AGENTS.md 等)の追従
* Patterns to follow:
  - provider 実装の公開入口は `cheaper_provider::<name>` module とし、
    `DevinProviderPlugin` は `cheaper_provider::devin` 経由で cli が
    登録する(composition root 構造は不変)。
  - Rig 型の直接接触は引き続き `cheaper-rig-adapter` と provider 実装
    module のみに限定する(隔離原則の例外は provider 実装側)。
* Patterns to avoid:
  - plugin contract(`CheaperProviderPlugin`)を `cheaper-provider` 側へ
    移動すること — contract は runtime 所有で固定(逆向き依存禁止)。
  - provider 実装を `cheaper-runtime` 内へ置くこと — runtime → provider
    依存は plugin contract の設計に反する。
  - 現段階での `crates/provider/` 入れ子化 — provider が 1 つの現状で
    workspace 構成を複雑化しない。
* Dependencies or configuration:
  - crate rename のみで機能追加は無し。devin の隔離 data dir
    (`<userConfigDir>/providers/devin/data`)・credential 経路・CLI 委譲
    auth の contract は不変。

## Verification

* [ ] `cargo test --workspace` が全 pass(rename 後のビルド・テスト)。
* [ ] `cargo tree -p cheaper-cli` の依存に `cheaper-provider` が現れ、
  `cheaper-provider-devin` が残っていない。
* [ ] `grep -r cheaper_provider_devin crates/` が 0 件。
* [ ] devin 関連テストが `tests/` 配下で pass する。
* [ ] `cheaper_provider::devin` 経由で `DevinProviderPlugin` が
  cli(composition root)から登録される。
* [ ] regression suite 全 pass(devin 経路の envelope 不変)。

## Pros and Cons

### PV-A: 現状(flat 増殖)

* Good, because provider ごとの依存が完全に分離される。
* Bad, because 配置規約が暗黙のままで、crate が増えると workspace 直下の
  発見性が下がる。

### PV-B: 単一 `cheaper-provider` crate + provider 別 module

* Good, because 置き場が 1 箇所に集約され、規約が明示される。
  1 provider の現状で最も単純。
* Bad, because provider 間で deps が合流する(異質依存が来たら PV-C へ
  移行)。

### PV-C: `crates/provider/` 配下に provider 別 crate

* Good, because 依存分離と配置規約を両立できる。
* Bad, because provider が 1 つの現状では入れ子 workspace 構成が過剰
  (YAGNI)。

## More Information

* 上流: DISC-034DxZdeAdWoFG3UuTyh5c(F8・PV-B 採用の確定記録)。
* 関連 spec: SPEC-034DyDAOW1my2ihNQIVNfN(実施スコープ F8 系統)。
* plugin contract・devin bundled plugin の背景は ADR-034r05iHrZUE6WqcIaH1CU
  (devin local provider)・provider-only registration ADR を参照。
* PV-C への移行トリガー: cheaper の依存モデルと合わない異質依存を持つ
  provider を bundle する必要が生じたとき。
