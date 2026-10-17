---
id: "IMPL-gY3xK9wV2mNbQ8rTzAhJdL5"
type: "impl"
status: "done"
title: "IR: builtin materialize content fingerprint"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "docs/tasks/builtin-materialize-content-fingerprint.md"
  changes:
    added: []
    modified:
      - {type: "file", path: "crates/cheaper-cli/src/embedded.rs"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/builtin-materialize-content-fingerprint.md"
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/tasks/builtin-materialize-content-fingerprint.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/embedded.rs"
  defers: []
  deferred-by: []
---

# IR: builtin materialize content fingerprint

## 実装内容

`crates/cheaper-cli/src/embedded.rs`:

- `collect_files(&Dir, &mut Vec<&File>)` 再帰 helper を新設。
  `include_dir` 0.7 の `Dir::files()`/`entries()` は直下のみ列挙する
  ため、`entries()` を再帰して全 nested file を収集する。
- `extract_dir` を `collect_files` 経由に書き換え — worker file の
  列挙を1箇所に集約し、extract 出力対象と fingerprint 入力対象の
  drift を構造的に排除(review-protocol の P1/P2 指摘に対応)。
- `embedded_fingerprint()` を新設 — workers 全 file の path+contents
  を path sort 順に `DefaultHasher` へ投入し、`policy.yaml`・
  `version.json` を labelled で追加。`{:016x}` で dir-safe な hex に。
- `materialize_builtin_dir` の target を
  `cheaper-builtin-{version}-{fingerprint}` に変更。
- header doc comment を新 dir 名に合わせて更新。

## レビュー対応

review-protocol (Reviewer A/B via bulk-read + Validator):

- P1: `Dir::files()` が非再帰 → literal 実装は schemas/ を fingerprint
  から漏らす → `collect_files` 再帰で解消(0.7.4 source で実証済)。
- fingerprint 走査と extract 走査の drift → 同一 helper 共有で解消。
- P3: `embedded_version()` が `Option<String>` を返す点は既存の
  `ok_or_else` を維持する形に設計を修正済。
- 非回帰(既存挙動)の指摘 — `.ready` torn 書込、OnceLock poison、
  temp dir 累積、path sanitize — は scope 外として記録のみ。

## 検証

- `cargo test -p cheaper-cli --lib embedded` — 3 件 pass
  (fingerprint 決定性/16hex、nested schemas 8 file の列挙、
  contents 差分で fingerprint が変わること)。
- `cargo fmt`/`clippy` clean。
- `mise run check` 全段グリーン(biome/fmt/clippy/version/doc-id/
  `cargo test --workspace`/node tests/lockfile/apm-payload)。
- 実動作: debug binary で `--version` 実行 →
  `%TEMP%/cheaper-builtin-1.0.0-9a4930bdb12941dd/` が新規 materialize
  され、展開 content は新 schema/policy( magnitude 制約撤廃済み)に
  一致。旧命名 dir(`cheaper-builtin-1.0.0`)は設計どおり残留。
