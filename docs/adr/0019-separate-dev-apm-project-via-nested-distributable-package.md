---
id: "ADR-0019"
type: "adr"
status: "accepted"
title: "development APM project と distributable cheaper package を nested package で物理分離する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0016-separate-dev-apm-project-from-distributable-package.md"
    - "https://github.com/scarecrow173/cheaper/issues/9"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0013-distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0014-cli-bundle-layout.md"
    - "../discovery/0006-apm-self-contained-cli-distribution.md"
    - "../discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/0016-separate-dev-apm-project-from-distributable-package.md"
  derived-by:
    - "../designs/0021-dev-apm-project-and-distributable-package-layout.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 19. development APM project と distributable cheaper package を nested package で物理分離する

## Context and Problem Statement

root `apm.yml` は開発用 dependency を宣言し、root `.apm/` が cheaper の
配布 primitive（4 skills・hooks.json・pre-read-code.js・bundled CLI
tree）を保持している。APM の `apm install` は project root 自身の
`.apm/` も target へ deploy するため、開発環境構築のたびに開発中の
cheaper 自身が同じ checkout へ install される自己参照状態になっている。

確認済みの事実は次のとおりである。

- APM は package root に `.apm/` を持つ package を primitives layout
  （複数 primitive を consumer runtime dirs へ hoist）として扱う。
- root `.gitignore` は `.agents/`・`.claude/` 等の deploy output を除外し、
  `.apm/` だけを Git 管理対象にしている。
- `scripts/bundle-cli.mjs` は `.apm/hooks/cli/` へ self-contained CLI を
  生成し、CI の bundle freshness check が `git diff --exit-code --
  .apm/hooks/cli` で同期を検査している。
- e2e（`packages/cli/test/e2e/apm-*.test.ts`）は既に repo `.apm/` を
  一時 `pkg/` へコピーした consumer fixture で検証している。
- APM は `owner/repo/path@ref` や `git:` + `path:` object form の virtual
  package（repo 内 sub-directory install）をサポートする。

## Decision Drivers

- development / distribution の責務分離と bootstrap 時の自己発火排除。
- hook 内の例外処理や環境変数による抑止ではなく、構造で解決する。
- consumer から見た install 形態と deploy 結果を従来どおり維持する。
- KISS・YAGNI: compatibility layer・redirect・duplicate artifact を
  作らない。runtime special case を持ち込まない。
- Issue #7 が root APM project をそのまま bootstrap に利用できること。

## Considered Options

- **Option 1**: root `.apm/` を nested distributable package
  （`packages/apm/`）へ移し、root `apm.yml` を development-only
  manifest にする。
- **Option 2**: `CHEAPER_DISABLE_HOOKS=1` 等の環境変数で hook を抑止する。
- **Option 3**: repository identity（package.json name・remote・dir 名）を
  hook が検出して早期 return する。
- **Option 4**: `apm install --root` で install 出力を redirect する。
- **Option 5**: user scope の global install に cheaper primitives を逃がす。

## Decision Outcome

**Option 1** を採用する。root `apm.yml` は development-only manifest と
なり、distributable `.apm/` tree は nested package（`packages/apm/`）
配下へ移動する。bundled CLI の生成先も同 package 内へ変更する。

理由は次のとおり。

- Option 2 は skills を deploy したまま残すため責務分離にならず、
  repository bootstrap の correctness を環境変数へ依存させる。
- Option 3 は distribution runtime に「自分を開発しているときだけ動かない」
  という特殊知識を持ち込み、minimal special cases の原則に反する。
- Option 4 は install output の redirect であり、source project の `.apm/`
  primitive 自体を install 対象から外す責務を果たさない。
- Option 5 は user scope を repository setup の前提にし、fresh
  checkout / worktree ごとの再現性と isolation を損なう。
- Option 1 は APM の layout 仕様（root `.apm/` = primitives package）に
  そのまま乗り、構造だけで自己 deploy を排除できる。consumer は
  virtual package（sub-path）install で同一の deploy 結果を得られる。

distributable package の manifest は consumer-facing の最小形とし、
`includes`・`targets` は consumer 側の `apm.yml` が担う既存 contract を
変えない。root `apm.yml` からは自己配布 primitive の所有を除き、
development dependencies の宣言に限定する。

## Consequences

- root `apm install` は development dependencies のみを復元し、cheaper
  自身の skills / hooks は deploy されない。
- `.apm/` 前提の実行可能参照（bundle script・CI・e2e fixture）は
  `packages/apm/.apm/` へ追従する。
- consumer の install reference は `owner/repo/packages/apm@ref` 相当の
  virtual package または local `path:` となる。
- 旧 root `.apm/` layout との compatibility shim は提供しない。
- Issue #7 はこの分離済み root APM project を前提に setup contract を
  設計できる。
