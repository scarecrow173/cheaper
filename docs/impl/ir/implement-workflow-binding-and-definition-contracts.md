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

approved design/plan に従い bindings/definition/DAG primitives を実装中。既存 scaffolding は export が空。

## Changes Made

strict JSON 境界、exact binding、result-root pointer、metadata/schema validation、DAG を対象にする。

## Validation and Evidence

これから failing contract test を先に実行する。baseline guard は既存 test のみ。

## Risks or Follow-ups

resource 推論は port、Worker input schema は core の正本を利用する。


2026-10-04: 初回 red は index 未実装で contract tests が失敗。primitives 実装後、it.each の array 展開により edge list が単一 object になった test 記述を object table に修正。green は 2 files / 23 tests passed、workflow build は exit 0。JSON Pointer・single-scan・DAG の source 責任を変更せず test 入力の原因を修正した。
