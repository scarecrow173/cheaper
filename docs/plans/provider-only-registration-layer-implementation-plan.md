---
id: "PLAN-034r05iHK8bA7g2BMsfjFu"
type: "plan"
status: "completed"
title: "Provider-only registration layer — implementation plan"
test-spec-skip: "Verified by unit tests inside the implementation tasks; no separate test-spec artifact needed."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/provider-only-registration-layer.md"
    - "../designs/provider-only-registration-layer.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/provider-only-registration-layer.md"
  implemented-by:
    - "../tasks/add-provider-plugin-contract-and-registration-order.md"
    - "../tasks/provider-plugin-tests-and-resource-isolation.md"
    - "../tasks/docs-status-sync-bundle-and-final-verification.md"
  depends-on:
    - "../adr/provider-only-registration-via-explicit-bundled-plugin-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/provider-only-registration-layer.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# Provider-only registration layer — implementation plan

## Goal

`runtime-pi` に provider-only plugin contract を追加し、
`createModelRuntime()` が bundled plugin を最初の auth/model
resolution より前に `ModelRuntime` へ登録する。Pi extension
loader は有効化しない。

## Scope

1 つの implementation stream（contract 定義 → factory 登録
順序 → test → docs/status sync）。分割不要。

## Upstream Documents

- `docs/specs/provider-only-registration-layer.md`
- `docs/adr/provider-only-registration-via-explicit-bundled-plugin-contract.md`

## Design Inputs

- `docs/designs/provider-only-registration-layer.md`

## File Map

- `packages/runtime-pi/src/provider-plugin.ts`（新規）-
  `CheaperProviderPlugin`・`ProviderPluginContext`・
  `bundledProviderPlugins()`
- `packages/runtime-pi/src/model-runtime.ts` -
  `providerPlugins` option・deferred refresh（create 時と同値
  の allowNetwork/timeout/signal mirror）・plugin 登録 loop
- `packages/runtime-pi/src/index.ts` - export 追加
- `packages/runtime-pi/test/provider-plugin.test.ts`（新規）-
  contract・context・登録順・fail-closed の test
- `packages/runtime-pi/test/model-runtime.test.ts` -
  plugin 登録順序の test 追加
- `docs/guides/providers.md` - bundled provider plugin の
  仕組みを追記（extension 非対応の明記）

## Implementation Sequence

- [ ] TASK-034r05iHWkNF7zJyX85ZOT: provider-plugin contract + createModelRuntime
      の deferred refresh 登録順 + index export
- [ ] TASK-034r05iHWkNF7zKMZPhFky: provider-plugin unit test + resource 隔離・
      auth 統合の検証 test
- [ ] TASK-034r05iHWkNF7zLmXY8W9D: docs 更新 + bundle regen + status sync +
      final verification

## Dependencies and Risks

- deferred refresh は `ModelRuntime.create` の create 時
  refresh semantics（`allowModelNetwork`・`modelRefreshTimeoutMs`
  ・`signal`・`PI_OFFLINE`）を同値に mirror する。mirror を
  誤ると plugin 無し構成の挙動が変わるため、test で
  `providerPlugins: []` 注入と既定 path の同一性を検証する。
- caller 明示の `refreshOnCreate: false` は manual refresh を
  省略して維持する（static-model caller の契約）。
- plugin `register` の throw は fail closed で伝播させる
  （半登録 runtime を返さない）。
- `providerPlugins` option は test seam — production caller
  は bundled registry の既定を使う。CLI の hook
  `createModelRuntime` も option を透過するため既存 test
  seam と衝突しない。

## Task Breakdown

- TASK-034r05iHWkNF7zJyX85ZOT → TASK-034r05iHWkNF7zKMZPhFky → TASK-034r05iHWkNF7zLmXY8W9D（直列。0119 は 0118 の
  contract/factory に依存、0120 は両方の完了に依存）

## Verification

- [ ] `pnpm lint` / `pnpm typecheck` / `pnpm test` green
- [ ] fake plugin を登録し `getAvailable` / `getProviders`・
      `listProviderAuth` に現れる（init 順序保証）
- [ ] `providerPlugins: []` で現行と同一挙動
- [ ] `CheaperResourceLoader` の空 resource test が引き続き
      green（変更なし）
- [ ] `node scripts/bundle-cli.mjs` で bundle 再生成、
      freshness clean

## Open Questions / Gaps

- `ProviderPluginContext` の最小面: `env`・`userConfigDir` のみ
  とする。`signal` 等は #27 の実装で必要になった時点で追加
  する（YAGNI）。
- e2e での plugin provider 確認は既存の
  `CHEAPER_TEST_FAKE_STREAM`（faux provider を
  `registerNativeProvider` する経路）が同等の検証を兼ねる
  ため、plugin 専用の新規 e2e は追加しない。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution
- Independent work streams: none（3 task は直列）
- User approval required before dispatch: yes
- If approved: execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same
  dependency graph and verification matrix.

## Review Handoff

- deferred refresh の mirror 実装は `ModelRuntime.create`
  のソース（`refreshFromNetwork`・timeout・signal 合成）との
  等価性を重点的に確認すること。
