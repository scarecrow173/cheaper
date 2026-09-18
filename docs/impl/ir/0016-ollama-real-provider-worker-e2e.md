---
id: "IMPL-0016"
type: "impl"
status: "completed"
title: "Real-provider worker E2E via Ollama (PLAN-0021)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/cli/test/e2e/real-provider-ollama.test.ts"
    modified:
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0108-implement-ollama-real-provider-e2e.md"
    - "docs/tasks/0109-skip-path-docs-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0021-ollama-real-provider-worker-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0026-ollama-real-provider-worker-e2e.md"
    - "docs/specs/0021-ollama-real-provider-worker-e2e.md"
    - "docs/adr/0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Real-provider worker E2E via Ollama (PLAN-0021)

## Summary

`packages/cli/test/e2e/real-provider-ollama.test.ts` を追加し、
Ollama コンテナが利用可能な環境では bulk-read / code-write /
doc-write の 3 worker を bundled consumer CLI 経由で実 provider
実行する E2E を `pnpm test` に組み込んだ。Ollama 未起動・model
不在・apm 不在では全 test が自動 skip され、CI や Ollama のない
開発環境でも suite は green を維持する。

## Context

TASK-0108/0109（PLAN-0021 / SPEC-0021 / ADR-0024 / DESIGN-0026）
に従う。Issue #12。PLAN-0016 の distributable package 分離を
前提に、`makeConsumerLayout` + `apmInstall` の既存 fixture で
`packages/apm` を隔離 consumer に install してから bundled CLI
を実行する（repo-local `.apm` 開発資産は使わない）。

## Implementation

### Changes

- `packages/cli/test/e2e/real-provider-ollama.test.ts`（新規）:
  `http://localhost:11434/v1/models` を probe し、対象 model
  （default `qwen3.5:9b`、`CHEAPER_E2E_OLLAMA_MODEL` で override）
  が pull 済みかを判定。利用不可なら `describe.skip` 相当の skip
  経路で終了。利用可能なら隔離 temp consumer を作り、
  `PI_CODING_AGENT_DIR` / `CHEAPER_CONFIG_DIR` / `HOME` /
  `USERPROFILE` を全て temp 配下に向け、`models.json` /
  `models.yaml` / `policy.yaml`（provider `ollama`、
  `openai-completions`、base URL `http://localhost:11434/v1`、
  `reasoning_effort: "none"`、`temperature: 0`、low tier 強制、
  `maxTurns: 16`・repair attempts 4・timeout 10m）を生成して
  bundled CLI を実行。envelope（`schemaVersion: 1`、`ok: true`、
  worker 名、`meta.model.provider === "ollama"`、
  `meta.model.id`、正の `meta.usage.input`）と worker 固有成果物
  （findings 配列 / `meta.changes.files` + 生成 file / 生成
  document）を assert。test timeout は各 600s
- `AGENTS.md`: 「検証用 Ollama コンテナ」節に自動実行・skip
  条件・default model・override 環境変数を追記

### Decisions During Implementation

- **skip は probe ベース**: availability を実行時判定にし、環境
  変数 flag による opt-in/out を持たない。Ollama 不在の CI で
  `pnpm test` が壊れないことが Issue の要件
- **品質 assert をしない**: model 出力の良し悪しではなく provider
  到達・envelope contract・成果物生成のみを検証し、model version
  差による flakiness を避ける（SPEC-0021 の受け入れ条件）
- **fixture 内の template literal 回避**: fixture ソース内の
  `` `hi ${name}` `` が biome の `noTemplateCurlyInString` を
  踏むため文字列連結に変更
- **`meta.changes.files` の optional chain**: `noUnsafeOptional
  Chaining` を避けるため local 変数に取り `Array.isArray` で assert
- **IMPL 番号**: in-flight で 0010（#8）・0013（#14）・0014
  （#11）・0015（#10）が使用済みのため 0016

## Validation

- live 実行（Ollama 稼働・`qwen3.5:9b` pull 済み）:
  `vitest run test/e2e/real-provider-ollama.test.ts` → 3/3 pass
  （bulk-read 約 19s・code-write 約 5s・doc-write 約 12s、
  いずれも `meta.model.provider === "ollama"`・正の usage input）
- skip 経路: `docker compose stop ollama` 後に同 test を実行 →
  3/3 skipped・suite pass（約 1.4s）。fail ではなく skip で
  終了することを確認し、コンテナは再起動済み
- `mise run check`（Ollama 稼働状態）exit 0: lint（warning 4 件、
  全て既存）→ typecheck → test（core 270 + runtime-pi 255 +
  cli 134 = 659 tests、real-provider 3 件を含む全 pass）→
  bundle freshness clean
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all`:
  198 files / no findings

## Risks

- Ollama が起動していると `pnpm test` / `mise run check` に実
  provider 実行分（〜50s）が加わる。skip 経路（未起動時）なら
  約 1.4s で済むため、速度を取るならコンテナ停止でよい
- model 出力の非決定性により稀に worker 固有 assert（file 生成
  等）が不安定になり得る。`temperature: 0`・
  `reasoning_effort: "none"`・repair attempts で緩和済みだが、
  品質 assert を入れていないため影響は限定的
- `CHEAPER_E2E_OLLAMA_MODEL` で未 pull の model を指定すると
  skip になる（fail ではない）。model 名の typo が静かに通る
  可能性は残る

## Follow-ups

- Issue #13: delegation の context/token/cost 計測。本 test が
  `meta.usage`（正の input tokens）を real provider で assert
  する足がかりになる
- Issue #15: README/ドキュメントの user journey 再構成（e2e の
  実行方法記載は AGENTS.md に留めた）
