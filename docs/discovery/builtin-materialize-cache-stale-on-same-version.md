---
id: "DISC-ar7NQWbsDe0TtIFAlAsUdv"
type: "discovery"
status: "draft"
title: "builtin materialize cache の stale 化(同一 version の embedded 変更)"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
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
  related: []
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

# builtin materialize cache の stale 化

## 背景

Issue #115。`materialize_builtin_dir`(`crates/cheaper-cli/src/embedded.rs`)
は embedded resource を `<temp>/cheaper-builtin-<version>/` へ展開し、
`.ready` marker があれば再展開しない。

## 問題

- cache key が **version のみ**(`embedded_version()` = build.rs 生成の
  `version.json` = `CARGO_PKG_VERSION`)。
- 同一 version で embedded resource(worker schema/property、
  `policy.yaml`)を変えた新 binary は `.ready` 済みの旧 dir をそのまま
  再利用する → 旧 schema/旧 policy で動作する。
- 顕在化例:
  - `policy.yaml` `execution.maxTurns: 8 → null`(#111 系)が cache 残存で
    反映されず reviewer worker が旧 cap で落ちた。
  - #113 の「以前は task 13k 文字が受理された」報告も旧 schema cache の
    可能性がある。
- release 運用では version bump で別 dir になるため実害は dev/HEAD
  install に集中するが、dev install は主要な利用形態の一つ。

## 設計の軸

- fingerprint = embedded content の決定的 hash(file path + contents 全体)。
- cache dir を `cheaper-builtin-<version>-<fingerprint>` にする案(A)は
  再展開ロジックを変えずに staleness を構造的に排除できる。
- `.ready` へ fingerprint を書いて不一致時に再展開する案(B)は dir を
  再利用するが、削除された file が残る partial-state を拭く必要がある。
- fingerprint 計算は起動1回・embedded file は小規模(数十ファイル)なので
  コストは無視できる。`DefaultHasher`(固定 seed)で十分 — 衝突耐性では
  なく差分検知が目的。
- `CliHooks.builtin_dir` override 経路は本 cache を通らない(対象外)。
