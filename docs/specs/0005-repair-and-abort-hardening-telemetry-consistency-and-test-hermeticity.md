---
id: "SPEC-0005"
type: "spec"
status: "proposed"
title: "Repair and abort hardening, telemetry consistency, and test hermeticity"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/0003-post-smoke-code-review-findings-on-abort-repair-and-timeout-semantics.md"
    - "../impl/exp/0002-ollama-real-provider-smoke-verification.jsonl"
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
    - "../adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
    - "../designs/0004-pi-execution-engine.md"
    - "../designs/0005-cli-contract-envelope-exit-codes.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0003-post-smoke-code-review-findings-on-abort-repair-and-timeout-semantics.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Repair and abort hardening, telemetry consistency, and test hermeticity

## What Should Be Built

実 provider smoke（EXP-0002）で判明した「repair は例外的処理ではなく cheap model では通常経路」という事実を踏まえ、repair session に main session と同等の abort・失敗分類の不変条件を適用する。あわせて telemetry（`meta.repairs`）の整合、unit test の設定隔離、abort grace timer の後始末、timeout 契約の設計書改訂、および実 provider 運用知識の文書化を行う。

## Why Now

EXP-0002 で repair が 2〜3 回発生する通常経路であることが実測された。レビュー指摘の検証（DISC-0003）により、repair 経路に abort grace 未実装・provider failure 分類の非対称・`meta.repairs` 未計上という不整合があり、小モデル運用で顕在化し得ることが確定した。

## Users and Value

- 親 Coding Agent: repair 中の timeout/abort でハングせず必ず envelope を受け取れる。`PROVIDER_FAILURE`（retryable）と `INVALID_OUTPUT`（non-retryable）の区別が repair 経路でも正しく機能し、再試行判断が狂わない。`meta.repairs` が実測値として監査に使える
- cheaper 開発者: `pnpm test` が開発者の user config に依存せず hermetic に動く。abort grace timer が process exit を遅らせない。実 provider の設定手順が文書化される

## Scope

### In Scope

- repair session の `session.prompt()` に main session と同じ abort grace（abort 後 5 秒で待機打ち切り）を適用する共通 helper 化
- repair 中の provider failure を `PROVIDER_FAILURE` に正規化（prompt rejection、および `stopReason === "error"` の resolve）
- repair attempt 消費を `meta.repairs` に成功・失敗・中断を問わず計上
- `runWorker` 開始時点を timeout 起点とする「agent execution timeout」への設計書改訂（DESIGN-0004）と、タイマー開始位置を run 先頭へ揃える実装修正
- abort grace timer の `clearTimeout`/`unref` による event loop 保持の解消
- unit test の user/project config 隔離（e2e と同じく `HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR` を tmp へ。必要なら `resolveConfigDirs` が env から homedir を解決するよう修正）
- README の「開発初期」記述の更新と、実 provider 運用知識（`models.json`/`models.yaml`/`reasoning_effort`/`response_format` 使い分け）の文書化

### Out of Scope

- timeout を CLI 入口 deadline に移す大掛かりな再設計（今回は設計書を agent execution timeout に改訂する判断。ADR-0010）
- `response_format` の main session への適用（tool call を抑制するため不採用。repair のみ）
- 新しい worker・policy・error code の追加
- CI 組み込み・provider 自動検出などの将来拡張

## Requirements

- repair の各 attempt でも abort signal 受信から 5 秒以内に `prompt()` が settle しなければ待機を打ち切り、run の abort 分類経路（`TIMEOUT`/`ABORTED`/…）に入ること
- repair 中の `session.prompt()` rejection は `PROVIDER_FAILURE`、repair session の `stopReason === "error"` も `PROVIDER_FAILURE` として分類されること（invalid JSON 扱いにしない）
- `repairOutput` が例外終了した場合でも消費した attempt 数が `meta.repairs` に反映されること
- timeout は runWorker 開始（model 解決・session 構築を含む agent execution 区間）から計測されること。CLI 前段（stdin 読込・worker/policy 解決・runtime 初期化）は対象外であることを設計書に明記
- abort grace timer が `prompt()` settle 後に event loop を保持しないこと
- unit test が実ユーザーの `~/.config/cheaper` と repo `.cheaper` を読まないこと
- README が現状（実装済み・実 provider 検証済み）を反映し、実 provider 設定手順が文書化されること

## Acceptance Criteria

- [ ] repair 中に timeout/abort した場合も global timeout 後にハングせず、abort 分類（例: `TIMEOUT`）の envelope が返る（grace 打ち切りのテストを含む）
- [ ] repair session の `session.prompt()` が reject した場合に `INTERNAL_ERROR` ではなく `PROVIDER_FAILURE` になる
- [ ] repair session が `stopReason === "error"` で resolve した場合に `INVALID_OUTPUT` ではなく `PROVIDER_FAILURE` になる
- [ ] repair attempts 使い切り（`INVALID_OUTPUT`）および repair 中 abort で `meta.repairs` が消費回数を示す
- [ ] timeout が model 解決・session 構築を含む run 区間をカバーし、DESIGN-0004 が agent execution timeout として記述される
- [ ] abort 経路で grace timer が残らず、CLI process が遅延なく exit する
- [ ] user config に `policy.yaml`/`models.yaml` を置いた状態でも `pnpm -r test` が green である
- [ ] README が実態を反映し、provider 設定手順（Ollama/`models.json`/`models.yaml`/`reasoning_effort`）が文書化される

## Deferred Design Concerns

- CLI 入口 deadline（stdin 読込・runtime 初期化を含む end-to-end deadline）は将来の拡張。親 Agent が全体 deadline を持つ設計にする場合に再検討
- repair 以外の session 種別（将来の multi-turn worker 等）への abort grace helper の汎用化

## Sources and Relations

- DISC-0003: レビュー指摘の実コード検証結果
- EXP-0002: repair が通常経路であることの実測
- ADR-0010: timeout 起点・repair 正規化の決定
- DESIGN-0004: abort grace・repair session の現行規定
