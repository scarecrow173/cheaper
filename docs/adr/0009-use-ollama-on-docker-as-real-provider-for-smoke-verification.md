---
id: "ADR-0009"
type: "adr"
status: "proposed"
title: "実 provider smoke 検証に Ollama on Docker を使う"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../impl/ir/0002-implement-cheaper-runtime-pi-plan-0002.md"
    - "../impl/ir/0003-implement-cheaper-cli-and-built-in-workers-plan-0003.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "0005-model-tier-model-resolver.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/0004-ollama-real-provider-smoke-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0002-real-provider-smoke-verification-via-ollama.md"
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

# 9. 実 provider smoke 検証に Ollama on Docker を使う

## Context and Problem Statement

`cheaper` の実 provider 経路を検証するための provider をどう用意するか。Pi に `devin` provider は存在せず Devin local は使えないため、別の実 provider が必要。検証は誰の環境でも再現でき、認証情報を要せず、リポジトリを汚さないことが求められる。

## Decision Drivers

* 認証情報なしで `getAvailable()` に載る provider が必要（Pi custom provider は `apiKey`/`oauth` のいずれかを要求するが、任意リテラルでよい）
* ホスト環境へのインストールを避け、コンテナで再現可能にしたい（ユーザー指示）
* low tier モデルを供給できること（built-in worker は両方 `tier: low`、`tierFallback: none`）

## Considered Options

* (1) Ollama を Docker コンテナで起動し、`models.json` の custom provider（`api: "openai-completions"`, `baseUrl: http://localhost:11434/v1`）として登録する
* (2) Ollama をホストへネイティブインストールする
* (3) 課金 hosted provider（OpenAI 等）+ API key を使う
* (4) 実 provider 検証を行わず fake stream のままにする

## Decision Outcome

Chosen option: "(1) Ollama on Docker + models.json custom provider", because キーレスでローカル完結し、ホストを汚さず再現可能であり、Pi の custom provider 機構にコード変更なしで乗るから。

### Consequences

* Good, because ランタイムのコード変更なしに実 provider 経路を検証できる。
* Good, because docker-compose で環境を再現でき、AGENTS.md に起動手順を残せる。
* Bad, because Docker が前提になり、モデル pull に時間とディスクを要する。
* Bad, because Ollama の小モデルは出力品質が低く、repair 経路を通る可能性がある。

### Confirmation

* `curl http://localhost:11434/v1/models` が応答し、`cheaper bulk-read` / `cheap-write` が exit 0 で `meta.model.provider` が Ollama provider id になることを Experiment Log で確認する。

## Implementation Plan

* Affected paths: `docker-compose.yml`（新規・リポジトリ内）、`AGENTS.md`（コンテナ手順の追記）、`%USERPROFILE%\.pi\agent\models.json`（リポジトリ外・マージ）、`$CHEAPER_CONFIG_DIR/models.yaml`（リポジトリ外）、`docs/impl/exp/`（Experiment Log）
* Patterns to follow: models.json は `ProviderConfigSchema`（`api`, `baseUrl`, `apiKey`, `models[].{id,cost,contextWindow,maxTokens}`）に従う。既存エントリは破壊せずマージ
* Patterns to avoid: ランタイムコード変更、`auth.json` への登録、環境依存設定のリポジトリへのコミット、`4b` 等の low パターン非合致モデルを overrides 無しで使うこと
* Dependencies or configuration: Docker / Docker Compose、`ollama/ollama` イメージ、`$CHEAPER_CONFIG_DIR`

## Verification

* [ ] `docker compose up -d` で Ollama が起動し `/v1/models` が応答する
* [ ] `ollama pull`（コンテナ経由）で小モデルが取得できる
* [ ] `bulk-read` / `cheap-write` の smoke が exit 0 で実測 `meta.usage` / `meta.changes` を返す
* [ ] `pnpm -r test` が green のまま

## Pros and Cons

### Option 1: Ollama on Docker

* Good, because 再現可能・キーレス・ホスト非侵襲。
* Bad, because Docker 必須・pull コスト。

### Option 2: Ollama native install

* Good, because コンテナ層が無く単純。
* Bad, because ホストへインストールが残り、環境差分の再現が難しい。

### Option 3: 課金 hosted provider + API key

* Good, because 応答品質が高い。
* Bad, because 秘密情報管理が必要で「秘密情報コミット禁止」の制約と相性が悪く、コストも発生する。

### Option 4: 実 provider 検証を行わない

* Good, because 何も変えなくてよい。
* Bad, because V1 成功条件の実経路証明が残ったままになる。

## More Information

* DISC-0002: Pi custom provider の availability 要件（apiKey 必須・literal 可）と tier 分類の実コード検証
* Pi `ProviderConfigSchema`: `packages/runtime-pi/node_modules/@earendil-works/pi-coding-agent/dist/core/model-config.d.ts`
