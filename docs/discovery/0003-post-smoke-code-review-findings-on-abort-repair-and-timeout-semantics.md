---
id: "DISC-0003"
type: "discovery"
status: "resolved"
title: "Post-smoke code review findings on abort, repair, and timeout semantics"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
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
    - "../designs/0004-pi-execution-engine.md"
    - "../designs/0005-cli-contract-envelope-exit-codes.md"
    - "../impl/exp/0002-ollama-real-provider-smoke-verification.jsonl"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/0005-repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md"
    - "../adr/0010-normalize-repair-abort-grace-provider-failure-classification-and-agent-execution.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-smoke code review findings on abort, repair, and timeout semantics

## Exploration Goal

実 provider smoke（EXP-0002）で repair が 2〜3 回の通常経路であることが実測されたことを受け、別セッションのコードレビュー指摘 6 件 + 文書指摘 1 件を実コード・設計書と照合し、修正が必要な事実と契約上の判断点を確定する。

## Key Issues and Assumptions

以下、全件を実コードで検証済み（confirmed）。

- **[High] repair の abort grace 未実装**: main session は `promptOutcome` と 5 秒 `gracePromise` を `Promise.race` する（`run.ts`）が、repair は signal 受信時に `session.abort()` を呼ぶのみで `await session.prompt()` を無期限に待つ（`repair.ts`）。provider が AbortSignal を無視する経路で global timeout 後もハング可能。DESIGN-0004:116「abort 後 5 秒以内に prompt() が resolve しなければ待機を打ち切り」との不一致。
- **[Important] repair の provider failure 分類が非対称**: main は `session.prompt()` rejection → `PROVIDER_FAILURE`、`stopReason === "error"` → `PROVIDER_FAILURE`。repair は rejection が生のまま上へ抜けて `INTERNAL_ERROR` になり得、`stopReason: "error"` で resolve すると invalid JSON 扱いで `INVALID_OUTPUT` になり得る。`PROVIDER_FAILURE` は retryable、`INVALID_OUTPUT` / `INTERNAL_ERROR` は non-retryable で親 Agent の再試行判断が変わる。
- **[Important] repair 失敗時 `meta.repairs` が 0**: `runWorker` の `repairs` は `repairOutput` 成功時のみ代入。attempts 使い切り `INVALID_OUTPUT` や repair 中 timeout では usage/turns は集計されるのに `meta.repairs` は 0 のまま。
- **[Important] user config が unit test に漏れる**: `resolveConfigDirs` は `env` を注入できるが homedir は常に `os.homedir()`。`main.test.ts` の `makeIo` は `env={}` / `cwd=process.cwd()` で、実ユーザーの `~/.config/cheaper` と repo `.cheaper` を読み得る。実測: smoke 用 `policy.yaml`（timeout 10m）で 2 tests 失敗。e2e は `HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR` を tmp 隔離済み。
- **[Important/Contract] timeout 起点が設計と異なる**: DESIGN-0004:114 は「CLI 入口で `setTimeout(effectiveTimeoutMs)`」だが、実装は `runWorker` 内で model 解決・`createWorkerSession` 完了後にタイマー開始。model discovery / runtime 初期化 / session 構築の遅延は policy timeout に含まれない。
- **[Medium] abort grace timer が clear されない**: `requestAbort` が `setTimeout(resolveGrace, graceMs)` を作るが handle を保持せず、provider が即座に abort に応答しても timer が残り event loop を保持。CLI が envelope 出力後に最大 5 秒 exit 遅延し得る。
- **[Docs] README が実態と矛盾**: 「開発初期（設計フェーズ）。実装はこれから。」のまま。実 provider 運用知識（models.json / models.yaml / `reasoning_effort: "none"` / `response_format` の使い分け）が文書化されていない。

## Alternatives and Comparison

timeout 起点の契約について（ユーザー決定済み: 「設計書を修正する」を選択）。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| CLI 入口 deadline に移す | 設計書現行どおり。runtime 初期化・model 解決・session 構築・repair を同一 deadline に含む | `runWorker` に deadline を渡す API 追加が必要。stdin 読込など CLI 前段も timeout に含まれ意味が曖昧 | no |
| 設計書を「agent execution timeout」に改訂 | 実装意図と一致。CLI 前段（stdin 読込・worker 解決）は呼び出し側の責務として明確 | 設計書の改訂が必要 | **yes** |
| runWorker 入口起点を新設計にする | model 解決・session 構築を含めつつ CLI 前段を除外 | 「設計書を修正」選択とほぼ同じ結論（runWorker 開始 = agent execution 開始） | yes（上と統合） |

repair abort grace の実装方式。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| main と repair で共通 helper（prompt + grace race） | 仕様の一箇所定義（DRY）。将来の session 種別追加にも再利用 | 小さな新規抽象 | **yes** |
| repair 側に同じ race を複写 | 変更が局所 | 同じ不変条件の複製で表現ドリフト | no |

## Tentative Conclusions and Hypotheses

- confirmed: 上記 6 件はすべて実コードで再現可能な不整合。repair が通常経路である以上、repair session は main session と同じ abort・失敗分類の不変条件を持つべき。
- confirmed: timeout 起点は「runWorker 開始時点の agent execution timeout」として設計書を改訂する方向（ユーザー選択）。model 解決・session 構築は含め、CLI 前段（stdin 読込・worker/policy 解決・runtime 初期化）は除外。
- confirmed: `meta.repairs` は attempt 消費時点で計上するのが整合（成功・失敗・中断を問わず実測値）。
- confirmed: unit test は e2e と同じく `HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR` を tmp へ向けるべき。`resolveConfigDirs` の homedir 解決が env（`USERPROFILE`/`HOME`）を見るようにすれば e2e と同じ隔離が成り立つ。
- confirmed: grace timer は `clearTimeout` または `unref()` で event loop を保持しないこと。
- confirmed: README 更新と provider 運用知識の文書化（`docs/guides/providers.md` または README）が必要。`response_format` は main session では tool call を抑制するため不採用、repair session（tool 無し）では `json_schema` で採用済み、という使い分けを残す。

## Open Questions

- なし（timeout 起点はユーザー決定済み。残りは実装方式の詳細で design 工程で確定）

## Promotion Candidates

- [x] spec-doc needed: abort/repair の不変条件・telemetry・test hermeticity・文書化の要求仕様（SPEC-0005）
- [x] adr-doc needed: timeout 起点の契約変更と repair の失敗分類正規化の決定（ADR-0010）

## Sources and Relations

- ユーザーからのレビュー指摘（2026-09-16、別セッションのコードレビュー結果）
- EXP-0002: 実 provider smoke で repair 2〜3 回・real usage を実測
- DESIGN-0004: abort grace・timeout 起点・repair セッションの規定
- 実コード: `packages/runtime-pi/src/run.ts` `repair.ts`、`packages/core/src/config/dirs.ts`、`packages/cli/src/cli.ts`、`packages/cli/test/main.test.ts`、`packages/cli/test/e2e/helpers.ts`
