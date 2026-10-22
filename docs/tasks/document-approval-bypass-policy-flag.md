---
id: "TASK-034KsL2829yYfRRGKl92t1"
type: "task"
status: "done"
title: "Document route.rewrite.approvalBypass in configuration guide"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/configuration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/tasks/register-cursor-pretooluse-hook.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/guides/configuration.md"
  defers: []
  deferred-by: []
---

# Document route.rewrite.approvalBypass in configuration guide

## Work

plan `docs/plans/implement-per-target-hook-rewrite-enablement.md`
(PLAN-034KsKo6nYOBOpIpkNoviY)の T5。`docs/guides/configuration.md`
の `policy.yaml` section に `route` section の記法・trust 境界・
per-target 検証状況を追記し、`mise run check` で plan を閉じる。

### 追記内容

- `route` section の記法:
  `route: {rewrite: {approvalBypass: [codex]}}` — bypassing emit
  (affirmative permission decision 併記の rewrite 応答)を許可する
  hook target 名の配列。default `[]`、未指定・`null`・空配列は
  等価。値 domain は `codex`/`cursor`/`copilot` で、未知値・重複・
  非配列・非文字列 item・非 object の `rewrite`/`route` は
  `InvalidRuntimePolicy` で fail-loud。
- trust 境界: `route` section は builtin/user-scope の
  `policy.yaml` でのみ authoring 可能 — project
  `.cheaper/policy.yaml` に `route` key を含めると policy load が
  `InvalidRuntimePolicy` で失敗する(`command` section と同型の
  構造保証。untrusted checkout が approval bypass を自己宣言
  できない)。
- per-target 検証状況:
  - codex — `approvalBypass` に列挙した場合のみ
    `permissionDecision:"allow"` + `updatedInput` を emit する
    (実機確定済み、codex-cli 0.162.1)。この emit は親 agent の
    approval prompt を素通しするが、codex 自身の execpolicy は
    迂回しない(rewritten command も `blocked by policy` 評価を
    受ける — 「policy opt-in で全検査を迂回する」誤解を防ぐ)。
  - cursor/copilot — emit contract が実機未検証のため、flag に
    列挙しても現在は no-op(fallback render を維持)。「flag は
    許可であり、有効化は emit contract 確定後」と明記。
  - claude(`pretooluse`)・devin — approval を温存する emit を
    既に持つため flag 不要・対象外。
- hook trust の注記: codex では未 review hook は実行されない
  (`--dangerously-bypass-hook-trust` 等の trust 確立が前提)旨に
  触れる。

追記位置は `command` section の記述(`command.grants` の
trusted-layer 限定と同じ段落群)の後あたり、`route` の独立
subsection か `policy.yaml` section 内の項として。記述トーンは
既存 guide(日本語、見出し・表の形)に揃える。

### 最終検証

`mise run check`(biome・fmt・clippy・version/doc-id・cargo test・
node --test・lockfile・apm payload)を全 green で通し、plan の
Verification 全項目を完了条件とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-per-target-hook-rewrite-enablement.md` — 本
  task が implement する plan(Dependency Graph の T5)
- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1)— doc 追記内容の要件(記法・
  trust 境界・検証状況)
- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy)— AC「user-facing doc が更新
  される」
- `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
  (ADR-034KrYGRHZuvOIx1d0S0nE)— D1・D6(命名・配置)

## Dependencies

- `docs/tasks/register-cursor-pretooluse-hook.md`(T3) — cursor
  登録の検証状況記述が T3 成果を前提。
- `docs/tasks/add-codex-rewrite-integration-tests.md`(T4) —
  codex emit の検証済み状況記述が T4 成果を前提。

## Affected Files

- `docs/guides/configuration.md`(modify — `policy.yaml` section
  に `route` 記述を追記)

## Done When

- [x] `route.rewrite.approvalBypass` の記法・default・値 domain・
      fail-loud が記述される
- [x] trust 境界(builtin/user のみ authoring 可、project 層は
      `route` key で load error)が `command` 先例と同型に記述
      される
- [x] per-target 検証状況(codex 実機確定済み・cursor/copilot
      検証 gated で現状 no-op・claude/devin flag 不要)と
      execpolicy 非迂回・hook trust が記述される
- [x] `mise run check` が全 green で plan を閉じる

## Out of Scope

- `AGENTS.md`・routing 系 guide の更新(本 issue の user-facing
  surface は configuration.md の `route` section のみ —
  routes.yaml 記法は既存 doc の責務)
- 実装・test(T1-T4)
- cursor/copilot emit 有効化時の doc 追記(その issue で更新)

## Verification

- [x] `mise run check` exits 0(CI 同等 pipeline 全 green)
- [x] `node scripts/check-doc-ids.mjs` exits 0(doc-id 規約)
- [x] `git diff docs/guides/configuration.md` に `route` section の
      記述が含まれる(目視 supplement)

## Done

2026-10-23 実施。`docs/guides/configuration.md` の `policy.yaml`
section に `route` の記述を追記し、plan を閉じた。

- `policy.yaml` の完全値 sample に builtin `resources/policy.yaml`
  と同じ `route: {rewrite: {approvalBypass: []}}` block を末尾へ
  追記(配置も builtin と同じ retry の後)。
- `### route.rewrite.approvalBypass（hook rewrite の approval
  opt-in）` subsection を `command` section 記述の直後に追加 —
  記法・default `[]`/未指定・null・空配列の等価・値 domain
  (codex/cursor/copilot)・fail-loud 検証・builtin/user 限定の
  trust 境界(project `route` key で load 失敗)・per-target 現状
  表(codex 実機確定済み 0.162.1 / cursor・copilot は emit
  contract 未検証で no-op / claude・devin は flag 不要)・
  execpolicy 非迂回・hook trust 前提を記述。
- companion fix: T3 で追加した cursor `preToolUse` に対し
  `scripts/test/lib/apm-fixture.mjs` の TARGET_NATIVE_EVENTS と
  `apm-install-targets.test.mjs` の expectedScript を更新
  (node --test の install smoke が旧 event 集合を pin していた
  ため 1st `mise run check` で検出 — commit 03e3576)。
- 検証: `mise run check` exits 0 — biome・cargo fmt --check・
  clippy -D warnings・version/doc-id・cargo test --workspace・
  node --test scripts(43 pass)/benchmarks(12 pass)・lockfile・
  apm payload 全 green。
