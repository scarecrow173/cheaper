---
id: "PLAN-034r05iHK8bA7g23Opaicg"
type: "plan"
status: "completed"
title: "Pi state under userConfigDir/pi — implementation plan"
test-spec-skip: "Verified by unit tests and e2e assertions inside the implementation tasks; no separate test-spec artifact needed."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/pi-state-under-cheaper-config-dir.md"
    - "../designs/pi-state-paths-under-userconfigdir-pi.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/pi-state-paths-under-userconfigdir-pi.md"
  implemented-by:
    - "../tasks/add-resolveuserconfigdir-helper-and-pi-state-path-wiring.md"
    - "../tasks/wire-env-into-createmodelruntime-callers-and-update-e2e-isolation.md"
    - "../tasks/update-docs-regenerate-bundle-and-run-final-verification.md"
  depends-on:
    - "../adr/pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/pi-state-paths-under-userconfigdir-pi.md"
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

# Pi state under userConfigDir/pi — implementation plan

## Goal

Pi `ModelRuntime` の永続 file（auth.json / models.json /
models-store.json）を `<userConfigDir>/pi/` へ集約し、
`PI_CODING_AGENT_DIR` への依存を排除する。

## Scope

1 つの implementation stream（core helper 抽出 → runtime-pi
factory → CLI wiring → test/docs 更新）。分割不要。

## Upstream Documents

- `docs/specs/pi-state-under-cheaper-config-dir.md`
- `docs/adr/pi-persistent-state-userconfigdir-pi-modelruntime-create-path.md`

## Design Inputs

- `docs/designs/pi-state-paths-under-userconfigdir-pi.md`

## File Map

- `packages/core/src/config/dirs.ts` - `resolveUserConfigDir()`
  抽出 + `resolveConfigDirs()` からの委譲
- `packages/core/src/index.ts` - export 追加（要確認）
- `packages/runtime-pi/src/model-runtime.ts` -
  `createModelRuntime` の `env` seam + pi path 組み立て +
  `mkdir -p`（0700）
- `packages/cli/src/cli.ts` - `createModelRuntime({ env: io.env })`
- `packages/cli/src/auth.ts` - `createModelRuntime({ env: io.env })`
- `packages/runtime-pi/test/model-runtime.test.ts` - path 解決・
  dir 作成・override 優先の unit test
- `packages/cli/test/e2e/helpers.ts` - `PI_CODING_AGENT_DIR`
  除去
- `packages/cli/test/e2e/delegation-loop.test.ts` - 同上 +
  `<configDir>/pi/` assert
- `packages/cli/test/e2e/real-provider-ollama.test.ts` -
  `models.json` を `<configDir>/pi/` へ配置
- `docs/guides/providers.md` - 保存場所表の更新
- `docs/guides/configuration.md` - env var 表の更新

## Implementation Sequence

- [ ] TASK-034r05iHWkNF7zFyMzc7zU: core `resolveUserConfigDir` + runtime-pi
      pi-path wiring + unit test
- [ ] TASK-034r05iHWkNF7zGlMzs17M: CLI/auth `env` wiring + e2e isolation 更新
- [ ] TASK-034r05iHWkNF7zIILagCbE: docs 更新 + bundle regen + status sync +
      final verification

## Dependencies and Risks

- `resolveUserConfigDir` を core に置くことで runtime-pi が
  core の canonical 解決を再利用する（表現ドリフト防止）。
- caller 明示 path が既定を上書く semantics は spread 順序で
  実現する（test seam が潰れないこと）。
- `mkdir` 失敗は fail-closed（Pi 既定 dir への silent
  fallback はしない）。

## Task Breakdown

- TASK-034r05iHWkNF7zFyMzc7zU → TASK-034r05iHWkNF7zGlMzs17M → TASK-034r05iHWkNF7zIILagCbE（直列。0116 は 0115 の
  factory 変更に依存、0117 は両方の完了に依存）

## Verification

- [ ] `pnpm lint` / `pnpm typecheck` / `pnpm test` green
- [ ] e2e: `PI_CODING_AGENT_DIR` 未設定で `<configDir>/pi/`
      に state が出る
- [ ] e2e: workspace `.cheaper/` に credential が出ない
- [ ] `node scripts/bundle-cli.mjs` で bundle 再生成、
      freshness check 相当の一致

## Open Questions / Gaps

- `modelsPath` を `null` にするか string にするか: Pi の型は
  `string | null`。SPEC-034r05iHK8bA7gxlRVZtAv の想定構成では file が存在する
  前提なので string を渡す。
- real-provider e2e は Ollama 依存の auto-skip テスト —
  変更は models.json 配置先のみ。

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

- core helper の切り出しは `resolveConfigDirs` の挙動を変え
  ない純粋な refactor として扱う。e2e helper の env 縮小は
  隔離目的を維持すること。
