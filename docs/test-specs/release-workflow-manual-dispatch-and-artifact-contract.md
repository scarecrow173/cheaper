---
id: "TSPEC-034z2o64UhWCtNmYIhRT20"
type: "test-spec"
status: "superseded"
title: "Release workflow manual-dispatch and artifact contract"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: ["TSPEC-034DmUtUjWCThCnJulok0c"]
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
  derived-by: []
  verifies:
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Release workflow manual-dispatch and artifact contract

## Purpose

issue #71 Phase 2 の release workflow が「常にユーザーの明示操作で
のみ実行される」ことと、「platform matrix で生成された artifact が
命名規則と version 一致を満たして GitHub Release に添付される」ことを
保証する意図を記録する。workflow が自動 trigger を持ったり version
不一致の artifact を release しないことを守る。release 経路が
廃止・再設計された場合にのみ retire する。

## Feature

手動 dispatch による SEA executable の GitHub Release 配布

## Rules

- workflow の `on:` は `workflow_dispatch` のみ。`push`・`schedule`・
  `pull_request` 等の自動 trigger を持たない
- dispatch 時に指定した `tag`（`v<X.Y.Z>`）の
  `packages/cli/package.json` version と一致しなければ job は fail
  し、artifact は upload されない
- matrix の各 job は runner の host target を `build-executable.mjs
  --target` で build し、`cheaper-<platform>-<arch>[.exe]` の命名に
  従う。対象は `win32-x64` / `linux-x64` / `linux-arm64`（macOS は
  需要が出るまで対象外）
- 各 job は artifact upload 前に smoke test（`--version` が tag
  version を出力、`models` が exit 0）を通す
- artifact は指定 tag の GitHub Release にのみ添付される

## Examples

- `tag` input に `v1.0.0` を指定して dispatch すると、checkout は
  `refs/tags/v1.0.0` を向き、package.json version が `1.0.0` であれば
  3 platform の executable が Release `v1.0.0` に添付される
- tag の package.json version が `1.0.0` でない（例: tag が古い
  commit を指す）場合、version 一致検証で fail し Release は
  作成・更新されない
- workflow file の `on:` に `push` trigger が追加された場合は
  本 spec 違反（review / lint で検出）

## Guarantees

- release artifact は手動操作なしに生成・公開されない
- tag と package version の不一致状態で artifact が release に
  載らない（version 宣言 drift の公開を防ぐ）
- smoke test を通らない executable は upload されない

## Non-goals

- 実 dispatch による E2E（GitHub Release 作成そのもの）は本 spec の
  検証対象外。workflow 構文・step の local 再現・contract の static
  検証までを守る
- macOS target・codesigning/notarization は対象外
- Release notes 内容の品質は検証しない（`--generate-notes` 採否は
  plan の open question）

## Risk

- この保証が落ちると、意図しないタイミングで release が作成されたり
  version のずれた artifact が配布され、consumer が壊れた executable
  を install するリスクがある
