---
id: "TSPEC-034EDnJWaK5A4nMhDKBedT"
type: "test-spec"
status: "approved"
title: "Post-feature structural refactoring behavior parity"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source: []
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
  derives-from:
    - "../designs/post-feature-structural-refactoring.md"
  derived-by: []
  verifies:
    - "../specs/post-feature-structural-refactoring-scope.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-feature structural refactoring behavior parity

## Purpose

構造整理（Phase 0/1）が観測可能挙動を変えないことを保証する parity 契約。
`cheaper` の CLI surface が廃止・変更される際のみ退役を検討する。

## Feature

behavior-preserving structural refactoring（crate 内 module 再編 + 共有
helper 集約）における public contract parity。

## Rules

- Rule: envelope の shape・key 順・`exit code`・`error.code`・`error.message` は
  変更しない。`workflow run --timeout` の details 欠落修正のみ例外として
  run.rs と同一の details を emit する。
- Rule: usage.jsonl の記録対象・行 format・savings/turns/toolCalls 集計は
  変更しない。
- Rule: code-unit sort・UTF-16 parity・abort の first-reason・drain の
  settle/unsettle 分類・process の cleanup 順序は変更しない。
- Rule: worker schema・tool manifest・policy.yaml の受理 contract は変更しない。
- Rule: `pub` API の module path 変更は external（他 crate/test）参照を壊さない。
  re-export で後方互換を保つか、caller を同一変更で更新する。

## Examples

- Example: `cheaper run`・`bulk-read`・`code-write`・`doc-write`・
  `count-tokens`・`workflow run` の stdin JSON → stdout envelope が既存
  envelope parity test・regression test で検出されない差分を持たない。
- Example: `cheaper-tools` の differential fixture replay が 10 fixture 全て
  expectRust 一致のまま緑である。
- Example: `--timeout` invalid 値で `cheaper run` と `cheaper workflow run` が
  同一 code + 同一 details 構造の error envelope を返す。

## Guarantees

- `mise run check`（biome・cargo fmt --check・clippy・version/doc-id・
  cargo test --workspace・node --test・lockfile・apm payload）が pass。
- tools の characterization test が process/grep/read の構造分割に先行して
  追加され、lease の親子 counter・observe の support 認可・grep の stop
  reason が pin されている。

## Non-goals

- Phase 2 候補（boundary adapter・args.rs table・ToolError enum 等）の
  挙動検証 — 本 test-spec の対象外。
- 新規機能・API 追加・error message の改善。

## Risk

parity を pin する防壁（differential fixture・envelope parity・
characterization test）が無いまま構造変更すると、error shape・abort 意味論・
sort 順の silent な drift を検出できない。
