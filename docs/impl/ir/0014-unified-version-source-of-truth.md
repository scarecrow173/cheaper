---
id: "IMPL-0014"
type: "impl"
status: "completed"
title: "Unified version source of truth"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "scripts/check-version.mjs"
      - type: "file"
        path: "scripts/set-version.mjs"
      - type: "file"
        path: "docs/guides/release.md"
    modified:
      - type: "file"
        path: "packages/cli/package.json"
      - type: "file"
        path: "packages/core/package.json"
      - type: "file"
        path: "packages/runtime-pi/package.json"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/version.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0104-add-version-check-and-set-scripts.md"
    - "docs/tasks/0105-unify-versions-wire-check-and-document-release.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/0019-unified-version-source-of-truth.md"
    - "docs/adr/0022-cli-package-json-as-canonical-version-with-check-enforcement.md"
    - "docs/designs/0024-unified-version-source-of-truth.md"
    - "docs/plans/0019-unified-version-source-of-truth.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0019-unified-version-source-of-truth.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Unified version source of truth

## Summary

製品 version の canonical source を `packages/cli/package.json` に定め、
全宣言（workspace package.json × 3、配布 manifest、生成物
`version.json`、`--version` 出力）を `1.0.0` に統一した（issue #11）。
不一致は `scripts/check-version.mjs` が検出し、`mise run check` に
組み込んだため CI で自動的に fail する。bump は
`scripts/set-version.mjs <X.Y.Z>` の 1 コマンドに集約し、release 手順を
`docs/guides/release.md` に文書化した。

## Context

TASK-0104 / TASK-0105（PLAN-0019 / SPEC-0019 / ADR-0022 / DESIGN-0024）
に従う。branch: `feature/unify-version-source-of-truth`
（`feature/repo-setup-contract` 上に stack — #9 の `packages/apm` layout
と #7 の `mise run check` entrypoint を前提とする）。

## Implementation

### Changes

- `scripts/check-version.mjs`（新規）: canonical（cli package.json）を
  読み、core/runtime-pi package.json・`packages/apm/apm.yml`（version:
  行、quote 有無両対応の regex 抽出）・生成物 `version.json` を照合。
  不一致は file 名と値を列挙して exit 1。root `apm.yml`（cheaper-dev
  dev project）は対象外。
- `scripts/set-version.mjs`（新規）: semver 引数を検証し、宣言 4 file
  を一括更新（package.json は JSON parse → 2-space 書戻し、apm.yml は
  version: 行のみ置換）。続けて `check-version.mjs` を呼び、生成物の
  不一致を検出したら bundle 再生成を促して exit 1。
- `packages/{cli,core,runtime-pi}/package.json`: `0.0.0` → `1.0.0`
  （配布 manifest が既に名乗っていた値に統一）。
- `mise.toml`: `check` task の `pnpm lint` 後に
  `node scripts/check-version.mjs` を追加。CI は同一 entrypoint を
  通るため別途 CI 設定は不要。
- `packages/apm/.apm/hooks/cli/version.json`: bundle 再生成で `1.0.0`。
  `cheaper.mjs` 自体は非変化（version 定数は実行時解決で非埋込）。
- `docs/guides/release.md`（新規）: canonical source、宣言一覧と
  対象外（root apm.yml）の区別、bump 手順、release 手順。
- `AGENTS.md`: 「Version と release」節を追加し canonical source と
  guide への pointer を記載。

### Decisions During Implementation

- **canonical = cli package.json**: `--version` の既存読み取り経路
  （package.json → version.json fallback）と一致し、表現ドリフトなし
  （ADR-0022 option A）。
- **宣言 + 検証（生成ではなく）**: 手書き manifest の生成管理を避け、
  宣言のまま残して check で一致を強制する構成を選択（同 ADR）。
- **idempotent bump の対応**: 初版 `set-version.mjs` は
  `replace(...) === text` を「version 行なし」判定に使い、同一値への
  bump で誤検出した。行存在チェック（`test()`）と置換を分離して修正。

## Verification

- `check-version.mjs`: 発散状態で正しく exit 1 + file 列挙、統一後に
  `version check OK: 1.0.0`。
- `set-version.mjs`: 4 file 一括更新 + 生成物不一致の検出と bundle
  再生成指示。不正引数・引数なしは usage + exit 1。
- `mise run check` 全 pass: lint（warning のみ）→ version check →
  typecheck → 656 tests（core 270 / runtime-pi 255 / cli 131）→
  bundle 再生成 → freshness clean。
- `cheaper --version` = `1.0.0`。
- `audit_docs.js --type all`: 198 files、no findings。

## Related Experiments

### Adopted

- check script を `mise run check`（SPEC-0017 canonical entrypoint）へ
  組み込み、CI 側に別途 check を足さない構成。SPEC-0017 の
  local/CI/agent parity の利点がそのまま効く。

### Rejected

- apm.yml を canonical にする案、および VERSION file を新設して全宣言を
  生成する案（ADR-0022 Considered Options 参照）。
