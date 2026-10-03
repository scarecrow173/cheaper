---
id: "IDEA-034z10aNxQyg2KNIm2Kew4"
type: "idea"
status: "promoted"
title: "Distribute cheaper as standalone executables including APM bundles"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/71"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../discovery/standalone-cheaper-executable-distribution-without-node-js.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Distribute cheaper as standalone executables including APM bundles

## Summary

`cheaper` CLI を Node.js 事前インストール不要な standalone executable
（Node 26 SEA）として配布し、APM package の payload にも同梱する。

## Problem and Motivation

- 現行の APM hook は `node ./pre-read-code*.js "./cli/bin/cheaper.mjs"`
  として起動するため、利用環境に Node.js が必要。
- Copilot / VS Code 向け hook bundle では nested JSON asset が除外される
  APM 側の制約があり、worker schema 等の外部 asset 配布に弱点がある。

## Expected Value

- Node.js を持たない環境の consumer が cheaper の hook 経由で delegation
  を利用できる。
- built-in worker / schema / policy を executable に埋め込むことで
  APM asset 欠落問題の影響を受けにくくなる。

## Open Questions

- Node 26 SEA の VFS / assets で built-in resources を通常 read-only file
  として参照できるか（dynamic import を含む Pi / OAuth 経路を含む）。
- platform launcher（POSIX / PowerShell）と hook adapter の Node.js
  dependency 除去範囲。
- standalone distribution と APM distribution が同一 executable
  artifact を使う release pipeline の形。

## Next Action

- [x] Promote to discovery-doc for deeper exploration
- [ ] Promote directly to spec-doc if requirements are clear
- [ ] Park for later reconsideration
- [ ] Discard — reason: <!-- why -->
