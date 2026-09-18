---
id: "PLAN-034r05iHE1Fbkp8emdVve6"
type: "plan"
status: "completed"
test-spec-skip: "Verified by package unit test suite; no dedicated test-spec doc authored."
title: "grep batchMaxBytes invariant enforcement"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/grep-batch-max-bytes-invariant-enforcement.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/grep-batch-max-bytes-invariant-enforcement.md"
    - "../specs/grep-batch-max-bytes-line-payload-invariant.md"
  implemented-by:
    - "../tasks/enforce-grep-batch-max-bytes-invariant.md"
    - "../tasks/grep-batch-invariant-final-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/grep-batch-max-bytes-line-payload-invariant.md"
    - "../adr/grep-batch-max-bytes-invariant-reject-at-construction.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/grep-batch-max-bytes-invariant-enforcement.md"
  derived-by: []
  verifies:
    - "../specs/grep-batch-max-bytes-line-payload-invariant.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-034r05iHE1Fbkp8emdVve6: grep batchMaxBytes invariant enforcement

## Goal

`grep` tool の `batchMaxBytes >= 3 * maxLineBytes` invariant を
DESIGN-034r05iHy2gdnoS2PWvNo0 どおり option JSDoc に明記し、`createGrepTool` 構築時に
違反を reject する（ADR-034r05iHrZUE6WgelFx89a）。違反注入を使う既存 test を整合化し、
invariant 適合設定での既存挙動を維持する。

## Scope

単一 implementation stream。対象は `packages/runtime-pi/src/tools/grep.ts`
と `packages/runtime-pi/test/tools/grep.test.ts` のみ。2 task
（TASK-034r05iHQeQpDqzPOrhXsY〜0103）で完結する規模のため plan 分割はしない。

### In Scope

- `GrepToolOptions.batchMaxBytes` / `maxLineBytes` の JSDoc への
  invariant 明記
- `createGrepTool` 冒頭での実効値 `B < 3 * L` 検出と `Error` 送出
- `grep.test.ts` の違反注入 2 件の整合化と reject 検証 test の追加

### Out of Scope

- 既定値（`GREP_MAX_LINE_BYTES` / `GREP_BATCH_MAX_BYTES`）の変更
- 単独行 over-B batch 送出の防御コード削除
- 他の grep 上限 option の invariant 整備

## Upstream Documents

- `docs/specs/grep-batch-max-bytes-line-payload-invariant.md`
- `docs/adr/grep-batch-max-bytes-invariant-reject-at-construction.md`

## Design Inputs

- `docs/designs/grep-batch-max-bytes-invariant-enforcement.md`
  （approved — invariant の定義・検証位置・test 整合化方針が確定済み）
- DESIGN-034r05iHy2gdnoNaWH2bos: streaming grep batch protocol の導入元（batch 締め
  ロジック・上限定数の現行契約）

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `packages/runtime-pi/src/tools/grep.ts` | `createGrepTool` 冒頭の invariant 検証 + option JSDoc 明記 | TASK-034r05iHQeQpDqzPOrhXsY。batch 締めロジック・既定値・framer は不変 |
| `packages/runtime-pi/test/tools/grep.test.ts` | 違反注入 test 2 件の整合化 + reject 検証 test | TASK-034r05iHQeQpDqzPOrhXsY |

変更しない file（明示）: `limits.ts`、`line-framer.ts`、その他の tool・
test file。

## Implementation Sequence

- [ ] TASK-034r05iHQeQpDqzPOrhXsY: `grep.ts` に構築時 invariant 検証と JSDoc を追加し、
      `grep.test.ts` を整合化（batch 締め test の fixture 修正、
      単独行 over-B test を reject 検証へ置き換え）
- [ ] TASK-034r05iHQeQpDqAVvmTPpC: typecheck・lint・runtime-pi test 全件と document
      audit を実行し、SPEC-034r05iHK8bA7gqGaD6qpk acceptance criteria を全て検証

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| TASK-034r05iHQeQpDqzPOrhXsY（実装 + test 整合化） | — | TASK-034r05iHQeQpDqAVvmTPpC |
| TASK-034r05iHQeQpDqAVvmTPpC（最終検証） | TASK-034r05iHQeQpDqzPOrhXsY | — |

- critical path: TASK-034r05iHQeQpDqzPOrhXsY → TASK-034r05iHQeQpDqAVvmTPpC（直列のみ）。

## Task Breakdown

- TASK-034r05iHQeQpDqzPOrhXsY: Enforce grep batchMaxBytes invariant at construction
  （`docs/tasks/enforce-grep-batch-max-bytes-invariant.md`）
- TASK-034r05iHQeQpDqAVvmTPpC: grep batch invariant final verification
  （`docs/tasks/grep-batch-invariant-final-verification.md`）

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 違反注入に依存する既存 test が想定外に多く fixture 修正が膨らむ | low | low | 調査済み: 違反注入は `grep.test.ts` の 2 件のみ（`batchMaxBytes: 10` 単独、`batchMaxBytes: 4` + `maxLineBytes: 100`）。`maxLineBytes` 単独注入の test は `B` 既定 1MiB で invariant 適合 |
| 2 | 構築時検証の実効値と `execute` 内の解決値がずれる | low | med | 検証は `opts.maxLineBytes ?? GREP_MAX_LINE_BYTES` / `opts.batchMaxBytes ?? GREP_BATCH_MAX_BYTES` と同じ既定適用式を使い、`execute` 側の解決コードは変更しない |
| 3 | 単独行 over-B 経路の防御コードが coverage から外れ lint/test が警告する | low | low | `batchLines.length > 0` ガード自体は invariant 適合時も batch 締め判定に必要な条件であり dead code ではない。削除しない |

### External Dependencies

- なし（依存追加・新 env・外部プロセスなし）。

## Rollback Strategy

- **Trigger**: 既存 suite に regression が残る場合。
- **Procedure**: 対象 commit を `git revert`。変更は `grep.ts` と
  `grep.test.ts` のみで他 module に波及しない。
- **Verification**: revert 後に `pnpm --filter @cheaper/runtime-pi test`
  が pass すること。

## Verification

### Verification commands（全 task 共通の最終ゲート）

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm lint
mise exec -- pnpm --filter @cheaper/runtime-pi test
mise exec -- node .claude/skills/doc-status/scripts/audit_docs.js --type all
```

`grep.ts` は runtime-pi の sandboxed tool であり bundled CLI
（`.apm/hooks/cli`）には含まれないため bundle 再生成は不要
（diff で非影響を確認する）。

### SPEC-034r05iHK8bA7gqGaD6qpk acceptance criteria → task/test mapping

| SPEC-034r05iHK8bA7gqGaD6qpk acceptance criterion | 対応 task / test |
|--------------------------------|------------------|
| `batchMaxBytes` option 定義に `B >= 3 * L` contract と根拠の記載 | TASK-034r05iHQeQpDqzPOrhXsY — `grep.ts` JSDoc |
| ADR 決定（reject）に従い違反 option の挙動が定義され、違反設定で構築が error になる test | TASK-034r05iHQeQpDqzPOrhXsY — `grep.test.ts` の reject 検証 test |
| invariant 適合 fixture で batch 締めの既存 test が通り、違反注入 test が整合化されている | TASK-034r05iHQeQpDqzPOrhXsY — fixture 修正（`maxLineBytes: 4` + `batchMaxBytes: 12`） |
| `pnpm typecheck`・`pnpm lint`・runtime-pi test 全件 | TASK-034r05iHQeQpDqAVvmTPpC — 最終ゲート |

## Self-Review

- [x] SPEC-034r05iHK8bA7gqGaD6qpk の全 acceptance criterion が task/test に map されている
- [x] placeholder なし
- [x] file path・relation key・status が upstream doc と一致

## Implementation Handoff

- Recommended mode: inline execution（2 file のみの小規模変更）
- User approval required before dispatch: yes
