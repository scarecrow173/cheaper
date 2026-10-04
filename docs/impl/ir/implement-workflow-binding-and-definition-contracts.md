---
id: "IMPL-034zQvMyE33qrrPAVVG0da"
type: "impl"
status: "completed"
title: "Implement workflow binding and definition contracts"
created: "2026-10-04"
updated: "2026-10-04"
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
  implements:
    - "docs/tasks/implement-workflow-binding-and-definition-contracts.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Workflow binding と definition contracts の実装記録

## Summary

approved design/plan に従い bindings/definition/DAG primitives と public exports を実装した。

## Changes Made

strict JSON 境界、exact binding、result-root pointer、metadata/schema validation、DAG を対象にする。

## Validation and Evidence

failing contract tests で未実装を確認してから実装し、既存 dependency guard とともに green を確認した。

## Risks or Follow-ups

resource 推論は port、Worker input schema は core の正本を利用する。


2026-10-04: 初回 red は index 未実装で contract tests が失敗。primitives 実装後、it.each の array 展開により edge list が単一 object になった test 記述を object table に修正。green は 2 files / 23 tests passed、workflow build は exit 0。JSON Pointer・single-scan・DAG の source 責任を変更せず test 入力の原因を修正した。

## Context

ADR D1/D5 と approved spec/design/plan。compiler の parser/schema/DAG 境界を担当する。

## Implementation

src/types.ts/json.ts/bindings.ts/schema.ts/dag.ts と binding/DAG tests を追加。compileSchema の再利用、Worker/tool metadata ports、安全な JSON clone、exact bindings、RFC 6901、sorted Kahn DAG を実装した。

## Related Experiments

既知の設計を実装したため experiment log は作成していない。red/green の実績は Validation and Evidence に記録した。

## Follow-ups

未分類の follow-up はない。compiler/hooks の後続 task で消費する。
