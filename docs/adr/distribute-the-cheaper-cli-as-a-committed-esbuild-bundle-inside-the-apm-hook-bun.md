---
id: "ADR-034r05iHrZUE6W5HVbwRSz"
type: "adr"
status: "accepted"
title: "Distribute the cheaper CLI as a committed esbuild bundle inside the APM hook bundle"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/apm-self-contained-cli-distribution.md"
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
  superseded-by: []
  related:
    - "../specs/apm-self-contained-cli-distribution.md"
    - "../designs/cli-bundle-layout.md"
    - "../plans/self-contained-cli-bundle.md"
    - "./default-large-read-threshold.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/apm-self-contained-cli-distribution.md"
  derived-by:
    - "../designs/cli-bundle-layout.md"
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

# 13. Distribute the cheaper CLI as a committed esbuild bundle inside the APM hook bundle

## Context and Problem Statement

`pre_read_code` hook と worker skill が呼び出す `cheaper` CLI の配布経路が
存在しなかった。`cheaper` が PATH に無い環境では hook が CLI を解決できず
常に fail-open となり、large-read block と `bulk-read` 委譲が機能しない。
DISC-034r05iHy2gdnp7GkO7QDK のユーザー決定により配布は APM package のみ（npm registry /
GitHub Release は作らない）と定まっていたため、APM package の deploy 機構
だけで consumer 環境へ CLI を届ける方法が必要だった。

本 ADR は DISC-034r05iHy2gdnp9k0MK7uO の実測で確定した判断を遡及的に記録するものであり、
実装は PLAN-034r05iHE1FbkoWI8p70oC / TASK-034r05iHQeQpDq2z00rDnn 系で完了済みである。

## Decision Drivers

- `cheaper` が PATH に無い環境でも hook の block 判定と `bulk-read` が
  動作すること（fail-open にならないこと）
- 配布経路は APM package のみ。追加の install 手順・build step を
  consumer に要求しない
- git dependency 経由の consumer（`apm install <git-url>`）でも動くこと。
  source build を要求しない
- hook command は Windows（PowerShell）と POSIX shell の両方で動く
  argv ベースの形にすること

## Considered Options

- npm global install — self-contained でなく、APM package のみで完結する
  という配布方針と矛盾
- `apm bin/` primitive — Claude Code 専用・user-scope のみで Windsurf へは
  deploy できない（DISC-034r05iHy2gdnp9k0MK7uO 実測）
- `apm_modules/` 直接参照 — `apm install` は package 全体を
  `apm_modules/` に materialize するが、consumer 側 config から安定的に
  参照できる path がない（DISC-034r05iHy2gdnp9k0MK7uO 実測）
- `.apm/hooks/cli/` への nested hook-bundle deploy + esbuild single-file
  bundle — hook bundle は再帰的に deploy され、`${PLUGIN_ROOT}` が
  deploy 先を指すため安定参照できる（採用）

## Decision Outcome

Chosen option: "`.apm/hooks/cli/` nested hook-bundle deploy + esbuild
bundle"。

- `.apm/hooks/cli/bin/cheaper.mjs` に esbuild 生成の single-file ESM
  bundle（minify + `createRequire` banner）を配置し、hook bundle deploy
  に乗せる。deploy 先は `.windsurf/hooks/cheaper/cli/bin/cheaper.mjs`
- layout は `<root>/bin/cheaper.mjs` + `<root>/{package.json, workers/,
  policy.yaml}`。bundle の `import.meta.url` 基準で `defaultBuiltinDir()`
  が `../` を解決できる形
- CJS 出力は top-level await で不可のため ESM + `createRequire` banner
  で CJS dependency の dynamic require を解消
- bundle は `pnpm bundle`（`scripts/bundle-cli.mjs`、esbuild pin）で生成し
  **commit する**。git-dep consumer が build step なしで動く唯一の経路
- hooks.json command は argv で CLI path を渡し、shell 文字列展開・env
  代入を避ける。hook script の CLI 解決順は argv[2] → `CHEAPER_CLI` env
  → `cheaper` on PATH

### Consequences

* Good, because `cheaper` が PATH に無い環境でも hook / skill が bundled
  CLI を解決でき、block 判定と委譲が機能する。追加 install 手順が不要。
* Good, because APM package のみの配布方針を維持したまま git-dep
  consumer を含む全 install 経路で動作する。
* Bad, because ~13MB の生成物を repo に commit する運用負荷がある
  （再生成忘れ・差分ノイズ）。CI の bundle freshness チェックで緩和。
* Bad, because bundle の再生成が必要な変更（CLI・依存更新）は
  `pnpm bundle` の実行と差分 commit を伴う。

### Confirmation

`pnpm bundle` 再生成との一致を CI（bundle freshness チェック）で検証済み。
EXP-0005 で deploy 済み `.windsurf/hooks/cheaper/cli/bin/cheaper.mjs` 経由の
OAuth 付き `bulk-read` 実動作を確認済み。

## Implementation Plan

* Affected paths: `scripts/bundle-cli.mjs`, `.apm/hooks/cli/`,
  `.apm/hooks/hooks.json`, `.apm/hooks/pre-read-code.js`,
  `packages/cli`（bundle 元）
* Patterns to follow: hook command は argv ベースの process 起動
  （`node ./pre-read-code.js "./cli/bin/cheaper.mjs"`）。shell 文字列実行
  しない
* Patterns to avoid: env 代入を command 内で行う記法（PowerShell / bash
  で構文が分かれる）、`apm_modules/` への直接参照、`apm bin/` への依存
* Dependencies or configuration: esbuild（pin 済み devDependency）、
  ESM format + `createRequire` banner。consumer 側設定は不要

## Verification

* [x] `pnpm bundle` が deterministic に bundle を再生成し CI で freshness
      が検証される
* [x] `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs --version` /
      `count-tokens` / `bulk-read`（OAuth 経由）が deploy 先 path で動作
      （EXP-0005）
* [x] hooks.json command が argv ベースで shell 展開を含まない
* [x] `cheaper` 不在環境でも hook が fail-open せず block 判定できる
      （bundled CLI 解決経路）

## Pros and Cons of the Options

### npm global install

* Good, because CLI の更新経路が通常の package 管理に乗る。
* Bad, because APM package のみで完結するという配布方針と矛盾し、
  consumer に追加 install step を要求する。hook が fail-open になる
  環境を作る。

### `apm bin/` primitive

* Good, because APM の正式な executable 配布面を使える。
* Bad, because Claude Code 専用・user-scope のみで Windsurf へ deploy
  できない（DISC-034r05iHy2gdnp9k0MK7uO 実測）。

### `apm_modules/` 直接参照

* Good, because bundle 同梱なしに source 配置だけで届けられる可能性が
  あった。
* Bad, because consumer 側 config から安定的に参照できる path がなく、
  materialize 先は install の実装詳細に依存する（DISC-034r05iHy2gdnp9k0MK7uO 実測）。

### nested hook-bundle deploy + esbuild bundle (chosen)

* Good, because hook bundle の再帰 deploy という APM の既存機構だけで
  完結し、`${PLUGIN_ROOT}` 経由で deploy 先を安定参照できる。build
  step を consumer に要求しない。
* Bad, because ~13MB の生成物を repo に commit する運用負荷と、bundle
  freshness の維持コストがある。

## More Information

* `docs/discovery/apm-self-contained-cli-distribution.md` — 実測根拠
  （bin primitive・apm_modules・esbuild bundle の検証）
* `docs/specs/apm-self-contained-cli-distribution.md` — 配布仕様
* `docs/designs/cli-bundle-layout.md` — layout 設計
* `docs/plans/self-contained-cli-bundle.md` — 実装計画
* ADR-034r05iHrZUE6W4XOqUBih — bundled CLI が提供する `count-tokens` の threshold 決定
