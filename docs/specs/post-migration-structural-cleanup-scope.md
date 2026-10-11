---
id: "SPEC-034DyDAOW1my2ihNQIVNfN"
type: "spec"
status: "approved"
title: "Post-migration structural cleanup scope"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/discovery/post-migration-structural-inventory-and-cleanup-policy.md"
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
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "../adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
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
---

# Post-migration structural cleanup scope

## What Should Be Built

Rust + Rig 移行後の workspace(7 crate)に対する構造整理の実施。
DISC-034DxZdeAdWoFG3UuTyh5c でレビュー確定した採用方針に従い、以下の
6 系統を behavior-preserving で整理する:

| # | 対象 | 採用方針 | 内容 |
| --- | --- | --- | --- |
| F3/F4/F7 | 移行残滓・表現ドリフト | M-B | `cheaper-rig-adapter::js_json` re-export shim 削除 + `cheaper-tools::js16` を `cheaper-core` へ移設(JS 互換知識の一元化)+ `cheaper-provider-devin` の `src/tests/` を `tests/` へ分離 |
| F5 | `poc-rig` spike | P-A | crate 削除(削除前に設計知見が design/ADR へ移管済みか確認) |
| F1 | cheaper-tools 構造 | T-B | crate 内 module 再編(glob 系・runtime spine 系の subdir 化)。crate 分割はしない |
| F6 | cheaper-runtime 構造 | R-B | module 整理のみ(auth/catalog の crate 分離はしない) |
| F8 | provider 配置規約 | PV-B | `cheaper-provider-devin` を `cheaper-provider` crate へ rename し、devin 実装を `devin` module として内包 |
| F2 | dev/test seam | S-B | `fake_provider`・`fixture`・`fake_stream` 等の dev seam を `dev-seam` cargo feature で gate し、default build(consumer install)から除外 |

## Why Now

移植直後は TS 構造の持ち込み歪み(shim・責務の偏り・test seam の公開面)が
最も見えやすい時期であり、機能追加再開前に整理基準と実施を済ませる。
この整理は後続フェーズ(ロギング/プロファイリング環境整備 → 性能最適化)
の前提でもある。

## Users and Value

- cheaper のメンテナ(スコープ確定・レビューの主体): 整理基準が明文化
  され、以後の構造判断がブレない。
- この repo で作業する coding agent: crate/module の責務境界が明確になり、
  変更対象の探索コストと誤配置リスクが下がる。

## Scope

### In Scope

- 上表 6 系統の実施(ファイル/module 移動・rename・feature 追加・削除)。
- `dev-seam` feature を必要とする全実行経路の更新(mise task・
  benchmarks・regression・ollama e2e)。
- crate rename に伴う依存宣言・パス・CI/スクリプトの更新。
- 実施に伴う AGENTS.md・関連 doc の crate 構成記述の追従。
- 実施順序はリスク昇順: 移行残滓除去(F3/F5/F7)→ 表現ドリフト集約(F4)
  → 境界再編(F1/F6)→ provider 再配置(F8)→ dev seam gate 化(F2)。

### Out of Scope

- crate **分割**(tools の T-C・runtime の R-C は不採用として棄却済み)。
- 機能追加・worker 追加・envelope schema 拡張・外部 contract の変更。
- ロギング/プロファイリング環境の設計・導入(別文書で後続)。
- 性能最適化・バイナリサイズ削減の実施。
- `packages/apm/`・launcher・hooks.json の変更。
- `poc-rig` 以外の dead code の全面走査(発見次第の除去は許容)。

## Requirements

- 全変更は behavior-preserving。envelope `schemaVersion:1` byte parity・
  exit code 表・usage.jsonl・hook 4 target・config 書式を変えない。
- 不変条件を維持: Rig 型は adapter と devin provider 実装のみ・
  `cheaper-core` は rig-free/tokio-free・usage 集計の正は hook
  accumulator・`panic = "unwind"`。
- `js16` の core 移設は caller 調査(参照が tools 内に閉じるかの確認)を
  先行させ、外部利用が無いことを確認してから実施する。
- `poc-rig` 削除は Phase 0 の実測知見が design/ADR に移っていることを
  確認してから実施する。
- `dev-seam` feature 有効化時に全テスト/benchmark 経路が現行通り動くこと。
  default feature では seam を含まない。
- `cheaper-provider` crate は `CheaperProviderPlugin` contract の実装面を
  変えず、cli の plugin 登録経路(composition root)を維持する。
- 実施は意味のある境界で分割コミット(Conventional Commits)。

## Acceptance Criteria

- [ ] `mise run check` が全工程 green(biome→fmt→clippy→version/doc-id→
  cargo test→node test→lockfile/payload check)。
- [ ] `cheaper-cli` の regression suite(55 cases)が無変更で pass
  (envelope byte parity・exit code・hook 挙動の維持を示す)。
- [ ] `cargo build -p cheaper-cli` の default feature では dev seam 系
  module(`fake_provider`・`fixture`・`fake_stream`)がコンパイル対象に
  含まれず、`CHEAPER_TEST_FAKE_STREAM` を設定しても seam が起動しない。
- [ ] `dev-seam` feature 有効の build/test/benchmark 経路が現行と同等に
  動作する(benchmarks/run.mjs・ollama e2e・fixture テスト)。
- [ ] `cheaper-rig-adapter` に `js_json` module/参照が残っていない。
- [ ] `js16` の定義が `cheaper-core` にあり、tools 側は re-export または
  core 参照に一本化されている。
- [ ] `crates/poc-rig` が存在せず、workspace が正常に build できる。
- [ ] `cheaper-provider` crate が `devin` module を持ち、devin 関連の
  全テストが `tests/` 配下で pass する。
- [ ] crate 名・module 配置を記述している doc(AGENTS.md 等)が新構成を
  反映している。

## Deferred Design Concerns

- ロギング環境(tracing 等)の導入 — 構造整理完了後に別文書で設計。
- プロファイリング/マイクロベンチ環境 — 同上。
- `crates/provider/` 配下への provider 別 crate 展開(PV-C)— 異質な
  依存を持つ provider を bundle する必要が生じた時点で再検討。

## Sources and Relations

- 上流: DISC-034DxZdeAdWoFG3UuTyh5c(棚卸し・評価基準・採用方針の確定)。
- 関連決定: dev-seam feature-gate 化と provider 配置規約は本 spec と
  並行して ADR 化する(created 後に `related` へ追記)。
