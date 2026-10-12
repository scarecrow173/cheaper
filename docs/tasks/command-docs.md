---
id: "TASK-034EOHYMMH4FnfoVhmfPeP"
type: "task"
status: "todo"
title: "T6: command tool の user-facing docs と index 更新"
created: "2026-10-12"
updated: "2026-10-12"
owners: []
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
    - "docs/plans/implement-policy-gated-command-tool.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-policy-gated-command-tool.md"
    - "docs/tasks/runtime-cli-plumbing.md"
    - "docs/tasks/command-tests.md"
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
    - "docs/test-specs/policy-gated-command-tool.md"
  references: []
  defers: []
  deferred-by: []
---

# T6: command tool の user-facing docs と index 更新

## Scope

`command` tool の user-facing 文書を整備する。T4 で確定した builtin
`policy.yaml` 最終形と T5 で固定した挙動を正確に mirror する。

### 所有ファイル

- `docs/guides/configuration.md` — builtin `policy.yaml` verbatim embed
  （:93-135）に `command` section と `tools.allow` の `command` を同期更新。
- `docs/guides/workers.md` — gate 記述（:37-38・:99）を `tools` 宣言 ∩
  `tools.allow` ∩ `command.grants` の三重ゲートへ更新し、`tools.allow` が
  exposure toggle であり authorization ではない（exposure ≠ authorization）点を
  明記。信頼境界は `command.grants` の trusted-layer-only authoring に集約。
- `docs/guides/command.md`（新規）または `docs/guides/managed-process.md` 拡張 —
  trust / sandbox 限界（OS sandbox 無し、`filesystem.denyPaths`・workspace
  boundary・writeScope が子の副作用を bound しない）、ambient PATH
  provenance（pin 時点の ambient PATH を信頼）と per-scope pin の残余
  TOCTOU、argv pattern 言語、config-driven executable の capability 限界と
  grant `env` の注入変数注意（`LD_PRELOAD`・`NODE_OPTIONS` 等）、deny list は
  containment でなく known-bad fail-fast、`process.tasks` の project extend と
  の divergence、writeScope 非両立。
- `docs/README.md` + root `README.md` — 新規 `command.md` を作成する場合、
  docs index と root README の guide links に追加。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-policy-gated-command-tool.md` — 実装対象 plan（T6）
- `docs/designs/policy-gated-command-tool.md` — Non-Functional Constraints・
  Risks and Trade-offs（docs 必須記載事項）
- `docs/specs/policy-gated-command-tool.md` — AC7
- `docs/test-specs/policy-gated-command-tool.md` — 検証 spec

## Dependencies

- `docs/tasks/runtime-cli-plumbing.md`（T4 — builtin `policy.yaml` 最終形）
- `docs/tasks/command-tests.md`（T5 — docs が記述する挙動の確定）

## Steps

1. `configuration.md` の verbatim embed を builtin `policy.yaml` と同期する。
2. `workers.md` の gate 記述を三重ゲートへ更新し、exposure ≠ authorization を
   明記する。
3. `command.md` 新規作成または `managed-process.md` を拡張し、trust /
   sandbox 限界・PATH provenance・argv pattern・capability 限界・
   divergence・writeScope 非両立を記述する。
4. 新規 `command.md` を作成した場合、`docs/README.md` index と root
   `README.md` の guide links を更新する。

## Done When

- [ ] trust / sandbox 限界・ambient PATH provenance・capability 限界・
  `process.tasks` divergence・三重ゲートが docs に記載されている
- [ ] `configuration.md` の verbatim embed が builtin `policy.yaml` と同期
- [ ] `command.md` 新規作成時は `docs/README.md` と root `README.md` の
  index/link が更新されている

## Verification

- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type all` exits 0
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `grep -n "command:" docs/guides/configuration.md` returns matches
  （builtin `policy.yaml` との差分がないこと）
