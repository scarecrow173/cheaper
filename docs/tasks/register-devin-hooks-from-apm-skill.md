---
id: "TASK-034GtBzKmwPf5z06kAahym"
type: "task"
status: "done"
title: "Register Devin hooks from APM skill"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  implemented-by:
    - "docs/impl/ir/register-devin-hooks-from-apm-skill.md"
  depends-on:
    - "docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md"
    - "docs/tasks/normalize-tools-and-add-devin-hook-capability.md"
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
    - "docs/test-specs/apm-distributed-devin-registration-preservation.md"
  references: []
  defers: []
  deferred-by: []
---

# Register Devin hooks from APM skill

## Work

plan の T2。`install-devin-hooks.ps1` を design の algorithm で新規
実装し（preview 既定・`-Apply`・capability handshake・
`Get-JsonShape` 保全 gate・known config conflict scan・atomic
write）、`devin-hooks-install.test.mjs` の installer 部分・
`docs/guides/distribution.md` 追記・`SKILL.md` への lead-authored
exact copy 挿入と Claude bullet 差し替えを行う。最後に手動 APM
integration gate を 1 回実行する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md`（approved、文面引用・fixture 仕様の正本）
- `docs/specs/case-insensitive-native-tools-and-devin-registration.md`（approved）
- `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`（accepted）
- `docs/designs/devin-native-hooks-and-apm-bootstrap.md`（approved、
  `## JSON Preservation Helper` が `Get-JsonShape` の canonical
  source）

## Dependencies

- T1（`normalize-tools-and-add-devin-hook-capability.md`）—
  capability contract `cheaper hook targets --json` が apply の
  preflight 前提。

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`（新規）
- `scripts/test/devin-hooks-install.test.mjs`（installer 部分追加）
- `docs/guides/distribution.md`
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

## Done When

- [x] installer test が red → 実装 → green（fixture 限定、Windows
      PS 5.1、他 OS 明示 skip）。
- [x] `SKILL.md` / `distribution.md` が plan の lead-authored copy で
      更新済み（文面の自前生成なし）。
- [x] 手動 integration gate（isolated consumer、APM 0.28.0
      `install --target agent-skills`、hash 一致・`.windsurf`/
      `.claude` hook 未生成）が 1 回 pass。

## Verification

- [x] `mise exec -- node --test scripts/test/devin-hooks-install.test.mjs` — exit 0、21 passed / 0 failed（installer 部分含む全体、Windows PS 5.1 full-path spawn、log `tmp/issue107-t2-node-green10.log`）。
- [x] 手動 integration gate: repo 内 ignored `tmp` の fresh
      isolated consumer（`apm.yml` に `packages/apm` local-path
      dependency）で `apm install --target agent-skills` を 1 回
      実行し、deploy 済み helper の hash が source と一致・
      `.windsurf`/`.claude` hook が未生成であること。network/model
      不要、admission bypass・global/consumer 設定変更なし。
      → 実行済み（`tmp/issue107-apm-consumer`、exit 0、SHA256
      `338e671b…e2fc` 一致、hook file 未生成を確認。最終 source の
      log `tmp/issue107-apm-gate5.log`）。
