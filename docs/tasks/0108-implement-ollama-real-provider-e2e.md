---
id: "TASK-0108"
type: "task"
status: "todo"
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
    - "docs/plans/0021-ollama-real-provider-worker-e2e.md"
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
---

# TASK-0108: Implement real-provider worker e2e and verify against live Ollama

## Work

`packages/cli/test/e2e/real-provider-ollama.test.ts` を
DESIGN-0026 どおりに実装し、稼働中の Ollama コンテナに対して
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

- `docs/plans/0021-ollama-real-provider-worker-e2e.md`
- `docs/designs/0026-ollama-real-provider-worker-e2e.md`
- `docs/specs/0021-ollama-real-provider-worker-e2e.md`
- `docs/adr/0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md`

## Dependencies

- なし（本 chain の先頭 task）

## Affected Files

- `packages/cli/test/e2e/real-provider-ollama.test.ts`（新規）

## Done When

- [ ] test file が DESIGN-0026 どおりに実装されている
- [ ] Ollama 稼働環境で 3 worker test が実 provider で pass
- [ ] `meta.model.provider === "ollama"`・usage 正値・成果物が
      assert されている

## Verification

- [ ] `vitest run test/e2e/real-provider-ollama.test.ts`（live）pass
- [ ] `pnpm typecheck`・`pnpm lint` pass

## Verification results

（未実施）
