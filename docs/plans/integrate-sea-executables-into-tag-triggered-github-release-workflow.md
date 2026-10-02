---
id: "PLAN-034z2bk7AEzZ66Mt0uXCyJ"
type: "plan"
status: "draft"
title: "Integrate SEA executables into tag-triggered GitHub Release workflow"
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
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Integrate SEA executables into tag-triggered GitHub Release workflow Implementation Plan

## Goal

issue #71 Phase 2 として、tag push（`v*`）を trigger に GitHub Actions 上の
platform matrix で SEA executable を build し、各 platform で smoke test
した上で GitHub Release に artifact を添付する release 経路を作る。

## Scope

issue #71 の Phase 2（cross-platform matrix + GitHub Release 接続）のみを
1 stream として扱う。Phase 3（APM payload / launcher）、Phase 4（hook
subcommand 化）は本 plan に含めず別 plan で扱う。

cross-target build（1 runner 上で target node binary を使う方式）は採らない。
matrix job を target platform の runner で実行し、既存の
`--target == host` 制約をそのまま使う（KISS: 注入元 node binary の
配布・管理が不要で、smoke test を実 platform で実行できる）。

## Upstream Documents

- docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md
- docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md
- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md

## Design Inputs

- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md
  （platform 別 artifact 命名規則・release workflow 接続・
  artifact smoke test が In Scope に記載済み）

## File Map

- `.github/workflows/release.yml` - 新規。`v*` tag push +
  `workflow_dispatch` を trigger に 5 platform matrix で
  `build-executable.mjs` を実行し smoke → GitHub Release へ upload
- `scripts/build-executable.mjs` - 変更（必要なら最小限）。
  CI での出力 path 確認用に artifact path を機械可読に出す程度。
  host 制約・config は変えない
- `docs/guides/release.md` - 変更。release 手順 step 6 の「tag /
  GitHub Release は手動」を workflow 連携に更新
- `mise.toml` - 変更なし（`jdx/mise-action` が lock 済み toolchain を
  そのまま使う）

## Implementation Sequence

- [ ] release trigger と version 一致 policy を決める
      （tag `v<X.Y.Z>` が `packages/cli/package.json` version と
      一致することを workflow 冒頭で検証し不一致なら fail）
- [ ] `.github/workflows/release.yml` を作成:
      `on.push.tags: ["v*"]` + `workflow_dispatch`、
      permissions `contents: write`
- [ ] matrix を定義（runner → `--target` → artifact 名）:
      `windows-latest` → `win32-x64` → `cheaper-windows-x64.exe`、
      `macos-latest` → `darwin-arm64` → `cheaper-darwin-arm64`、
      `macos-13` → `darwin-x64` → `cheaper-darwin-x64`、
      `ubuntu-latest` → `linux-x64` → `cheaper-linux-x64`、
      `ubuntu-24.04-arm` → `linux-arm64` → `cheaper-linux-arm64`
- [ ] 各 job: checkout → `jdx/mise-action`（locked install。
      `mise run setup` 全体は不要、`pnpm install --frozen-lockfile`
      で esbuild を解決できればよい）→ `node scripts/build-executable.mjs
      --target <target>` → smoke（`--version` が tag version と一致、
      `models` が exit 0）→ `gh release upload` / `gh release create`
- [ ] tag version と `packages/cli/package.json` version の一致を
      検証する step を先頭 job または matrix 内に置く
- [ ] `docs/guides/release.md` を更新（tag push → workflow が
      Release + artifact を作成する手順に）
- [ ] workflow の構文を検証（YAML parse + 各 step のコマンドを
      current host で個別実行して確認。実 tag push による E2E は
      ユーザーの release 操作時に初めて走る前提とする）

## Dependencies and Risks

- **Node version**: `build-executable.mjs` が呼ぶ `node --build-sea`
  には lock 済み Node ≥26.9.0 が必須。matrix job でも mise の locked
  install で 26.10.0 が入ることを確認する
- **macos-13 runner**: Intel mac（darwin-x64）用。GitHub hosted runner
  の提供状況は変わり得るため、無ければ `macos-15-intel` 等への置換を
  open question に残す
- **ubuntu-24.04-arm**: linux-arm64 用 ARM runner。public repo では
  利用可。private 化した場合は要見直し
- **unsigned binary**: macOS では未署名 binary に quarantine attribute
  が付く（Gatekeeper）。design で本格 notarization は out of scope —
  release notes に `xattr -d com.apple.quarantine` 等の案内を含めるかは
  実装時に guide 更新の一部として扱う
- **artifact 名衝突**: `build-executable.mjs` は `--out-dir` 既定で
  `dist/sea/` を作り直す。job ごと独立 runner なので衝突しない
- **breaking-change risk: なし**。既存 `ci.yml`・bundle 経路は不変
- **data-loss risk: なし**。Release への artifact 添付のみ

## Task Breakdown

Implementation Sequence を task-doc 粒度に分解する際は
「release workflow + matrix」「version 一致検証 + smoke step」
「release.md 更新」「local 検証 + 記録」の 4 タスクを目安とする。

## Verification

- [ ] workflow YAML が parse される（`node -e` で yaml parse または
      `gh workflow` の構文確認相当）
- [ ] current host で `node scripts/build-executable.mjs --target
      win32-x64` の各 step がそのまま CI で動く構成になっている
      （mise locked install → pnpm install → build → smoke の手順を
      ローカルで再現して確認）
- [ ] `mise run check` が pass
- [ ] `docs/guides/release.md` の手順と workflow trigger が一致
- [ ] 実 tag push は本 plan の検証に含めない（ユーザーの release
      操作時に初実施。`workflow_dispatch` で事前に workflow のみ
      試行する選択肢を guide に記す）

## Open Questions / Gaps

- matrix 定義を workflow YAML 内に持つか、script 側の共有 data に
  分離するか。design は「target matrix を data として分離」とするが、
  Phase 2 では workflow YAML の matrix が事実上その data となる
  （script の命名規則と一致しているかは task で確認）
- Intel mac runner の availability（`macos-13` deprecation 状況）
- `gh release create` の draft/非 draft、prerelease 扱いの方針
  （`v*` は安定版として publish、`-rc` 等の suffix tag は
  `gh release create --prerelease` に分岐するかは実装時に最小構成で
  決める）
- Release notes の自動生成（`--generate-notes`）を使うか

## Rollback Strategy

- workflow file と guide 更新のみなので `git revert` で戻せる
- Trigger: workflow が tag push で誤動作 / artifact が壊れる
- Procedure: `git revert <commit>`。誤って作成された Release は
  `gh release delete` で消す（運用手順として guide に記載しない —
  GitHub 標準操作）
- Verification: `mise run check` が pass し `ci.yml` が無影響

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（workflow 定義は単一ファイルで
  検証は local 再現中心。delegated dispatch の利点が薄い）
- Independent work streams: なし（直列）
- User approval required before dispatch: yes

## Review Handoff

- 前提: Phase 1 PoC で `build-executable.mjs` の host 制約・命名規則・
  asset 構成が確立済み（commit `a9f5593`）
- 前提: GitHub hosted runner（windows-latest / macos-latest /
  macos-13 / ubuntu-latest / ubuntu-24.04-arm）が利用可能
- 着目点: tag↔version 一致検証の堅さ、matrix runner の対応表、
  smoke test の範囲（`--version` + `models` で十分か）
