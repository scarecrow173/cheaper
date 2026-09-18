---
id: "IMPL-0011"
type: "impl"
status: "completed"
title: "Separate development APM project from distributable package (PLAN-0016)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/apm/apm.yml"
      - type: "file"
        path: "packages/cli/test/e2e/apm-package-layout.test.ts"
    modified:
      - type: "file"
        path: "apm.yml"
      - type: "file"
        path: "scripts/bundle-cli.mjs"
      - type: "file"
        path: ".github/workflows/ci.yml"
      - type: "file"
        path: "packages/cli/test/e2e/apm-fixture.ts"
      - type: "file"
        path: "packages/cli/test/e2e/pre-read-hook.test.ts"
      - type: "file"
        path: "README.md"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "docs/discovery/0010-bulk-reader-huge-file-safe-io.md"
    deleted: []
    renamed: []
    moved:
      - type: "dir"
        path: ".apm/ -> packages/apm/.apm/"
    generated: []
  implements:
    - "docs/tasks/0095-move-apm-primitives-to-distributable-package.md"
    - "docs/tasks/0096-repoint-bundle-pipeline-and-ci-freshness.md"
    - "docs/tasks/0097-update-consumer-verification-and-no-self-deploy-tests.md"
    - "docs/tasks/0098-update-readme-agents-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0016-dev-apm-and-distributable-package-separation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0021-dev-apm-project-and-distributable-package-layout.md"
    - "docs/specs/0016-separate-dev-apm-project-from-distributable-package.md"
    - "docs/adr/0019-separate-dev-apm-project-via-nested-distributable-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0016-separate-dev-apm-project-from-distributable-package.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Separate development APM project from distributable package (PLAN-0016)

## Summary

repository root に同居していた development APM project と
distributable primitives を分離した。`.apm/` tree 全体を nested
package `packages/apm/` へ `git mv` し、root `apm.yml` を
development-only manifest（`name: cheaper-dev`、`includes` 削除）に
変更。bundle pipeline・CI freshness・consumer e2e を新 path に追従
させ、root install が cheaper own skill/hook を deploy しないことを
静的検証と実 `apm install` e2e の 2 層で保証するテストを追加した。

## Context

TASK-0095〜0098（PLAN-0016 / SPEC-0016 / ADR-0019 / DESIGN-0021）に
従う。Issue #9。旧 layout では root `apm install` が `.apm/` の
cheaper-* skill/hook を自身の開発環境へ deploy し、開発用依存と
配布物の責務が分離できていなかった。

## Implementation

### Changes

- `.apm/` → `packages/apm/.apm/`: hooks（hooks.json・
  pre-read-code.js・cli/ bundle 生成物）と skills（cheaper-* 4 件）を
  `git mv` で rename 移動。内容無変更
- `packages/apm/apm.yml`（新規）: `name: cheaper`・`version: "1.0.0"`
  の最小 distributable manifest。`targets`・`includes`・
  `dependencies` は持たない（consumer 側 apm.yml が target を選ぶ
  既存 contract を維持）。`package.json` は置かず pnpm workspace の
  対象外
- `apm.yml`: `name: cheaper-dev`・`includes: auto` 削除。
  targets・dependencies.apm・version は維持
- `scripts/bundle-cli.mjs`: `outDir` を
  `packages/apm/.apm/hooks/cli` へ。生成物構成・挙動は不変
- `.github/workflows/ci.yml`: bundle freshness の `git diff` 対象を
  `packages/apm/.apm/hooks/cli` へ
- `packages/cli/test/e2e/apm-fixture.ts`: `apmPackageDir` 定数を
  追加し、`makeConsumerLayout` が実 `packages/apm`（apm.yml +
  .apm/）を一時 `pkg/` へコピーする形に変更。fixture 自前の最小
  apm.yml 生成を廃止
- `packages/cli/test/e2e/pre-read-hook.test.ts`: `hookScript` /
  `bundledCli` を `apmPackageDir` から導出
- `packages/cli/test/e2e/apm-package-layout.test.ts`（新規）:
  root `.apm/` 不存在・package layout・dev-only root manifest・
  package name の静的検証 4 件 + `.apm/` なし dev-project fixture の
  `apm install` が cheaper skill/hook を deploy しない e2e 1 件
- `packages/apm/.apm/hooks/pre-read-code.js`: `packages/**` lint
  scope 入りに伴い biome format を適用（挙動変更なし）
- `README.md`・`AGENTS.md`: development/distribution 分離と
  `packages/apm` への install 参照（virtual package / local path）を
  記述
- `docs/discovery/0010`: source relation の stale path を新 path へ
  修正（audit 検出）

### Decisions During Implementation

- **stale `dist/` による bundle 差分**: bundle 再生成で旧ブランチの
  build 残滓（`openUrl` 注入コード）が混入したため、`pnpm -r build`
  で source から再 build してから再生成。結果、移動後の生成物と
  byte-identical となり freshness ゼロ差分を確認
- **dev-project fixture の最小化**: root `apm.yml` の remote dep を
  そのまま install すると非 hermetic になるため、同形の
  `.apm/` なし project を最小 manifest（name/targets のみ）で作り
  no-self-deploy を実測する方式にした（DESIGN-0021 通り）
- **IMPL 番号**: #8 の PR が open のまま 0010 を使っているため、
  merge 時の衝突回避で 0011 を採用

## Validation

- `mise exec -- pnpm -r build`: exit 0
- `mise exec -- pnpm lint`: exit 0（biome、141 files / warnings のみ）
- `mise exec -- pnpm typecheck`: exit 0
- `mise exec -- pnpm --filter cheaper test`: 14 files / 131 tests pass
  - `apm-package-layout.test.ts`: 静的 4 件 + no-self-deploy e2e
    （apm CLI 実測）を含む 5 件 pass
  - `apm-install*.test.ts`: 全 target（windsurf/copilot/codex/
    claude/cursor/agent-skills）で実 `packages/apm` package の
    consumer install・deploy・bundled CLI 動作を確認
  - `delegation-loop`・`pre-read-hook` e2e も新 path で全 pass
- `node scripts/bundle-cli.mjs` 後の `git diff --exit-code --
  packages/apm/.apm/hooks/cli`: clean（byte-identical）
- `node .claude/skills/doc-status/scripts/audit_docs.js --type all`:
  185 files / no findings

## Risks

- root を直接 install していた consumer は `packages/apm` sub-path
  参照への変更が必要（ADR-0019 で互換 shim を提供しない判断済み。
  README に記載）
- `apm.lock.yaml` は root に残り dev deps を lock し続ける。dep ref
  は不変のため lock の再生成は不要

## Follow-ups

- Issue #7: `mise run setup`/`mise run check` の repository-owned
  setup contract（本分離を前提に fresh worktree 用 restore を定義）
- Issue #11: `apm.yml` ×2・`version.json`・`--version` の version
  source 統一
