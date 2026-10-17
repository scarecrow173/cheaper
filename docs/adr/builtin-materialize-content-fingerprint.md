---
id: "ADR-KFmeF9E3oB3ManY01TWZVX"
type: "adr"
status: "accepted"
title: "materialize cache key を version+fingerprint にする"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "docs/discovery/builtin-materialize-cache-stale-on-same-version.md"
    - "https://github.com/scarecrow173/cheaper/issues/115"
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
  related:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/builtin-materialize-content-fingerprint.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/embedded.rs"
  defers: []
  deferred-by: []
---

# ADR: materialize cache key に content fingerprint を混入

## Context

`materialize_builtin_dir` の cache dir は `cheaper-builtin-<version>` と
version のみで key 付けされる。同一 version で embedded resource を
変更した新 binary は `.ready` 済みの旧 dir を再利用し、古い
schema/policy で動作する(#115、#111/#113 の検証中に顕在化)。

## Decision Outcome

fingerprint(embedded content の決定的 hash)を dir name に混入し、
`cheaper-builtin-<version>-<fingerprint>` とする。

- fingerprint 入力: `WORKERS_DIR` の全 file(path + contents, 順序は
  ソートして決定的化)+ `POLICY_YAML` + `VERSION_JSON`。
- hash: `std::collections::hash_map::DefaultHasher`(固定 seed で
  process 間決定的)。hex で dir 名に安全な文字列化。
- `.ready` marker と展開ロジックは変更しない — fingerprint 差異は
  「別 dir」として自然に再展開される。
- 旧命名の dir(`cheaper-builtin-<version>`)は放置 — temp 管理に委ねる。

## Considered Options

- **B: `.ready` に fingerprint を書き不一致で同 dir 再展開**:
  削除された file が dir に残留する partial-state を拭くため wipe または
  per-file reconcile が要り、複雑化する。dir 名差異にする案 A の方が
  「展開ロジック不変」で最小。
- **C: 起動毎に常時再展開**: IO 増。不要。
- **D: version.json に build timestamp/commit を埋め込み**:
  reproducible build を壊し、cargo install 時に未コミット差分を
  反映しない(resource は compile 時取り込みで version ではない)。
  fingerprint の方が直接正しい。

## Consequences

- 長所: staleness の構造的排除。展開ロジック不変。hook override 無影響。
- 短所: embedded 変更毎に temp dir が増える(従来の version bump と同程度)。
- 副作用: 同 content 同士は同 dir を共有 → `.ready` skip の効率は維持。
