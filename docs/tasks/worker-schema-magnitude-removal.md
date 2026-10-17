---
id: "TASK-u2PMRboiXvyOu5ebYhDvVE"
type: "task"
status: "done"
title: "worker schema magnitude 制約の撤廃と maxFindings nullable 化"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/113"
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/bulk-reader.input.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/code-writer.input.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/code-writer.output.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/doc-writer.input.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/doc-writer.output.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/builtin_workers.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
  implemented-by: []
  depends-on:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
  blocks:
    - "docs/tasks/worker-prose-maxfindings-null.md"
    - "docs/tasks/consumer-docs-schema-magnitude.md"
    - "docs/tasks/regression-and-final-verification-113-114.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/worker-schema-structure-not-magnitude.md"
  defers: []
  deferred-by: []
---

# worker schema magnitude 制約の撤廃と maxFindings nullable 化

## Work

1. `crates/cheaper-cli/resources/workers/schemas/` の input×4・output×4
   から payload magnitude 制約を除去する(design §1a の表どおり):
   `maxLength`・`maxItems`・`maximum`。
2. `bulk-reader.input` `maxFindings`・`tool-operator.input`
   `output.maxFindings` の `maximum` 除去 + `type` を
   `["integer","null"]` に拡張(`minimum: 1` 維持)。
3. 維持対象(design §1b): `required`・`type`・`enum`・
   `additionalProperties: false`・`minLength: 1`・構造的 `minItems`・
   `$defs.path` pattern・`lines >= 1`・`grantRequests` 内制約は
   一切触らない。
4. schema corpus / golden test を更新・追加:
   - `task` 4000/6000 字超が受理される。
   - `findings[].title` 121 字超が受理される。
   - `maxFindings` 省略・`null`・正整数受理、`0`/負拒否。
   - 構造違反(required 欠落・型違い・path pattern・`grantRequests`
     制約)は引き続き拒否。

## Done

- 8 schema から magnitude 制約が除去され、構造制約のみ残る。
- schema corpus test が新挙動を pin し pass する。
