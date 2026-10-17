---
id: "IMPL-Dc1ijUuRtOtwRZigVnv16d"
type: "impl"
status: "completed"
title: "worker schema の構造検証原則と INVALID_OUTPUT raw output 永続化"
created: "2026-10-17"
updated: "2026-10-17"
owners: []
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
        path: "crates/cheaper-cli/resources/workers/bulk-reader.md"
      - type: "file"
        path: "crates/cheaper-cli/resources/workers/tool-operator.md"
      - type: "file"
        path: "crates/cheaper-core/src/worker/schema/mod.rs"
      - type: "file"
        path: "crates/cheaper-core/src/worker/validate.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/repair.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/builtin_workers.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/normalize.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-repair-exhausted.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-repair-success.json"
      - type: "file"
        path: "scripts/test/bulk-reader-skill-limits.test.mjs"
      - type: "file"
        path: ".agents/skills/cheaper-bulk-read/SKILL.md"
      - type: "file"
        path: ".agents/skills/cheaper-code-write/SKILL.md"
      - type: "file"
        path: ".agents/skills/cheaper-doc-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-bulk-read/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-code-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-doc-write/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
    - "docs/tasks/worker-schema-magnitude-removal.md"
    - "docs/tasks/invalid-output-raw-output-persistence.md"
    - "docs/tasks/invalid-input-errors-cap.md"
    - "docs/tasks/worker-prose-maxfindings-null.md"
    - "docs/tasks/consumer-docs-schema-magnitude.md"
    - "docs/tasks/regression-and-final-verification-113-114.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/worker-schema-structure-not-magnitude.md"
    - "docs/adr/worker-schema-structural-validation-not-magnitude.md"
    - "docs/adr/invalid-output-raw-output-persistence.md"
    - "docs/designs/worker-schema-structure-not-magnitude.md"
    - "docs/discovery/worker-schema-magnitude-constraints-and-output-recovery.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/worker-schema-structure-not-magnitude.md"
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

# worker schema の構造検証原則と INVALID_OUTPUT raw output 永続化

## Summary

Issue #113/#114 の統合対応。「JSON Schema は構造的 contract を検証し、
magnitude(文字数・件数)を検証しない」原則を全 built-in worker schema
(input×4・output×4)へ適用した。`INVALID_OUTPUT` 確定時は最終 raw
output の全文を `<userConfigDir>/invalid-outputs/run-<pid>-<nanos>.txt`
へ best-effort 保存し、`details.rawOutputPath` で回収経路を返す。
`INVALID_INPUT` の `details.errors` も `INVALID_OUTPUT` と同じ 20 件
cap に揃えた。

## What was implemented

- **schema magnitude 撤廃**: 8 schema から `maxLength`/`maxItems`/
  `maximum` を除去。`maxFindings` 系は `type: ["integer","null"]` +
  `minimum: 1`(上限なし)へ。構造制約(`required`・`type`・`enum`・
  `additionalProperties: false`・`minLength: 1`・`minItems`・path
  pattern・`lines >= 1`)と `tool-operator.output.grantRequests` の
  policy 連動上限は維持。
- **raw output 永続化**: `RepairOptions.user_config_dir` を追加し、
  `repair_output` の attempts 枯渇後に `save_raw_output` で atomic
  write(unix mode 0o600)。raw が空なら保存 skip、失敗は
  `rawOutputPath` 省略のみ。`details.errors`(20 件 cap)と
  `rawOutput`(先頭 2000 字 excerpt)は不変。
- **INVALID_INPUT errors cap**: `validate_worker_input` の
  `details.errors` を `MAX_DETAIL_ERRORS`(20)に揃え — 定数を
  `cheaper_core::worker::schema` へ移して両経路で共有。
- **worker prose**: bulk-reader/tool-operator に `maxFindings` が
  absent/`null` の場合は件数を model が決める旨を明記。
- **consumer docs**: `.agents/` と `packages/apm/` の skill 4 種
  (bulk-read・code-write・doc-write・tool-operate)+ `cheaper-usage`
  の上限値表記を撤去し、`rawOutputPath` 回復手順を追記。
- **drift guard 更新**: `bulk-reader-skill-limits.test.mjs` を新契約
  へ書き換え — magnitude 制約の再混入防止 + 構造契約/rawOutputPath
  記載の存在確認。

## Verification

- `builtin_workers::builtin_schemas_validate_structure_not_magnitude`:
  旧上限超過入力/出力の受理・構造違反の拒否を全 4 worker で pin。
- `run_invalid_output_persists_full_raw_output` /
  `run_invalid_output_save_failure_omits_path`: file 保存・path 露出・
  非 truncate・best-effort を pin。
- `validate_worker_input_caps_error_details`: 25 違反 → 20 件 cap。
- regression `run-repair-exhausted`/`run-repair-success` expected に
  `rawOutputPath` 追加、normalize に `run-<id>.txt` scrub rule 追加。
- `mise run check` 全段 pass(biome・fmt・clippy・version・doc-id・
  cargo test --workspace・node test・lockfile・apm payload)。

## Findings

- builtin materialize cache は version のみで key 付けされるため、同一
  version で embedded resource(schema/policy)が変わっても stale dir が
 再利用される(本 PR の根因の一つとして起票候補)。
- `INVALID_OUTPUT` の repair prompt は現状も raw output を無制限で
  session prompt へ載せる(回帰ではない既存状態 — 後続検討事項)。
