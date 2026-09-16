---
id: "TASK-0074"
type: "task"
status: "done"
title: "CLI bundle script and hook argv resolution"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/0009-self-contained-cli-bundle.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0009-self-contained-cli-bundle.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
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

# CLI bundle script and hook argv resolution

## Work

`scripts/bundle-cli.mjs`（esbuild ESM+minify+createRequire banner、
package.json/workers/policy.yaml コピー）と root `bundle` script を実装し、
`.apm/hooks/cli/` を生成して commit。hooks.json の command に
`"./cli/bin/cheaper.mjs"` argv を追加し、pre-read-code.js の CLI 解決を
argv[2] → CHEAPER_CLI → PATH にする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/designs/0014-cli-bundle-layout.md`
- `docs/specs/0009-apm-self-contained-cli-distribution.md`

## Dependencies

- なし

## Affected Files

- `scripts/bundle-cli.mjs`、`package.json`、`.apm/hooks/{hooks.json,pre-read-code.js,cli/**}`

## Done When

- [ ] `pnpm bundle` で cli/ が再生成される
- [ ] bundled CLI が `--version`/`count-tokens` を実行できる

## Verification

- [ ] `mise exec -- pnpm bundle` 後に `node .apm/hooks/cli/bin/cheaper.mjs --version` が動く
