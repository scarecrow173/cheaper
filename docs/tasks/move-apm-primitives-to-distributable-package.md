---
id: "TASK-034r05iHQeQpDqrbmtswfZ"
type: "task"
status: "done"
title: "Move .apm primitives to nested distributable package and make root apm.yml dev-only"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
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
    - "docs/plans/dev-apm-and-distributable-package-separation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/dev-apm-and-distributable-package-separation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/dev-apm-project-and-distributable-package-layout.md"
    - "docs/specs/separate-dev-apm-project-from-distributable-package.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHQeQpDqrbmtswfZ: Move .apm primitives to nested distributable package

## Work

DESIGN-034r05iHy2gdnoPoUu7BiO の Target Layout に従い、root `.apm/` の配布 primitive
全体を `packages/apm/.apm/` へ `git mv` で移動し、manifest 2 件を
作成・更新する。

### 1. primitive の移動

```bash
mkdir packages/apm
git mv .apm packages/apm/.apm
```

移動対象: `.apm/hooks/`（hooks.json・pre-read-code.js・cli/ bundle
生成物）と `.apm/skills/`（cheaper-usage・cheaper-bulk-read・
cheaper-code-write・cheaper-doc-write）の全 tree。内容・権限は
無変更で移動のみ。

### 2. `packages/apm/apm.yml` の新設

DESIGN-034r05iHy2gdnoPoUu7BiO「Manifests」節の通り、最小 manifest を作成する:

```yaml
name: cheaper
version: "1.0.0"
description: Parent-agent delegation primitives for cheaper (skills, pre_read_code hook, bundled CLI)
author: scarecrow173
```

`targets`・`includes`・`dependencies` は持たない。`package.json` は
置かない（pnpm workspace の対象外にするため）。

### 3. root `apm.yml` の dev-only 化

- `name: cheaper` → `name: cheaper-dev`
- `includes: auto` 行を削除
- `targets`・`dependencies.apm`・`version`・`description`・`author`
  は現行維持（version source 統一は Issue #11 の scope）

`apm.lock.yaml` は root に残し、lock 内容はこの task では変更しない
（dependency ref は不変のため）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/dev-apm-and-distributable-package-separation.md`
- `docs/designs/dev-apm-project-and-distributable-package-layout.md`
- `docs/specs/separate-dev-apm-project-from-distributable-package.md`

## Dependencies

- なし（PLAN-034r05iHE1Fbkp5YZSKJM2 の先頭 task）

## Affected Files

- `.apm/` 全 tree → `packages/apm/.apm/` へ移動
- `packages/apm/apm.yml`（新規）
- `apm.yml`（name・includes）

変更しない file（明示）: `scripts/bundle-cli.mjs`・CI・test・
README/AGENTS.md は TASK-034r05iHQeQpDqsCIiTDOz〜0098 の担当。

## Done When

- [x] `packages/apm/.apm/` に hooks/skills 全 tree が移動している
- [x] repository root に `.apm/` が存在しない
- [x] `packages/apm/apm.yml` が DESIGN-034r05iHy2gdnoPoUu7BiO 通りの最小 manifest
- [x] root `apm.yml` が `name: cheaper-dev` で `includes` を持たない

## Verification

- [x] `git status` で rename として追跡されている
- [x] `ls packages/apm/.apm/hooks` / `skills` で移動内容を確認
