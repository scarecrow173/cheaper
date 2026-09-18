---
id: "DESIGN-034r05iHy2gdnoPoUu7BiO"
type: "design"
status: "approved"
title: "Development APM project / distributable cheaper package layout"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/separate-dev-apm-project-from-distributable-package.md"
    - "../adr/separate-dev-apm-project-via-nested-distributable-package.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/separate-dev-apm-project-from-distributable-package.md"
  implemented-by:
    - "../plans/dev-apm-and-distributable-package-separation.md"
  depends-on:
    - "./cli-bundle-layout.md"
    - "./cross-agent-distribution-verification.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/separate-dev-apm-project-via-nested-distributable-package.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/separate-dev-apm-project-from-distributable-package.md"
    - "../adr/separate-dev-apm-project-via-nested-distributable-package.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata: {}
---

# DESIGN-034r05iHy2gdnoPoUu7BiO: Development APM project / distributable cheaper package layout

## Context

ADR-034r05iHrZUE6WdKBH2eNM Option 1 に従い、root `.apm/` の配布 primitive を nested
distributable package `packages/apm/` へ移し、root `apm.yml` を
development-only manifest にする。SPEC-034r05iHK8bA7go4BQtGNS の受け入れ条件を構造で
満たすための具体的な layout・manifest・bundle path・test topology を
ここで確定する。

## Target Layout

```text
cheaper/
├── apm.yml                          # development-only APM project
├── apm.lock.yaml                    # dev dependency lockfile（root に残す）
│
├── packages/
│   ├── cli/
│   ├── core/
│   ├── runtime-pi/
│   └── apm/                         # distributable cheaper APM package
│       ├── apm.yml                  # package manifest（consumer-facing）
│       └── .apm/
│           ├── hooks/
│           │   ├── hooks.json
│           │   ├── pre-read-code.js
│           │   └── cli/             # bundled CLI（Git 管理の生成物）
│           └── skills/
│               ├── cheaper-usage/
│               ├── cheaper-bulk-read/
│               ├── cheaper-code-write/
│               └── cheaper-doc-write/
└── scripts/
    └── bundle-cli.mjs               # 出力先を packages/apm/.apm/hooks/cli へ
```

`packages/apm` は pnpm package ではない（`package.json` を持たない）。
pnpm workspace の `packages/*` glob は `package.json` を持つ dir だけを
拾うため、pnpm build/test の対象に混入しない。

## Manifests

### root `apm.yml`（development-only）

- `name` を `cheaper-dev` に変更し、distribution package（`cheaper`）と
  区別する。
- `targets`・`dependencies.apm`・`author`・`description` は現行を維持する
  （development dependencies の宣言・復元が責務）。
- `includes: auto` は削除する。root に `.apm/` も root-level primitive
  source（`skills/`・`hooks/` 等）も存在しないため、pack/install source
  discovery の宣言は不要（KISS）。
- `version` は現行 `1.0.0` を維持する（version source 統一は Issue #11
  の scope）。

### `packages/apm/apm.yml`（distributable package）

```yaml
name: cheaper
version: "1.0.0"
description: Parent-agent delegation primitives for cheaper (skills, pre_read_code hook, bundled CLI)
author: scarecrow173
```

- `targets` は持たない。consumer 側 `apm.yml` が install target を選ぶ
  既存 contract を維持する。
- `includes` も持たない。`.apm/` は pack/install の既定 source であり、
  package は hook bundle + skills の primitives layout として解釈される。
- consumer からの参照は virtual package
  （`owner/repo/packages/apm#ref`）または local `path:` dep。

## Bundle Pipeline

`scripts/bundle-cli.mjs` の `outDir` を
`packages/apm/.apm/hooks/cli` に変更する。生成物の構成
（`bin/cheaper.mjs`・`version.json`・`workers/`・`policy.yaml`）と
Git 管理方針は不変。`version.ts` の `../version.json` fallback、
`hooks.json` の `./cli/bin/cheaper.mjs` 参照はいずれも deploy 済み
bundle 内の相対 path であり、repo 内 path 移動の影響を受けない。

CI の bundle freshness check を `git diff --exit-code --
packages/apm/.apm/hooks/cli` へ追従する。

## Test Topology

### consumer fixture の更新

`packages/cli/test/e2e/apm-fixture.ts` の `makeConsumerLayout` は現在
repo `.apm/` を一時 `pkg/` へコピーしている。これを `packages/apm`
package 全体（`apm.yml` + `.apm/`）のコピーへ変更し、実際の
distributable package を install する形にする。fixture が自前で書く
最小 `apm.yml` は廃止し、実 manifest を使う。

`pre-read-hook.test.ts` の `hookScript` / `bundledCli` path を
`packages/apm/.apm/hooks/...` へ追従する。

### self-deploy 排除の regression test

root development install が cheaper own primitive を deploy しないことは、
構造の静的検証と APM 挙動の検証の 2 層で保証する。

1. **静的検証（unit）**: repository root に `.apm/` が存在せず、
   `packages/apm/.apm/` に skills/hooks が存在し、root `apm.yml` が
   自己参照 dependency を持たないことを assert する。
2. **挙動検証（e2e）**: 一時 dir に root `apm.yml` と同形の
   dev-project fixture（`.apm/` を持たない project）を作り
   `apm install` を実行し、target dir に cheaper-* skill が deploy
   されないことを確認する。remote dep を解決しない hermetic な形に
   するため、fixture は `apm install` が受理する最小 manifest
   （依存なし or local dep のみ）に寄せ、root `apm.yml` 自体の静的
   検証と組み合わせる。`apm` CLI 非存在環境では既存同様 skip する。

## File Map

| File | Change |
| --- | --- |
| `.apm/` → `packages/apm/.apm/` | `git mv` で全 primitive を移動。 |
| `packages/apm/apm.yml` | 新規。distributable package manifest。 |
| `apm.yml` | `name: cheaper-dev`・`includes` 削除の development-only manifest 化。 |
| `scripts/bundle-cli.mjs` | `outDir` を `packages/apm/.apm/hooks/cli` へ。コメント更新。 |
| `.github/workflows/ci.yml` | bundle freshness check の path を新 `packages/apm/.apm/hooks/cli` へ。 |
| `packages/cli/test/e2e/apm-fixture.ts` | `makeConsumerLayout` が `packages/apm` をコピー。 |
| `packages/cli/test/e2e/pre-read-hook.test.ts` | `hookScript` / `bundledCli` の path 追従。 |
| `packages/cli/test/e2e/apm-install-targets.test.ts` 等 | fixture 変更に伴う期待値の追従（deploy 結果は不変のはず）。 |
| `packages/cli/test/` | self-deploy 排除の検証を追加（unit 静的検証 + e2e fixture）。 |
| `README.md` | `.apm/` 記述を `packages/apm/` と development/distribution 分離に更新。 |
| `AGENTS.md` | セットアップ節に distribution package の存在を反映（最小限）。 |

## Risks and Trade-offs

- **`git mv` 後の deploy output**: root の `.agents/` 等は gitignore 済みで
  repo 状態に影響しない。既存 deploy output は次回 `apm install` で
  development deps のみに更新される。
- **consumer の install reference 変更**: repo root を直接 install する
  consumer は `packages/apm` sub-path 参照へ変更が必要。README で明記
  する。実運用前のため互換 shim は提供しない（ADR-034r05iHrZUE6WdKBH2eNM）。
- **`cheaper-dev` rename の影響範囲**: `apm.yml` の `name` は dev
  project 識別用であり、deploy される dir 名は dep の参照名由来のため
  影響は限定的。
- **bundle 生成の path 二重管理**: `outDir` を script 定数の一箇所で
  持ち、CI は `git diff` の対象 path だけを持つ。表現ドリフトを避ける
  ため test 側も fixture helper の定数から path を引く。

## Verification Notes

- `pnpm lint` / `pnpm typecheck` / `pnpm test` / bundle freshness
  （`node scripts/bundle-cli.mjs && git diff --exit-code --
  packages/apm/.apm/hooks/cli`）を通す。
- `apmAvailable()` 環境では e2e が consumer workspace へ skills/hooks/
  bundled CLI の deploy と `pre_read_code` 動作を確認する。
- root `.apm/` 不存在の静的検証と dev-project fixture の no-self-deploy
  e2e が追加される。
