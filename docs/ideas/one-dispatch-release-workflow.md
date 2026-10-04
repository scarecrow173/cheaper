---
id: "IDEA-034zSO5nJ7nAuF11FrikUD"
type: "idea"
status: "promoted"
title: "One-dispatch release workflow"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/73"
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
  derived-by: ["../discovery/one-dispatch-release-workflow.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# One-dispatch release workflow

## Summary

#73-B として、3 target executable、SHA256SUMS、APM bundle の作成と公開を一度の workflow dispatch にまとめたい。

## Problem and Motivation

- 現行 release は runner artifact を download して LFS commit し、再度 release workflow を実行する往復を必要とする。

## Expected Value

- release 担当者は同一 run で検証済み asset を揃えられ、#73-A launcher は固定 version の SHA256SUMS を取得できる。

## Open Questions

- 旧 LFS 経路との併存、bundle の内容、公開前検証の境界を discovery で明文化する。

## Next Action

- [x] discovery-doc に昇格した。coordinator の brief に沿って spec / ADR / design を提案する。
