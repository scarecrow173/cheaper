---
id: "IDEA-034zIIqQ6OptX5Bz9rXpiV"
type: "idea"
status: "promoted"
title: "Core resource identity and conservative manifest normalization"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/65"
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
    - "../discovery/core-resource-manifest-normalization.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Core resource manifest normalization

## Summary

#65-A と共有 ADR D4 の ResourceClaim identity / ResourceManifest 正規化を core の単一境界に実装する。

## Problem and Motivation

#59 / #61 が同じ resource ルールを使うため、不存在 output、symlink alias、glob と ownership の違いを統一する必要がある。

## Expected Value

runtime と workflow が canonical claim と pure predicate を共用できる。

## Open Questions

lease と compiler は後続 slice が所有する。

## Next Action

- [x] discovery → spec に昇格する。
