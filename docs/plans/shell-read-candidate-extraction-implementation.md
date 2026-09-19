---
id: "PLAN-034rBrrA2j5usWH2G77b8G"
type: "plan"
status: "approved"
title: "Shell read-candidate extraction implementation"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/42"
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
    - "docs/designs/shell-read-candidate-extraction.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/shell-read-blocking-contract-tests.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    horizon: "single-chain"
---

# Shell read-candidate extraction implementation Implementation Plan

## Goal

`routing-policy.js` の `evaluateShell` に保守的な shell read-candidate
抽出を実装し、`cat`/`head`/`tail`/`less`/`more` 経由の巨大 file read を
canonical large-read 判定で block する。

## Scope

単一 implementation stream。変更は `routing-policy.js` 1 file の policy
実装、contract test、guide 更新の 3 task で完結する。adapter 層・
`hooks.json`・bundle pipeline は #43 の構造を維持し変更しない。

## Upstream Documents

- `docs/specs/shell-read-large-file-blocking.md`
  (SPEC-034rBhR3SNIRFZ1B6n26iH, approved)
- `docs/adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md`
  (ADR-034rBhRB7QyWkAxRiTDXXq, accepted)
- `docs/discovery/shell-read-large-file-detection-boundary.md`
  (DISC-034rBfZ3dqNmEQGXCIwiLn, resolved)

## Design Inputs

- `docs/designs/shell-read-candidate-extraction.md`
  (DESIGN-034rBoPPFNHXR5eSfv0hsO, approved)

## File Map

- `packages/apm/.apm/hooks/routing-policy.js` — `tokenizeShell` /
  `READ_COMMANDS` table / `extractReadCandidates` / `evaluateShell`
  実装。`evaluateFileRead` への判定委譲と command 入り deny reason。
- `packages/cli/test/e2e/cross-agent-routing-adapters.test.ts` — shell
  read blocking の contract test 拡張（issue の test リストを網羅）。
- `docs/guides/parent-agent-integration.md` — shell 経由検出・target 別
  対応・fail-open/fail-closed・既知制限の反映。

## Implementation Sequence

- [ ] `routing-policy.js` に tokenizer / operand 抽出 / `evaluateShell`
      を実装する
- [ ] contract test を拡張し代表 command の allow/deny を固定する
- [ ] `parent-agent-integration.md` を更新し `mise run check` を通す

## Dependencies and Risks

- tokenizer の誤 block リスク: 疑わしい構文（`$`・redirect・glob・
  未閉鎖 quote）は segment skip で fail-open に倒す設計で緩和。
- `cd src && cat huge.ts` は hook cwd 基準で解決する既知制限。
- test は stdin 注入 contract test で網羅し、実 agent binary への
  依存を持たない従来方針を維持する。

## Task Breakdown

- `docs/tasks/shell-read-candidate-extractor-in-routing-policy.md`
- `docs/tasks/shell-read-contract-tests.md`
- `docs/tasks/shell-read-docs-and-final-verification.md`

## Verification

- [ ] `pnpm --filter @cheaper/cli test` で hook contract test を含む
      全件 pass
- [ ] `mise run check` が pass

## Open Questions / Gaps

- なし（要件・構造ともに spec/adr/design で固定済み）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（単一 file 中心の小規模変更）。
- Independent work streams: なし（policy 実装 → test → docs の順序依存）。
- User approval required before dispatch: no（Orchestrator 委任により自律継続）。
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 重点確認: shell 側に threshold・`count-tokens`・`FILE_TOO_LARGE`
  解釈の複製がないこと、fail-open 境界（解釈不能 segment・infra
  failure）が既存 contract と一致すること、deny diagnostics が
  issue 要件 7 を満たすこと。
