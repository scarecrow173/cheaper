---
id: "TSPEC-034Jg5X0Tcrvv5lp9x578v"
type: "test-spec"
status: "draft"
title: "Builtin route resources and embedded materialize"
created: "2026-10-20"
updated: "2026-11-19"
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
- `temp_dir()` が non-absolute を返す場合、materialize は target dir
  組立て段階(`materialize_target`)で `ErrorKind::InvalidInput`
  (`system temp dir is not absolute: <value>`)を返して fail-closed に
  失敗し、`.ready` 判定・extract を一切実行しない(issue #217)。
  代替 dir への fallback は行わない。

## Examples

- materialize 後の builtin dir に routes.yaml・
  workflows/routed-exec.workflow.json・workers/condense.md が存在。
- relative・空の temp dir 入力に対し `materialize_target` は
  `InvalidInput` を返し、absolute temp dir に対しては
  `<temp>/cheaper-builtin-<version>-<fp>` を join して返す。

## Guarantees

- fingerprint 差分で旧 materialized dir が再生成される。
- relative temp dir が cwd 相対へ展開され workspace を汚染することは
  構造的に不可能(non-absolute は組立て段階で reject されるため)。

## Non-goals

- user/project 層 resource の embedding。

## Risk

- 列挙漏れは builtin route が load 不能な staleness を生む。
