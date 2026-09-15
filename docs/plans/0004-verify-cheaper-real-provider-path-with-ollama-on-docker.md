---
id: "PLAN-0004"
type: "plan"
status: "approved"
title: "Verify cheaper real-provider path with Ollama on Docker"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/0004-ollama-real-provider-smoke-verification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/0009-use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/0004-ollama-real-provider-smoke-verification.md"
    - "docs/designs/0009-ollama-real-provider-smoke-verification-setup.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Verify cheaper real-provider path with Ollama on Docker Implementation Plan

## Goal

Ollama（Docker コンテナ）を実 provider として `bulk-read` / `cheap-write` の smoke を実実行し、envelope・usage・changes の実測を確認して Experiment Log に記録する。

## Scope

単一の実装ストリーム。環境構築（compose・models.json・models.yaml）→ 実実行 → 記録の一直線であり、分割しない。ランタイムのコード変更は行わない（必要になった場合は実装せず報告して停止）。

## Upstream Documents

- `docs/specs/0004-ollama-real-provider-smoke-verification.md`
- `docs/adr/0009-use-ollama-on-docker-as-real-provider-for-smoke-verification.md`
- `docs/discovery/0002-real-provider-smoke-verification-via-ollama.md`

## Design Inputs

- `docs/designs/0009-ollama-real-provider-smoke-verification-setup.md`（approved）

## File Map

- `docker-compose.yml` — Ollama コンテナ定義（新規・リポジトリ内）
- `AGENTS.md` — コンテナ未起動時の検知と起動手順を追記（変更）
- `%USERPROFILE%\.pi\agent\models.json` — Ollama provider をマージ（リポジトリ外）
- `~/.config/cheaper/models.yaml`（`$CHEAPER_CONFIG_DIR` 未設定時の既定）— tier 割り当て（リポジトリ外）
- `docs/impl/exp/0002-*.jsonl` または `.md` — Experiment Log（新規。既存 exp/0001 の形式に合わせる）
- tmp workspace（`tmp/` 配下・git 管理外）— smoke 実行対象

## Implementation Sequence

- [ ] `docker-compose.yml` を作成（`ollama/ollama`、ポート 11434、named volume）
- [ ] `docker compose up -d` で起動し `curl http://localhost:11434/v1/models` で応答確認
- [ ] `docker compose exec ollama ollama pull qwen3.5:4b`（量子化済みタグ。存在しなければ利用可能な q4 系タグを選択）でモデル取得
- [ ] `%USERPROFILE%\.pi\agent\models.json` に `ollama` provider をマージ（`api: "openai-completions"`, `baseUrl: "http://localhost:11434/v1"`, `apiKey: "ollama"`, `models[]` に `id`/`cost:0`/`contextWindow`/`maxTokens`）
- [ ] `~/.config/cheaper/models.yaml` に `overrides: {"ollama/qwen3.5:4b": "low"}` と `preferences.low: ["ollama/*"]` を作成
- [ ] `mise exec -- pnpm -r build`
- [ ] tmp workspace で `bulk-read` を実行し exit 0・envelope 1 行・`meta.model.provider`・`meta.usage` を確認
- [ ] tmp workspace で `cheap-write` を実行し実ファイル変更と `meta.changes` を確認
- [ ] Experiment Log を `docs/impl/exp/` に記録
- [ ] AGENTS.md にコンテナ手順を追記
- [ ] `pnpm -r test` で回帰なしを確認

## Dependencies and Risks

- Docker / Docker Compose が必要（本機では Docker 29.6.1 + Compose v5.3.0 を確認済み）
- モデル pull は数 GB のダウンロードを伴う
- 小モデルが structured output を安定して返せず repair 経路を通る、または schema 適合に失敗する可能性 → その場合は結果を記録して報告（ランタイム変更はしない）
- `models.json` 既存内容の破壊リスク → 事前に内容確認のうえマージのみ

## Task Breakdown

- TASK-A: docker-compose.yml 作成 + AGENTS.md 手順追記
- TASK-B: コンテナ起動・モデル pull・疎通確認
- TASK-C: models.json / models.yaml 設定
- TASK-D: ビルド + bulk-read / cheap-write smoke 実実行
- TASK-E: Experiment Log 記録 + `pnpm -r test` 回帰確認

## Verification

- [ ] `curl http://localhost:11434/v1/models` がモデル一覧を返す
- [ ] `bulk-read` が exit 0、stdout が envelope 1 行のみ、`meta.model.provider` が `ollama`、`meta.usage` が実測値
- [ ] `cheap-write` が tmp workspace を実変更し `meta.changes` が実測値
- [ ] `pnpm -r test` が green

## Open Questions / Gaps

- `qwen3.5:4b` の正確なタグ名は pull 時に確認する（`q4_K_M` 等の量子化バリアントを優先）

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（検証作業であり環境状態に依存するため）
- Independent work streams: none
- User approval required before dispatch: yes

## Review Handoff

- 前提: Docker 利用可能（確認済み）、Ollama コンテナは未作成
- 反証済み事実: cost 0 → medium 判定のため `overrides` で low ピン留めが必須（DISC-0002）
