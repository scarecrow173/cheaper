---
id: "IMPL-gY3xK9wV2mNbQ8rTzAhJdL5"
type: "impl"
status: "completed"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# IR: builtin materialize content fingerprint

## Summary

Issue #115。builtin resource の materialize cache が version のみで
key 付けされ、同一 version で embedded resource(schema/policy)が
変わっても stale dir が再利用される問題を、cache key へ
content fingerprint を加えることで解消した。materialize target は
`cheaper-builtin-{version}-{fingerprint}` となった。

## Context

worker schema 変更(#113/#114 対応)後も builtin dir が旧 schema を
保持し続ける不具合が #113/#114 実装中の Findings として発見され、
issue #115 として起票された。原因は materialize cache の key が
`{version}` のみで、resource contents を考慮しない設計にあった。

## Implementation

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

### review-protocol 対応

review-protocol (Reviewer A/B via bulk-read + Validator):

- P1: `Dir::files()` が非再帰 → literal 実装は schemas/ を fingerprint
  から漏らす → `collect_files` 再帰で解消(0.7.4 source で実証済)。
- fingerprint 走査と extract 走査の drift → 同一 helper 共有で解消。
- P3: `embedded_version()` が `Option<String>` を返す点は既存の
  `ok_or_else` を維持する形に設計を修正済。
- 非回帰(既存挙動)の指摘 — `.ready` torn 書込、OnceLock poison、
  temp dir 累積、path sanitize — は scope 外として記録のみ。

## Related Experiments

なし。

## Validation

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

## Risks

- review-protocol で記録された非回帰(既存挙動)の残件 —
  `.ready` torn 書込・OnceLock poison・temp dir 累積・path
  sanitize — は scope 外として記録のみで未対応。

## Follow-ups

なし(本 task の残件は記録されていない)。
