---
id: "TSPEC-034Jg5X0Tcrvv5lp9x578v"
type: "test-spec"
status: "draft"
title: "Builtin route resources and embedded materialize"
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

# Builtin route resources and embedded materialize

## Purpose

builtin routes.yaml・routed-exec.workflow.json・condense worker が
embedded materialize と fingerprint 列挙に正しく含まれることを
pin する。

## Feature

binary embedded resource が routes/workflow/worker を materialize する。

## Rules

- builtin routes.yaml は enabled route を schema-valid に宣言する。
- routed-exec workflow は exec job + `when` 条件付き analyze
  condense worker で構成され compile が通る。
- analyze when の pointer は `/details/outputTokens`
  (CanonicalToolResult.details 階層)。
- condense は新規 builtin worker(md + input/output schema)として
  builtin_workers 列挙へ加わる。
- extract_builtin/embedded_fingerprint の resource 列挙が
  routes.yaml・workflows/・worker を含み、.ready dir が全 file を
  持つ。

## Examples

- materialize 後の builtin dir に routes.yaml・
  workflows/routed-exec.workflow.json・workers/condense.md が存在。

## Guarantees

- fingerprint 差分で旧 materialized dir が再生成される。

## Non-goals

- user/project 層 resource の embedding。

## Risk

- 列挙漏れは builtin route が load 不能な staleness を生む。
