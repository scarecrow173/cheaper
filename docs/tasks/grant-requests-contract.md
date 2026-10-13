---
id: "TASK-034FpqpiEJPjTB70PuT2jp"
type: "task"
status: "todo"
title: "T4: grant-requests-contract — tool-operator output schema の grantRequests と prompt contract"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/invocation-scoped-command-grants.md"
  references: []
  defers: []
  deferred-by: []
---

# T4: grant-requests-contract — tool-operator output schema の grantRequests と prompt contract

## Scope

tool-operator の output schema に optional top-level `grantRequests[]`
field を追加し、worker prompt に emit 条件の contract を記述する
（design §(e)）。`grantRequests` は machine-readable な advisory
request であり、parent agent が blocked → approval → `--grant` re-run
loop を組み立てる粒度を持つ。schema file と worker file は他 task と
file 非交差のため、本 task は完全独立に実行できる。

- In: `tool-operator.output.schema.json` への `grantRequests` 追加
  （strict-mode 適合維持）、`tool-operator.md` `## Output rules` への
  emit 条件条項（英語 — worker file 言語規約）。
- Out: emit 結果の検証（T5 — `run_fake_stream_tool_operator_blocked_
  emits_grant_requests` が envelope `result` parse + schema 適合を
  固定）、`grantRequests` の実行（request は grant の実行ではなく
  invoker への提示）、docs/APM の loop 記述（T6）。

### 所有ファイル

- `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`
  — optional top-level `grantRequests`（design §(e) verbatim —
  `maxItems: 20`、item は `required: ["executable"]` +
  `argvHint?: string[]`（items maxLength 4096・maxItems 256）+
  `name?: string`（grant `propertyNames` と同一 pattern
  `^[a-z][a-z0-9_-]{0,63}$` + `maxLength: 64`）、全階層
  `additionalProperties: false`）。
- `crates/cheaper-cli/resources/workers/tool-operator.md` —
  `## Output rules`（:51-）へ emit 条件条項を英語で追加: gated tool
  欠如由来の `blocked`/`partial` のみ populate する、entry は
  `executable` 必須 + `argvHint`/`name` optional、request は実行すべき
  grant ではなく invoker への提示である、全 field は model 生成の
  prompt-injectable な untrusted 値である旨。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T4）
- `docs/designs/invocation-scoped-command-grants.md` — §(e) の
  `grantRequests` shape
- `docs/specs/invocation-scoped-command-grants.md` — AC5
- `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
  — `grantRequests` shape・advisory 性（request 由来 spec の review は
  parent agent / user の責務 — Risk #7）
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （schema 適合の行為保証。emit 条件の prompt 遵守自体は test-spec の
  Non-goals — fake stream は emit 済み結果の受理のみを検証する）

## Dependencies

- なし（plan のみ。schema/worker file は他 task と file 非交差）

## Steps

1. `tool-operator.output.schema.json` に design §(e) verbatim shape の
   `grantRequests` を追加し、strict-mode 規約（`additionalProperties:
   false` 全階層）を維持する。
2. `tool-operator.md` の `## Output rules` に emit 条件条項を英語で
   追加する（emit 条件・field 要件・advisory・untrusted 明記）。
3. `builtin_workers` test（`all_builtin_workers_resolve_with_schemas`）
   が extended schema を strict-mode で compile して pass することを
   確認する。

## Done When

- [ ] `tool-operator.output.schema.json` が `grantRequests`（
  `required: ["executable"]` + optional `argvHint`/`name`、
  `additionalProperties: false`）を持ち、
  `all_builtin_workers_resolve_with_schemas` が strict-mode compile
  で pass する
- [ ] `tool-operator.md` が emit 条件（gated tool 欠如由来の
  `blocked`/`partial` のみ）と request の advisory / untrusted 性を
  英語で明記する

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --test builtin_workers` exits 0
