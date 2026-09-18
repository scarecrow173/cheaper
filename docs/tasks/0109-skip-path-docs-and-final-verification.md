---
id: "TASK-0109"
type: "task"
status: "done"
title: "Verify skip path, document e2e in AGENTS.md, sync statuses, final verification"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0021-ollama-real-provider-worker-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0021-ollama-real-provider-worker-e2e.md"
    - "docs/tasks/0108-implement-ollama-real-provider-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0026-ollama-real-provider-worker-e2e.md"
    - "docs/specs/0021-ollama-real-provider-worker-e2e.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0021-ollama-real-provider-worker-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-0109: Verify skip path, AGENTS.md note, status sync, final verification

## Work

PLAN-0021 の仕上げ task。skip 経路の実測・AGENTS.md 追記・status
同期・全量検証を行う。

### 1. skip 経路検証

Ollama を停止（`docker compose stop ollama`）した状態で
`vitest run test/e2e/real-provider-ollama.test.ts` を実行し、全
test が skip（fail でない）ことを実測する。検証後はコンテナを
再起動しておく（AGENTS.md の contract に従い、後続の実検証が
できる状態を維持）。

### 2. `AGENTS.md` 追記

「検証用 Ollama コンテナ」節に、コンテナ起動 + model pull 済みで
あれば `pnpm test` が `test/e2e/real-provider-ollama.test.ts` で
実 provider 検証を自動実行し、未起動なら skip する旨を 1〜2 行で
追記する（対象 model・`CHEAPER_E2E_OLLAMA_MODEL` override も明記）。

### 3. status 同期

- SPEC-0021 → `implemented`
- PLAN-0021 → `completed`
- TASK-0108・0109 → `done`
- 各 index の status を同期

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/0021-ollama-real-provider-worker-e2e.md`
- `docs/designs/0026-ollama-real-provider-worker-e2e.md`
- `docs/specs/0021-ollama-real-provider-worker-e2e.md`

## Dependencies

- `docs/tasks/0108-implement-ollama-real-provider-e2e.md`
  （test が実装・live 検証済みである前提）

## Affected Files

- `AGENTS.md`
- `docs/specs/0021-*.md`・`docs/plans/0021-*.md`・
  `docs/tasks/0108..0109-*.md` の status と各 index

## Done When

- [x] Ollama 停止状態で test が全件 skip する実測記録がある
- [x] AGENTS.md に e2e 自動検証・skip 条件・model override が
      記載されている
- [x] SPEC-0021/PLAN-0021/TASK-0108..0109 と各 index の status が
      同期されている
- [x] 全量検証（lint・typecheck・全 test・audit）が pass

## Verification

- [x] `vitest run test/e2e/real-provider-ollama.test.ts`（Ollama
      停止時）→ 全件 skip
- [x] `mise run check`（Ollama 稼働時）pass
- [x] `audit_docs.js --type all` が no findings

## Verification results

- Ollama 停止状態で `vitest run test/e2e/real-provider-ollama.test.ts`
  を実行: 3 件全て skipped、suite は pass（約 1.4s）。fail ではなく
  skip でクリーンに終了することを確認。検証後に `docker compose up -d`
  でコンテナを再起動済み。
- `AGENTS.md`「検証用 Ollama コンテナ」節に、real-provider e2e の
  自動実行・skip 条件・default model `qwen3.5:9b`・
  `CHEAPER_E2E_OLLAMA_MODEL` override を追記済み。
- SPEC-0021 → `implemented`、PLAN-0021 → `completed`、
  TASK-0108/0109 → `done`、各 index も同期済み。
- `mise run check`（Ollama 稼働状態）exit 0:
  - `pnpm lint`: pass（warning 4 件、すべて既存の生成物・既知
    optional-chain 指摘）
  - `pnpm typecheck`: pass
  - `pnpm test`: 全 pass（core 270・runtime-pi 255・cli 134、
    うち real-provider-ollama 3 件は Ollama live 実行で pass、
    約 50s）
  - bundle freshness: clean（`git diff --exit-code` 差分なし）
