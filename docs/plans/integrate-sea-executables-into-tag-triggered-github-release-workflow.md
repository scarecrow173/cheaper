---
id: "PLAN-034z2bk7AEzZ66Mt0uXCyJ"
type: "plan"
status: "completed"
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
  verified-by:
    - "docs/test-specs/release-workflow-manual-dispatch-and-artifact-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Integrate SEA executables into tag-triggered GitHub Release workflow Implementation Plan

## Goal

issue #71 Phase 2 として、**手動実行（`workflow_dispatch`）のみ**を
trigger に GitHub Actions 上の platform matrix で SEA executable を
build し、各 platform で smoke test した上で GitHub Release に
artifact を添付する release 経路を作る。tag push 等の自動 trigger は
設けない。

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

- `.github/workflows/release.yml` - 新規。`workflow_dispatch` のみ
  （tag input で対象 release tag を指定）を trigger に 3 platform
  matrix で `build-executable.mjs` を実行し smoke → GitHub Release
  へ upload。macOS target は需要が出るまで対象外
- `scripts/build-executable.mjs` - 変更（必要なら最小限）。
  CI での出力 path 確認用に artifact path を機械可読に出す程度。
  host 制約・config は変えない
- `docs/guides/release.md` - 変更。release 手順 step 6 の「tag /
  GitHub Release は手動」を workflow 連携に更新
- `mise.toml` - 変更なし（`jdx/mise-action` が lock 済み toolchain を
  そのまま使う）

## Implementation Sequence

- [ ] release trigger と version 一致 policy を決める
      （`workflow_dispatch` の `tag` input で `v<X.Y.Z>` を受け取り、
      その tag の `packages/cli/package.json` version と一致する
      ことを workflow 冒頭で検証。不一致なら fail）
- [ ] `.github/workflows/release.yml` を作成:
      `on.workflow_dispatch` のみ（inputs: `tag`）。
      push/schedule 等の自動 trigger は一切設けない。
      permissions `contents: write`
- [ ] matrix を定義（runner → `--target` → artifact 名）:
      `windows-latest` → `win32-x64` → `cheaper-windows-x64.exe`、
      `ubuntu-latest` → `linux-x64` → `cheaper-linux-x64`、
      `ubuntu-24.04-arm` → `linux-arm64` → `cheaper-linux-arm64`。
      macOS（darwin-arm64/x64）は需要が出るまで対象外
      （追加時は matrix 行の追加のみで拡張できる構造にする）
- [ ] 各 job: checkout（`refs/tags/<tag>`）→ `jdx/mise-action`
      （locked install。`mise run setup` 全体は不要、
      `pnpm install --frozen-lockfile` で esbuild を解決できればよい）
      → `node scripts/build-executable.mjs --target <target>` →
      smoke（`--version` が tag version と一致、`models` が exit 0）
      → `gh release upload` / `gh release create`
- [ ] tag version と `packages/cli/package.json` version の一致を
      検証する step を先頭 job または matrix 内に置く
- [ ] `docs/guides/release.md` を更新（tag を打った後に
      Actions から手動 dispatch → workflow が Release + artifact を
      作成する手順に）
- [ ] workflow の構文を検証（YAML parse + 各 step のコマンドを
      current host で個別実行して確認。実 dispatch による E2E は
      ユーザーの release 操作時に初めて走る前提とする）

## Dependencies and Risks

- **Node version**: `build-executable.mjs` が呼ぶ `node --build-sea`
  には lock 済み Node ≥26.9.0 が必須。matrix job でも mise の locked
  install で 26.10.0 が入ることを確認する
- **macOS target は今回対象外**（需要が出るまで。未署名 binary の
  Gatekeeper/quarantine 問題もその時点で再検討する）。matrix は行追加で
  拡張できる構造にする
- **ubuntu-24.04-arm**: linux-arm64 用 ARM runner。public repo では
  利用可。private 化した場合は要見直し
- **自動 trigger なし**: `workflow_dispatch` のみとし、release は
  常にユーザーの明示操作で開始される。誤って `push` trigger を
  足さないよう workflow 冒頭の `on:` を最小構成にする
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
- [ ] 実 dispatch は本 plan の検証に含めない（ユーザーの release
      操作時に初実施）

## Open Questions / Gaps

- matrix 定義を workflow YAML 内に持つか、script 側の共有 data に
  分離するか。design は「target matrix を data として分離」とするが、
  Phase 2 では workflow YAML の matrix が事実上その data となる
  （script の命名規則と一致しているかは task で確認）
- `gh release create` の draft/非 draft、prerelease 扱いの方針
  （tag が `-rc` 等の suffix を持つ場合 `--prerelease` に分岐するかは
  実装時に最小構成で決める）
- Release notes の自動生成（`--generate-notes`）を使うか
- macOS target を追加する際の runner 選択（`macos-latest` / Intel は
  別 runner）と未署名 binary の案内方法は、その時点の plan で扱う

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
- 前提: GitHub hosted runner（windows-latest / ubuntu-latest /
  ubuntu-24.04-arm）が利用可能
- 要件: 手動実行（workflow_dispatch）以外の trigger を持たないこと
- 着目点: tag↔version 一致検証の堅さ、matrix runner の対応表、
  smoke test の範囲（`--version` + `models` で十分か）
