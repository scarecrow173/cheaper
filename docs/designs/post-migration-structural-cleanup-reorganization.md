---
id: "DESIGN-034DyR22wfkjn91McXS5jv"
type: "design"
status: "approved"
title: "Post-migration structural cleanup reorganization"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/post-migration-structural-cleanup-scope.md"
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
    - "../plans/execute-post-migration-structural-cleanup.md"
    - "../discovery/post-migration-structural-inventory-and-cleanup-policy.md"
    - "../adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/post-migration-structural-cleanup-scope.md"
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-migration structural cleanup reorganization

## Context

Rust + Rig 移行後の 7-crate workspace に対する behavior-preserving な構造整理。
対象・採否基準・実施順序は spec(SPEC-034DyDAOW1my2ihNQIVNfN)で確定済みで、
横断判断 2 件は ADR(dev-seam feature gate、provider crate 集約)で確定済み。
本 design は spec の 6 stream を「どこに・どの形で配置し直すか」の
target layout に落とす。振る舞い・公開 contract は一切変更しない。

## Scope

### In Scope

- `cheaper-tools` 内部モジュール再配置(F1 / T-B)
- `cheaper-runtime` 内部モジュール再配置(F6 / R-B)
- `js_json` shim 削除(F3)、`js16` の core 集約(F4)、provider test 移動(F7)
- `poc-rig` 削除(F5)
- `cheaper-provider-devin` → `cheaper-provider` rename + `devin` module 化(F8 / PV-B)
- `dev-seam` cargo feature による dev/test seam の gate(F2 / S-B)

### Out of Scope

- crate 分割・統合の追加判断、公開 API/外部 contract の変更
- logging/profiling 基盤の実装(後続フェーズで別文書)
- 機能追加・worker 追加・envelope schema 変更・性能/サイズ最適化
- `packages/apm`・launcher・`hooks.json`・無関係な dead-code 整理

## Components and Boundaries

### `cheaper-core`(F4)

- `js16` を新規 module `src/js16.rs` として追加し `pub mod js16` で公開。
  `cheaper-tools::js16` から実装を移動し、`cheaper-tools` 側は
  `pub use cheaper_core::js16;` の thin re-export のみ残す(既存
  `cheaper_tools::js16` 参照の互換のため)。
- Rig 非依存・Tokio 非依存の既存境界に抵触しない(純粋な UTF-16 helper)。

### `cheaper-rig-adapter`(F3)

- `src/js_json.rs` を削除。`js_json` を参照する consumer は
  `cheaper_core::json::stringify` を直接参照するよう import を更新。

### `cheaper-tools`(F1 / T-B)

crate は分割しない。target layout:

```
src/
  glob/
    mod.rs            # 現 glob.rs
    tables.rs         # 現 glob_tables.rs(生成テーブル)
  tools/
    process/
      mod.rs          # 現 process.rs
      executable.rs   # 現 process_executable.rs
      tree.rs         # 現 process_tree.rs
    # boundary, count_tokens, edit, find, grep, grep_eval, ls, read, write は現位置
  # runtime spine(access, accounting, artifact, catalog, definition,
  # execution_metrics, invocation, journal, limits, line_framer, operation,
  # registry, resources, scheduler, walker)と bin/ は root 維持
```

- `pub use` 再配置で外部公開 API(`cheaper_tools::*` の path)を維持する。
  `mod glob` 宣言は module 化に追従、参照 path は `crate::glob::*` で不変。
- `grep_eval` は dev seam 性質を持つ可能性があるため、F2 の gate 判定時に
  consumer を確認して分類する(今回の stream では移動しない)。

### `cheaper-runtime`(F6 / R-B + F2 / S-B)

crate は分割しない。target layout:

```
src/
  auth/               # 現行維持
  catalog/            # 現行維持
  run/
    mod.rs            # 現 run.rs
    error.rs          # 現 run_error.rs
    abort.rs          # 現 abort.rs
    grace.rs          # 現 grace.rs
    repair.rs         # 現 repair.rs
    output.rs         # 現 output.rs
    meta.rs           # 現 meta.rs
    panic_boundary.rs # 現 panic_boundary.rs
  dev/
    mod.rs            # #[cfg(feature = "dev-seam")] で gate された module
    fake_provider.rs  # 現 fake_provider.rs
    fixture.rs        # 現 fixture.rs
  lib.rs, plugin.rs, runtime.rs   # 現行維持
```

- run pipeline 関連の root module を `run/` に集約し orchestration 単位を
  明示する。`pub use` で公開 path を維持。
- `dev/` module は `#[cfg(feature = "dev-seam")]` で compile 対象化。
  default build では一切含まれない。

### `cheaper-provider`(F8 / PV-B + F7)

- ディレクトリ `crates/cheaper-provider-devin/` → `crates/cheaper-provider/`。
- package/lib 名 `cheaper-provider-devin` → `cheaper-provider`
  (crate 参照 `cheaper_provider`)。
- target layout:

```
crates/cheaper-provider/
  src/
    lib.rs            # `pub mod devin;` のみを公開する薄い root
    devin/
      mod.rs          # DevinProviderPlugin 公開面(現 lib.rs/plugin.rs の公開 API)
      cli.rs, context_map.rs, credentials.rs, jwt.rs, metadata.rs,
      models.rs, plugin.rs, rig_provider.rs, stream.rs, wire.rs
  tests/              # 現 src/tests/ から移動(統合 test として)
    cli_credentials.rs, context_map.rs, jwt.rs, models.rs,
    plugin.rs, rig_provider.rs, stream.rs, wire.rs
```

- consumer 参照 path は `cheaper_provider::devin::*`。
- `dev` module 名は provider 名と一致させ、将来 provider は `src/<name>/`
  を追加する規約とする(ADR-034DyGzba1FVqBR7PZCchF)。
- `CheaperProviderPlugin` contract は `cheaper-runtime` 所有のまま。
  composition root(`cheaper-cli`)での明示 register 経路も不変。

### `dev-seam` feature(F2 / S-B)

- `cheaper-runtime/Cargo.toml`: `[features] dev-seam = []`。
- `cheaper-cli/Cargo.toml`: `[features] dev-seam = ["cheaper-runtime/dev-seam"]`。
- gate 対象: runtime `dev/` module(fake_provider・fixture)、cli の
  `fake_stream` および fixture plumbing(session_factory・env overlay・
  `cheaper-fake` plugin 登録、scripted auth 経路)。
- `CHEAPER_TEST_FAKE_STREAM`・`NODE_ENV=test` の活性化 semantics は
  feature 有効時に現行通り維持する。
- feature 前提の経路(benchmarks、regression 55 case、Ollama e2e、
  integration test)は feature 有効 binary で実行するよう
  `mise.toml` task / `benchmarks/run.mjs` / test 手順を更新する。

### `poc-rig` 削除(F5)

- `crates/poc-rig` を削除。workspace `exclude` エントリも除去。
- 有用な知見は移行設計書・ADR に取り込み済みであることを削除前に確認する。

## Data and Control Flow

振る舞い変更は無いため実行時 flow は不変。変更の適用順序は spec の
risk-oriented order に従う:

1. `js_json` 削除 + `poc-rig` 削除 + provider test 移動(低リスクの残滓除去)
2. `js16` の core 集約(表現知識の中央化)
3. `cheaper-tools` / `cheaper-runtime` の module 再配置
4. provider crate rename + `devin` module 化
5. `dev-seam` feature 化(default build からの seam 除外は最後に行い、
   各段階で `cargo test --workspace` が green であることを保つ)

## Non-Functional Constraints

- 外部 contract 不変: envelope `schemaVersion: 1`・exit code 表・
  `usage.jsonl`・hook 4 target・config/worker schema・credential/catalog
  persistence path・devin isolated data dir。
- Rig 参照は `cheaper-rig-adapter` と bundled provider 実装のみ。
  `cheaper-core`/`cheaper-workflow` は Rig-free、core は Tokio-free。
- runtime ↔ provider の依存方向は provider → runtime(plugin contract 側)で
  cycle を作らない。`panic = "unwind"` 維持。
- hook accumulator が usage 集計の正であることを維持。
- feature-off(default)build で dev/test seam コードを含まない。

## Risks and Trade-offs

- `pub use` 維持を怠ると内部 module 移動が外部参照を壊す → 各 stream で
  `cargo build --workspace` + 参照 grep を verification に含める。
- `dev-seam` gate は test/benchmark/e2e の activation semantics を
  feature 有効時にのみ保つ → 無効時に env が残っていても fake 経路は
  絶対に起動しないことを compile 単位で保証する。
- provider rename は workspace 参照・doc 参照の洗い替えが広い →
  `cheaper_provider_devin` 残存参照 0 を acceptance criteria で検査。
- `cheaper-tools` の spine module をさらに subdir 化する案は見送り:
  現状の flat 配置は module 単位が self-contained で、人工的な深い階層を
  作る利益がない(KISS)。glob・process のみ凝集度が明白なため限定して移動。

## Deferred Design Concerns

- PV-C(`crates/provider/` 配下の provider 別 crate)| 現状は provider 1 個で
  YAGNI | 異質な依存を持つ provider の bundle 要求が発生した時点 | 単一
  `cheaper-provider` への依存合流。
- `cheaper-tools`・`cheaper-runtime` の crate 分割 | 境界が適切なら大きさは
  問題にしない方針確定済み | 境界違反・build 影響の実測が出た時点 |
  変更面の局所性が下がる。
- logging/profiling 基盤 | 構造整理後に別文書で設計 | 本 cleanup 完了後 |
  計測不能なまま最適化を進めると根拠のない変更になる。

## Verification Notes

- [ ] `mise run check` green(cargo test workspace・clippy・fmt・doc-id)
- [ ] `cargo build -p cheaper-cli`(default features)で fake_provider・
      fixture・fake_stream が binary に含まれない
- [ ] feature 有効 build で benchmark harness・regression 55 case・
      Ollama e2e が現行通り動作
- [ ] `cheaper-rig-adapter` に `js_json` 参照残存 0、`js16` が
      `cheaper_core::js16` で唯一の定義となる
- [ ] `crates/poc-rig` 削除後に workspace build/test が green
- [ ] `cheaper_provider_devin` 参照残存 0、provider test が top-level
      `tests/` で実行される
