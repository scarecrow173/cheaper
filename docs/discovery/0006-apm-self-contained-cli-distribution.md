---
id: "DISC-0006"
type: "discovery"
status: "resolved"
title: "Self-contained cheaper CLI distribution inside the APM hook bundle"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
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
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0009-apm-self-contained-cli-distribution.md"
  refines: []
  refined-by: []
  derives-from:
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
  derived-by:
    - "../specs/0009-apm-self-contained-cli-distribution.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Self-contained cheaper CLI distribution inside the APM hook bundle

## Exploration Goal

hook script と skill が参照する `cheaper` CLI を、APM package のみで
consumer 環境へ届ける方法を確定する。`cheaper` が PATH に無い環境でも
hook が block 判定を実行でき、parent agent が `bulk-read` を実行できる
ことが要件。

## Confirmed Findings

### APM の executable 配布面の調査（docs 実読）

- `bin/` primitive deploy は **Claude Code 専用**・user-scope のみ。
  Windsurf には使えない
- `apm pack` の bundle は primitive（skills/hooks/instructions 等）のみを
  運び、任意 file（ビルド済み CLI 等）は含まない
- 一方 `apm install` は package 全体を `apm_modules/` に materialize する
  が、その path は consumer 側 config から安定的に参照できない

### hook bundle 内への CLI 同梱（実測、`/tmp/apm-probe`）

- hook command の `${PLUGIN_ROOT}` は **deploy された hook bundle dir**
  （`.windsurf/hooks/<pkg>/`）へ解決される。package root ではないため
  package 内の任意 file を指す用途には使えない
- `.apm/hooks/` 以下の nested directory は hook bundle として
  **再帰的に deploy** される。`./cli/bin/cheaper.mjs` を command arg に
  書くと `.windsurf/hooks/<pkg>/cli/bin/cheaper.mjs` へ deploy され、
  generated config は repo-relative path を指す
- JS bundle には commonjs default の `package.json` sidecar が bundle root
  に自動生成される。`.mjs` 拡張の ESM entry は影響を受けない

### esbuild bundle 実測

- `packages/cli/bin/cheaper.js` を esbuild で single-file ESM bundle 可能
  （pi SDK・js-tiktoken 込み ~19MB、minify で ~12.7MB）
- CJS 出力は top-level await で不可。ESM + `createRequire` banner で
  CJS dep の dynamic require を解消
- bundle の `import.meta.url` 基準で `defaultBuiltinDir()` が `../` を
  指すため、`<root>/bin/cheaper.mjs` + `<root>/{package.json,workers/,
  policy.yaml}` の layout で `--version`/`count-tokens`/`bulk-read`
  （model 解決まで到達し MODEL_TIER_UNAVAILABLE）が動作確認済み

## Decisions Recorded

- CLI は `.apm/hooks/cli/` 配下に esbuild bundle + assets として同梱し、
  hook bundle deploy に乗せる。deploy 先は
  `.windsurf/hooks/cheaper/cli/bin/cheaper.mjs`（pkg name = `cheaper`）
- hooks.json command は `node ./pre-read-code.js "./cli/bin/cheaper.mjs"`
  とし、CLI path を argv で渡す（env 代入は powershell/bash で構文が
  分かれるため避ける）
- hook script の CLI 解決順: argv[2] → `CHEAPER_CLI` env → `cheaper` on PATH
- bundle は `pnpm bundle` で生成し **commit する**。git-dep consumer が
  build step なしで動く唯一の経路であり、生成物の reproducibility は
  `pnpm bundle` が保証する（CI での freshness 検証は後続課題）
- skill 側の CLI 参照は `cheaper`（PATH）を正規形とし、bundled path
  `node .windsurf/hooks/cheaper/cli/bin/cheaper.mjs` を fallback として
  併記する

## Open Items

- 13MB の生成物を repo に commit する運用負荷（再生成忘れ・差分ノイズ）。
  CI で `pnpm bundle` 再生成との一致を検証する仕組みは後続 Phase
- consumer install での executables 承認フロー（`apm approve`）の
  ドキュメント化
- `cheaper` を PATH に載せる正式な install 経路は別途（現状は bundled
  path で完結）

## Sources and Relations

- DISC-0005: hook deploy 形式の先行実測
- SPEC-0009: 本 discovery から派生する配布仕様
