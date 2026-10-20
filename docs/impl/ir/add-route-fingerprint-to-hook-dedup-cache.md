---
id: "IMPL-034Jjg7aEBfEFtFuSpSEi3"
type: "impl"
status: "completed"
title: "Add route fingerprint to hook dedup cache"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "docs/tasks/add-route-dedup-fingerprint.md"
    - "docs/test-specs/hook-dedup-route-fingerprint.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-dedup-fingerprint.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
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

# Add route fingerprint to hook dedup cache

## Summary

shell verdict dedup cache の fingerprint 入力へ effective routes(builtin /
user / project 3 層の `routes.yaml`)の stat signature を追加し、route 宣言の
変更で stale shell verdict が serve されないことを構造的に保証した。併せて
`is_shareable` gate を publish・serve 両側へ設置し、`Verdict::Rewrite`
(C2 で導入予定)が cache へ永続化・共有されない境界を先行して確立した。

## Context

- implements: `docs/tasks/add-route-dedup-fingerprint.md`
- derives-from: `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`(§dedup — route fingerprint / Rewrite 非 persist)
- test spec: `docs/test-specs/hook-dedup-route-fingerprint.md`

## Implementation

### Changes

- `FingerprintInput.routes_sig: Vec<PolicySig>` を追加。`prepare_dedup` で
  Shell variant のみ 3 層 `routes.yaml` の `PolicySig`(policy file と同一の
  mtime+len stat signature / `Embedded` marker / `Absent` / `Unresolved`)を
  計算して hash 入力へ載せる。層 path 解決は `route::routes::load` と同一
  規約(`builtin_dir/routes.yaml`・`<userConfigDir>/routes.yaml`・
  `<root>/.cheaper/routes.yaml`)。
- 非 Shell variant では `routes_sig` は空 vec のまま — File/Search eval は
  route 非入力のため、routes.yaml の変更で fingerprint を不必要に割らない。
- `is_shareable(verdict)` を追加し、`verdict_record_body`(publish 側)で
  非 shareable verdict の record 生成を拒否、`read_fresh_verdict`(serve
  側)でも防御的に非 shareable record を absent 扱いとした。allowlist 形の
  exhaustive match にしたため、`Verdict::Rewrite` 追加時に compile error
  で明示判断が強制される。

### Decisions During Implementation

- routes signature は route 宣言内容の hash ではなく policy file と同じ
  stat signature(mtime+len)を採用 — dedup fingerprint 全体の確立済み
  規約に合わせ、file 内容を read せず invalidation だけを実現する。
- `Verdict::Rewrite` は C2 で導入されるため、本 task では gate 機構のみ
  確立した。exhaustive match により C2 の variant 追加時に shareable
  判定を忘れる余地がない。

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `cargo check -p cheaper-cli`: pass
- `cargo test -p cheaper-cli --lib eval_cache`: 10 pass(新規 3 件 —
  `fingerprint_includes_routes_signature` /
  `prepare_dedup_scopes_routes_sig_to_shell_variant` /
  `verdict_record_body_only_serializes_shareable_verdicts`)
- `cargo fmt -p cheaper-cli` / `cargo clippy -p cheaper-cli --all-targets`:
  clean

## Risks

- routes.yaml の内容を変えず mtime のみ変わる rewrite で fingerprint は
  変わるが、方向は「余計に再評価する」側のみで stale serve にはならない
  (safe direction)。
- `Verdict::Rewrite` の実際の非 persist 検証は C2 で variant が存在して
  から行われる。gate 自体は本 task の unit test で構造確認済み。

## Follow-ups

- C2(`add-hook-route-rewrite`)で `Verdict::Rewrite` 追加時、
  `is_shareable` の match が exhaustive でないため compile error になる
  — `Rewrite => false` を明示して解消すること。
