---
id: "PLAN-034Bcr7oMH85Ak8UCpCDmy"
type: "plan"
status: "approved"
title: "Migrate cheaper runtime to Rust + Rig"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
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
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  derived-by:
    - "docs/plans/phase-0-rig-poc-go-no-go-gate.md"
    - "docs/plans/phase-1-port-cheaper-core.md"
    - "docs/plans/phase-2-port-cheaper-tools.md"
    - "docs/plans/phase-3-port-runtime-adapter-providers.md"
    - "docs/plans/phase-4-port-cli-and-workflow.md"
    - "docs/plans/phase-5-switch-distribution-to-rust-binary.md"
    - "docs/plans/phase-6-remove-typescript-node-pi.md"
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-envelope-parity.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
    - "docs/test-specs/rust-migration-run-semantics.md"
    - "docs/test-specs/rust-migration-usage-log-hooks.md"
    - "docs/test-specs/rust-migration-auth-catalog.md"
    - "docs/test-specs/rust-migration-distribution.md"
  references: []
  defers: []
  deferred-by: []
---

# Migrate cheaper runtime to Rust + Rig Implementation Plan(親 plan)

## Goal

Pi SDK / TypeScript / Node.js ランタイム依存を除去し、Rust + Rig 実装の
`cheaper` CLI を既存の外部 contract(envelope・exit code・usage.jsonl・
hook・launcher・Release asset)を維持したまま段階的に切り替える。

## Scope

- 本 plan は**親 plan**として全体の段階構成・共通ルール・横断リスクのみを
  管理する。各フェーズの詳細(実装手順・ファイルマップ・検証・ロールバック)
  は `derived-by` の 7 つの phase plan が担う。
- 非スコープ: 配布 asset 名・schemaVersion・launcher 改修、Pi state 移行
  (再 login 要求)、models.dev online refresh(v1 不実装)、新機能追加。

## Phase Plans

| Phase | Plan | 依存 |
| --- | --- | --- |
| 0 | [phase-0-rig-poc-go-no-go-gate.md](phase-0-rig-poc-go-no-go-gate.md) | —(gate) |
| 1 | [phase-1-port-cheaper-core.md](phase-1-port-cheaper-core.md) | Phase 0 go |
| 2 | [phase-2-port-cheaper-tools.md](phase-2-port-cheaper-tools.md) | Phase 1 |
| 3 | [phase-3-port-runtime-adapter-providers.md](phase-3-port-runtime-adapter-providers.md) | Phase 0 go + Phase 1/2 |
| 4 | [phase-4-port-cli-and-workflow.md](phase-4-port-cli-and-workflow.md) | Phase 3 |
| 5 | [phase-5-switch-distribution-to-rust-binary.md](phase-5-switch-distribution-to-rust-binary.md) | Phase 4 + differential golden 全パス |
| 6 | [phase-6-remove-typescript-node-pi.md](phase-6-remove-typescript-node-pi.md) | Phase 5 + 並走保守期間経過 |

## Common Rules(全 Phase 共通)

- `rig`/`rig-core`/`rig-agent` は `=0.43.x` pin。Rig 型の参照は
  `cheaper-rig-adapter` と `cheaper-provider-devin` のみ。
- stdout 契約: envelope 1 JSON + 末尾改行のみ。診断・secret は stderr/
  永続 store。
- release binary(strip 後)≤20MB を受け入れ閾値、5-15MB を目標。
- usage/turns/toolCalls は hook ベース逐次集計を唯一の正とする。
- `usage.costUsd` は pricing 層算出。differential 比較では normalize
  除外(設計書 §Verification Notes の normalize 規則が正)。
- 並走期間中は TS 版を CI で生かし、Phase 0 go 後は TS 側新機能凍結。
- panic/JoinError は `INTERNAL_ERROR`(exit 70)envelope へ写像、stdout
  panic 出力を抑止、hook は fail-open 不変。

## Dependencies and Risks(横断)

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | Rig 0.x breaking change | high | med | `=0.43.x` pin + adapter 隔離 + 計画 upgrade |
| 2 | devin provider custom transport 不達 | med | high | Phase 0 で先行検証、不達なら Goose ピボット |
| 3 | バイナリサイズ超過 | low | high | Phase 0 実測 + Phase 5 CI size assert |
| 4 | 並走期間の二重保守ドリフト | med | med | Phase 0 go 後 TS 新機能凍結 |
| 5 | Phase 0 no-go | low | high | 本 plan を `blocked` にし Goose GDK 案(migration-plan 済)を評価 |

## Verification(全体完了条件)

- [ ] 全 Phase plan が `completed`
- [ ] 設計書 §Verification Notes の全項目が各 Phase plan で消化済み
- [ ] `packages/` の TS 実装・Pi 依存・Node SEA ビルド経路が撤去済み
- [ ] Windows/Linux/macOS release binary ≤20MB、asset 名・SHA256SUMS・
      launcher 無改修動作

## Rollback Strategy

- **Phase 5 未着手までは**: Rust crate は workspace 内に留まり配布物は TS
  SEA のまま。`git revert` で撤去可能。
- **Phase 5 後**: Release asset を旧 SEA version へロールバック(launcher
  は version 固定で pin 済み)。旧 binary asset 名を維持することが前提。
- **Pi state**: Rust 側は `<userConfigDir>/pi/` を読まないため TS 版へ
  戻しても credential/state は破壊されない。

## Self-Review

- [x] Every spec requirement maps to a phase plan(詳細は各 plan で検証)。
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation(各 Phase plan 内で
  crate 単位の並行 dispatch 可)
- Independent work streams: Phase 1/2 は Rig 非依存で並行先行可
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 上流: spec・ADR・design(approved、blocking 10 件解消済み)。
  non-blocking 指摘の担当 Phase への割当は各 plan で吸収済みか確認のこと。
- レビュー重点: Phase 0 go 条件の判定手順が再現可能か、Phase 間依存
  (特に Phase 3 が Phase 1/2 の出力を前提にする点)が欠落していないか。
