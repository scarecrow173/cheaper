---
id: "TSPEC-034Jg5ViJvfxJIh9xELlBL"
type: "test-spec"
status: "draft"
title: "Workflow job when condition"
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
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow job when condition

## Purpose

job `when` predicate の schema・compile dep 推論・engine
condition-skip・null materialization が design の演算子意味論通り
動くことを pin する。

## Feature

job が `when: {left, op, right}` で条件実行される。

## Rules

- exists は非 null materialization のみ真。
- eq/ne は JSON 等価、gt/lt は数値比較のみ(非数値 operand は false)。
- runtime binding 失敗は job failure で silent skip ではない。
- condition-skip は diagnostic CONDITION_SKIPPED を持ち、dependent へ
  伝播せず workflow error 集約に計上しない。
- condition-skipped producer への `$from` 参照は null を
  materialize する。
- compiler は `when` 内 binding を dep edge 推論へ含め、`$from`
  path を検証する。

## Examples

- `when: {left:{$from:exec,path:/details/outputTokens},op:gt,
  right:{$input:tokenThreshold}}` — outputTokens が数値かつ
  threshold 超過時のみ analyze job が走る。
- when=false の job に依存する job は CONDITION_SKIPPED で
  自身も skip されるが failure にならない。

## Guarantees

- skip が workflow の error カウントを増やさない。

## Non-goals

- 論理演算子の複合(and/or)、文字列比較。

## Risk

- dep 推論漏れは producer 未完了での when 評価を許す。
