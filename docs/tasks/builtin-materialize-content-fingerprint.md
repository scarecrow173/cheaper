---
id: "TASK-NQhhbHCzfcBiPGkcoasEIZ"
type: "task"
status: "done"
title: "embedded.rs: collect_files 集約 + fingerprint 混入"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "docs/plans/builtin-materialize-content-fingerprint.md"
  changes:
    added: []
    modified:
      - {type: "file", path: "crates/cheaper-cli/src/embedded.rs"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/builtin-materialize-content-fingerprint.md"
  implemented-by:
    - "docs/impl/ir/builtin-materialize-content-fingerprint.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/builtin-materialize-content-fingerprint.md"
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

# Task: embedded.rs fingerprint 混入

## やること

1. `collect_files(&Dir, &mut Vec<&File>)` 再帰 helper を追加
   (`Dir::entries()` 走査 — `Dir::files()` は直下のみなので不可)。
2. `extract_dir` の走査を `collect_files` 経由に置き換え、worker file
   の列挙を1箇所に集約(fingerprint 入力 = extract 出力対象)。
3. `embedded_fingerprint()` を追加:`collect_files` 結果を path で
   sort → `DefaultHasher` へ path+contents を投入 → `policy.yaml`・
   `version.json` を labelled 投入 → `{:016x}`。
4. `materialize_builtin_dir` の target を
   `cheaper-builtin-{version}-{fingerprint}` に変更。
5. file header doc comment を `cheaper-builtin-<version>-<fingerprint>`
   へ更新。
6. unit test を追加:
   - fingerprint が決定的(2回呼んで同値・16hex 形式)
   - collect_files が nested `schemas/*.json` を含む全 worker file を
     列挙する(非再帰 `files()` との回帰防止)
   - extract 対象集合 = fingerprint 入力集合であることの pin
     (`collect_files` が `extract_builtin` と共有されていることを
     構造で担保)。

## Acceptance

- `cargo test -p cheaper-cli embedded` グリーン。
- `cargo clippy`/`cargo fmt` clean。
- 既存 CLI 挙動不変(hook override 経路・`.ready` skip)。
