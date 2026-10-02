---
id: "TASK-034z2paWfkTKout3vzftc8"
type: "task"
status: "todo"
title: "Create manual-dispatch release workflow with 3-platform SEA matrix"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  implemented-by: []
  depends-on:
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Create manual-dispatch release workflow with 3-platform SEA matrix

## Work

`.github/workflows/release.yml` を新規作成する。`on:` は
`workflow_dispatch` のみ（inputs に `tag` を必須で受け取る）とし、
push/schedule 等の自動 trigger は設けない。

構成:

- 先頭で tag 一致検証: input `tag`（`vX.Y.Z`）の ref を checkout し、
  `packages/cli/package.json` の version と一致しなければ fail
- matrix job: `windows-latest`→`win32-x64`、`ubuntu-latest`→
  `linux-x64`、`ubuntu-24.04-arm`→`linux-arm64`（macOS 対象外、
  matrix 行追加で拡張可能な形）
- 各 job: checkout（`refs/tags/<tag>`）→ `jdx/mise-action`（
  `install_args: --locked`）→ `pnpm install --frozen-lockfile` →
  `node scripts/build-executable.mjs --target <target>` → smoke
  （`--version` が tag version と一致、`models` が exit 0）→
  `gh release upload`（Release が無ければ `gh release create`）
- permissions は `contents: write` に限定

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md
- docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md

## Dependencies

- Phase 1 で確立した `scripts/build-executable.mjs`（host 制約・
  命名規則 `cheaper-<platform>-<arch>[.exe]`）

## Affected Files

- `.github/workflows/release.yml`（新規）
- `scripts/build-executable.mjs`（必要なら artifact path 出力の
  最小限の変更のみ）

## Done When

- [ ] `release.yml` が `workflow_dispatch` のみを trigger に持つ
- [ ] 3 platform matrix・version 一致検証・smoke・upload の step が
      揃っている
- [ ] workflow YAML が parse 可能

## Verification

- [ ] YAML parse（`node -e` + yaml パーサ、または `pnpm exec` の
      YAML ツール）で構文エラーなし
- [ ] `on:` セクションに `workflow_dispatch` 以外が無いことを確認
- [ ] matrix の runner→target 対応が plan の表と一致
