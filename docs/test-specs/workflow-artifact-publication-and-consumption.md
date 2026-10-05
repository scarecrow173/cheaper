---
id: "TSPEC-034AJ8C35s1GpJJoxU3sey"
type: "test-spec"
status: "proposed"
title: "Workflow における明示 bounded artifact read"
created: "2026-10-05"
updated: "2026-10-06"
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
    - "../designs/one-shot-workflow-execution-and-cli-composition.md"
  derived-by: []
  verifies:
    - "../specs/one-shot-workflow-engine-and-run-cli.md"
  verified-by: []
  references: ["../test-specs/run-scoped-artifact-boundaries.md"]
  defers: []
  deferred-by: []
---

# Workflow における明示 bounded artifact read

## Purpose

既存 store 保証が tool/Worker/CLI 配線で失われないよう adapter境界を検証する。artifact を利用しなくなった場合のみ再評価する。

## Feature

Workflow における明示 bounded artifact read。

## Rules

- Rule: workspace publication は owner が producer を固定し sandbox/deny/regular-file/image/NUL を維持。
- Rule: source read は cap+1 まで bounded、overflow は put 前に拒否。
- Rule: artifact read は namespace を明示し workspace resolver を呼ばない。
- Rule: 三つの read modes は排他的で既存 workspace read は互換。

## Examples

- Example: read({path,artifact:name}) はcontent空のreferenceを返し、Worker が byte offset/maxBytes で続きを読む。
- Example: mixed mode、foreign/tampered ref、無い publisher、source増大のoverflow は拒否。
- Example: preview/source の巨大値を final metadata に dump せず、cleanup後 availability unavailable。

## Guarantees

ArtifactReader/store の既存 identity/caps/copy/lifetime 正本を再利用し、公開previewは declared outputs のみ。

## Non-goals

store単体保証の再定義、暗黙artifact化、model token savings。

## Risk

workspace境界bypass、cap越えmemory、private preview漏洩。
