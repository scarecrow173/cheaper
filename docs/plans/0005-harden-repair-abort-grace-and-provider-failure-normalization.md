---
id: "PLAN-0005"
type: "plan"
status: "completed"
title: "Harden repair abort grace and provider failure normalization"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/specs/0005-repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/0010-repair-abort-and-failure-normalization-hardening.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Harden repair abort grace and provider failure normalization Implementation Plan

## Goal

repair session に main session と同等の abort grace・provider failure 分類を適用し、`meta.repairs`・timeout 契約・test 隔離・abort grace timer・文書の不整合を解消する。

## Scope

単一の実装ストリーム。`run.ts`/`repair.ts` の abort・分類系は直列依存（同一ファイル）だが、config 隔離と文書化は独立。ランタイムの公開 API 変更は `RepairOptions.onAttempt` と `resolveConfigDirs` の homedir 解決のみ。

## Upstream Documents

- `docs/specs/0005-repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md`
- `docs/adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md`
- `docs/discovery/0003-post-smoke-code-review-findings-on-abort-repair-and-timeout-semantics.md`

## Design Inputs

- `docs/designs/0010-repair-abort-and-failure-normalization-hardening.md`（approved）
- `docs/designs/0004-pi-execution-engine.md`（timeout 起点の改訂対象）

## File Map

- `packages/runtime-pi/src/run.ts` — `promptWithAbortGrace` 適用・timeout タイマー開始位置の移動・`requestAbort` 縮退・`onAttempt` 配線
- `packages/runtime-pi/src/repair.ts` — helper 適用・`PROVIDER_FAILURE` 正規化・`onAttempt` 追加
- `packages/runtime-pi/src/output.ts` または新規モジュール — `lastAssistantStop` の共有化
- `packages/core/src/config/dirs.ts` — homedir 解決を `env.USERPROFILE`/`env.HOME` 優先に変更
- `packages/cli/test/main.test.ts` — `makeIo` を tmp 隔離（`HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR`・`cwd`）
- `packages/runtime-pi/test/run.test.ts` / `repair.test.ts` — abort grace・分類・repairs の回帰テスト追加
- `docs/designs/0004-pi-execution-engine.md` — timeout 起点の改訂
- `README.md` / `docs/guides/providers.md` — 実態反映と provider 手順

## Implementation Sequence

- [ ] `promptWithAbortGrace` helper を新規モジュールに実装し `run.ts` の main prompt を置き換え（grace timer 解放を含む）
- [ ] `repair.ts` に helper を適用し `unlink` を吸収、`PROVIDER_FAILURE` 正規化を追加
- [ ] `repairOutput` に `onAttempt` を追加し `meta.repairs` を失敗時も計上
- [ ] timeout タイマー開始を `runWorker` 先頭へ移し、DESIGN-0004 を agent execution timeout に改訂
- [ ] `resolveConfigDirs` の homedir 解決を env 優先に変更し、`main.test.ts` を tmp 隔離
- [ ] README 更新と `docs/guides/providers.md` 作成
- [ ] `pnpm -r test` / `pnpm -r build` で回帰なしを確認

## Dependencies and Risks

- `run.ts`/`repair.ts` は同一変更者が直列で扱う（helper 抽出 → repair 適用 → repairs → timeout の順）
- abort タイミングのリファクタは既存 abort 系テストで回帰確認（fake stream が signal を無視するケースを新規追加）
- `env.USERPROFILE`/`env.HOME` 優先は実環境で `os.homedir()` と一致するため実害なし（git-bash 等の `HOME` あり環境は `USERPROFILE` 優先で不変）

## Task Breakdown

- TASK-A: `promptWithAbortGrace` helper 抽出 + `run.ts` 適用 + grace timer 解放
- TASK-B: `repair.ts` への helper 適用 + provider failure 正規化 + `lastAssistantStop` 共有化
- TASK-C: `meta.repairs` 失敗時計上（`onAttempt`）
- TASK-D: timeout タイマー開始位置の移動 + DESIGN-0004 改訂
- TASK-E: `resolveConfigDirs` env homedir + unit test 隔離
- TASK-F: README 更新 + `docs/guides/providers.md`

## Verification

- [ ] repair 中 abort で grace 打ち切りによりハングしないテストが追加され green
- [ ] repair の prompt rejection / `stopReason:"error"` が `PROVIDER_FAILURE` になるテストが green
- [ ] repair 失敗時 `meta.repairs` が消費回数になるテストが green
- [ ] user config を置いた環境でも `pnpm -r test` が green
- [ ] `pnpm -r build` が green

## Open Questions / Gaps

- なし（timeout 起点は ADR-0010 で確定済み）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（同一ファイル群の直列修正であり環境依存も小さい）
- Independent work streams: TASK-E（config/test 隔離）、TASK-F（文書）はコード群と独立
- User approval required before dispatch: yes

## Review Handoff

- 前提: repair が通常経路（EXP-0002）。helper 抽出は不変条件の一箇所定義が目的
- 注意: `promptWithAbortGrace` への移行で main の abort 経路が変わるため、既存 abort テストの回帰を必ず確認
