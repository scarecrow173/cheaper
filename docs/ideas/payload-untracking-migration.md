---
id: "IDEA-034zWkslEDhnpFKJZpyzU4"
type: "idea"
status: "promoted"
title: "Payload untracking migration"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/73"]
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
  derived-by: ["DISC-034zWqZnTAsRi8D1paYgln"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Payload untracking migration

## Summary
巨大な SEA payload を Git/LFS から外し、固定 version の launcher 取得を Git-based APM consumer に適用する。

## Problem and Motivation
現在の binary 同梱・再生成・commit が checkout と release の負担を増やす。

## Expected Value
consumer と開発者が軽量な source checkout を使い、Node 非依存の hook を維持できる。

## Open Questions
必須 payload check と既存 install の移行契約を discovery で確定する。

## Next Action
DISC-034zWqZnTAsRi8D1paYgln へ昇格し、issue #73-C の acceptance を仕様化する。
