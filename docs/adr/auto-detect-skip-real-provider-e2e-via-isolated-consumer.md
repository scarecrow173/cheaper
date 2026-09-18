---
id: "ADR-034r05iHrZUE6WjfsVTWjO"
type: "adr"
status: "accepted"
title: "real-provider e2e は Ollama auto-detect skip + apm install 隔離 consumer で実 provider 到達を検証する"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/ollama-real-provider-worker-e2e.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by:
    - "../designs/ollama-real-provider-worker-e2e.md"
  depends-on:
    - "./use-ollama-on-docker-as-real-provider-for-smoke-verification.md"
    - "./separate-dev-apm-project-via-nested-distributable-package.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../impl/ir/run-real-provider-smoke-test-with-ollama.md"
    - "../designs/cross-agent-distribution-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/ollama-real-provider-worker-e2e.md"
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

# 24. real-provider e2e は Ollama auto-detect skip + apm install 隔離 consumer で実 provider 到達を検証する

## Context and Problem Statement

SPEC-034r05iHK8bA7guAnTdpdi は built-in worker の実 provider e2e を要求する。決めるべき
こと: (a) 実行条件（skip vs opt-in flag）、(b) 実行対象（repo の
`bin/cheaper.js` vs apm install した bundled CLI）、(c) model /
provider 設定の fixture 化、(d) CI との分離。

## Decision Drivers

- Issue #12: 「`apm install` した隔離環境で」「コンテナ未起動時は
  skip または明示的 opt-in」「CI 必須ゲートとは分離」を明示する。
- IMPL-034r05iHy2gdnpqyE8VXmQ の実測 recipe（models.json custom provider・
  `reasoning_effort:"none"`/`temperature:0`・policy 延長・
  preferences.low への pin）が確立済みで、fixture 化すれば
  deterministic に再現できる。
- `qwen3.5:9b` は built-in classify 表で low に match しない
  （`\b(1|3|7|8)b\b` のみ、9b は非該当）→ overrides で low に
  分類する必要がある。
- CI（GitHub Actions）には Docker/Ollama が無い → auto-detect で
  skip すれば分離は自動に達成される。

## Considered Options

### 実行条件

- **A: opt-in env flag**（`CHEAPER_E2E_OLLAMA=1` が無いと skip）。
  意図しない実行を完全に防ぐが、flag を知らないと実 provider 経路が
  永久に未検証のままになりやすい。
- **B: auto-detect skip**（採用）。`/v1/models` 到達 + model 存在を
  probe し、どちらか欠ければ skip。container があれば自動で実検証、
  無ければ deterministic skip — issue の「skip または opt-in」の
  前者を満たし、CI 分離も自動。

### 実行対象

- **i: repo `bin/cheaper.js`** を隔離 env で実行。速いが配布形態を
  検証しない。
- **ii: `apm install` した bundled `cheaper.mjs`**（採用）。issue
  指定どおり、consumer が実際に受け取る artifact（bundle・worker
  definition・policy.yaml）ごと検証する。

## Decision Outcome

- **auto-detect skip**: beforeAll で
  `GET http://localhost:11434/v1/models` を probe し、到達不能・
  対象 model 不在・apm CLI 不在のいずれかで `describe` 全体を
  skip する。opt-in flag は設けない。
- **bundled CLI 経由**: `makeConsumerLayout` + `apmInstall`
  （windsurf target — `.json` stripping の無い target）で得た
  `bundledCli`（`.windsurf/hooks/pkg/cli/bin/cheaper.mjs`）を
  consumer 内で実行する。repo の `.apm/hooks`・skill は使わない。
- **config fixture（tmpdir 内に全て生成）**:
  - `<root>/pi-agent/models.json`: `ollama` provider
    （`api:"openai-completions"`・`baseUrl:http://localhost:11434/v1`
    ・`apiKey:"ollama"` literal・model に
    `samplingParams:{reasoning_effort:"none",temperature:0}`・
    `cost`/`contextWindow`/`maxTokens`）— IMPL-034r05iHy2gdnpqyE8VXmQ recipe の固定。
  - `<root>/cheaper-config/models.yaml`:
    `overrides:{"ollama/<model>":low}` で tier を low に pin し、
    `preferences.low:["ollama/<model>"]` で第一候補にする（built-in
    表で 9b は low に match しないため overrides が必要）。
  - `<root>/cheaper-config/policy.yaml`: `execution.timeout:10m`・
    `maxTurns:16`・`output.repairAttempts:4`（小モデルの repair
    収束に必要 — IMPL-034r05iHy2gdnpqyE8VXmQ 実測値）。
  - env: `PI_CODING_AGENT_DIR=<root>/pi-agent`・
    `CHEAPER_CONFIG_DIR=<root>/cheaper-config`・`HOME/USERPROFILE=
    <root>` で実 user config から完全隔離。
- **model 選択**: default `qwen3.5:9b`（IMPL-034r05iHy2gdnpqyE8VXmQ で schema 適合
  確認済み。4b は不適合だったため default にしない）、env
  `CHEAPER_E2E_OLLAMA_MODEL` で override 可。
- **worker 検証**: `bulk-read`（findings）・`code-write`（
  changes.files + 実 file 作成）・`doc-write`（同）を実行し、
  `meta.model.provider==="ollama"`・`meta.usage` の正の token 数を
  assert。出力内容の品質は検証しない（到達 + 契約 shape のみ）。

## Consequences

- Ollama 稼働環境では `pnpm test` に実 provider 検証が自動で入る
  （各 worker 実行に数十秒〜数分かかり得る — test timeout を明示）。
- CI では probe が即座に fail → skip、既存ゲートに影響しない。
- model 更新・Ollama 仕様変更があると recipe（samplingParams 等）の
  見直しが必要になる — fixture に recipe 由来の comment を残す。
- bundled CLI 経由のため、consumer 配布形態（worker .md・schema・
  policy.yaml 同梱）も同時に検証される。

## Confirmation

- Ollama 稼働環境で test 実行 → 3 worker が exit 0・ok envelope・
  `meta.model.provider === "ollama"`・usage 実測値。
- `docker compose down` 相当の未到達状態で同 test が skip。
- `pnpm test`・`mise run check` が Ollama 不在でも green。
