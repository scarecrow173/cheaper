---
id: "TSPEC-034EeitmhHjX9tjAWLQ7gk"
type: "test-spec"
status: "draft"
title: "Setup-boundary abort classification contract"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/abort-classification-at-setup-boundary.md"
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
    - "docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/abort-classification-at-setup-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/abort-classification-at-setup-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Setup-boundary abort classification contract

## Purpose

issue #94 の fix(setup boundary での記録済み abort code 復元 +
code→`ErrorCode` 逆引きの cheaper-core 集約)が
SPEC-034EdS8ly2SqcS7cJ8hvfJ の acceptance criteria を満たすことを
固定する意図を記録する。

本契約の核心は 2 つの不変条件である: (1) `RunAbortReason` が生成する
code 保持 payload が、到達区間(stream vs setup)に依らず同じ
`ErrorCode` 分類へ復元される分類対称性。(2) code→`ErrorCode` 逆引きが
`ERROR_TABLE`/`as_str` を唯一の語彙源とする 1 箇所の authoritative
実装にのみ存在し、語彙外文字列は `None` を返す寛容な parse である
こと。

abort 分類の transport(`AbortReason.code()` の describeError 語彙)や
`ERROR_TABLE` 語彙自体が変更された場合、あるいは逆引きの責務配置が
再検討された場合にのみ退役する。

## Feature

`cheaper-core` の `ErrorCode::from_str` 逆引き、`cheaper-runtime` の
`setup_abort_error` における `error_object()` downcast → `code()`
復元 → `INTERNAL_ERROR` fallback の優先順序、`cheaper-rig-adapter`
の `abort_reason_to_cheaper` による同一共有 API の使用、および
`hang_grace` fixture(`tests/fixtures/10-hang-grace.json`、
`required-features = ["dev-seam"]` の `run_fixture_test` 経由)の
deterministic な pass。

## Rules

- Rule: reverse-lookup round-trip — `ErrorCode::from_str` は
  `ERROR_TABLE` が列挙する全変種について `from_str(code.as_str())
  == Some(code)` を満たす。逆引きの語彙源は `ERROR_TABLE` ×
  `as_str()` 照合のみで、第二の match table は存在しない
  (core unit test で固定)。
- Rule: lenient parse — `ErrorCode` 語彙外の文字列(Node errno
  `"ENOENT"`・`"ECONNRESET"`・generic payload code・空文字列)は
  `None` を返す。照合は完全一致・case-sensitive であり、小文字化
  した `"aborted"` 等も `None`。
- Rule: setup-boundary restore — `RunAbortReason::Timeout`/
  `Signal`/`MaxTurns`/`MaxToolCalls` の `abort_payload` 経路
  (`RunAbort::request_abort` → `signal().reason()`)が
  `setup_abort_error` でそれぞれ `TIMEOUT`/`ABORTED`/
  `MAX_TURNS_EXCEEDED`/`MAX_TOOL_CALLS_EXCEEDED` に分類される。
  復元された `CheaperError` の message は `"{context} failed:
  {reason.message()}"` 形を維持する。
- Rule: priority order — `error_object()` が `CheaperError` へ
  downcast できる `AbortReason` は従来どおり code + message +
  details の rebuild を返し、`code()` 復元分岐より優先される
  (既存経路の挙動不変)。
- Rule: residual fallback — `code()` が無い、または語彙外の
  `AbortReason` は `INTERNAL_ERROR` + `"{context} failed:
  {message}"` の形を維持する。
- Rule: single mapping site — adapter の private table
  `error_code_from_str` は除去され、`abort_reason_to_cheaper` は
  `reason.code().and_then(ErrorCode::from_str)` で同じ core API を
  使う。code→`ErrorCode` 写像は cheaper-core 内の 1 箇所のみに
  存在する。
- Rule: deterministic regression — `hang_grace` fixture は
  `cargo test --workspace --features cheaper-runtime/dev-seam,
  cheaper-cli/dev-seam` で `ABORTED`(exit code 8)を deterministic
  に返す。複数回(目安 3 回)の workspace test run で再現しない
  ことを確認する。
- Rule: invariants — envelope schema(`schemaVersion`)・exit code・
  `ERROR_TABLE` の内容・`cheaper-core` の Rig/Tokio-free・新規
  dependency ゼロは不変。

## Examples

- Example: `crates/cheaper-core` の unit test が `ERROR_TABLE` の全
  25 変種で `from_str(as_str())` round-trip を検査し、
  `ErrorCode::from_str("ENOENT")`・`ErrorCode::from_str("aborted")`
  (小文字)・`ErrorCode::from_str("")` が `None` を返す。
- Example: `RunAbort` に `request_abort(RunAbortReason::Signal)` し、
  `signal().reason()` で得た `AbortReason`(`name="CheaperError"`・
  `code="ABORTED"`・error object 無しの `abort_payload` 形)を
  `setup_abort_error("tool scope ready", ...)` に渡すと
  `ErrorCode::Aborted` で message `"tool scope ready failed:
  worker run aborted: signal"` が返る。同様に Timeout→
  `Timeout`・MaxTurns→`MaxTurnsExceeded`・MaxToolCalls→
  `MaxToolCallsExceeded`。
- Example: `AbortReason::from_error` で `CheaperError`(code・
  details 保持)を包んだ reason は `code()` 復元分岐に到達せず、
  downcast rebuild(既存挙動)が返る。
- Example: `AbortReason::new("Error", "io failed",
  Some("ENOENT"))`(語彙外 code)や `AbortReason::msg("x")`(code
  無し)は `INTERNAL_ERROR` + `"tool scope ready failed: ..."` を
  維持する。
- Example: `crates/cheaper-rig-adapter` で `error_code_from_str`
  を grep して 0 件 — private table が除去されている。
- Example: `cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` を連続実行し、
  `fixtures_replay_all` 内の `hang_grace` が毎回 `ABORTED`/`exitCode
  8` で pass する(INTERNAL_ERROR flake が再現しない)。

## Guarantees

- setup boundary(`scope.ready()` 等)に着弾した abort が envelope
  `error.code` に記録済み分類を返し、stream 経路と一致する —
  到達区間による分類非対称が解消される。
- `ErrorCode` 変種追加時に逆引きの更新忘れが構造的に起きない
  (`ERROR_TABLE`・`as_str` を変更すれば round-trip test が追随を
  強制する)。
- 語彙外 code(Node errno 等)を運ぶ `AbortReason`・code 無し
  `AbortReason`・`error_object` 保持 `AbortReason` は全て現行
  挙動のまま — 後段追加のみで回帰面を持たない。
- `cheaper-core` に Rig/Tokio・新規 dependency は流入しない
  (追加は純粋関数のみ)。

## Non-goals

- code-less `AbortReason`(DOMException 型 payload 等)の setup
  boundary fallback 分類 — 現行 `INTERNAL_ERROR` を維持する範囲の
  まま検証し、`ABORTED` 寄りにすべきかの判断は deferred(spec の
  Deferred Design Concerns)。
- adapter 経路の message 形(`"worker run aborted: {message}"`)と
  setup 経路(`"{context} failed: {message}"`)の一致 — 一致する
  invariant は `error.code` のみで message 一致は要件外。
- `abort_reason_to_cheaper` の `ABORTED` fallback 契約 — 共有化の
  対象は写像 table のみで変換関数の契約は検証しない(既存 test が
  cover)。
- abort semantics 自体(reason 先勝ち記録・abort grace・signal
  配線)— `first_reason_wins`・`abort_error_codes_and_details` 等の
  既存 test の領域。
- `hang_grace` 以外の flake・stream 側 timing — 本 spec の
  regression 保証は `hang_grace` の deterministic pass に限る。
- 実行結果の記録 — pass/fail・測定回数は impl-doc experiment log
  の領域。

## Risk

- 逆引きが第二の table として再び複製されると、`ErrorCode` 変種
  追加のたびに分類非対称が再発する — 本 bug の原因構造そのものの
  回帰。
- `setup_abort_error` の復元分岐が pin されないと、将来の
  `AbortReason` payload 形変更(例: `abort_payload` の code 削除)で
  setup 分類が silent に `INTERNAL_ERROR` へ退化し、`hang_grace`
  flake が復活しても原因特定が難しい。
- lenient parse が壊れる(`None` でなく panic・大文字小文字の曖昧
  一致)と、`describeError` が運ぶ非 CheaperError code を誤分類し
  envelope `error.code` の語彙外への拡散が起きる。
- determinism 検証が欠けると `mise run check` の信頼性が毀損された
  まま残り、issue #94 の症状が latent に存続する。
