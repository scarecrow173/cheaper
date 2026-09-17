---
id: "PLAN-0021"
type: "plan"
status: "approved"
title: "Real-provider worker E2E via Ollama — implementation plan"
test-spec-skip: "The deliverable is itself an e2e test specification in executable form; verified by running it against a live Ollama container."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0021-ollama-real-provider-worker-e2e.md"
    - "../designs/0026-ollama-real-provider-worker-e2e.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/0026-ollama-real-provider-worker-e2e.md"
  implemented-by: []
  depends-on:
    - "../adr/0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/0026-ollama-real-provider-worker-e2e.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# Real-provider worker E2E via Ollama — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `packages/cli/test/e2e/real-provider-ollama.test.ts` | add | 実 provider e2e（本 plan の唯一の code artifact） |
| `AGENTS.md` | modify | Ollama 節に e2e 自動検証の存在を追記 |

## Tasks

- **TASK-0108**: test file 実装 + Ollama 稼働環境での実検証
- **TASK-0109**: skip 経路検証・AGENTS.md 追記・status 同期・
  全量検証

## Dependency Graph

```
TASK-0108 (test impl + live verify)
   └─> TASK-0109 (skip path + docs + final verification)
```

## Risk Register

- 小モデルの repair 非収束で flaky: recipe（temperature 0・
  repairAttempts 4・maxTurns 16）は IMPL-0008 で収束実績あり。
  内容 assert を避け契約 shape のみに留めることで耐性を確保。
- probe の network timeout 誤判定: 3s の短い timeout で fail→skip
  なので誤検出でも安全側。
- bundled CLI の実行が実 user config を読む: env を tmpdir へ
  完全に向ける（PI_CODING_AGENT_DIR・CHEAPER_CONFIG_DIR・HOME）。

## Rollback

- test file 削除のみで完全復帰（実装コード非変更有り — 副作用は
  新規 test と AGENTS.md の 1 行追記に限定される）。

## Verification Matrix

| Requirement | Verification |
|---|---|
| auto-detect skip | Ollama stop 状態で test が skip することを実測 |
| 実到達証明 | meta.model.provider==="ollama" + usage 正値 assert |
| worker 成果物 | bulk-read findings / code-write・doc-write file 作成 assert |
| 隔離 | tmpdir env・apm install consumer・repo skill 非使用 |
| 回帰 | pnpm test 全件・typecheck・lint・audit |
