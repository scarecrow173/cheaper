---
id: "TSPEC-034Jg5WcioU9vlqfkgBjPR"
type: "test-spec"
status: "draft"
title: "Hook dedup cache route fingerprint and Rewrite exclusion"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
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
  derived-by: []
  verifies:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Hook dedup cache route fingerprint and Rewrite exclusion

## Purpose

hook verdict dedup cache に effective routes fingerprint が含まれ、
Rewrite verdict が persist/serve 対象外であることを pin する。

## Feature

shell verdict cache key が route 宣言内容で invalidate される。

## Rules

- effective routes 内容 hash(DefaultHasher 準拠の決定的 hash)は
  shell verdict cache key の入力に含まれる。
- Rewrite verdict は env 値を含み得るため persist も serve もしない。
- Allow/Deny dedup は従来通り有効。
- routes 変更後の同一 command が stale verdict を返さない。

## Examples

- routes.yaml 追加後に同一 command を eval → cache miss で新規
  resolve が走る。

## Guarantees

- Rewrite verdict がディスクへ永続化されない(env leak 防止)。

## Non-goals

- route 宣言自体の versioned migration。

## Risk

- fingerprint 未参加は stale rewrite を残す。
