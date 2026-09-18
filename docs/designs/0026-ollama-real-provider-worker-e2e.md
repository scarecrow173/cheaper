---
id: "DESIGN-0026"
type: "design"
status: "approved"
title: "Real-provider worker E2E via Ollama"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0021-ollama-real-provider-worker-e2e.md"
    - "../adr/0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0021-ollama-real-provider-worker-e2e.md"
  implemented-by:
    - "../plans/0021-ollama-real-provider-worker-e2e.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./0018-cross-agent-distribution-verification.md"
    - "../impl/ir/0008-run-real-provider-smoke-test-with-ollama.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/0024-auto-detect-skip-real-provider-e2e-via-isolated-consumer.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Real-provider worker E2E via Ollama

## Context

SPEC-0021 / ADR-0024 の実装設計。`packages/cli/test/e2e/` に新規
test file を追加し、apm install した隔離 consumer の bundled CLI
が Ollama provider へ実到達することを検証する。

## Scope

### In Scope

- `packages/cli/test/e2e/real-provider-ollama.test.ts`（新規）
- AGENTS.md「検証用 Ollama コンテナ」節への 1 行追記（e2e が
  コンテナ起動時に自動で実 provider 検証を行う旨）

### Out of Scope

- 実装コード（runtime/CLI）の変更 — 検証のみ
- CI workflow の変更（auto-skip で分離済み）

## Design

### Test file 構成

```
describe("real-provider e2e (Ollama)", () => {
  let available: boolean;   // probe 結果
  let root, consumer, cli;  // consumer layout + bundled CLI
  env = { PI_CODING_AGENT_DIR, CHEAPER_CONFIG_DIR, HOME, USERPROFILE, PATH... }
})
```

**beforeAll**:

1. `apmAvailable()` → false なら `available=false`。
2. `fetch("http://localhost:11434/v1/models")`（timeout ~3s、失敗は
   catch）→ `data[].id` に対象 model（env
   `CHEAPER_E2E_OLLAMA_MODEL` or default `qwen3.5:9b`）が無ければ
   `available=false`。
3. 両方 OK なら `makeConsumerLayout(root, "windsurf")` +
   `apmInstall` → `bundledCli`。
4. config fixture 書き込み:
   - `<root>/pi-agent/models.json`（ollama provider、IMPL-0008
     recipe: `reasoning_effort:"none"`・`temperature:0`・
     cost/contextWindow/maxTokens）
   - `<root>/cheaper-config/models.yaml`:
     `version:1` + `overrides:{"ollama/<model>":low}` +
     `preferences.low:["ollama/<model>"]`
   - `<root>/cheaper-config/policy.yaml`:
     `execution.timeout:"10m"`・`maxTurns:16`・
     `output.repairAttempts:4`
5. `env` 構築: `PI_CODING_AGENT_DIR`・`CHEAPER_CONFIG_DIR`・
   `HOME/USERPROFILE=<root>`・`NODE_ENV=production`（fake stream
   無効化と同じ条件）。

**各 test**（`it`）は先頭で `if (!available) t.skip(...)`:

- **bulk-read**: consumer 内に `src/greeter.ts` を配置し
  `{"task":"...","paths":["src/greeter.ts"],"maxFindings":3}` を
  stdin で実行 → exit 0・envelope ok・`worker==="bulk-reader"`・
  `meta.model.provider==="ollama"`・`meta.usage.input>0`・
  findings array 存在。
- **code-write**: `{"task":"...","targets":[{"path":"src/farewell.ts","intent":"create"}]}` → exit 0・ok・`meta.changes.files` 非空・
  file が実際に disk 上に存在。
- **doc-write**: `{"task":"...","documentType":"readme","targets":[{"path":"docs/hello.md","intent":"create"}]}` → exit 0・ok・file 存在。

実行は `spawnSync`/`spawn` で `node <bundledCli> <worker> --cwd <consumer>`。timeout は `it` 第 3 引数で 10 分（小モデルの repair 収束に余裕を持たせる）。

### assert の範囲

到達 + 契約 shape のみ: exit 0、`expectEnvelopeOnly` 相当の単一
envelope、`ok===true`、provider id、usage 正値、worker 固有の
成果物（findings 非空 / file 作成）。出力の内容品質・token 数値の
大きさは assert しない（小モデル依存で flaky になりうるため）。

## Test Design

- Ollama 未到達 / model 不在 / apm 不在 → describe 内全 test skip
- Ollama 稼働 → 3 test が実 provider で pass
- 回帰: 既存 e2e（fake stream 系）は env 分離で無影響
  （PI_CODING_AGENT_DIR・CHEAPER_CONFIG_DIR は test 固有 tmpdir）

## Verification

- Ollama 稼働環境で `vitest run test/e2e/real-provider-ollama.test.ts`
- `docker compose stop ollama` 相当の未到達状態で同 test が skip
- `pnpm typecheck`・`pnpm lint`・`pnpm test` 全件（skip 経路含む）
- `audit_docs.js --type all`
