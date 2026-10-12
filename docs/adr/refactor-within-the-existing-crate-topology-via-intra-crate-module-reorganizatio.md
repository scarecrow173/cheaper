---
id: "ADR-034EDk9uMH1AuZQn3BMpe4"
type: "adr"
status: "accepted"
title: "Refactor within the existing crate topology via intra-crate module reorganization"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/post-feature-structural-refactoring-inventory.md"
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
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../designs/post-feature-structural-refactoring.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/post-feature-structural-refactoring-inventory.md"
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

# Refactor within the existing crate topology via intra-crate module reorganization

## Context and Problem Statement

v1.0.0 時点の workspace 全体調査（DISC-034EDjXUQnxTiHz4batpQu）で、依存 DAG が
一方向・循環なし・dev-seam gate 完備と健全である一方、crate 内の巨大ファイル・
移植由来の小規模重複・責務の同居（cli の composition root 同居、runtime の
catalog 肥大、tools の巨大関数、workflow の Pump workaround 等）が確認された。
どの粒度で整理するかの方針を決める必要がある。

## Considered Options

* **crate トポロジ再編**（crate 統合・新規 crate 分割）— 境界は既に明確で
  依存方向も正しいため、得られる効果に対して差分・検証コストが大きい。
* **crate 内 module 再編 + 共有 helper 集約** — 観測可能挙動を変えずに
  段階的に実施でき、differential fixture・`cheaper-cli/tests/envelope.rs`・
  regression test で防衛可能。
* **現状維持** — 巨大ファイルと重複が育ち、変更コストが上がり続ける。

## Decision Outcome

Chosen option: "crate 内 module 再編 + 共有 helper 集約"。crate トポロジ・
依存方向・観測可能挙動は不変条件として維持し、課題は crate 内の
module/関数分割と同一知識の一権威実装化で解消する。Phase 2（boundary
adapter 単一化・args.rs table 化・ToolError enum 化等の設計判断を要する
もの）は本判断の対象外として deferred とする。

## Implementation Plan

* Affected paths: `crates/cheaper-cli`、`crates/cheaper-core`、
  `crates/cheaper-runtime`、`crates/cheaper-tools`、
  `crates/cheaper-workflow`、`crates/cheaper-provider`、
  `crates/cheaper-rig-adapter` の src/tests、`Cargo.toml`（workspace deps）、
  `.gitignore`（Node boilerplate 整理）、`benchmarks/results/`（保持方針の
  明文化 — 明文化先は `benchmarks/` 配下の README 等を実装時に決定）、
  `docs/research/structural-refactoring-inventory/`（調査レポートの
  canonical 保存）。
* Constraints: envelope/error code/exit code・code-unit sort・abort 意味論・
  usage.jsonl・worker schema・CLI 引数 surface を変更しない。例外は
  `workflow run --timeout` の `details` 欠落修正のみ（調査で確認した
  contract drift の是正 — spec Requirements の例外規定を参照）。
  `glob/mod.rs`・`line_framer.rs`・resources.rs の deferred callback 設計・
  lock 順序は対象外。

## Verification

* [ ] `mise run check` が全 pass する
* [ ] envelope/error/exit code の差分が既存テストで検出されない
  （意図的な `workflow_run` timeout details 追補のみ許容）
* [ ] tools の characterization test が構造変更に先行している
