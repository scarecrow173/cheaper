---
id: "PLAN-034z3OwYgplUxa7Usrjwxq"
type: "plan"
status: "draft"
title: "Implement cheaper hook subcommands and migrate hooks.json off Node adapters"
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
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  implemented-by: []
  depends-on:
    - "docs/plans/assemble-apm-executable-payload-with-platform-launchers.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md"
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement cheaper hook subcommands and migrate hooks.json off Node adapters Implementation Plan

## Goal

issue #71 Phase 4 として、`cheaper hook <event> --target <agent>` CLI
command に hook payload の正規化・routing 判定・verdict 応答を移植し、
`hooks.json` を launcher 経由の executable 呼び出しに切り替えて
Node.js 依存の JS adapter を除去する。

## Scope

issue #71 の Phase 4（hook 脱 Node 化）のみを 1 stream として扱う。
routing 判定ルール自体の変更は行わない（DESIGN-034z1EhuztlLSb2x2XXxgD
の Out of Scope と一致）。

## Upstream Documents

- docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md
- docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md
- docs/plans/assemble-apm-executable-payload-with-platform-launchers.md

## Design Inputs

- docs/designs/cheaper-hook-subcommands-replacing-node-based-hook-adapters.md

## File Map

- `packages/cli/src/hook/` - 新規 dir。routing policy 移植 + target
  adapter + command 実装
  - `routing-policy.ts` - `routing-policy.js` の TS 移植。
    `countFileTokens`（`@cheaper/core`）を in-process で呼び、子
    process spawn・`CHEAPER_CLI` 解決を除去
  - `targets.ts` - windsurf / copilot / claude・codex / cursor の
    payload normalize + verdict render
  - `hook.ts` - `cheaper hook <event> --target <agent>` の stdin→
    verdict→stdout/stderr/exit 実装
- `packages/cli/src/args.ts` / `cli.ts` - `hook` command 追加
- `packages/cli/test/hook/` - target 別 contract test（payload
  fixture → stdout/stderr/exit code）
- `packages/apm/.apm/hooks/hooks.json` - command を launcher 経由
  `./cheaper hook <event> --target <agent>` へ切替
- `packages/apm/.apm/hooks/*.js` - JS adapter 5 本 + routing-policy.js
  を削除（移行が確認できてから）

## Implementation Sequence

- [ ] `routing-policy.js` を `packages/cli/src/hook/routing-policy.ts`
      に移植。`count-tokens` 子 process 呼び出しを in-process の
      `countFileTokens`（cli.ts と同一経路: canonicalize → policy
      symlink → countMaxBytes）に置き換え、FILE_TOO_LARGE は
      `FileTooLargeError` catch で fail-closed verdict に変換
- [ ] shell tokenizer・`extractReadCandidates`・`CWD_CHANGING_COMMANDS`
      等をそのまま移植（挙動不変）
- [ ] target adapter を実装（stdin JSON → canonical request、
      verdict → target 別 response）:
      windsurf（stderr+exit 2）/ copilot（`permissionDecision:"deny"`
      + 常に exit 0）/ claude・codex（`hookSpecificOutput` + exit 0）/
      cursor（常に schema-valid `permission` JSON + exit 0、
      `file_path`/`command` key で両 event 共有）
- [ ] `cheaper hook <event> --target <agent>` を args/cli に追加
      （command 名は design の surface 例に従う）
- [ ] target 別 contract test を追加（deny/allow/fail-open 各系統。
      malformed payload → allow、infra error → allow、
      FILE_TOO_LARGE → deny）
- [ ] `hooks.json` の command を `./cheaper hook ...` /
      `./cheaper.ps1 hook ...` 系に書き換え
- [ ] executable 環境（`dist/sea/cheaper-windows-x64.exe hook ...`）で
      fixture payload を流して adapter parity を確認
- [ ] JS adapter 6 ファイル（pre-read-code*.js×4・pre-run-command.js・
      routing-policy.js）を削除し freshness check を通す

## Dependencies and Risks

- **in-process 化による timeout 意味の変化**（design の risk と同一）:
  30s subprocess timeout が無くなる。`countFileTokens` は
  `maxBytes`（default 16MiB）で打ち切るため実質 bounded。呼び出し
  側で追加の timeout は設けない（KISS）。IR に記録する
- **target payload drift**: adapter の正規化は現行 JS と fixture test
  で 1:1 固定する
- **hooks.json 切替と adapter 削除を同一 commit にしない**: design の
  「切替と削除を分ける」に従い、切替 commit → parity 確認 → 削除
  commit の順
- **`hooks.json` command の launcher 参照**: `command` は `./cheaper`
  （POSIX）、`powershell` キーは `./cheaper.ps1` を指す。hooks dir が
  cwd である前提は現行と同じ
- **breaking-change risk: 中**。hook 経路の入口が変わるため、削除前に
  launcher + exe の deploy 確認が前提（Phase 3 で payload 同梱済み）
- **data-loss risk: なし**

## Task Breakdown

「routing policy 移植」「target adapter + hook command」「contract
test」「hooks.json 切替」「adapter 削除 + 最終検証」の 5 タスクを
目安とする。

## Verification

- [ ] 移植 unit test（tokenizer・read candidate・file-read verdict）
      が JS 版と同一 fixture で pass
- [ ] target 別 contract test pass（windsurf exit 2/copilot deny JSON/
      claude hookSpecificOutput/cursor schema-valid）
- [ ] `dist/sea/cheaper-windows-x64.exe hook <event> --target <t>` が
      Node 非依存で stdin payload を処理する
- [ ] `hooks.json` が `node ` を含まない
- [ ] JS adapter 削除後も `mise run check` pass（bundle freshness含む）

## Open Questions / Gaps

- hooks.json の `command`（POSIX）と `powershell` の両キーで
  launcher の呼び方がどうなるか（`./cheaper hook ...` /
  `./cheaper.ps1 hook ...` の形で現行構造を維持する想定）
- in-process `countFileTokens` の呼び出しに必要な policy/dirs 解決を
  hook command 内でどこまで行うか（count-tokens command と同じ
  resolveConfigDirs 経路を使う想定）
- cursor の `beforeReadFile`/`beforeShellExecution` が同一 adapter で
  よいかは payload key 判定で確定（design は共有を示唆）

## Rollback Strategy

- hooks.json 切替・adapter 削除は commit 分割のため段階 revert 可能
- Trigger: hook が target で発火しない / 誤判定する
- Procedure: hooks.json を JS adapter 経路に revert（adapter file は
  git 履歴から復元可）
- Verification: revert 後に `mise run check` + 手動 hook 発火確認

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（adapter parity の実機確認が逐次
  必要）
- Independent work streams: なし（移植→command→切替は直列）
- User approval required before dispatch: yes

## Review Handoff

- 前提: Phase 3 で launcher + exe payload が hooks tree に存在
- 前提: `countFileTokens` / `canonicalizeWorkspaceRoot` /
  `FileTooLargeError` / `countMaxBytes` が core/cli から再利用可能
- 着目点: 移植時の挙動不変性（fail-open/fail-closed・閾値・tokenizer）
  と hooks.json command の launcher 表現
