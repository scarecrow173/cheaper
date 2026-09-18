---
id: "SPEC-034r05iHK8bA7g9pm5z8s2"
type: "spec"
status: "implemented"
title: "Ollama real-provider smoke verification"
created: "2026-09-16"
updated: "2026-09-17"
owners: ["scarecrow173"]
relations:
  source:
    - "../impl/ir/implement-cheaper-runtime-pi-plan-0002.md"
    - "../impl/ir/implement-cheaper-cli-and-built-in-workers-plan-0003.md"
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
    - "../adr/use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
    - "../designs/model-resolver.md"
    - "../designs/pi-execution-engine.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/real-provider-smoke-verification-via-ollama.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Ollama real-provider smoke verification

## What Should Be Built

Ollama（Docker コンテナ）を実 provider として、`cheaper bulk-read` / `cheaper cheap-write` を実モデル呼び出しで実行し、成功経路が実 provider でも成立することを検証する。あわせて、検証環境を再現可能にする docker-compose 定義と、コンテナ未起動時の検知・起動手順を AGENTS.md に記載する。

## Why Now

V1 の成功経路は `CHEAPER_TEST_FAKE_STREAM` の scripted fixture でのみ検証されており、実 provider 経路（model availability → tier 分類 → session → tool 実行 → usage 計測 → envelope 出力）が一度も実モデルで通っていない（IR-0002/0003 の Review Handoff 残件）。

## Users and Value

- cheaper 開発者: 実 provider での end-to-end 動作を確認でき、今後の provider 追加時の検証手順を再利用できる
- 親 Coding Agent: `cheaper` への委譲が fake stream ではなく実モデルで機能することを確認できる

## Scope

### In Scope

- Ollama を Docker コンテナで起動するための docker-compose 定義（リポジトリ内に配置）とモデル pull 手順
- Pi の `models.json`（`%USERPROFILE%\.pi\agent\models.json`）への Ollama custom provider 追加（既存エントリはマージし破壊しない）
- `models.yaml` による low tier への Ollama モデル割り当て（環境依存設定のため `$CHEAPER_CONFIG_DIR` 配下を基本とする）
- `bulk-read` / `cheap-write` の tmp workspace での実実行と envelope・`meta.model`・`meta.usage`・`meta.changes` の実測確認
- 結果の Experiment Log（`docs/impl/exp/`）への記録
- AGENTS.md へのコンテナ起動・確認手順の記載

### Out of Scope

- cheaper ランタイム本体のコード変更（models.json だけで available にならない等の場合は「何が不足しているか」を報告して停止）
- CI への Ollama smoke 組み込み
- Ollama 以外の provider 検証、複数モデル比較、モデル品質・性能評価
- `auth.json` への認証情報登録（Ollama はキーレスのため不要）

## Requirements

- Ollama コンテナが `http://localhost:11434/v1/models` に応答し、少なくとも 1 つの小モデルが pull 済みであること
- `models.json` の provider 定義は `api: "openai-completions"`、`baseUrl: "http://localhost:11434/v1"`、任意リテラルの `apiKey` を含み、モデル定義に `cost` / `contextWindow` / `maxTokens` を持つこと
- 選択モデルが cheaper の tier 分類で `low` になること（model id が built-in パターンに合致するか、`models.yaml` の `overrides` で `"<provider>/<id>": "low"` を明示）
- 検証後に `pnpm -r test` が従来どおり green であること
- 秘密情報・環境依存パスを含む設定をリポジトリへコミットしないこと

## Acceptance Criteria

- [ ] `curl http://localhost:11434/v1/models` が pull 済みモデルを返す
- [ ] `echo '<input>' | node packages/cli/bin/cheaper.js bulk-read` が exit 0 で、stdout が envelope 1 行のみ、`meta.model.provider` が Ollama provider、`meta.usage` が実測値を持つ
- [ ] `cheap-write` が tmp workspace に実際の変更を行い、`meta.changes` が実測の diff 統計を持つ
- [ ] 成功時の `models.json` / `models.yaml` 手順が秘密情報を含まない形で Experiment Log に残る
- [ ] AGENTS.md にコンテナ未起動時の検知方法と起動手順が記載される
- [ ] `pnpm -r test` が green のままである

## Deferred Design Concerns

- CI での実 provider smoke（Docker 依存のため別途設計が必要）
- `meta.model` の provider id 表記（models.json の providerId 文字列）が親 Agent 向け表示として適切か

## Sources and Relations

- ユーザーからのタスク指示（2026-09-16）
- DISC-034r05iHy2gdnp5sN3kF1s: 実コード検証による Pi custom provider の availability 要件と tier 分類の確認結果
- ADR-034r05iHlWosDpQ6Z1LgJo: Ollama on Docker 採用の決定
