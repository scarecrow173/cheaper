---
id: "IMPL-034zQvN42aaLRbRi0R2fI4"
type: "impl"
status: "in-progress"
title: "Implement workflow compilation and public hooks"
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
    - "docs/tasks/implement-workflow-compilation-and-public-hooks.md"
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

# Workflow compiler と public hooks の実装記録

## Summary

approved design/plan に従い compiler/projection を実装中。実行系は scope 外。

## Changes Made

default snapshot、non-scan、static/deferred validation、terminal output、resource manifest 保持を対象にする。

## Validation and Evidence

これから failing tests を実行する。plan gate msg_54c265f472f0 は APPROVED: plan。

## Risks or Follow-ups

#62/#70 consumer が exported hooks を利用する。AC7 guidance は本 IR に記録する（plan review で IR のみも受理可）。
