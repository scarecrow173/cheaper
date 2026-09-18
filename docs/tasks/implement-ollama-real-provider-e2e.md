---
id: "TASK-034r05iHQeQpDqHVQkd8t7"
type: "task"
status: "done"
title: "Implement real-provider worker e2e and verify against live Ollama"
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
    - "docs/plans/ollama-real-provider-worker-e2e.md"
  implemented-by: []
  depends-on:
    - "docs/plans/ollama-real-provider-worker-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/ollama-real-provider-worker-e2e.md"
    - "docs/specs/ollama-real-provider-worker-e2e.md"
    - "docs/adr/auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/ollama-real-provider-worker-e2e.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# TASK-034r05iHQeQpDqHVQkd8t7: Implement real-provider worker e2e and verify against live Ollama

## Work

`packages/cli/test/e2e/real-provider-ollama.test.ts` を
DESIGN-034r05iHy2gdnoWvkDigEM どおりに実装し、稼働中の Ollama コンテナに対して
実検証する。

### 1. test file

- beforeAll: `apmAvailable()` → `GET http://localhost:11434/v1/models`
  probe → model 存在確認（env `CHEAPER_E2E_OLLAMA_MODEL`、default
  `qwen3.5:9b`）→ `makeConsumerLayout(root,"windsurf")` +
  `apmInstall` → `bundledCli`。
- config fixture: `<root>/pi-agent/models.json`（ollama provider、
  `samplingParams:{reasoning_effort:"none",temperature:0}`）・
  `<root>/cheaper-config/models.yaml`（overrides low pin +
  preferences.low 先頭）・`policy.yaml`（timeout 10m・maxTurns 16・
  repairAttempts 4）。
- env: `PI_CODING_AGENT_DIR`・`CHEAPER_CONFIG_DIR`・
  `HOME/USERPROFILE=<root>`・`NODE_ENV=production`。
- 3 tests（各 `t.skip` guard + 10 分 timeout）:
  - bulk-read: `src/greeter.ts` 配置 → findings assert
  - code-write: `src/farewell.ts` create → changes.files + file 存在
  - doc-write: `docs/hello.md` create → file 存在
- 全 test で exit 0・`ok===true`・`meta.model.provider==="ollama"`・
  `meta.usage.input>0` を assert。

### 2. live 検証

`docker compose up -d` + `qwen3.5:9b` pull 済みの環境で
`vitest run test/e2e/real-provider-ollama.test.ts` を実行し、
3 test の pass を確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/ollama-real-provider-worker-e2e.md`
- `docs/designs/ollama-real-provider-worker-e2e.md`
- `docs/specs/ollama-real-provider-worker-e2e.md`
- `docs/adr/auto-detect-skip-real-provider-e2e-via-isolated-consumer.md`

## Dependencies

- なし（本 chain の先頭 task）

## Affected Files

- `packages/cli/test/e2e/real-provider-ollama.test.ts`（新規）

## Done When

- [x] test file が DESIGN-034r05iHy2gdnoWvkDigEM どおりに実装されている
- [x] Ollama 稼働環境で 3 worker test が実 provider で pass
- [x] `meta.model.provider === "ollama"`・usage 正値・成果物が
      assert されている

## Verification

- [x] `vitest run test/e2e/real-provider-ollama.test.ts`（live）pass
- [x] `pnpm typecheck`・`pnpm lint` pass

## Verification results

2026-09-18 実施（Docker Ollama コンテナ稼働・`qwen3.5:9b` pull 済み）。

| Command | Result |
|---------|--------|
| `vitest run test/e2e/real-provider-ollama.test.ts` | 3/3 pass — bulk-read（7.3s）・code-write（4.1s）・doc-write（10.8s）が apm install した bundled CLI 経由で Ollama に実到達。envelope ok・`meta.model.provider==="ollama"`・`meta.model.id==="qwen3.5:9b"`・usage 正値・findings/changes/files 作成を実測 |
| `pnpm typecheck` | pass |
| `pnpm lint` | pass（warnings のみ — 既存生成物・hook のもの） |

2 回目の実行でも 3/3 pass（flaky なし）。lint 修正は fixture 文字列
（template literal 回避）と assert 形状のみで test 意味は不変。
