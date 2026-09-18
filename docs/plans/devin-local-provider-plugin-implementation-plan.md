---
id: "PLAN-034r05iHK8bA7g45xvDT4a"
type: "plan"
status: "completed"
title: "Devin Local provider plugin — implementation plan"
test-spec-skip: "Verified by unit tests inside the implementation tasks; no separate test-spec artifact needed."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/devin-local-bundled-provider.md"
    - "../designs/devin-local-provider-plugin.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/devin-local-provider-plugin.md"
  implemented-by:
    - "../tasks/vendor-devin-protocol-modules-and-cli-credentials-adapters.md"
    - "../tasks/devin-provider-plugin-registration-and-tests.md"
    - "../tasks/devin-docs-status-sync-bundle-and-final-verification.md"
  depends-on:
    - "../adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/devin-local-provider-plugin.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "medium"
    verification-policy: "task-verified"
---

# Devin Local provider plugin — implementation plan

## Goal

`devin` provider を bundled `CheaperProviderPlugin` として
実装する。認証・catalog は Devin CLI（isolated data dir 環境
の child process）に委譲し、transport は upstream `pi-devin`
（MIT）vendored の Connect-RPC stream とする。

## Scope

1 つの implementation stream（vendored module → CLI/
credentials/model adapter → plugin 登録 → test → docs）。
分割不要。

## Upstream Documents

- `docs/specs/devin-local-bundled-provider.md`
- `docs/adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md`

## Design Inputs

- `docs/designs/devin-local-provider-plugin.md`
- upstream `pi-devin`（kashyab12、MIT）src 一式

## File Map

- `packages/runtime-pi/src/providers/devin/wire.ts`（vendored）
- `packages/runtime-pi/src/providers/devin/metadata.ts`（vendored）
- `packages/runtime-pi/src/providers/devin/jwt.ts`（vendored）
- `packages/runtime-pi/src/providers/devin/context-map.ts`（vendored）
- `packages/runtime-pi/src/providers/devin/stream.ts`（vendored・
  apiKey/options 経路はそのまま）
- `packages/runtime-pi/src/providers/devin/cli.ts`（adapted・
  runner seam・isolated child env）
- `packages/runtime-pi/src/providers/devin/credentials.ts`
  （adapted・cheaper data dir のみ）
- `packages/runtime-pi/src/providers/devin/models.ts`（adapted・
  catalog parse + FALLBACK bootstrap）
- `packages/runtime-pi/src/providers/devin/plugin.ts`（新規・
  `createDevinProviderPlugin`）
- `packages/runtime-pi/src/provider-plugin.ts`・`index.ts`
  （registry 登録 + export）
- `NOTICE` または同等 attribution
- `packages/runtime-pi/test/providers/devin/`（test）
- `docs/guides/providers.md`（Devin セクション）

## Implementation Sequence

- [ ] TASK-034r05iHWkNF7zNyXgVkrI: vendored protocol module + CLI/credentials/
      models adapter + attribution
- [ ] TASK-034r05iHWkNF7zNWvK4AnL: plugin registration + unit test 一式
- [ ] TASK-034r05iHWkNF7zPe6DL42d: docs + status sync + bundle regen + final
      verification

## Dependencies and Risks

- vendored stream の protocol 正確性は upstream に依る —
  MIT attribution と出所を明記し、改変は credentials/cli
  経路に限定する。
- `XDG_DATA_HOME` env override が Devin CLI の実際の
  credential path と一致することを前提とする（issue の
  想定構成 `data/devin/credentials.toml` と整合）。
  win32 の `LOCALAPPDATA` 併用は保険。
- real Devin account 依存の検証は opt-in smoke のみ —
  automated test は fake runner で完結させる。
- `refreshModels` の失敗は Pi 側の refresh error として
  記録され、bootstrap model が残る（upstream 同様）。

## Task Breakdown

- TASK-034r05iHWkNF7zNyXgVkrI → TASK-034r05iHWkNF7zNWvK4AnL → TASK-034r05iHWkNF7zPe6DL42d（直列）

## Verification

- [ ] `pnpm lint` / `pnpm typecheck` / `pnpm test` green
- [ ] fake runner で login/catalog/env 隔離を検証
- [ ] `registerProvider("devin", ...)` が plugin 経由で呼ばれる
- [ ] `node scripts/bundle-cli.mjs` で bundle 再生成、
      freshness clean

## Open Questions / Gaps

- Windows での Devin CLI の実 credential path は未検証 —
  `XDG_DATA_HOME` + `LOCALAPPDATA` の併用でカバーし、実環境
  差異は docs の experimental caveat に記載する。
- `devin` binary が存在しない環境では plugin provider は
  bootstrap model のみ・auth 未configured として見える
  （actionable error は login/catalog 呼び出し時）。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- vendored file の改変差分（upstream vs vendored）を確認
  し、credential/global path 参照が残っていないことを重点
  的にレビューすること。
