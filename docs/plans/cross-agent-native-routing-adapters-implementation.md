---
id: "PLAN-034rAaIojIXLbCI0pJGpYg"
type: "plan"
status: "approved"
title: "Cross-agent native routing adapters implementation"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/43"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/cross-agent-native-routing-adapter-architecture.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cross-agent-adapter-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    horizon: "single-chain"
---

# Cross-agent native routing adapters implementation Implementation Plan

## Goal

canonical routing request + shared policy module + per-target adapter 構造を
`packages/apm/.apm/hooks/` に実装し、Windsurf / Claude Code / Cursor /
Copilot / Codex の native hook event が shared large-read policy へ到達
するようにする。

## Scope

単一 implementation stream。`packages/apm/.apm/hooks/` の adapter 群と
`hooks.json`、それに対応する contract / install E2E test、matrix・guide
更新までを一つの chain で完結させる。#42 の shell parser 本体と Codex
file-read interception は scope 外。

## Upstream Documents

- `docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md`
  (SPEC-034rzTJydXfLvQhTIqiDqW, approved)
- `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`
  (ADR-034rzTJYDRmre6D5OS8HDW, accepted)
- `docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md`
  (DISC-034rA43JiB611pUjLPA4yR, resolved)

## Design Inputs

- `docs/designs/cross-agent-native-routing-adapter-architecture.md`
  (DESIGN-034rA79Uog7JOa7evw6rOL, approved)

## File Map

- `packages/apm/.apm/hooks/routing-policy.js` — canonical request
  （`file-read` / `shell`）と shared policy `evaluate()`、deny reason
  formatter、threshold / CLI 解決 helper。
- `packages/apm/.apm/hooks/pre-read-code.js` — Windsurf `pre_read_code`
  adapter。判定を policy 呼出しへ移譲（挙動不変）。
- `packages/apm/.apm/hooks/pre-run-command.js` — Windsurf
  `pre_run_command` adapter（shell request → policy、新規）。
- `packages/apm/.apm/hooks/pre-read-code-pretooluse.js` — Claude Code +
  Codex 共用 adapter（`tool_name`/`tool_input`、deny は
  `hookSpecificOutput` JSON、新規）。
- `packages/apm/.apm/hooks/pre-read-code-cursor.js` — Cursor adapter
  （`beforeReadFile` / `beforeShellExecution`、`{permission}` JSON、
  常に schema 適合応答、新規）。
- `packages/apm/.apm/hooks/pre-read-code-copilot.js` — Copilot adapter
  に shell tool mapping と foreign-shape allow 応答を追加。
- `packages/apm/.apm/hooks/hooks.json` — event 登録更新
  （`pre_run_command` / `PreToolUse` nested group / `beforeReadFile` /
  `beforeShellExecution` / `preToolUse` matcher）。
- `packages/cli/test/e2e/pre-read-hook.test.ts` — 既存 Windsurf
  regression（変更不要のはず）。
- `packages/cli/test/e2e/copilot-pre-read-hook.test.ts` — Copilot
  regression + shell mapping test 追加。
- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts` — 新規
  adapter の contract test（pretooluse / cursor / windsurf shell）。
- `packages/cli/test/e2e/apm-install-targets.test.ts` — per-target
  generated config / adapter deploy assertion 拡張。
- `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md` —
  matrix 更新（Codex re-verification 結果、APM pass-through 実測）。
- `docs/guides/parent-agent-integration.md` — per-target routing
  support / fail semantics 記述更新。

## Implementation Sequence

- [ ] routing-policy.js に canonical request / evaluate / deny formatter
      を実装し、pre-read-code.js（windsurf）を移譲へ書き換える
- [ ] pre-run-command.js / pre-read-code-pretooluse.js /
      pre-read-code-cursor.js を新規実装、pre-read-code-copilot.js を拡張
- [ ] hooks.json の event 登録を更新する
- [ ] adapter contract test を追加し、既存 Windsurf/Copilot test を維持する
- [ ] apm-install-targets.test.ts を per-target assertion に拡張する
- [ ] matrix と parent-agent guide を実装・実測結果へ更新する
- [ ] `mise run check` を実行して pass を確認する

## Dependencies and Risks

- Copilot `.json` asset 欠落は upstream 制限として継続（drift assert 維持）。
- Cursor permission hook の「exit 0 + schema 不適合は block」に備え、
  cursor adapter は全経路で `{permission:"allow"|"deny"}` を返す。
- Claude target で flat `preToolUse` が `PreToolUse`+matcher `*` へ
  正規化される挙動は APM 実測済み。matcher の lowercase 限定で
  cross-fire を抑止するが、case-insensitive matcher の場合は adapter 側
  shape 判定が no-op に倒す。
- Codex `PreToolUse` deny の runtime 解釈は実機未検証。best-effort
  deny + fail-open を matrix に記録する。

## Task Breakdown

- `docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md`
- `docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md`
- `docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md`
- `docs/tasks/contract-matrix-and-parent-agent-guide-updates.md`

## Verification

- [ ] `pnpm test`（adapter contract / copilot / windsurf / apm install
      e2e を含む）が全件 pass
- [ ] `pnpm typecheck` / `pnpm lint` が pass
- [ ] `mise run check` が pass

## Open Questions / Gaps

- Codex `PreToolUse` の `hookSpecificOutput` deny が runtime で確実に
  解釈されるかは公式 doc の example では未確認。unsupported field での
  hook failure は Codex が tool call を継続するため fail-open 側に倒れる。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（同一 worktree・同一 agent で
  adapter → test → doc の順に実施）
- Independent work streams: なし（adapter 実装 → hooks.json → test →
  doc の順序依存）
- User approval required before dispatch: no（Orchestrator 委任により
  自律継続）
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 重点確認: adapter に判定ロジックの複製がないこと、Copilot
  never-nonzero の維持、Cursor の schema 適合応答、generated config が
  各 target の native event を含むこと。
