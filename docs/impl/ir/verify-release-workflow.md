---
id: "IMPL-034z2rZiB6WUNTje88kRt1"
type: "impl"
status: "completed"
title: "verify release workflow"
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
    - "docs/tasks/verify-release-workflow-locally-and-record-smoke-evidence.md"
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

# verify release workflow

## Summary

release workflow の各 step を current host（Windows x64）で
local 再現して検証した。YAML 構文・trigger 構成・toolchain・
build・smoke がすべて pass。実 dispatch は未実施。

## Context

docs/tasks/verify-release-workflow-locally-and-record-smoke-evidence.md
（PLAN-034z2bk7AEzZ66Mt0uXCyJ task 3）。TSPEC-034z2o64UhWCtNmYIhRT20
の contract 検証（local 再現の範囲で）。

## Implementation

### Changes

コード変更なし（検証のみ）。対象は `.github/workflows/release.yml`
と `docs/guides/release.md`。

### Decisions During Implementation

- 実 dispatch / `gh release` 系 step は GitHub 上の副作用が
  あるため local 検証から除外し、build・smoke・version 比較
  ロジックのみ再現した

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `npx yaml-lint .github/workflows/release.yml` → pass
- `on:` が `workflow_dispatch` のみであることを確認
- `node --version` → `v26.10.0`（≥26.9.0 要件を満たす）
- `node scripts/build-executable.mjs --target win32-x64` →
  `dist/sea/cheaper-windows-x64.exe` 生成成功
- `./dist/sea/cheaper-windows-x64.exe --version` → `1.0.0`
- `./dist/sea/cheaper-windows-x64.exe models` → exit 0
- matrix runner→target→artifact 対応が命名規則と一致
- guide の手順記述が workflow の trigger・input と一致
- `mise run check` → pass（lint・version・doc-id・typecheck・
  test・bundle freshness 全緑）

## Risks

- matrix job の他 2 target（linux-x64・linux-arm64）は runner
  未実測。初回 dispatch で初確認になる
- `pnpm install --frozen-lockfile` step は local で依存が導入
  済みのため未再実行（CI では checkout 直後に実行される標準
  手順）

## Follow-ups

- 初回 release dispatch の結果を IR または issue #71 に記録
- linux runner で VFS mount point（`import.meta.dirname`）の
  挙動が Windows と同じか確認する余地あり
