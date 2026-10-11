---
id: "ADR-034DyGyITd0YaATg8fkld7"
type: "adr"
status: "accepted"
title: "Gate dev/test seams behind a dev-seam cargo feature"
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
    - "./consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
    - "./benchmark-harness-placement-and-accounting.md"
    - "./use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
    - "./auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/post-migration-structural-inventory-and-cleanup-policy.md"
  derived-by:
    - "../designs/post-migration-structural-cleanup-reorganization.md"
    - "../plans/execute-post-migration-structural-cleanup.md"
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

# Gate dev/test seams behind a dev-seam cargo feature

## Context and Problem Statement

Rust workspace 内の dev/test seam が production コードと完全に同居して
いる(DISCO の F2):

- `cheaper-runtime` が `fake_provider`(949 行・scripted provider seam)と
  `fixture`(905 行・fixture DSL + loader)を `pub mod` + `pub use` で公開
  API として export している。
- `cheaper-cli` の `fake_stream`(`CHEAPER_TEST_FAKE_STREAM` env seam)と
  `run.rs`/`workflow_run.rs` の fixture plumbing(session_factory fake 化・
  env overlay・`cheaper-fake` plugin 登録・`ScriptedAuthInteraction`)が
  production の run 経路に組み込まれている。
- workspace の cargo feature は現状ゼロ。consumer install
  (`cargo install --git --locked cheaper-cli`)で seam も含めて全量が
  ビルド・同梱される。

一方で seam は benchmarks(`CHEAPER_TEST_FAKE_STREAM` を release binary へ
設定)・`cheaper-runtime/tests`(run.rs・run_fixture_test.rs・ollama e2e の
skip 判定経路)・`cheaper-cli` regression(run-* fake stream cases)の実行
基盤であり、単純削除や `#[cfg(test)]` 移動では動かない。

## Decision Drivers

* production crate の公開 API・consumer binary から test 用実装を外す
* dev/test/benchmark/e2e の全実行経路を現行通り維持する
* behavior-preserving(seam 自体の機能・fixture 契約は変更しない)
* feature flag の導入はこれが初めて — 運用コストを正当化できるか
* fake seam が catalog/auth/stream の 3 層に跨る既存設計(G7)を壊さない

## Considered Options

* S-A: 現状維持(src + pub export のまま)
* S-B: `dev-seam` cargo feature で gate(default off、dev/test 経路で有効化)
* S-C: testsupport crate へ分離

## Decision Outcome

Chosen option: "S-B: `dev-seam` cargo feature で gate"、because seam の機能を
保ったまま consumer build から除外できる唯一の選択肢であり、注入点
(内部 trait 実装)が pub に残る S-C より実益が大きい。

### Consequences

* Good, because consumer install が seam コードを含まない binary を得る
  (攻撃面・公開 API の純化)。
* Good, because 「seam は開発用」という意図が feature 宣言として明示化される。
* Good, because `cargo test --features` の形で既存テストがそのまま活きる。
* Bad, because workspace に初めて cargo feature が入り、mise task・bench・
  e2e・regression の全実行経路に feature 有効化の配線が必要になる。
* Bad, because `run.rs`/`workflow_run.rs` の fixture plumbing を `#[cfg]` で
  分岐させる箇所が増え、読みにくさが若干増す。

### Confirmation

`SPEC-034DyDAOW1my2ihNQIVNfN` の acceptance criteria により検証される
(default build で seam 非含有・feature 有効で全経路 pass)。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-runtime/Cargo.toml` — `[features] dev-seam = []` 追加
  - `crates/cheaper-runtime/src/lib.rs` — `fake_provider`・`fixture` の
    `pub mod`/`pub use` を `#[cfg(feature = "dev-seam")]` で gate
  - `crates/cheaper-cli/Cargo.toml` —
    `dev-seam = ["cheaper-runtime/dev-seam"]` 追加
  - `crates/cheaper-cli/src/fake_stream.rs` — module 全体を gate
  - `crates/cheaper-cli/src/run.rs`・`workflow_run.rs` — fixture plumbing
    (`load_test_stream` 呼出し・`fixture` 引数・`cheaper-fake` plugin 登録・
    `ScriptedAuthInteraction`)を `#[cfg]` で分岐
  - `mise.toml` — test/bench 系 task の cargo 引数へ feature 有効化を追加
  - `benchmarks/`・`crates/cheaper-cli/tests/regression` — binary build 時の
    feature 有効化を明文化
* Patterns to follow:
  - feature 名は `dev-seam` 固定(runtime と cli で同名、cli は
    `cheaper-runtime/dev-seam` へ forward)。
  - feature 有効化は workspace 経由で
    `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
    の形で指定(対象 crate を明示して解決の曖昧さを避ける)。
  - feature off 時の fallback は「seam を呼ばない」でなく「seam コード自体を
    コンパイル対象外にする」— `load_test_stream` 相当を呼ぶ側を cfg 分岐する。
* Patterns to avoid:
  - seam を `#[cfg(test)]` で隠すこと — integration test・benchmarks・
    regression harness から到達できなくなる。
  - feature 名の複数化(`fake`/`fixture`/`test-utils` 等) — seam は 1 機能
    として扱い `dev-seam` に集約。
  - default feature への暗黙追加 — consumer install が seam を拾うと目的を
    破壊する。
* Dependencies or configuration:
  - `CHEAPER_TEST_FAKE_STREAM`・`NODE_ENV=test` の activation 規則は不変
    (feature 有効時のみ意味を持つ)。
  - `cargo install --git --locked cheaper-cli` は default feature のまま
    (feature 指定を consumer に要求しない)。

## Verification

* [ ] `cargo build -p cheaper-cli`(default)で `fake_provider`・`fixture`・
  `fake_stream` がコンパイル対象外(`cargo check` + env 設定でも seam 不発)。
* [ ] `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` が全 pass。
* [ ] `mise run check` green(task 更新込み)。
* [ ] benchmarks(`benchmarks/run.mjs`)が feature 有効 binary で動作する。
* [ ] regression suite の run-* cases(fake stream 使用)が pass する。
* [ ] ollama e2e(`CHEAPER_E2E_OLLAMA=1`)が feature 有効で動作する。

## Pros and Cons

### S-A: 現状維持

* Good, because 変更ゼロ。全経路がそのまま動く。
* Bad, because 公開 API と consumer binary に test 実装が残り続ける。
  「seam は dev 用」の意図がコード構造から読めない。

### S-B: `dev-seam` feature gate

* Good, because consumer build から除外しつつ dev 経路を維持できる。
* Bad, because feature 運用(有効化の配線・認知)のコストが新規に発生する。

### S-C: testsupport crate 分離

* Good, because 本番 crate の責務が純化される。
* Bad, because fake session は runtime 内部 trait を実装するため注入点は
  pub に残り、binary からの除外効果が得られない。crate 追加の割に実益が薄い。

## More Information

* 上流: DISC-034DxZdeAdWoFG3UuTyh5c(F2・S-B 採用の確定記録)。
* 関連 spec: SPEC-034DyDAOW1my2ihNQIVNfN(実施スコープ F2 系統)。
* fake seam の 3 層設計(catalog/auth/stream)の背景は
  DESIGN-034Bbq1qY8WFBdL26tUdBw §crate 構成(G7)を参照。
