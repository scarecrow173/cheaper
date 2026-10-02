---
id: "IMPL-034z2rYkK5IZuxqTMYHtAs"
type: "impl"
status: "completed"
title: "create manual dispatch release workflow"
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
    - "docs/tasks/create-manual-dispatch-release-workflow-with-3-platform-sea-matrix.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# create manual dispatch release workflow

## Summary

`.github/workflows/release.yml` を新規作成した。trigger は
`workflow_dispatch` のみ（`tag` input 必須）で、自動 trigger は
一切持たない。3 platform matrix（win32-x64・linux-x64・linux-arm64）
で `build-executable.mjs` を実行し、version 一致検証と smoke を
経て GitHub Release に artifact を upload する。

## Context

docs/tasks/create-manual-dispatch-release-workflow-with-3-platform-sea-matrix.md
（PLAN-034z2bk7AEzZ66Mt0uXCyJ task 1）。TSPEC-034z2o64UhWCtNmYIhRT20
の contract（manual dispatch のみ・version 一致・命名規則・smoke
先行）を workflow に落とす。

## Implementation

### Changes

- `on.workflow_dispatch.inputs.tag`: 対象 release tag を必須 input
  として受け取る。push/schedule/pull_request trigger なし
- `permissions: contents: write` のみ
- matrix `include`: `windows-latest`→`win32-x64`→
  `cheaper-windows-x64.exe`、`ubuntu-latest`→`linux-x64`→
  `cheaper-linux-x64`、`ubuntu-24.04-arm`→`linux-arm64`→
  `cheaper-linux-arm64`。macOS は plan 決定により対象外
- steps: checkout（`ref: inputs.tag`）→ jdx/mise-action
  （`install_args: --locked`）→ `pnpm install --frozen-lockfile` →
  `pnpm build` → version 一致検証 → `build-executable.mjs
  --target` → smoke（`--version` が tag と一致 + `models` exit 0）
  → `gh release create`（既存なら skip）→ `gh release upload
  --clobber`

### Decisions During Implementation

- cross-target build は採らず各 platform の runner で host build
  （既存 `--target == host` 制約を維持。注入用 node binary の
  配布管理が不要）
- Release 作成は `gh release view || gh release create
  --generate-notes` の冪等パターンを各 matrix job に置き、
  専用 job を増やさない（同時 create の race は片側が失敗しても
  upload には影響しない）
- `pnpm build` は `build-executable.mjs` が `dist/sea-main.js` を
  要求するため必須。`mise run setup` 全体は使わず install+build のみ
- smoke の version 比較は `--version` 出力が bare version のため
  `v` prefix 両対応

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `npx yaml-lint .github/workflows/release.yml` → pass
- `on:` が `workflow_dispatch` のみ（`push`/`schedule`/
  `pull_request` なし）を目視・grep で確認
- matrix の target→artifact 命名が `build-executable.mjs` の
  `PLATFORM_LABEL` + `cheaper-<platform>-<arch>[.exe]` 規則と一致

## Risks

- 実 dispatch（GitHub Release 作成）は未実施。初回 release 操作時に
  E2E が初めて走る。matrix runner の availability（特に
  `ubuntu-24.04-arm`）はその時点で確定
- bash step は windows runner の Git Bash に依存（hosted runner の
  標準環境）

## Follow-ups

- Phase 3: APM executable payload + launcher
  （DESIGN-034z1EgnQwax4VamK2xmdT）
- Phase 4: `cheaper hook` subcommand 化
- macOS target 追加時は matrix 行追加 + unsigned binary の案内を
  別 plan で扱う
