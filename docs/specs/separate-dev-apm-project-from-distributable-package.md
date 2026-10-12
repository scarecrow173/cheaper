---
id: "SPEC-034r05iHK8bA7go4BQtGNS"
type: "spec"
status: "implemented"
title: "Separate the development APM project from the distributable cheaper package"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/9"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/dev-apm-project-and-distributable-package-layout.md"
  depends-on:
    - "./apm-self-contained-cli-distribution.md"
    - "./apm-delegation-skills-and-read-hook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
    - "../designs/cli-bundle-layout.md"
    - "../designs/cross-agent-distribution-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/apm-self-contained-cli-distribution.md"
  derived-by:
    - "../adr/separate-dev-apm-project-via-nested-distributable-package.md"
    - "./user-facing-docs-information-architecture.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  spec:
    priority: "must"
---

# Separate the development APM project from the distributable cheaper package

## What Should Be Built

repository root の APM project を「`cheaper` を開発するための環境」に
限定し、parent Coding Agent へ配布する cheaper primitives
（skills / hooks / bundled CLI）を独立した nested distributable APM
package へ移す。root で `apm install` を実行しても cheaper 自身の
skills / hooks が development target に deploy されない構造にする。
一方、consumer project が distributable package を install した場合は
現在と同じ skills / hooks / bundled CLI が利用できることを維持する。

## Why Now

現在 root `apm.yml` は開発用 dependency を宣言しつつ、root `.apm/` が
cheaper の配布 primitive を保持している。`apm install` は dependency
復元と同時に project root 自身の `.apm/` も target へ deploy するため、
開発中の cheaper 自身が同じ checkout へ install される自己参照状態に
なっている。

この状態では broken な hook / skill を安全に修正できず、fresh checkout
が cheaper 自身の挙動から独立して bootstrap できない。また今後の
repository setup contract（Issue #7）で `apm install` を canonical
setup に組み込むと、この自己参照が canonical behavior として固定されて
しまう。

## Users and Value

- cheaper 開発者・Coding Agent: 開発対象の機能が無効でも安全に修正・
  bootstrap できる開発環境。
- consumer（parent agent の利用者）: 従来どおり cheaper skills / hooks /
  bundled CLI を install して利用できる。
- repository 外の実行基盤: cheaper 固有の内部手順を知らなくても安全な
  `apm install` を前提にできる。

## Scope

### In Scope

- root `apm.yml` の development-only manifest 化（自己配布 primitive を
  持たない）。
- root `.apm/` 配下（skills ×4、hooks.json、pre-read-code.js、bundled
  CLI tree）の nested distributable package 配下への移動。
- distributable package 用 `apm.yml` の作成。
- `scripts/bundle-cli.mjs` の出力先変更と bundle freshness check の
  新 path 追従。
- consumer fixture / temporary workspace による distribution test の
  分離・更新（root への self-install に依存しない）。
- 実行可能な path reference（scripts・CI・tests・package scripts）と
  現行 contract を説明する文書（README・AGENTS.md）の更新。
- 上記を維持する lint / typecheck / test / bundle freshness の成功。

### Out of Scope

- worktree manager、worktree 作成・削除・lifecycle 管理、branch
  lifecycle、sibling worktree discovery。
- `mise run setup` / `mise run check` の repository-owned setup contract
  化、CI を canonical entrypoint へ寄せる変更（Issue #7 側）。
- `CHEAPER_DISABLE_HOOKS` 等の環境変数による runtime 抑止、repository
  identity を hook が検出して早期 return する特殊分岐、`apm install
  --root` による回避、global install への退避。
- generated target artifacts（`.agents/`・`.claude/` 等）の Git 管理化、
  旧 root `.apm/` layout との compatibility layer / redirect / duplicate
  artifact。
- user credential / provider config の repository 内管理。

## Requirements

1. **責務分離**: root APM project は development dependencies のみを
   宣言・復元し、cheaper 自身の distribution primitive を所有しない。
   distributable package は consumer-facing artifact として独立して
   存在し、repository bootstrap dependency にしない。
2. **自己 deploy の排除**: repository root で `apm install` を実行しても
   cheaper 自身の skills が development target に deploy されず、
   `pre_read_code` hook が有効化されない。
3. **consumer 配布の維持**: distributable package を consumer workspace
   に install すると、cheaper skills が deploy され、hook が対象
   target に deploy され、bundled CLI が hook から解決できる。
   `pre_read_code` の delegation behavior は従来どおり動作する。
4. **bundle pipeline の追従**: bundled CLI は distribution package 内に
   生成され、source of truth（`packages/cli`）から bundle までの関係を
   維持する。生成済み bundle を Git 管理する方針は維持し、bundle
   freshness check は新しい path を検証する。
5. **hermetic な distribution test**: distribution behavior の検証は
   cheaper repository 自身への self-install に依存せず、temporary
   consumer workspace / fixture で行う。real user home / global APM
   state への依存を避ける。
6. **実行可能参照の整合**: root `.apm/` の存在を前提とする code /
   scripts / CI / tests / package scripts は新しい package path に
   追従する。旧 layout の互換 shim は作らない。
7. **文書の整合**: README / AGENTS.md が development install と
   distribution の責務分離を説明し、Coding Agent が cheaper repository
   で `cheaper-bulk-read` 等の存在を前提にしない記述になる。

## Acceptance Criteria

- [ ] root `apm.yml` が development-only APM project になっている
      （development dependencies の宣言・復元のみ）。
- [ ] repository root に distributable `.apm/` primitive が存在しない。
- [ ] cheaper の distributable APM package が nested package として
      明確に存在する。
- [ ] root で `apm install` を実行しても cheaper 自身の skills が
      development target に deploy されない。
- [ ] root install で cheaper `pre_read_code` hook が有効化されない。
- [ ] distributable package を consumer workspace に install すると
      cheaper skills が deploy される。
- [ ] consumer workspace で cheaper hook が deploy される。
- [ ] bundled CLI が distribution package 内に生成され hook から
      利用できる。
- [ ] bundle freshness check が新しい path を検証する。
- [ ] distribution test が cheaper repository 自身への self-install に
      依存しない。
- [ ] lint / typecheck / test が通る。
- [ ] README / AGENTS.md が development と distribution の責務分離を
      説明している。
- [ ] Issue #7 が本構造の root APM project をそのまま bootstrap に
      利用できる。

## Sources and Relations

- Issue #9: 問題の構造説明・実装要件・受け入れ条件の起点。
- SPEC-034r05iHK8bA7gexEML9oZ / SPEC-034r05iHK8bA7gffRgD3Fp / ADR-034r05iHrZUE6W5HVbwRSz / DESIGN-034r05iHy2gdnoFHbdllXV / DESIGN-034r05iHy2gdnoH3XDJcXx /
  DISC-034r05iHy2gdnp9k0MK7uO: 現在の delegation skills・hook・self-contained CLI bundle・
  配布形態の既存契約。本仕様は配布物の内容ではなく配置と責務を変更する。
- DESIGN-034r05iHy2gdnoLTC43RP5 / DISC-034r05iHy2gdnpdai9KGpW: cross-agent distribution の検証済み contract
  matrix と per-target 挙動。consumer 側の検証期待値を維持する。
- Issue #7: 本仕様の完了後に repository-owned setup contract を
  canonical 化する。

nested package の directory 名・manifest field・bundle 出力 path・
consumer install 形態は後続設計で確定する。本仕様では特定の実装方式を
選定しない。
