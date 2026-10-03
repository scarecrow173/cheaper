---
id: "TASK-034z52LkJfYTGEpl69G239"
type: "task"
status: "done"
title: "verify launcher deploy and switch hooks.json to executable invocations"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
  implemented-by:
    - "docs/impl/ir/launcher-deploy-verify-and-hooks-json-switch.md"
  depends-on:
    - "docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md"
    - "docs/tasks/add-cheaper-hook-command-with-target-adapters.md"
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
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# verify launcher deploy and switch hooks.json to executable invocations

## Work

`apm install` で launcher / exe の deploy を事前実測し、
`hooks.json` の command を executable 経由に切り替える。

- 事前検証: `apm install` で `cheaper`（extensionless）/
  `cheaper.ps1` / `cli/bin/*/cheaper*` が deploy されるか、exec bit
  が残るか、copilot の `.json` 除外の影響を確認。exec bit が欠け
  る場合は `command` を `sh ./cheaper ...` に変更する
- `hooks.json` 切替:
  - POSIX `command`: `./cheaper hook <event> --target <t>`
  - `powershell`: `& "./cli/bin/win32-x64/cheaper.exe" hook
    <event> --target <t>; exit $LASTEXITCODE`（ExecutionPolicy を
    避けるため `.ps1` を介さない）
  - copilot: `|| exit 0`、cursor:
    `|| echo '{"permission":"allow"}'` の shell 側 fail-open
  - claude nested entry: `"shell": "bash"` を付与（Windows Git
    Bash 強制）
  - windsurf entry: suffix 不付与（exit 2 の deny 意味を潰さない）
  - `pretooluse` を `--target claude`/`codex` の共用 label として
    hooks.json には `--target pretooluse` を書く
- launcher `cheaper` に LFS pointer 検出（先頭
  `version https://git-lfs`）を追加して明確な stderr error にする
- `apm-install*.test.ts` の command 文字列 assert を executable 経由
  に更新

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/implement-cheaper-hook-subcommands-and-migrate-hooks-json-off-node-adapters.md
- docs/designs/apm-executable-payload-layout-and-platform-launchers.md

## Dependencies

- docs/tasks/add-cheaper-hook-command-with-target-adapters.md

## Affected Files

- `packages/apm/.apm/hooks/hooks.json`
- `packages/apm/.apm/hooks/cheaper`（LFS pointer 検出）
- `packages/cli/test/e2e/apm-install*.test.ts`

## Done When

- [ ] `apm install` で launcher・exe が deploy されることが実測済み
- [ ] `hooks.json` の全 entry が `node ` を含まない
- [ ] fail-closed target（copilot/cursor）に shell 側 fail-open が
      付いている

## Verification

- [ ] `apm install` 後の generated command を verbatim replay で
      executable に到達する
- [ ] `mise run check` の bundle freshness diff が clean
