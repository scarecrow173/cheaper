---
id: "PLAN-034EDNoLUwAoBOtSAMRMNY"
type: "plan"
status: "approved"
title: "Execute post-feature structural refactoring"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "../specs/post-feature-structural-refactoring-scope.md"
  implemented-by:
    - "../tasks/p0-core-shared-helpers.md"
    - "../tasks/p0-tools-dead-assets.md"
    - "../tasks/p0-cli-shared-helpers.md"
    - "../tasks/p0-cli-tests-common.md"
    - "../tasks/p0-runtime-provider-adapter-helpers.md"
    - "../tasks/p0-workspace-deps-hygiene.md"
    - "../tasks/p1-tools-characterization-tests.md"
    - "../tasks/p1-runtime-catalog-split.md"
    - "../tasks/p1-runtime-run-split.md"
    - "../tasks/p1-cli-compose-separation.md"
    - "../tasks/p1-tools-module-splits.md"
    - "../tasks/p1-workflow-runstate.md"
    - "../tasks/p1-core-schema-loader-split.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/post-feature-structural-refactoring-inventory.md"
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
    - "../test-specs/post-feature-structural-refactoring-parity.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../designs/post-feature-structural-refactoring.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# Execute post-feature structural refactoring Implementation Plan

## Goal

spec（Phase 0: hygiene 18 件 + Phase 1: structural 17 件・計 35 候補）を
behavior-preserving で実施する。

## Scope

単一の implementation stream。crate 単位の task 分割で進め、crate をまたぐ
依存（core helper を tools/cli から参照する等）は task graph の dependency
edge で順序づける。Phase 2 候補・crate トポロジ変更は対象外。

## Upstream Documents

- `docs/specs/post-feature-structural-refactoring-scope.md`（approved）
- `docs/adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md`（accepted）
- `docs/discovery/post-feature-structural-refactoring-inventory.md`（resolved）

## Design Inputs

- `docs/designs/post-feature-structural-refactoring.md`（approved）
- `docs/test-specs/post-feature-structural-refactoring-parity.md`（approved）

## File Map

- `crates/cheaper-cli/src/{errors,signal,compose}.rs` - 新規 module（error
  helper・OS signal 集約・composition root）
- `crates/cheaper-cli/src/{run,workflow_run,models,auth,usage_report,count,
  fake_stream}.rs` - composition root 参照・helper 利用への統一
- `crates/cheaper-cli/tests/common/mod.rs` - test helper 集約（新規）
- `crates/cheaper-core/src/**` - NAME_PATTERN/schema_error_entries/
  absent-file/MAX_SAFE_INTEGER 正源化、`worker/schema/{mod,strict}.rs` 分割、
  `read_policy_file`/`read_models_config` 共通骨格
- `crates/cheaper-runtime/src/catalog/{types,compose,store}.rs` - catalog 分割
- `crates/cheaper-runtime/src/run/{mod,session,boundary}.rs` - run 分割
- `crates/cheaper-runtime/src/auth/oauth/openai_codex{/,tests.rs}` - in-file
  test の sibling 分離
- `crates/cheaper-runtime/src/dev/`・`src/lib.rs` - fixture/fake_provider
  alias 解消
- `crates/cheaper-provider/src/{plugin,models}.rs` - CliError→CheaperError 集約
- `crates/cheaper-rig-adapter/src/error.rs`・`lib.rs` - 可視性整理
- `crates/cheaper-tools/src/js16.rs`・`src/bin/glob_probe.rs` - 削除
- `crates/cheaper-tools/src/tools/{process/{prepare,spawn,monitor,cleanup}.rs,
  grep.rs,read.rs}` - 構造分割（characterization test 先行）
- `crates/cheaper-workflow/src/engine.rs`・`compiler.rs`・`types.rs` -
  RunState 化・schema compile 重複解消
- `Cargo.toml`（workspace deps）・`.gitignore`・`benchmarks/` - 横断整理

## Implementation Sequence

- [ ] Phase 0 task を crate 単位で実施（core → tools → runtime → provider →
      adapter → cli → 横断の順で依存を上向きに解消）
- [ ] tools characterization test を追加（tools 構造分割の必須先行条件）
- [ ] Phase 1 task を実施（runtime catalog/run 分割 → tools 分割 → cli compose
      分離 → workflow RunState → core schema/loader 分割）
- [ ] 全 task 完了後に `mise run check` で回帰確認

## Dependencies and Risks

- tools の構造分割は characterization test task に依存（spec AC の明示条件）。
- core の helper 集約は tools/cli の caller 更新に先行する（同方向変更を
  一 commit に含めて跨 crate の中間不整合を避ける）。
- `uuid` workspace dep 化は `=1.27.0` vs `1` の pin 差異を保持方針どおり
  扱う（committed lockfile check が resolved version 変化を検出する）。
- workflow `JobValidation.input` は `JobKind::Tool` で使用中 — 除去せず
  worker job の冗長 compile のみ解消する（design 参照）。

## Task Breakdown

- [T01](../tasks/p0-core-shared-helpers.md): core helper 集約（NAME_PATTERN・schema_error_entries・
  absent-file・MAX_SAFE_INTEGER）
- [T02](../tasks/p0-tools-dead-assets.md): tools js16 shim 削除 + glob_probe 削除 + fixture 明文化
- [T03](../tasks/p0-cli-shared-helpers.md): cli error/signal helper 集約 + `Runtime::create` 単一入口 +
  workflow_run timeout details 修正 + count.rs BoxError
- [T04](../tasks/p0-cli-tests-common.md): cli tests common 集約（VecSink/PipeStdin/ambient_env/Fixture）
- [T05](../tasks/p0-runtime-provider-adapter-helpers.md): runtime 共通 ctor・dev alias 解消・provider CliError→CheaperError・
  adapter pub 整理
- [T06](../tasks/p0-workspace-deps-hygiene.md): 横断 workspace deps・`.gitignore`・benchmarks 明文化
- [T07](../tasks/p1-tools-characterization-tests.md): tools characterization test 追加（先行必須）
- [T08](../tasks/p1-runtime-catalog-split.md): runtime catalog/mod.rs 分割
- [T09](../tasks/p1-runtime-run-split.md): runtime run/mod.rs 分割 + RunCtx stage 化 + openai_codex test 分離
  + ProviderEntry refresh + compose_provider auth + atomic write
- [T10](../tasks/p1-cli-compose-separation.md): cli compose.rs 分離 + workspace preamble + fake_stream 集約
- [T11](../tasks/p1-tools-module-splits.md): tools process/grep/read 分割
- [T12](../tasks/p1-workflow-runstate.md): workflow RunState 化 + stop! fn 化 + schema compile 共有
- [T13](../tasks/p1-core-schema-loader-split.md): core worker/schema 分割 + read_*_file 共通骨格

## Verification

- [ ] 各 task 完了時に `cargo test -p <対象crate>`（`mise exec --` 経由、
      dev-seam feature 付き）で対象範囲の回帰を確認
- [ ] 全 task 完了後 `mise run check` が pass
- [ ] `cheaper-cli/tests/envelope.rs`・regression 55 case・tools
      differential fixture が緑（timeout details の意図的差分のみ許容）

## Open Questions / Gaps

- `uuid` の version requirement 統一方針は実装時に確定（spec 記載の選択肢
  どちらかを採用して commit message / task doc に記録）。
- dev-seam plumbing 集約の境界形状（fake_stream.rs に寄せる範囲）は
  [T10](../tasks/p1-cli-compose-separation.md) 実施時に確定。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated（task 単位で subagent 化可能な crate 境界）
- Independent work streams: [T01](../tasks/p0-core-shared-helpers.md)〜06（Phase 0）と [T07](../tasks/p1-tools-characterization-tests.md) は互いに
  独立並列可能。[T08](../tasks/p1-runtime-catalog-split.md)〜13 は [T07](../tasks/p1-tools-characterization-tests.md)（tools 系）および対象 crate の
  Phase 0 完了に依存。
- User approval required before dispatch: yes（plan-doc 承認）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- workflow `JobValidation.input`・cli timeout details 例外・uuid pin 差異は
  review で再確認された scope 補正済み。実装時に挙動差分を出さないこと。
