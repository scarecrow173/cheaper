---
id: "TASK-EnsmZdrtj5ic4bLBfseeL3"
type: "task"
status: "done"
title: "INVALID_OUTPUT の raw output 全文 file 永続化"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/114"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-runtime/src/run/repair.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/normalize.rs"
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
    - "docs/adr/invalid-output-raw-output-persistence.md"
  defers: []
  deferred-by: []
---

# INVALID_OUTPUT の raw output 全文 file 永続化

## Work

1. `crates/cheaper-runtime/src/run/` に保存 helper を追加
   (`invalid_output.rs` 新規 or `checkpoint.rs` 同型):
   `<userConfigDir>/invalid-outputs/run-<pid>-<nanos>.txt` へ
   `atomic_write`(unix mode 0o600)。内容は最終 attempt の
   raw output 全文(fence 除去前 original)。
2. `RepairOptions` に `user_config_dir: &Path` を追加し、
   `extract_or_repair` から `deps.runtime.user_config_dir()` を渡す。
   `repair_output` の loop 尽き後、`invalid_output_error` 組立前に
   保存を試み、成功時のみ `details.rawOutputPath` を追加。
   `raw_output.is_empty()` なら保存 skip。
3. 保存失敗は best-effort — 分類・exit code・`errors`/`rawOutput`
   excerpt は不変、path key を省略。
4. dev-seam test:
   - `INVALID_OUTPUT` で `details.rawOutputPath` が存在し file 内容が
     最終 raw output と一致。
   - 書き込み不能で `rawOutputPath` 省略・error 分類不変。
   - `no-assistant-message` で path なし。

## Done

- `INVALID_OUTPUT` の `details` に `rawOutputPath` が載り、file から
  全文を回収できる。保存失敗でも既存挙動を損なわない。
