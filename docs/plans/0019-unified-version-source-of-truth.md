---
id: "PLAN-0019"
type: "plan"
status: "completed"
test-spec-skip: "Verified by check-version.mjs itself plus package test suites; no dedicated test-spec doc authored."
title: "Unified version source of truth"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../designs/0024-unified-version-source-of-truth.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0024-unified-version-source-of-truth.md"
    - "../specs/0019-unified-version-source-of-truth.md"
  implemented-by:
    - "../tasks/0104-add-version-check-and-set-scripts.md"
    - "../tasks/0105-unify-versions-wire-check-and-document-release.md"
  depends-on:
    - "../plans/0017-repo-owned-setup-check-entrypoints.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0019-unified-version-source-of-truth.md"
    - "../adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0024-unified-version-source-of-truth.md"
  derived-by: []
  verifies:
    - "../specs/0019-unified-version-source-of-truth.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PLAN-0019: Unified version source of truth

## Goal

`packages/cli/package.json` を canonical version source に定め、
`check-version.mjs`（一致検証）と `set-version.mjs`（一括 bump）を
用意し、全宣言を `1.0.0` に統一。version check を `mise run check`
に組み込み、release 手順を `docs/guides/release.md` に記す
（DESIGN-0024 / ADR-0022）。

## Scope

単一 implementation stream。script 2 本 + 宣言同期 + mise 組込 +
手順文書。2 task（TASK-0104〜0105）。

### In Scope

- `scripts/check-version.mjs` / `scripts/set-version.mjs`（新規）
- `packages/{cli,core,runtime-pi}/package.json` → `1.0.0`
- `mise.toml` check task への version check 組み込み
- `docs/guides/release.md` + AGENTS/README pointer
- bundle 再生成（`version.json` = `1.0.0`）

### Out of Scope

- root `apm.yml` version（dev project、対象外）
- tag / GitHub Release 自動化、`pnpm version` 統合
- check script の unit test（script 自体が検証器）

## Upstream Documents

- `docs/specs/0019-unified-version-source-of-truth.md`
- `docs/adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md`

## Design Inputs

- `docs/designs/0024-unified-version-source-of-truth.md`（approved —
  一致対象・script 仕様・mise 組込位置・release 手順が確定済み）
- DESIGN-0014: `version.json` fallback 経路（変更不要）
- DESIGN-0022 / PLAN-0017: `mise run check` canonical entrypoint

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `scripts/check-version.mjs` | version 一致検証（新規） | TASK-0104 |
| `scripts/set-version.mjs` | 一括 bump（新規） | TASK-0104 |
| `packages/cli/package.json` | canonical、`1.0.0` | TASK-0105 |
| `packages/core/package.json` | `1.0.0` 同期 | TASK-0105 |
| `packages/runtime-pi/package.json` | `1.0.0` 同期 | TASK-0105 |
| `mise.toml` | check task に version check 追加 | TASK-0105 |
| `docs/guides/release.md` | bump + release 手順（新規） | TASK-0105 |
| `AGENTS.md` | release 手順への pointer | TASK-0105 |
| `packages/apm/.apm/hooks/cli/version.json` | bundle 再生成で `1.0.0` | TASK-0105（生成物） |
| `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` | version 文字列を含む bundle | TASK-0105（生成物） |

変更しない file（明示）: `apm.yml`（dev project）、
`packages/apm/apm.yml`（既に `1.0.0`）、`packages/cli/src/version.ts`
（読み取り経路は不変）。

## Implementation Sequence

- [ ] TASK-0104: `check-version.mjs` と `set-version.mjs` を作成し、
      手動でずらした宣言に対する fail 動作を確認
- [ ] TASK-0105: 宣言を `1.0.0` に統一、`mise.toml` check に組込、
      `docs/guides/release.md` 作成、bundle 再生成、`mise run check`
      で全検証

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| TASK-0104（scripts） | — | TASK-0105 |
| TASK-0105（統一 + 組込 + 文書 + 検証） | TASK-0104 | — |

## Task Breakdown

- TASK-0104: Add version check and set scripts
  （`docs/tasks/0104-add-version-check-and-set-scripts.md`）
- TASK-0105: Unify versions, wire check, and document release
  （`docs/tasks/0105-unify-versions-wire-check-and-document-release.md`）

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | `apm.yml` version 行の正規表現抽出が format 差異（quote 有無等）で読み違える | low | low | 現行 format（`version: "1.0.0"`）を確認済み。regex は quote 有無両対応。読み取り失敗時は fail するため silent miss にならない |
| 2 | workspace package.json の version 変更が pnpm の依存解決に影響 | low | low | workspace dep は `workspace:*` で version 非依存。`pnpm install` 後の lockfile 差分を確認 |
| 3 | check を check task へ入れると既存 CI が version 不一致で即 fail する | med | low | 同一 commit で宣言を `1.0.0` に統一するため CI は pass する |
| 4 | bundle 内の version 文字列（minified）が `version.json` とずれる | low | low | `version.ts` は実行時に package.json/version.json を読むため bundle 内に version 定数は埋め込まれない。bundle 再生成で `version.json` だけ更新されることを diff で確認 |

### External Dependencies

- なし（Node 標準 API のみ、依存追加なし）。

## Rollback Strategy

- **Trigger**: version check が正しい宣言でも fail する、または宣言統一が
  既存動作を壊す場合。
- **Procedure**: 対象 commit を `git revert`。script 追加・宣言変更・
  mise 組込は互いに独立して revert 可能（宣言統一だけ戻すと check が
  fail するため、check 組込と宣言統一はセットで扱う）。
- **Verification**: revert 後に `mise run check` が pass すること。

## Verification

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm lint
mise exec -- pnpm test
node scripts/check-version.mjs
mise run check
node .claude/skills/doc-status/scripts/audit_docs.js --type all
```

加えて手動注入検証: 宣言を一時的にずらして `check-version.mjs` が
exit 1 になることを確認（確認後に戻す）。

### SPEC-0019 acceptance criteria → task/test mapping

| SPEC-0019 acceptance criterion | 対応 task / test |
|--------------------------------|------------------|
| canonical source が ADR・文書で一意に定められる | TASK-0105 — `docs/guides/release.md` + AGENTS pointer |
| `--version`・`apm.yml`・`version.json`・全 package.json が同値 | TASK-0105 — `1.0.0` 統一 + bundle 再生成 + `--version` 目視 |
| ずらすと check が fail | TASK-0104 — 手動注入検証 |
| bump の 1 コマンド操作が存在 | TASK-0104 — `set-version.mjs` |
| release 手順文書が存在 | TASK-0105 — `docs/guides/release.md` |
| typecheck・lint・test 全件 | TASK-0105 — `mise run check` |

## Self-Review

- [x] SPEC-0019 の全 acceptance criterion が task/test に map されている
- [x] placeholder なし
- [x] file path・relation key・status が upstream doc と一致

## Implementation Handoff

- Recommended mode: inline execution（script 2 本 + 宣言同期の小規模変更）
- User approval required before dispatch: yes
