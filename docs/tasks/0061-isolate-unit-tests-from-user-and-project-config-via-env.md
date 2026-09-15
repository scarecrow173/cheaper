---
id: "TASK-0061"
type: "task"
status: "done"
title: "Isolate unit tests from user and project config via env"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/plans/0005-harden-repair-abort-grace-and-provider-failure-normalization.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0005-harden-repair-abort-grace-and-provider-failure-normalization.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Isolate unit tests from user and project config via env

## Work

`resolveConfigDirs` の homedir 解決を `opts.homedir ?? env.USERPROFILE ?? env.HOME ?? os.homedir()` に変更し、env 注入による隔離を有効にする。`main.test.ts` の `makeIo` を e2e helpers と同じ方針で tmp 隔離（`HOME`/`USERPROFILE`/`CHEAPER_CONFIG_DIR` を tmp dir、`cwd` を tmp workspace）にし、実ユーザーの `~/.config/cheaper` と repo `.cheaper` を読まないようにする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/specs/0005-repair-and-abort-hardening-telemetry-consistency-and-test-hermeticity.md`
- `docs/designs/0010-repair-abort-and-failure-normalization-hardening.md`

## Dependencies

- なし（コード群と独立）

## Affected Files

- `packages/core/src/config/dirs.ts`
- `packages/core/test/config/`（dirs の単体テストがあれば更新/追加）
- `packages/cli/test/main.test.ts`

## Done When

- [ ] `resolveConfigDirs` が `env.USERPROFILE`/`env.HOME` を `os.homedir()` より優先する
- [ ] `main.test.ts` が実ユーザーの `~/.config/cheaper` と repo `.cheaper` を読まない
- [ ] user config に `policy.yaml`/`models.yaml` を置いた環境でも `pnpm -F @cheaper/cli test` が green

## Verification

- [ ] `mise exec -- pnpm -F @cheaper/core test` / `pnpm -F @cheaper/cli test` が green
- [ ] 実 user config ディレクトリに smoke 用設定を置いた状態でテストが落ちないことを確認
