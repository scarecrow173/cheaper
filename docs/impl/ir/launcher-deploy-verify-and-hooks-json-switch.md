---
id: "IMPL-034z57kkaNGIsoFKFnVOwn"
type: "impl"
status: "completed"
title: "launcher deploy verify and hooks.json switch"
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
    - "docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md"
  implemented-by: []
  depends-on: []
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# launcher deploy verify and hooks.json switch

## Summary

`apm install` で launcher（`cheaper`・`cheaper.ps1`）と LFS
executable payload の deploy を実機検証し、`hooks.json` の全
entry を `./cheaper hook <event> --target <t>` /
`cheaper.exe` 直起動（powershell）に切り替えた。POSIX launcher
には LFS pointer 検出を追加した。

## Context

- task: docs/tasks/verify-launcher-deploy-and-switch-hooks-json-to-executable-invocations.md
- design: docs/designs/apm-executable-payload-layout-and-platform-launchers.md

## Implementation

### Changes

- `packages/apm/.apm/hooks/hooks.json`: 全 entry から `node ` /
  JS adapter 参照を除去。POSIX `command` は `./cheaper hook …`、
  `powershell` は `& "./cli/bin/win32-x64/cheaper.exe" hook …`
  の exe 直起動。copilot に `|| exit 0`（powershell は `; exit 0`）、
  cursor に `|| echo '{"permission":"allow"}'`（powershell は
  `$LASTEXITCODE -ne 0` で allow JSON を emit）を付与。windsurf
  entry は suffix なし（exit 2 を保持）。claude/codex 共用の
  nested `PreToolUse` は `--target pretooluse` + `"shell": "bash"`
- `packages/apm/.apm/hooks/cheaper`: payload 先頭 44 byte が
  `version https://git-lfs` で始まる場合に stderr error + exit 1
  で終了する pointer 検出を追加（command substitution の null byte
  warning を避けるため `head -c 44 | grep -q` 形式）
- `cli/bin/win32-x64/cheaper.exe`: `hook` command 実装を含む
  最新 SEA build に更新（LFS payload refresh）
- `cli/bin/cheaper.mjs`: `pnpm bundle` で hook 経路を含む最新化
- `test/e2e/apm-install.test.ts`・`apm-install-targets.test.ts`:
  command assert を executable 呼び出し形に更新し、`node ` /
  `.js` adapter 参照が無いことを assert に追加

### Decisions During Implementation

- **powershell の cursor fallback**: `if ($LASTEXITCODE -ne 0)
  { '{"permission":"allow"}' }` で exe 不在・起動失敗時でも
  schema-valid allow を emit（cursor は schema-invalid を block
  と解釈するため）
- **copilot powershell `; exit 0`**: exe 不在時に非 0 で Copilot
  deny-all になるのを防ぐ。deny は stdout JSON なので exit 0 強制
  で contract は壊れない

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `apm install --target windsurf`（local path dep、apm 0.28.0）:
  - deploy: `cheaper`（exec bit 付き）、`cheaper.ps1`、
    `cli/bin/win32-x64/cheaper.exe`（117 MB、実 binary）が
    `.windsurf/hooks/pkg/` に deploy される
  - rewrite: `./cheaper` → `.windsurf/hooks/pkg/cheaper`、
    `./cli/bin/win32-x64/cheaper.exe` → 同様に rewrite される
    ことを generated `.windsurf/hooks.json` で確認
  - verbatim replay: generated command を consumer cwd で実行し
    deny exit 2 を確認
- launcher LFS pointer 検出: pointer テキストを置いた fixture で
  stderr error + exit 1 を確認
- cursor `|| echo` fallback 経路: sh で replay し allow JSON を確認
- `vitest run apm-install.test.ts apm-install-targets.test.ts
  hook-contract.test.ts`: 23 pass / 1 skipped

## Risks

- `"shell": "bash"` を codex が受け入れるかは実機未検証（未知
  field として拒否された場合は nested entry を分離する要検討。
  Open Question として残置）
- remote install（git transport）での LFS smudge は未検証 —
  consumer の clone に git-lfs が無いと pointer が届くが、launcher
  の検出で明示 error になる（deferred）
- copilot target では APM が `.json` を deploy しない既知 gap
  （version.json 等）は継続 — hook 実行には影響しない

## Follow-ups

- task 5: docs 更新（Node adapter 記述の除去）
- task 6: JS adapter 6 file 削除 + 最終検証
