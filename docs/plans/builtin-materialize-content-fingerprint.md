---
id: "PLAN-1OcqXwgY7qMQZeFcn7KJHn"
type: "plan"
status: "approved"
test-spec-skip: "Verified by new embedded.rs unit tests (fingerprint determinism, recursive collect_files coverage of workers/schemas/) plus cargo clippy/fmt and existing CLI regression corpus; no dedicated test-spec doc authored."
title: "builtin materialize content fingerprint 実装計画"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
    - "docs/adr/builtin-materialize-content-fingerprint.md"
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  implemented-by:
    - "docs/tasks/builtin-materialize-content-fingerprint.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/builtin-materialize-content-fingerprint.md"
    - "docs/tasks/builtin-materialize-content-fingerprint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/embedded.rs"
  defers: []
  deferred-by: []
---

# Plan: builtin materialize content fingerprint

## スコープ

`crates/cheaper-cli/src/embedded.rs` のみ。worker file 列挙を
`collect_files` に集約し、fingerprint を dir 名に混入する。

## ファイルマップ

- `crates/cheaper-cli/src/embedded.rs`: `collect_files` helper 追加、
  `extract_dir` の走査を `collect_files` 経由に置き換え、
  `embedded_fingerprint` 追加、`materialize_builtin_dir` の dir 名に
  `-<fingerprint>` suffix、header doc comment 更新、unit test 追加。

## タスク

1. `docs/tasks/builtin-materialize-content-fingerprint.md` — 実装 +
   unit test + `mise run check` 相当の検証。

## リスク

- 低。変更面は `embedded.rs` 単一ファイル・dir 名のみ。
- `.ready` skip・extract atomicity・OnceLock cache・hook override は
  不変。

## 検証

- `cargo test -p cheaper-cli embedded` (unit test)
- `cargo clippy`/`fmt` clean
- 手動: debug binary で `cheaper` を実行し temp に
  `cheaper-builtin-<version>-<hex>` dir が生成されることを確認。
