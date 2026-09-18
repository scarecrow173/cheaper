---
id: "DISC-034r05iHy2gdnp5sN3kF1s"
type: "discovery"
status: "resolved"
title: "Real provider smoke verification via Ollama"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "cheaper-v1-pi-sdk.md"
    - "../designs/model-resolver.md"
    - "../designs/pi-execution-engine.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/ollama-real-provider-smoke-verification.md"
    - "../adr/use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Real provider smoke verification via Ollama

## Exploration Goal

`cheaper` の成功経路はこれまで `CHEAPER_TEST_FAKE_STREAM` の scripted fixture でのみ検証されており、実 provider 経由の smoke test が未検証だった（IR-0002/0003 の Review Handoff 残件）。どの実 provider を、どの設定経路で cheaper の model resolution に乗せられるかを確認する。

## Key Issues and Assumptions

- Pi (`@earendil-works/pi-coding-agent@0.85.1`) に `devin` provider は存在せず、Devin local は独自プロトコル + セッション認証のため使えない
- `packages/cli/src/cli.ts` は `createModelRuntime()` を無引数で呼ぶ → Pi 既定パス `~/.pi/agent/models.json`（`PI_CODING_AGENT_DIR` で変更可）の custom provider がそのまま使える
- Pi custom provider は `apiKey` も `oauth` も無いと `provider-composer.js` が "no authentication method configured" を throw し provider 自体が構成されない → キーレスな Ollama でも `apiKey` に任意リテラル（例: `"ollama"`）が必要。リテラルは env 参照とみなされず configured 扱いになる
- `api: "openai-completions"` は pi-ai に登録済みの API provider であり、Ollama の `/v1` OpenAI 互換 endpoint に対応する

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Ollama on Docker | キーレス・ローカル完結・再現可能・ホストを汚さない | Docker 必須、モデル pull に時間 | yes |
| Ollama native install | キーレス・ローカル完結 | ホストへインストールが必要、環境差分が残る | no |
| 課金 hosted provider + API key | 高品質な応答 | 認証情報の管理が必要、コスト発生、秘密情報をコミットできない制約と相性が悪い | no |
| 実 provider 検証を行わない | 変更不要 | V1 の成功条件「Pi available model から tier 分類・選択できる」が実経路で未証明のまま | no |

## Tentative Conclusions and Hypotheses

- confirmed: models.json の custom provider + literal `apiKey` で `getAvailable()` に Ollama モデルが載る（`provider-composer.js` の `check`/`resolve` 実装で確認）
- confirmed: cost 0・cost 欠落のモデルは built-in heuristic で **medium** に分類される（`classify.ts` — 引継ぎ側の「cost 0 → low」という想定は実コードで反証済み）
- confirmed: low tier に分類されるには model id が built-in 表のパターン（`haiku|mini|nano|flash|lite|small|codex-mini|\b(1|3|7|8)b\b`）に合致するか、`models.yaml` の `overrides` で `"<provider>/<id>": "low"` をピン留めする必要がある。`qwen3:4b` は `4b` がパターン外のため medium になる
- confirmed: `models.yaml` の探索順は `<userConfigDir>/models.yaml` → `<cwd>/.cheaper/models.yaml`（`userConfigDir` = `$CHEAPER_CONFIG_DIR` または `~/.config/cheaper`）

## Open Questions

- 採用モデルは tier 分類を overrides でピン留めする前提で `qwen3:4b` とするか、パターン合致する `llama3.2:3b` / `qwen3:8b` とするか（実用上はどちらでもよいが、overrides を使う方が model 選択の自由度が高い）

## Promotion Candidates

- [x] spec-doc needed: 実 provider smoke 検証の範囲と受け入れ条件 → `docs/specs/0004`
- [x] adr-doc needed: 実 provider として Ollama on Docker を採用する決定 → `docs/adr/0009`

## Sources and Relations

- ユーザーからのタスク指示（Ollama on Docker での smoke 実施、AGENTS.md への手順記載）
- IR-0002 §137 / IR-0003 §135: 「実 provider での smoke は手動手順として残る」
- 実コード検証: `packages/runtime-pi/node_modules/@earendil-works/pi-coding-agent/dist/core/{model-config.d.ts,provider-composer.js,model-runtime.js}`、`packages/core/src/model/{classify,config,resolve}.ts`、`packages/core/src/config/dirs.ts`
