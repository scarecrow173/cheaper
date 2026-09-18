---
id: "SPEC-034r05iHK8bA7gffRgD3Fp"
type: "spec"
status: "implemented"
title: "Self-contained cheaper CLI distribution inside the APM hook bundle"
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
  depends-on:
    - "./apm-delegation-skills-and-read-hook.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/apm-self-contained-cli-distribution.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Self-contained cheaper CLI distribution inside the APM hook bundle

## What Should Be Built

`cheaper` CLI 全体（Worker 実行を含む）を esbuild single-file bundle と
して `.apm/hooks/cli/` に生成・同梱し、APM install だけで hook と
parent agent の双方が CLI を利用できる状態にする。

- `scripts/bundle-cli.mjs` — esbuild（ESM + createRequire banner + minify）
  で `bin/cheaper.mjs` を生成し、`package.json`（version 単一 source:
  `packages/cli/package.json`）・`workers/`・`policy.yaml` をコピーして
  `.apm/hooks/cli/` を再生成する
- root `package.json` に `bundle` script を追加
- `.apm/hooks/hooks.json` — command を
  `node ./pre-read-code.js "./cli/bin/cheaper.mjs"` に更新
- `pre-read-code.js` — CLI 解決順を argv[2] → `CHEAPER_CLI` env →
  `cheaper` on PATH に変更
- skills — bundled CLI path
  （`node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs`）を fallback として
  記載
- 生成物 `.apm/hooks/cli/` は commit する
- consumer 側 fresh install smoke（local path dep → `apm install
  --target windsurf` → deploy 確認 + hook block 確認）を e2e test 化

## Why Now

Phase 3 で hook/skill は作ったが、CLI 本体の配布経路が無く、
`cheaper` が PATH に無い環境では hook は常に fail-open、skill の手順も
実行不能だった。DISC-034r05iHy2gdnp9k0MK7uO で唯一の self-contained 経路（hook bundle
同梱）が確定した。

## Users and Value

- consumer: `apm install` だけで delegation 全経路が動く
- parent agent: bundled CLI path で `bulk-read` 等を実行可能

## Scope

### In Scope

- bundle 生成 script・生成物の commit・hook argv 解決・skill 記載更新
- local path dep consumer での e2e install smoke（`apm` 不在環境は skip）

### Out of Scope

- `cheaper` の PATH install 経路（npm global 等）
- bundle freshness の CI 検証
- `apm pack` 成果物への CLI 同梱（pack は任意 file を運ばない）
- version 番号の正式決定・release workflow（後続 Phase）

## Requirements

- `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs --version` が
  consumer workspace で動く
- `count-tokens`・`bulk-read`（model 解決まで）が bundled CLI で動く
- hook は bundled CLI を argv 経由で呼び、不在時は既存の fail-open
- `pnpm bundle` は決定的に `.apm/hooks/cli/` を再生成する

## Acceptance Criteria

- local path dep consumer に `apm install --target windsurf` すると
  `.windsurf/hooks/cheaper/` 配下に hook script + CLI bundle が deploy
  され、hooks.json が repo-relative path を指す
- その CLI で `--version` / `count-tokens` が動く
- threshold 超過 file の pre_read_code が bundled CLI 経由で block される
- `pnpm bundle` → install → smoke の再現手順が test/手順として存在する

## Sources and Relations

- DISC-034r05iHy2gdnp9k0MK7uO: bundle・deploy の実測
- SPEC-034r05iHK8bA7gexEML9oZ: skill/hook 本体
- SPEC-034r05iHK8bA7gdbCbWupr: bundled CLI が提供する contract
