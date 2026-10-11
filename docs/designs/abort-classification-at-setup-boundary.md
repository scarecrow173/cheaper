---
id: "DESIGN-034EehnT1xOPAee9dK22Kx"
type: "design"
status: "approved"
title: "Setup-boundary abort classification via core ErrorCode reverse lookup"
created: "2026-10-11"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/specs/abort-classification-at-setup-boundary.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/abort-classification-at-setup-boundary.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
    - "docs/designs/cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/abort-classification-at-setup-boundary.md"
    - "docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Setup-boundary abort classification via core ErrorCode reverse lookup

## Context

SPEC-034EdS8ly2SqcS7cJ8hvfJ / ADR-034EdSlppBqVgIegIeGXgH(option A)の
実装設計。issue #94(`hang_grace` flake)の fix は ~50 行規模で、本
design が確定するのは ADR が実装時へ留保した 2 点:

- `ErrorCode` 逆引き API の正確な形状(inherent method vs `FromStr`
  trait、戻り値型)
- `setup_abort_error` の `code()` 復元分岐が構成する `CheaperError`
  の message 形

配置の決定(cheaper-core・語彙単一 source・寛容な `Option` parse)は
ADR で確定済みであり、本 design はそれを実装可能な形に写す。

## Scope

### In Scope

- `crates/cheaper-core/src/errors.rs` — `ErrorCode` への逆引き API
  追加(`as_str` 隣)と round-trip unit test
- `crates/cheaper-runtime/src/run/mod.rs` — `setup_abort_error` への
  `code()` 復元分岐追加と分類 pin unit test
- `crates/cheaper-rig-adapter/src/error.rs` — private table
  `error_code_from_str` の削除と core API への切替

### Out of Scope

- abort semantics 自体(reason 先勝ち記録・abort grace・signal 配線・
  `abort_payload` の形)の変更
- `code()` を持たない `AbortReason` の setup boundary fallback 見直し
  (現行 `INTERNAL_ERROR` を維持 — spec の Deferred Design Concerns)
- retry policy・`ERROR_TABLE`(exit code/retryable)・envelope
  schema・exit code 表の変更
- `hang_grace` 以外の flake 調査

## Components and Boundaries

### `ErrorCode::from_str`(cheaper-core、`errors.rs`)

`as_str` の直後に inherent method として追加:

```rust
impl ErrorCode {
    /// `as_str` の逆引き。ErrorCode 語彙外の文字列(Node errno
    /// `"ENOENT"` 等、`AbortReason.code()` が運ぶ describeError
    /// code)は `None` を返す寛容な parse。
    // inherent `from_str` は `clippy::should_implement_trait` に抵触するが、
    // ADR が `and_then(ErrorCode::from_str)` 合成のため名前を確定したので局所 allow。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        ERROR_TABLE
            .iter()
            .map(|(code, _)| *code)
            .find(|code| code.as_str() == s)
    }
}
```

形状の決定:

- **inherent `from_str` で `Option<Self>` を返す**(`std::str::FromStr`
  trait ではない)。ADR の call site 合成 `reason.code().and_then(
  ErrorCode::from_str)` が要求するのは `&str -> Option` であり、
  `FromStr` trait は `Result<Self, Self::Err>` を要求するため
  `Err` 型の設計論点(意味を持たない unit error)を持ち込む。
  YAGNI — trait 適合が必要な consumer は現れていない。
  ただし inherent `from_str` は `clippy::should_implement_trait`
  に抵触し(`mise run check` の `cargo clippy` は `-D warnings`
  付き)、そのままでは lint error となる。ADR が call site 合成
  `reason.code().and_then(ErrorCode::from_str)` のために名前を
  確定しているため、`#[allow(clippy::should_implement_trait)]`
  を付けて lint を局所的に許容する。
- **語彙の単一 source**: 実装は `ERROR_TABLE` の entry 列挙 +
  `as_str()` 照合のみで、第二の match table を作らない。
  `ErrorCode` 変種追加時は `as_str` + `ERROR_TABLE` の更新で
  逆引きが自動的に追随する(既存 test
  `error_table_covers_every_expected_code_exactly_once`・
  `serde_name_matches_as_str` が両 table の整合を既に保証)。
- 照合は **完全一致・case-sensitive**。`AbortReason.code()` が
  CheaperError 以外の code(Node errno・generic payload)も運ぶ
  transport であるため、厳密一致で `None` を返すのが安全側。

### `setup_abort_error`(cheaper-runtime、`run/mod.rs`)

優先順序は `error_object()` downcast → `code()` 復元 →
`INTERNAL_ERROR` fallback の後段追加。既存経路の挙動は変えない:

```rust
fn setup_abort_error(context: &str, reason: AbortReason) -> CheaperError {
    if let Some(obj) = reason.error_object() {
        if let Some(e) = obj.downcast_ref::<CheaperError>() {
            // ...既存どおり code + message + details を rebuild して返す
        }
    }
    if let Some(code) = reason.code().and_then(ErrorCode::from_str) {
        return CheaperError::new(
            code,
            format!("{context} failed: {}", reason.message()),
        );
    }
    CheaperError::new(
        ErrorCode::InternalError,
        format!("{context} failed: {}", reason.message()),
    )
}
```

- `code()` 復元で構成する `CheaperError` の message は fallback と同じ
  `"{context} failed: {reason.message()}"` 形を維持する(ADR の確定
  ルール)。adapter 経路(`abort_reason_to_cheaper` が
  `reason.message()` をそのまま使う)とは message 形が異なるが、
  setup boundary は call site(`scope.ready()` 等)の context を
  message に含める契約であり、ここでは「失敗した区間」の情報を落とさ
  ない方を取る。envelope `error.code` だけが stream 経路と一致する
  invariant であり、message 一致は要件外。
- `details` は付けない — `reason` の describeError 形には details
  が無く、復元できるのは code のみ。`abort_payload` 由来の reason
  文字列("timeout"/"signal"/...)は message に残る。
- `code()` 復元分岐の到達範囲は spec の文言より意図的に広い:
  `error_object` は存在するが `CheaperError` ではない `AbortReason`
  (downcast miss)も本分岐を通る。ただしその場合 `code()` は
  describeError 語彙外(Node errno 等)で `from_str` は `None` を
  返すため、到達可能な挙動差は無い(spec 自身の分析どおり) —
  adapter 経路と同じ規則を共有する parity でもある。

### adapter(cheaper-rig-adapter、`error.rs`)

- `abort_reason_to_cheaper` 内の `reason.code().and_then(
  error_code_from_str)` を `reason.code().and_then(
  ErrorCode::from_str)` に置き換える。
- private fn `error_code_from_str`(error.rs:216-245)と直上の
  「cheaper-core に `FromStr` は無いので adapter 側で閉じて持つ」
  コメントを削除 — 写像の二重管理(本 bug の直接原因)を解消する。
- 変換関数自体の契約(`ABORTED` fallback・`"worker run aborted:
  {message}"` 形)は不変 — 共有するのは写像 table のみ。

## Data and Control Flow

```text
RunAbort::request_abort(RunAbortReason)
  -> abort_payload(): AbortReason{name:"CheaperError", message, code:"TIMEOUT"等}
  -> signal.fire -> scope.ready() が AbortReason で reject
  -> setup_abort_error("tool scope ready", reason)
       error_object()? -> downcast CheaperError -> rebuild(既存)
       code() -> ErrorCode::from_str -> Some(code) -> 復元(新規)
       それ以外 -> INTERNAL_ERROR fallback(既存)
  -> envelope.error.code = TIMEOUT/ABORTED/MAX_TURNS_EXCEEDED/
     MAX_TOOL_CALLS_EXCEEDED (stream 経路と一致)
```

## Non-Functional Constraints

- `cheaper-core` は Rig/Tokio-free を維持 — 追加は純粋な
  `&str -> Option` 関数のみで新規 dependency なし
- 依存方向 `runtime → {rig-adapter, tools} → core` を壊さない —
  両 consumer は既に `cheaper-core` に依存しており edge 追加なし
- envelope schema(`schemaVersion`)・exit code・`ERROR_TABLE` の
  内容は不変
- 逆引きは O(語彙数=25) の線形 scan — run 当たり最大 1 回の error
  path で性能無視可能。辞書構造の導入は YAGNI

## Risks and Trade-offs

- `AbortReason.code()` が ErrorCode 語彙の文字列を偶然名乗る非
  CheaperError payload(将来の producer)が誤分類される理論上の余地 —
  現時点で語彙 code を生成するのは describeError と
  `abort_payload` のみであり、spec がこの規則を安全と確認済み。
  name 照合(`name() == "CheaperError"`)を要件に加えないのも adapter
  経路との parity(同じ規則)
- `INTERNAL_ERROR` residual が code 復元の後段に残るため、語彙外
  code・code 無しの reason は現行挙動のまま — 後段追加のみで回帰
  面なし

## Deferred Design Concerns

- code-less `AbortReason`(DOMException 型 payload 等)の setup
  boundary fallback を `ABORTED` 寄りにすべきか | 本修正で観測された
  症状(code 保持 payload)の外、実害未観測 | 実害が観測された時点
  | 稀な code-less abort が `INTERNAL_ERROR` として残留する可能性
  (spec の Deferred Design Concerns と同一)

## Verification Notes

- [ ] `ErrorCode` 全変種で `from_str(as_str()) == Some(self)` の
      round-trip + 未知文字列・小文字化(`"aborted"`)→ `None` の
      unit test(core)
- [ ] `RunAbortReason` 全種の `abort_payload` 経路(
      `RunAbort::request_abort` → `signal().reason()`)が
      `setup_abort_error` で正しい `ErrorCode` へ復元される unit
      test(runtime)
- [ ] `error_object` 保持 `AbortReason` の分類不変・語彙外 code・
      code 無しが `INTERNAL_ERROR` + `"{context} failed: {message}"`
      を維持する pin test(runtime)
- [ ] adapter private table 除去と共有 API 使用の確認
- [ ] `hang_grace` fixture が dev-seam 付き workspace test run で
      deterministic に pass
