---
id: "SPEC-034r05iHK8bA7grnYLdwgl"
type: "spec"
status: "implemented"
title: "Unified version and release source of truth"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/11"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/unified-version-source-of-truth.md"
  depends-on:
    - "./separate-dev-apm-project-from-distributable-package.md"
    - "./repo-owned-setup-contract-for-fresh-checkouts.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/cli-bundle-layout.md"
    - "../designs/dev-apm-project-and-distributable-package-layout.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../adr/cli-package-json-as-canonical-version-with-check-enforcement.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "should"
---

# Unified version and release source of truth

## What Should Be Built

製品 version の canonical source of truth を 1 箇所に定め、`--version`
出力・APM 配布 manifest・bundle の `version.json`・全 workspace package
宣言が同一の値を返す契約を定める。version bump と release の手順を
文書化し、不一致を自動検出する check を用意する。

## Why Now

version 宣言が複数箇所にあり値が発散している（issue #11）:
`packages/{cli,core,runtime-pi}/package.json` は `0.0.0`、
`packages/apm/apm.yml`（配布 package `cheaper`）は `1.0.0`。
`cheaper --version` は `0.0.0` を返すが配布 manifest は `1.0.0` を
名乗り、どれが真の version か曖昧で release 時に何を更新するかも
未定義である。

## Users and Value

- release 実施者: 更新箇所と検証方法が一意に決まり、手順ミスを防げる。
- consumer: `--version` と APM manifest が一致し、報告された version を
  信頼できる。
- 開発者: version 不一致が CI で即座に検出される。

## Scope

### In Scope

- canonical version source の選定と明文化。
- `packages/cli/package.json`、`packages/core/package.json`、
  `packages/runtime-pi/package.json`、`packages/apm/apm.yml`、生成物
  `version.json`、`--version` 出力の一致保証。
- 不一致を検出する check script と canonical check 経路（`mise run
  check`）への組み込み。
- version bump と release の手順文書。

### Out of Scope

- root `apm.yml`（`cheaper-dev` dev project）の version — 製品 version
  ではなく dev project の識別子であり、独立した lifecycle を持つ。
- semantic versioning 規則そのものの策定（major/minor/patch の意味論は
  release 文書で最小限の約束のみ記す）。
- git tag・GitHub Release・配布 channel の自動化。
- `apm.lock.yaml` や consumer 側の pin 管理。

## Requirements

1. **canonical source**: 製品 version の真の source を 1 箇所に定める。
   選定は ADR で確定する。
2. **全宣言の一致**: workspace package の version 宣言と配布 manifest が
   canonical と一致する。`--version` は canonical 由来の値を返し、
   bundle の `version.json` は bundle 時に canonical から派生する。
3. **不一致の検出**: 宣言が発散した場合に fail する check script を用意
   し、`mise run check`（canonical verification entrypoint、SPEC-034r05iHK8bA7gpcCJQya9）
   から呼び出す。CI は同一 entrypoint を通るため自動的に検出される。
4. **bump 手順**: version を上げる操作を 1 コマンドで行えるようにし、
   更新される全 file と、bump 後に必要な生成物再生成を明記する。
5. **release 手順の文書化**: bump → 検証 → commit → 配布物確認 の手順を
   `docs/guides/` に記す。
6. **dev project の区別**: root `apm.yml`（`cheaper-dev`）は製品 version
   と独立であることを文書で明記し、check の対象外とする。

## Acceptance Criteria

- [ ] canonical source が ADR と文書で一意に定められている。
- [ ] `cheaper --version`・`packages/apm/apm.yml`・
      `packages/apm/.apm/hooks/cli/version.json`・全 workspace
      package.json が同じ値を返す。
- [ ] いずれかの宣言をわざとずらすと `mise run check`（ないし check
      script 直接実行）が fail する。
- [ ] version bump の 1 コマンド操作が存在し、実行後に全宣言と生成物が
      一致する。
- [ ] release 手順文書が存在し、bump・検証・配布物確認の流れが記されて
      いる。
- [ ] `pnpm typecheck`・`pnpm lint`・test suite 全件が通る。

## Sources and Relations

- Issue #11: 本仕様の起点。
- SPEC-034r05iHK8bA7go4BQtGNS / DESIGN-034r05iHy2gdnoPoUu7BiO: dev project と distributable package の分離。
  配布 manifest は `packages/apm/apm.yml`。
- SPEC-034r05iHK8bA7gpcCJQya9 / ADR-034r05iHrZUE6WeDAi63F3: `mise run check` canonical verification
  entrypoint。version check はここへ組み込む。
- DESIGN-034r05iHy2gdnoH3XDJcXx: `version.json` fallback 経路と bundle 生成の現行契約。

canonical source の選定（`packages/cli/package.json` 等）と一致保証の
方式（生成 vs 検証）は後続 ADR で確定する。
