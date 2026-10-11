---
id: "TASK-034EekRVLtDbzc0slwVFX0"
type: "task"
status: "todo"
title: "T1: cheaper-core ErrorCode::from_str reverse lookup"
created: "2026-10-11"
updated: "2026-10-11"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
  blocks:
    - "docs/tasks/restore-setup-boundary-abort-classification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/abort-classification-at-setup-boundary.md"
  references: []
  defers: []
  deferred-by: []
---

# T1: cheaper-core ErrorCode::from_str reverse lookup

## Work

plan `docs/plans/implement-abort-classification-at-setup-boundary.md`
(PLAN-034Eek9Q2fFlfb2FzIBUtz)の T1。`cheaper-core` の `ErrorCode` に
code 文字列からの逆引き API `pub fn from_str(s: &str) ->
Option<Self>` を `as_str` 隣に追加する。語彙の単一 source として
`ERROR_TABLE` entry 列挙 + `as_str()` 完全一致照合のみで実装し、
第二の match table を作らない。`std::str::FromStr` trait ではなく
inherent method(戻り値 `Option` — design の確定形状)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-abort-classification-at-setup-boundary.md` —
  本 task が implement する plan。File Map「T1」が作業の正
- `docs/designs/abort-classification-at-setup-boundary.md`
  (DESIGN-034EehnT1xOPAee9dK22Kx)— API 形状・語彙単一 source・
  case-sensitive 完全一致の AUTHORITATIVE 定義(実装疑似コードあり)
- `docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md`
  (ADR-034EdSlppBqVgIegIeGXgH)— 配置決定(option A)と寛容
  `Option` parse の根拠

## Dependencies

- 前提 task なし。`docs/tasks/restore-setup-boundary-abort-classification.md`
  (T2)を block する — T2 の両 consumer が本 API を呼ぶ

## Affected Files

- `crates/cheaper-core/src/errors.rs`:
  - `impl ErrorCode` に `pub fn from_str(s: &str) -> Option<Self>`
    を `as_str` の直後に追加。実装は
    `ERROR_TABLE.iter().map(|(code, _)| *code).find(|code|
    code.as_str() == s)` のみ — 語彙外文字列は `None`
  - `#[cfg(test)] mod tests` に unit test を追加:
    - `ERROR_TABLE` 全変種で `ErrorCode::from_str(code.as_str())
      == Some(*code)` の round-trip
    - 語彙外文字列(`"ENOENT"`・`"ECONNRESET"` 等の Node errno 形・
      空文字列)→ `None`
    - 小文字化した `"aborted"` → `None`(case-sensitive 完全一致
      の pin)

## Done When

- [ ] `ErrorCode::from_str` が `Option<Self>` を返す inherent
      method として `as_str` 隣に存在し、`std::str::FromStr`
      impl・第二の match table が存在しない
- [ ] `ERROR_TABLE` 全変種の round-trip test・語彙外/小文字/
      空文字列 → `None` の test が追加され pass する
- [ ] `cheaper-core` に新規 dependency がなく Rig/Tokio を
      引き込んでいない(変更は純粋関数のみ)

## Out of Scope

- `setup_abort_error`・adapter 側の consumer 配線 — T2 の範囲
- `ErrorCode` 変種・`ERROR_TABLE` 内容の変更 — 不変条件

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` exits 0
- [ ] `mise exec -- cargo build -p cheaper-core` exits 0
- [ ] `grep -n "impl FromStr for ErrorCode\|impl std::str::FromStr"
      crates/cheaper-core/src/errors.rs` が 0 件(trait 非採用の確認)
