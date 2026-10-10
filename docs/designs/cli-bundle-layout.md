---
id: "DESIGN-034r05iHy2gdnoH3XDJcXx"
type: "design"
status: "superseded"
title: "cheaper CLI bundle layout inside .apm/hooks/cli"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/apm-self-contained-cli-distribution.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/apm-self-contained-cli-distribution.md"
  implemented-by: []
  depends-on:
    - "./apm-skills-and-pre-read-hook.md"
  blocks: []
  supersedes: []
  superseded-by: ["docs/designs/cargo-source-install-distribution-and-thin-path-launchers.md"]
  related:
    - "../discovery/apm-self-contained-cli-distribution.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper CLI bundle layout inside .apm/hooks/cli

## Context

SPEC-034r05iHK8bA7gffRgD3Fp の実装設計。生成物の layout・生成 script・hook/skill からの
参照方法を定める。

## Components

### 生成 layout（`.apm/hooks/cli/`）

```text
.apm/hooks/cli/
  bin/cheaper.mjs     # esbuild ESM bundle（minify + createRequire banner）
  package.json        # {"name":"cheaper","version":<cli package.json>}
  workers/            # packages/cli/workers のコピー
  policy.yaml         # packages/cli/policy.yaml のコピー
```

`defaultBuiltinDir()` が bundle の `import.meta.url` から `../` を指す
ため、この形で worker/policy/package.json を解決できる。

### 生成 script（`scripts/bundle-cli.mjs`）

1. `.apm/hooks/cli/` を削除して再作成
2. `npx esbuild packages/cli/bin/cheaper.js --bundle --platform=node
   --format=esm --target=node20 --minify
   --banner:js="import { createRequire as __cr } from 'node:module';const require = __cr(import.meta.url);"`
   → `cli/bin/cheaper.mjs`
3. `packages/cli/package.json` から `{name, version}` のみ抽出して
   `cli/package.json` を書く（version の single source は cli package）
4. `packages/cli/workers/` → `cli/workers/`、`packages/cli/policy.yaml`
   → `cli/policy.yaml` を再帰コピー
5. 完了時に bundle size を stderr に出す

root `package.json` scripts に `"bundle": "node scripts/bundle-cli.mjs"`。

### hook 側の CLI 解決（`pre-read-code.js`）

`argv[2]` → `CHEAPER_CLI` env → `cheaper` on PATH の順。argv[2] は
hooks.json が `"./cli/bin/cheaper.mjs"` で渡す bundled path。
`.js`/`.mjs` suffix は `process.execPath`（node）経由で起動。

```json
{"command": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\"",
 "powershell": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\""}
```

### skill 側の CLI 参照

`cheaper-usage` に「PATH に `cheaper` が無い環境では
`node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs`（APM install 済みの
Windsurf workspace）を使う」と追記。worker skill は cheaper-usage 参照の
まま変更なし。

## Non-Functional Constraints

- bundle は commit 対象の生成物。手編集禁止（`pnpm bundle` で再生成）
- consumer は node runtime のみ必要（bundle は node20+ target）

## Risks and Trade-offs

- ~13MB の生成物 commit。差分ノイズは minify 一行出力のため大きいが、
  再生成コストは低い。pack/配布の成熟後に artifact repo 等へ移行可能
- workers/ の二重管理（packages/cli/workers ↔ .apm/hooks/cli/workers）。
  bundle script が毎回全コピーするため drift しない。CI で再生成一致を
  検証するのが理想だが後続課題

## Verification Notes

- local consumer install smoke（e2e test、`apm` 不在時 skip）
- bundled CLI で `--version` / `count-tokens` / `bulk-read`（tier 解決
  まで）を直接実行して確認済み（DISC-034r05iHy2gdnp9k0MK7uO）
