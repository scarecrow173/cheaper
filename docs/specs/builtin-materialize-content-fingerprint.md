---
id: "SPEC-Ihed47KqbjDSmZt2jp7L7M"
type: "spec"
status: "approved"
title: "builtin materialize cache に embedded content fingerprint を混入"
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
    - "docs/adr/builtin-materialize-content-fingerprint.md"
    - "docs/designs/builtin-materialize-content-fingerprint.md"
  refines:
    - "docs/discovery/builtin-materialize-cache-stale-on-same-version.md"
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

# builtin materialize cache への content fingerprint 混入

## 要件

- R1: embedded resource(worker dir、`policy.yaml`、`version.json`)の
  内容から決定的な fingerprint を起動時に計算する。
- R2: materialize cache dir の key を `version` 単独から
  `version + fingerprint` に変更し、同一 version で embedded content が
  異なれば別 dir へ展開する。
- R3: `.ready` marker による「展開済み skip」は dir 単位で維持する
  (fingerprint が違えば別 dir になるため再展開ロジック自体は不変)。
- R4: `CliHooks.builtin_dir` 指定時の override 経路は変更しない。
- R5: fingerprint は差分検知用であり暗号学的強度は要求しない。
  決定的(同一 content → 同一値)であることのみ必須。

## 非要件

- cache dir の prune/gc(従来どおり OS の temp 管理に委ねる)。
- `extract_builtin` 自体の atomicity 改善(`.ready` による現行挙動を維持)。

## Acceptance Criteria

- AC1: 同一 version で embedded schema/policy を変更した binary から
  `builtin_dir()` を呼ぶと、変更前とは異なる dir が返る。
- AC2: 同一 binary からの連続呼出は同一 dir を返し、2回目は再展開しない
  (`.ready` skip が維持される)。
- AC3: 既存の `CliHooks.builtin_dir` override を使う caller の挙動は不変。
