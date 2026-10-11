---
id: "IMPL-034Efz8DzzN46lcybHvrfu"
type: "impl"
status: "completed"
title: "Abort classification at setup boundary"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/errors.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/error.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/fixtures/08-signal-abort.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-errorcode-from-str-reverse-lookup-in-cheaper-core.md"
    - "docs/tasks/restore-setup-boundary-abort-classification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-abort-classification-at-setup-boundary.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/abort-classification-at-setup-boundary.md"
    - "docs/designs/abort-classification-at-setup-boundary.md"
    - "docs/test-specs/abort-classification-at-setup-boundary.md"
    - "docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md"
  refines: []
  refined-by: []
  derives-from: []
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

# Abort classification at setup boundary

## Summary

issue #94。run pipeline の setup boundary(`scope.ready()` 区間)で
`AbortReason` が throw されたとき `INTERNAL_ERROR` へ落ちていた分類
bug を修正し、code→`ErrorCode` 逆引き写像を cheaper-core の
`ErrorCode::from_str` へ集約した。`setup_abort_error` に
`error_object()` downcast → `code()` 復元 → `INTERNAL_ERROR`
fallback の分岐順で復元経路を追加し、adapter の private table
`error_code_from_str` を共有 API への委譲へ置き換えた。あわせて
fixture `08-signal-abort` の `abort.after_ms` を 40 → 120 へ
引き上げ、JSON replay harness の setup 区間への abort 着弾 race を
除去した。

## Context

plan `docs/plans/implement-abort-classification-at-setup-boundary.md`
(PLAN-034Eek9Q2fFlfb2FzIBUtz)の T1–T2。根本原因は分類非対称:
prompt/stream 経路は adapter の `abort_reason_to_cheaper` が
`reason.code()` を逆引きして記録済み分類 code を復元するのに対し、
setup 経路の `setup_abort_error` は `error_object()` downcast だけを
見ていた。`RunAbortReason::abort_payload` が生成する payload は code
を保持するが error object を持たないため downcast は `None` となり、
`hang_grace` fixture が期待する `ABORTED` ではなく `INTERNAL_ERROR`
(`"tool scope ready failed: worker run aborted: signal"`)へ
fallback していた。spec `docs/specs/abort-classification-at-setup-boundary.md`
(SPEC-034EdS8ly2SqcS7cJ8hvfJ)、design
`docs/designs/abort-classification-at-setup-boundary.md`
(DESIGN-034EehnT1xOPAee9dK22Kx)、ADR
`docs/adr/centralize-the-abort-code-to-errorcode-reverse-lookup-in-cheaper-core.md`
(ADR-034EdSlppBqVgIegIeGXgH)、test-spec
`docs/test-specs/abort-classification-at-setup-boundary.md` が正。

## Implementation

### Changes

- `crates/cheaper-core/src/errors.rs`(T1)— `impl ErrorCode` に
  `pub fn from_str(s: &str) -> Option<Self>` を `as_str` 直後に追加
  (errors.rs:114-125)。実装は `ERROR_TABLE` entry 列挙 +
  `as_str()` 完全一致照合のみ — 語彙の単一 source、第二の match
  table 無し、語彙外文字列は `None` を返す寛容な parse。
  `std::str::FromStr` trait ではなく inherent method(戻り値
  `Option`)、`#[allow(clippy::should_implement_trait)]` と 1 行の
  根拠 comment(ADR が `and_then(ErrorCode::from_str)` 合成のため
  名前を確定)を付与。unit test 2 件(`ERROR_TABLE` 全 entry の
  round-trip、語彙外・小文字・空文字列・前後空白 → `None`)。
- `crates/cheaper-runtime/src/run/mod.rs`(T2)— `setup_abort_error`
  (mod.rs:941-961)に `error_object()` downcast の後・
  `INTERNAL_ERROR` residual の前へ
  `reason.code().and_then(ErrorCode::from_str)` 復元分岐を追加。
  message は fallback と同じ `"{context} failed: {reason.message()}"`
  形を維持し details は付けない。unit test 3 件(全 `RunAbortReason`
  の分類 pin、`error_object` rebuild 優先、語彙外 code・code 無し・
  非 CheaperError error object の `INTERNAL_ERROR` residual)。
- `crates/cheaper-rig-adapter/src/error.rs`(T2)—
  `abort_reason_to_cheaper` の `and_then(error_code_from_str)` を
  `and_then(ErrorCode::from_str)` に置換し、private fn
  `error_code_from_str` と直上 comment を削除。`ABORTED` fallback・
  message 形は不変。
- `crates/cheaper-runtime/tests/fixtures/08-signal-abort.json` —
  `abort.after_ms` 40 → 120。`after_ms: 40` は JSON replay harness
  の重い setup と race し、負荷時に abort が `scope.ready()` 区間へ
  着弾して post-setup 着弾前提の `errorDetails` assert と衝突していた
  (詳細は Decisions During Implementation)。

### Commits

- `95ec17a` feat(core): add ErrorCode::from_str reverse lookup for abort code restore
- `1113d6e` fix(runtime): restore abort code at setup boundaries via ErrorCode::from_str
- `8515c76` refactor(adapter): use core ErrorCode::from_str in abort_reason_to_cheaper
- `fb145df` test(runtime): widen signal_abort fixture abort delay past setup window

### Decisions During Implementation

- **分岐順 `error_object()` → `code()` → `INTERNAL_ERROR`**:
  `error_object` 保持 `AbortReason`(`AbortReason::from_error` 経由)
  は code/message/details を保持したまま rebuild で返す既存経路を
  最優先で維持し、復元分岐はその後段に追加。既存経路の挙動は不変。
- **message 形・details 非付与**: 復元分岐も fallback と同じ
  `"{context} failed: {message}"` prefix を維持し details は付けない
  (design 確定)。setup 着弾 abort は post-setup 着弾
  (`classified()` 経由の `details.reason` + streamed meta)と異なる
  envelope を返し得る — fixture 08 の hardening が必要だった直接因。
- **fixture 08 `after_ms` 40 → 120 を in-scope-adjacent として含有**:
  同一 race(setup 区間への abort 着弾)は修正前から存在し、修正で
  失敗 signature が `errorCode` 不一致から `errorDetails` 不一致へ
  移っただけ。code fix だけでは setup-landing abort の details/meta
  assert を満たせないため、fixture の意図(post-setup 着弾時の
  details/meta 契約)を deterministic に検証するよう timing を
  引き上げた。根拠は fixture 09(`partial-mutation`)の 120ms・CLI
  regression case(800ms/500ms)の先例。review で in-scope-adjacent
  と判断済み。
- **`hang_grace` fixture は変更不要**: expect が landing-agnostic
  (code のみ)のため、setup 着弾でも prompt 着弾でも `ABORTED` /
  exit 8 へ収束するのが本 fix の効果。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo build -p cheaper-core` / `cargo build --workspace`
  — PASS。
- `mise exec -- cargo test -p cheaper-core` — PASS(189 lib + 22
  integration、新規 2 test 含む)。
- `mise exec -- cargo test -p cheaper-runtime -p cheaper-rig-adapter`
  — PASS(新規 unit test 3 件・`run::tests` 4 件含む)。
- `mise exec -- cargo test --workspace --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — 連続 3 回全て
  exit 0。`fixtures_replay_all`(`hang_grace` 含む 24 fixture)が
  deterministic pass、fixture bump 後 3/3。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  PASS。`cargo fmt --all -- --check` — PASS。
- `node scripts/check-lockfile.mjs` — PASS。
- `git grep -n "error_code_from_str" -- crates/` — 0 件。
  `git grep -n "and_then(ErrorCode::from_str)" -- crates/
  cheaper-runtime crates/cheaper-rig-adapter` — 2 件(mod.rs:954・
  error.rs:205、両 consumer が共有 API を使用)。
- `grep -n "impl FromStr for ErrorCode\|impl std::str::FromStr"
  crates/cheaper-core/src/errors.rs` — 0 件(trait 非採用)。
- envelope schema(`schemaVersion`)・exit code 表・`ERROR_TABLE` —
  無変更を確認。

## Risks

- fixture 09(`partial-mutation`、`after_ms: 120`)も同種の構造的
  race(abort が setup 着弾すると期待する mutation/cause が無い)を
  理論上は持つ。headroom が既に取られているため今回は変更なしだが、
  さらに setup が遅い環境では同様の flake が理論上残る。
- code-less `AbortReason` の setup boundary fallback は引き続き
  `INTERNAL_ERROR`(spec の Deferred Design Concerns どおり維持)。
- `error_object` 非 CheaperError residual は `code()` 復元分岐を
  通るが語彙外 code は `from_str` が `None` を返すため到達可能な
  挙動差なし(unit test で pin 済み)。

## Follow-ups

- fixture 09(`partial-mutation`)の理論上の残留 race — 実害が
  観測された時点で `after_ms` の headroom 拡大を検討(今回は
  noted-not-fixed)。
- `setup_abort_error` の doc comment(mod.rs:936)が
  「scope `ready()`/`sandboxed()` の `AbortReason` throw」と記すが、
  現行 call site は `scope.ready()`(mod.rs:668)のみ —
  `sandboxed()` 経由の呼び出しは存在せず comment が stale。
  cosmetic のため本 scope 外、次回の run pipeline 変更時に修正可。
