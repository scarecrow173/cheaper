---
id: "ADR-034r05iHrZUE6W2tpzbgKv"
type: "adr"
status: "accepted"
title: "Normalize repair abort grace, provider failure classification, and agent-execution timeout anchor"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/post-smoke-code-review-findings-on-abort-repair-and-timeout-semantics.md"
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
    - "../specs/repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
    - "../designs/pi-execution-engine.md"
    - "../impl/exp/ollama-real-provider-smoke-verification.jsonl"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/post-smoke-code-review-findings-on-abort-repair-and-timeout-semantics.md"
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

# 10. Normalize repair abort grace, provider failure classification, and agent-execution timeout anchor

## Context and Problem Statement

実 provider smoke（`docs/impl/exp/ollama-real-provider-smoke-verification.jsonl`）で repair session が通常経路（2〜3 回発生）であることが判明し、続くコードレビュー（DISC-034r05iHy2gdnp6g5GTWKG）で repair 経路に main session と非対称な不変条件があることが確定した。

- repair は `session.abort()` を呼ぶだけで `await session.prompt()` を無期限に待ち、abort grace（5 秒打ち切り）が無い
- repair の `session.prompt()` rejection は生のまま抜けて `INTERNAL_ERROR`、`stopReason === "error"` の resolve は `INVALID_OUTPUT` になり得る（main は両方 `PROVIDER_FAILURE`）
- repair 失敗時に `meta.repairs` が 0 のまま
- DESIGN-034r05iHrZUE6WvaQsRRsM は「CLI 入口で setTimeout」と規定するが実装は `runWorker` 内の session 構築後にタイマー開始しており、契約と実装が不一致

## Decision Drivers

* repair は例外的処理ではなく通常経路であり、main session と同じ abort・失敗分類の不変条件を持つべき
* `PROVIDER_FAILURE`（retryable）と `INVALID_OUTPUT`/`INTERNAL_ERROR`（non-retryable）の区別は親 Agent の再試行判断に直結する
* timeout の契約は「何を policy timeout で保護するか」を明確にすべき（CLI 前段は呼び出し側の責務か、runtime の責務か）

## Considered Options

* Option A: timeout を CLI 入口 deadline に移す（設計書現行どおりに実装を修正）
* Option B: 設計書を「agent execution timeout」（runWorker 開始時点）に改訂し、実装は model 解決・session 構築を含む run 区間をタイマー対象に揃える
* Option C: timeout は現状のまま（session 構築後）とし、設計書のみ追記

repair の abort grace・provider failure 正規化・`meta.repairs` 計上は対案なし（不変条件の統一として必須）。

## Decision Outcome

Chosen option: "Option B", because CLI 前段（stdin 読込・worker/policy 解決・runtime 初期化）は呼び出し側の責務であり、policy timeout は「agent が実際に実行を開始してから終了まで」を保護する方が意味が明確で、実装とも整合するため。

併せて以下を決定する。

- repair session の `prompt()` に main と同じ abort grace（abort 後 5 秒で待機打ち切り）を適用する共通 helper を runtime-pi に定義する
- repair 中の `session.prompt()` rejection および `stopReason === "error"` の resolve を `PROVIDER_FAILURE` に正規化する（invalid JSON 扱いしない）
- `meta.repairs` は attempt 消費時点で計上し、成功・失敗・中断を問わず実測値を返す
- abort grace timer は `prompt()` settle 後に `clearTimeout`/`unref` で解放し、event loop を保持しない

### Consequences

* Good, because repair 経路でハング・誤分類・telemetry 欠落が解消され、親 Agent の再試行判断が正しく機能する。
* Good, because timeout の責務境界（CLI 前段は呼び出し側、run 区間は runtime）が明確になる。
* Bad, because DESIGN-034r05iHrZUE6WvaQsRRsM の「CLI 入口で setTimeout」の記述を改訂する必要がある（契約変更）。CLI 前段の遅延は policy timeout では検出できず、呼び出し側の deadline に委ねる。

### Confirmation

`pnpm -r test` で abort grace・`PROVIDER_FAILURE` 分類・`meta.repairs` の回帰テストが追加され green。DESIGN-034r05iHrZUE6WvaQsRRsM が agent execution timeout として改訂される。

## Implementation Plan

* Affected paths: `packages/runtime-pi/src/run.ts`, `packages/runtime-pi/src/repair.ts`（+ 共通 helper 追加）、`packages/core/src/config/dirs.ts`、`packages/cli/test/main.test.ts`、`docs/designs/pi-execution-engine.md`、`README.md`（+ provider 手順ドキュメント）
* Patterns to follow: main session の `promptOutcome`/`gracePromise` race を repair にも適用する共通 helper。`lastAssistantStop` による `stopReason === "error"` 正規化を repair にも適用。e2e helpers の `HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR` tmp 隔離を unit test にも適用。
* Patterns to avoid: repair 側への grace race 複写（不変条件の重複）。`response_format` の main session 適用（tool call 抑制）。CLI 前段を policy timeout に含める変更。
* Dependencies or configuration: なし（新規依存なし）

## Verification

* [ ] repair 中 abort/timeout で grace 打ち切りによりハングしないことのテスト
* [ ] repair の prompt rejection / `stopReason: "error"` が `PROVIDER_FAILURE` になるテスト
* [ ] repair 失敗時 `meta.repairs` が消費回数になるテスト
* [ ] user config を置いた環境でも `pnpm -r test` が green であること
* [ ] DESIGN-034r05iHrZUE6WvaQsRRsM が agent execution timeout に改訂されていること

## Pros and Cons of the Options

### Option A: CLI 入口 deadline に移す

* Good, because 設計書現行どおりで、runtime 初期化・model 解決も policy timeout に含まれる。
* Bad, because stdin 読込・worker/policy 解決・runtime 初期化は「agent execution」ではなく呼び出し側の準備であり、policy timeout で保護する意味が薄い。`runWorker` に deadline を渡す API 変更が必要。

### Option B: agent execution timeout に改訂

* Good, because 「policy timeout が保護する区間 = agent の実行区間」と意味が明確。実装もタイマー開始位置を run 先頭に揃えるだけでよい。
* Bad, because 設計書の改訂が必要。CLI 前段の遅延は policy timeout の対象外になる。

### Option C: 現状維持（session 構築後起点）

* Good, because コード変更不要。
* Bad, because model 解決・session 構築が timeout 対象外のまま残り、「run の実行時間」を保護する意図とずれる。

## More Information

- DISC-034r05iHy2gdnp6g5GTWKG: 指摘の実コード検証結果
- `docs/impl/exp/ollama-real-provider-smoke-verification.jsonl`: repair が通常経路であることの実測
- DESIGN-034r05iHrZUE6WvaQsRRsM:116: abort grace の現行規定（repair にも適用すべき不変条件）
