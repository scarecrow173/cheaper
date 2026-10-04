---
id: "TSPEC-034A9ykvVgiieTjkU129tm"
type: "test-spec"
status: "approved"
title: "managed process task の認可と resource ownership"
created: "2026-10-04"
updated: "2026-10-04"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/managed-process-canonical-tool.md"
  derived-by: []
  verifies:
    - "../specs/managed-process-canonical-tool.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# managed process task の認可と resource ownership

## Purpose

named task が唯一の認可境界であり、宣言 I/O の ownership を偽造できないことを維持する。process tool 廃止時にのみ退役可能。

## Feature

Feature: policy-defined task の strict config、cap 合成、canonical manifest。

## Rules

- Rule: raw command/shell、未知 task、不正 cwd/env/config を拒否する。
- Rule: legacy default と trusted→project maxConcurrent の min を維持する。
- Rule: outputs の producer authority を超える宣言、部分集合 producer glob、writeScope の矛盾を拒否する。

## Examples

- Example: trusted cap=4 / project=8 は4、project=2 は2、omission後の project=8 は1。
- Example: `out/a.json` と `reports/**` は exact/subtree producer。`out/*.json` は producer として拒否。
- Example: task の out/a に対し Workflow resources.produces=out/b は拒否。

## Guarantees

- config の snapshot、非空 tasks の exposure、scope policy 継承と不一致拒否、task output authority と boundary は adapter 共通。

## Non-goals

- OS tracing/sandbox、未宣言 access の観測、実 provider model。

## Risk

- 任意実行や cap 緩和、producer edge の偽装、Worker writeScope の拡大が起きる。
