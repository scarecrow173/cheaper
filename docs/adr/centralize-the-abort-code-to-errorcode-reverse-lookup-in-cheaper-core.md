---
id: "ADR-034EdSlppBqVgIegIeGXgH"
type: "adr"
status: "accepted"
title: "Centralize the abort code to ErrorCode reverse lookup in cheaper-core"
created: "2026-10-11"
updated: "2026-10-11"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/94"
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
    - "../specs/abort-classification-at-setup-boundary.md"
    - "./normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
    - "./fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
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
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Centralize the abort code to ErrorCode reverse lookup in cheaper-core

## Context and Problem Statement

issue #94(`hang_grace` flake)の根本原因は、`AbortReason` の code を
`ErrorCode` へ復元する逆引き写像が adapter の private fn に閉じており、
setup 経路から到達不能なことによる分類非対称である。

- `AbortReason`(`cheaper-tools::operation`)は describeError 形の
  `{name, message, code}` と元 error object を運ぶ。
- prompt/stream 経路: adapter の `abort_reason_to_cheaper`
  (`crates/cheaper-rig-adapter/src/error.rs:204-212`)が `code()` を
  private table `error_code_from_str`(同:216-245)で逆引きし、
  記録済み分類(`ABORTED`/`TIMEOUT`/`MAX_TURNS_EXCEEDED`/
  `MAX_TOOL_CALLS_EXCEEDED`)を復元する。
- setup boundary(`scope.ready()`、run/mod.rs:664-668): runtime の
  `setup_abort_error`(`crates/cheaper-runtime/src/run/mod.rs:937-951`)
  は `error_object()` の downcast のみを見る。
  `RunAbortReason::abort_payload`(`crates/cheaper-runtime/src/run/
  abort.rs:69-75`)は code を保持するが error object を持たないため、
  `INTERNAL_ERROR` fallback へ落ちる。

逆引き写像を runtime・adapter 双方が共有する 1 箇所の authoritative
実装へ集約する必要がある。配置先は、依存方向
`cli → {workflow, provider} → runtime → {rig-adapter, tools} → core`
上で両 consumer から到達可能な leaf crate — cheaper-core または
cheaper-tools に限られる。なお実際の edge は
`cheaper-workflow → {core, tools}`(runtime の sibling であり、
cli が runtime executor を注入する)、
`cheaper-provider → {runtime, rig-adapter, core}`、
`cheaper-runtime → {core, tools, rig-adapter}` であり、
workflow は runtime に依存しない。

## Decision Drivers

* code 文字列の vocabulary は `ErrorCode::as_str()`(cheaper-core)が
  所有する。逆引きはその語彙の逆写像であり、owner と同居させるのが
  drift 防止に最も強い。
* `AbortReason.code()` は CheaperError code 専用ではない。
  `describe_error_object`(`operation.rs:196-`)は Node errno
  (`"ENOENT"` 等)や generic payload の code も運ぶため、
  `AbortReason` は ErrorCode 語彙の owner ではなく transport である。
  逆引きは未知文字列で `None` を返す寛容な parse でなければならない。
* 依存方向は一方向。runtime は adapter に依存するが、adapter の
  `abort_reason_to_cheaper` は `ABORTED` fallback と
  `"worker run aborted: {message}"` の message 形を返す prompt 経路
  固有の契約であり、setup boundary の `"{context} failed: {message}"`・
  `INTERNAL_ERROR` residual・`error_object` 優先順位とは一致しない。
  共有すべきは写像 table のみであり、変換関数全体ではない。
* `cheaper-core` は Rig/Tokio-free を維持する(新規依存なし)。
* 写像の二重管理は本 bug の直接原因 — 同じ語彙の表を再び2箇所に
  作らない。

## Considered Options

* A: cheaper-core の `ErrorCode` に逆引き API(例:
  `from_str(&str) -> Option<ErrorCode>`)を追加し、adapter・runtime
  の双方が使う
* B: cheaper-tools の `AbortReason` 隣に code→ErrorCode helper を置く
  (例: `AbortReason::error_code()`)
* C: runtime に adapter の table を複製する(写像を2箇所に持つ)
* D: runtime が adapter の `abort_reason_to_cheaper` を直接再利用する
  (runtime→adapter の依存方向自体は合法)
* E: adapter の既存 `error_code_from_str` table を export し、runtime
  から呼び出す(runtime→adapter の依存は合法)

## Decision Outcome

Chosen option: "A" — code 語彙の owner は `ErrorCode` であり、
`as_str` の逆写像は同じ impl 隣に置くのが自然。cheaper-core は両
consumer(runtime・adapter)の共通 leaf 依存で、純粋な `&str -> Option`
の追加は Rig/Tokio-free を維持し新規依存も要らない。

`AbortReason`(Option B)は CheaperError code 以外の describeError code
も運ぶ transport であり、ErrorCode 語彙の owner ではないため、table を
tools に置くと core 所有の語彙が downstream に複製される — 本 bug の
原因と同じ drift 構造を再生する。core API へ委譲する薄い accessor
(`AbortReason::error_code()`)は可能だが、2 call site のために API
surface を増やすほどではなく(YAGNI)、各 call site が
`reason.code().and_then(ErrorCode::from_str)` と合成すれば十分。

実装形状(確定は実装時、本 ADR は配置の決定):

- `ErrorCode` に逆引き API を `as_str` 隣へ追加。実装は `ERROR_TABLE`
  (または `as_str` 結果との照合)を唯一の語彙源とし、第二の match
  table を作らない。未知文字列は `None`。
- adapter の private `error_code_from_str` は削除して共有 API 呼び出しに
  置き換える。
- `setup_abort_error` は `error_object()` downcast 後・`INTERNAL_ERROR`
  fallback 前に `reason.code()` 復元分岐を挟む。message は
  `"{context} failed: {reason.message()}"` の prefix を維持する。

### Consequences

* Good, because 写像が1箇所になり、`ErrorCode` 変種追加時の逆引き
  更新忘れが構造的に起きない(`serde_name_matches_as_str` test が
  往復を保証)。
* Good, because adapter の error.rs:214-215 コメント「cheaper-core に
  `FromStr` は無いので adapter 側で閉じて持つ」が示していたギャップを
  正規の場所で解消する。
* Good, because core への追加は純粋関数のみで Rig/Tokio-free・dep 構成が
  不変。
* Bad, because 各 call site が「`code()` を `ErrorCode` として解釈する」
  合成(`code().and_then(...)`)を書く — ただし語彙知識の重複ではなく
  単純な組立てであり許容範囲。

### Confirmation

SPEC-034EdS8ly2SqcS7cJ8hvfJ の acceptance criteria で検証する
(全 `RunAbortReason` の setup 分類 unit test・`hang_grace` の
deterministic pass・写像の単一化確認)。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-core/src/errors.rs` — `ErrorCode` へ逆引き API を
    `as_str` 隣に追加(`ERROR_TABLE` scan 等、語彙を単一 source に
    保つ実装。未知文字列は `None`)。
  - `crates/cheaper-rig-adapter/src/error.rs` — private
    `error_code_from_str`(L216-245)を削除し共有 API 呼び出しへ置き換え。
  - `crates/cheaper-runtime/src/run/mod.rs` — `setup_abort_error`
    (L937-951)に `reason.code()` 復元分岐を追加。
  - `crates/cheaper-runtime` の test — 全 `RunAbortReason` の setup
    boundary 分類を pin する unit test。
* Patterns to follow:
  - 優先順序は `error_object()` downcast → `code()` 復元 →
    `INTERNAL_ERROR` fallback。既存経路への後段追加のみ。
  - 逆引きは `ERROR_TABLE`/`as_str` を唯一の語彙源とし、第二の
    match table を作らない。
  - message は `"{context} failed: {reason.message()}"` を維持する。
* Patterns to avoid:
  - adapter に private table を残したまま runtime へ第二 table を作ること
    (本 bug の非対称を恒常化する)。
  - `abort_reason_to_cheaper` のそのまま再利用 — `ABORTED` fallback・
    message 形・`error_object` 非考慮が setup 契約と一致しない。
  - cheaper-core への新規 dependency 追加(serde は既存 dep)。
* Dependencies or configuration: なし。envelope schema・exit code・
  `ERROR_TABLE` は不変。

## Verification

* [ ] `ErrorCode` 全変種で `from_str(as_str()) == Some(self)` の
  round-trip が成立する test がある。
* [ ] 未知 code 文字列(Node errno `"ENOENT"` 等)が `None` を返す
  test がある。
* [ ] `RunAbortReason` 全種について setup boundary で code が復元される
  unit test がある。
* [ ] adapter が共有 API を使い、private `error_code_from_str` が
  除去されている。
* [ ] dev-seam 付き `cargo test --workspace` で `hang_grace` が
  deterministic に pass する。
* [ ] `mise run check` が全工程 green。

## Pros and Cons of the Options

### A: cheaper-core の `ErrorCode` 逆引き API(採用)

* Good, because 語彙 owner(`as_str`)と同じ場所に逆写像を置け、
  `ErrorCode` 変種追加時の片側更新忘れが起きない。両 consumer から
  到達可能な最下流で、dep 追加・Rig/Tokio 混入なし。
* Bad, because 各 call site で `reason.code().and_then(...)` の合成を
  書く(薄い冗長さ)。

### B: cheaper-tools の `AbortReason` 隣 helper

* Good, because carrier 型の隣にあるため発見しやすく、
  `reason.error_code()` の ergonomics が得られる。
* Bad, because table 自体を tools に置くと core 所有の ErrorCode 語彙が
  downstream に複製され、本 bug と同じ drift 構造を再生する。core API
  へ委譲する薄い accessor に留める案もあるが、2 call site のために
  API surface を増やす必要はない(YAGNI)。

### C: runtime への table 複製

* Good, because 変更面が最小。
* Bad, because 写像が2箇所になり、`ErrorCode` 変種追加・分類変更の
  たびに非対称が再発する — 本 bug の原因構造そのもの。却下。

### D: adapter の `abort_reason_to_cheaper` を runtime から再利用

* Good, because runtime→adapter の依存は既に合法で、一見コード再利用に
  見える。
* Bad, because `ABORTED` fallback・`"worker run aborted: {message}"` の
  message 形・`error_object` downcast を持たない点が setup boundary の
  契約(`"{context} failed:"`・`INTERNAL_ERROR` residual・downcast
  優先)と一致しない。共有すべきは関数ではなく写像 table。却下。

### E: adapter の `error_code_from_str` table を export し runtime から呼ぶ

* Good, because runtime→adapter の依存方向は既に合法であり、table を
  1箇所に保ったまま最小の変更面で共有できる。
* Bad, because code 文字列の語彙は downstream の `ErrorCode`
  (cheaper-core)が所有する。authoritative table を語彙より上流の
  adapter に置いたままでは、Option C と同じ downstream 語彙の
  duplication drift 構造が場所を変えて残るだけであり、`ErrorCode`
  変種追加時に adapter 側 table の更新忘れが再発しうる。却下。

## More Information

* 上流: issue #94(<https://github.com/scarecrow173/cheaper/issues/94>)、
  SPEC-034EdS8ly2SqcS7cJ8hvfJ。
* 根本原因箇所: `abort.rs:69-75`(code 保持・object 非保持の payload)、
  `run/mod.rs:937-951` + call site `scope.ready()`(L664-668)、
  `error.rs:204-245`(adapter は復元済み — 非対称)。
* `AbortReason.code()` の語彙: `describe_error_object`
  (`crates/cheaper-tools/src/operation.rs:196-`)は `CheaperError` の
  `code.as_str()` に加え `NodeIoError` の errno code(`"ENOENT"` 等)
  も `code` へ入れる — 逆引きは寛容な `Option` parse が必須。
* 関連決定: ADR-034r05iHrZUE6W2tpzbgKv(abort grace・失敗分類
  正規化の系譜)、ADR fix-tool-runtime-and-workflow-package-topology
  (crate 依存方向の前提)。
