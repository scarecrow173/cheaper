---
id: "PLAN-034z3OwYgplUxa7Usrjwxq"
type: "plan"
status: "approved"
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
  verified-by:
    - "docs/test-specs/cheaper-hook-subcommands-and-hooks-json-executable-contract.md"
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
    process spawn・`CHEAPER_CLI` 解決を除去。containment →
    `statSync.isFile` → byte fast path の順序を verbatim 維持し、
    measured path 全体を `AbortSignal.timeout(30_000)` で囲む
  - `targets.ts` - windsurf / copilot / pretooluse（claude・codex 共用）
    / cursor の payload normalize + verdict render
  - `hook.ts` - `cheaper hook <event> --target <agent>` の stdin→
    verdict→stdout/stderr/exit 実装。envelope・recordUsage・runCli
    汎用 catch を bypass し、最深部 catch-all で全 error →
    target 適合 allow + exit 0 に変換
- `packages/cli/src/args.ts` / `cli.ts` - `hook` command 追加
  （`--target` option、HELP、hook 用 raw stdin reader）
- `packages/cli/test/hook/` - 新規 contract test + 既存 adapter e2e
  （`pre-read-hook` / `copilot-pre-read-hook` /
  `cross-agent-routing-adapters` / `apm-install*` /
  `delegation-loop`）の `cheaper hook` 呼び出しへの移植
- `packages/apm/.apm/hooks/hooks.json` - command を executable 経由に
  切替（POSIX `./cheaper` launcher、`powershell` は exe 直起動）
- `packages/apm/.apm/hooks/cheaper` - LFS pointer 検出を追加
- `packages/apm/.apm/hooks/*.js` - JS adapter 5 本 + routing-policy.js
  を削除（移行が確認できてから）

## Implementation Sequence

- [ ] `routing-policy.js` を `packages/cli/src/hook/routing-policy.ts`
      に移植。`count-tokens` 子 process 呼び出しを in-process の
      `countFileTokens`（cli.ts と同一経路: canonicalize → policy
      symlink → countMaxBytes）に置き換え、FILE_TOO_LARGE は
      `FileTooLargeError` catch で fail-closed verdict に変換。
      containment→`isFile`→byte fast path の順序を verbatim 維持し、
      `countMaxBytes` を cli.ts から export して共有。
      `WorkspaceEscapeError`・ENOENT・その他全 error → allow（現行
      spawn error → allow と同意味）。measured path を
      `AbortSignal.timeout(30_000)` で囲む。count/policy 系 import は
      byte fast path 通過後に dynamic import に限定（tokenizer eager
      init が hook 起動毎に載るのを防ぐ）
- [ ] shell tokenizer・`extractReadCandidates`・`CWD_CHANGING_COMMANDS`
      等をそのまま移植（挙動不変）
- [ ] `cheaper hook <event> --target <agent>` を args/cli に追加。
      args.ts の `options` に `target` を追加し `hook` 以外の
      `--target` 誤用ガードを置く。parse 失敗を含む全経路で
      「target 適合の allow + exit 0」（target 不明なら silent
      exit 0）に倒す catch-all を hook entry に設け、runCli の
      envelope / recordUsage / 汎用 catch 経路に到達させない。
      stdin は `readAll` 相当の raw 読み（`readTaskInput` の JSON
      object 強制を回避）。HELP に `hook` 行を追加
- [ ] target adapter を実装（stdin JSON → canonical request、
      verdict → target 別 response）:
      windsurf（stderr+exit 2）/ copilot（`permissionDecision:"deny"`
      + 常に exit 0、異形 payload への dual-schema allow 応答も
      移植）/ pretooluse（claude・codex 共用 `hookSpecificOutput`
      + 無言 exit 0）/ cursor（常に schema-valid `permission` JSON
      + exit 0、`file_path`/`command` key で両 event 共有）。
      symlinked cwd の quirk（raw cwd resolve → canonical root 不一致
      → `WorkspaceEscapeError` → allow）は parity として再現し、
      `relPosix` は raw cwd ベースの `path.relative` で算出
- [ ] 新規 contract test を追加 + 既存 adapter e2e（`pre-read-hook` /
      `copilot-pre-read-hook` / `cross-agent-routing-adapters` /
      `apm-install*` / `delegation-loop`）を `cheaper hook` 呼び出しに
      移植。contract test は `node packages/cli/bin/cheaper.js hook`
      （or bundled mjs）を直接呼び、launcher 経由は host exe 存在時の
      smoke に分離（CI ubuntu で linux-x64 exe 未配置のため）
- [ ] `apm install` で `cheaper`/`cheaper.ps1`/exe の deploy・
      extensionless file の exec bit・copilot `.json` 除外の影響を
      事前実測し、`hooks.json` の command を切替:
      - POSIX `command`: `./cheaper hook <event> --target <t>`
      - `powershell`: `& "./cli/bin/win32-x64/cheaper.exe" hook
        <event> --target <t>; exit $LASTEXITCODE`（ExecutionPolicy
        を避けるため `.ps1` を介さない）
      - copilot: `|| exit 0`、cursor: `|| echo '{"permission":"allow"}'`
        の shell 側 fail-open を command に付与
      - claude nested entry に `"shell": "bash"` を付与（Windows Git
        Bash 強制。codex 耐性は実機確認）
      - windsurf entry は exit 2 の deny 意味を潰さないよう suffix 不
        付与
- [ ] executable 環境（`dist/sea/cheaper-windows-x64.exe hook ...`）で
      fixture payload を流して adapter parity を確認
- [ ] docs 整合: `cheaper-usage` SKILL.md の transitional 記述・
      `parent-agent-integration.md` の adapter 名・`cli/bin/cheaper.mjs`
      の去就（fallback として残す）を更新
- [ ] JS adapter 6 ファイル（pre-read-code*.js×4・pre-run-command.js・
      routing-policy.js）を削除し freshness check を通す

## Dependencies and Risks

- **in-process 化による timeout 意味の変化**: `countFileTokens` は
  `maxBytes`（default 16MiB）で打ち切るが、FIFO/device file の
  `readFile` hang や死んだ network FS の `stat` hang は防げない。
  measured path 全体を `AbortSignal.timeout(30_000)` で囲み旧 subprocess
  cap を等価復元する
- **catch-all の必須性**: hook 経路の error が runCli 汎用 catch に
  漏れると `INVALID_REQUEST`（exit 2）で windsurf/claude/cursor が
  block、copilot は非 0 全 deny になる。hook entry で target 適合
  allow + exit 0 に必ず変換する
- **hook 起動毎の eager init**: `@cheaper/core` の tokenizer は module
  scope で全 encoding を初期化するため、static import にすると
  tool call 毎の hook 起動に decode コストが載る。count 系 import は
  fast path 後に dynamic import に限定
- **symlinked cwd の挙動**: 現行は raw cwd resolve + canonical root
  不一致で常に fail-open（quirk）。in-process 化でも同じ順序を再現し、
  `relPosix` は raw cwd 基準で文言を変えない
- **launcher/exe 障害の deny-all 化**: launcher は exe 不在で exit 1
  → copilot 非 0 全 deny。hooks.json 側に shell fail-open を付与
  （copilot `|| exit 0` / cursor `|| echo ...`）して吸収
- **未同梱 platform（macOS 等）**: launcher exit 1 が copilot deny-all
  化するが、shell 側 fail-open で吸収（design 記載済み）
- **remote install の LFS pointer**: consumer に exe が pointer text
  で届く可能性。launcher で `version https://git-lfs` prefix を検出
  して明確な error を出す
- **APM deploy の exec bit / extensionless 検出**: `apm install` 実走
  で `./cheaper` の deploy・exec bit 保存を事前実測してから切替
- **claude/codex 共用 target**: `PreToolUse` nested entry は 1 command
  のみ持つため `--target pretooluse` で両 target を共用する
- **`powershell` key の ExecutionPolicy**: `.ps1` 直叩きは Restricted
  で阻止されるため exe 直起動にする
- **`cheaper.mjs` の去就**: `hooks.json` は参照しなくなるが、旧 adapter
  の fallback と skill transitional 記述のため残す
- **usage.jsonl**: hook 経路は `recordUsage` を呼ばない（計測は
  telemetry 対象外。意図的決定として IR に記録）
- **per-task bundle regen**: cli src 変更のたび `pnpm bundle` 再生成で
  freshness check を維持する
- **target payload drift**: adapter の正規化は現行 JS と fixture test
  で 1:1 固定する
- **hooks.json 切替と adapter 削除を同一 commit にしない**: design の
  「切替と削除を分ける」に従い、切替 commit → parity 確認 → 削除
  commit の順
- **breaking-change risk: 中**。hook 経路の入口が変わるため、削除前に
  launcher + exe の deploy 確認が前提（Phase 3 で payload 同梱済み）
- **data-loss risk: なし**

## Task Breakdown

「routing policy 移植」「target adapter + hook command」「contract
test 新設 + 既存 e2e 移植」「launcher deploy 事前検証 + hooks.json
切替」「docs 整合」「adapter 削除 + 最終検証」の 6 タスクを目安とする。

## Verification

- [ ] 移植 unit test（tokenizer・read candidate・file-read verdict）
      が JS 版と同一 fixture で pass
- [ ] target 別 contract test pass（windsurf exit 2/copilot deny JSON/
      pretooluse hookSpecificOutput/cursor schema-valid）
- [ ] `dist/sea/cheaper-windows-x64.exe hook <event> --target <t>` が
      Node 非依存で stdin payload を処理する
- [ ] `hooks.json` が `node ` を含まない
- [ ] JS adapter 削除後も `mise run check` pass（bundle freshness含む）
- [ ] `cheaper hook` 経路が runCli の汎用 catch まで到達せず、args
      parse 失敗・policy 破損・cwd 不存在を含む全 error で target
      適合 allow + exit 0 を返す
- [ ] `apm install` で `cheaper`/`cheaper.ps1`/exe が deploy され、
      POSIX `command` が verbatim で executable に到達する
- [ ] symlinked cwd で現行と同じ fail-open quirk が維持される
- [ ] `usage.jsonl` に hook 計測が記録されない（recordUsage 非呼出し）

## Open Questions / Gaps

- claude/codex の `PreToolUse` nested entry に `"shell": "bash"` を
  付与しても codex が未知 field を許容するか（実機で確認。reject
  なら copilot と同じ fail-open suffix に切替）
- remote install で `cheaper.exe` が LFS pointer 化するかの実測
  （consumer 環境での初回 install 検証に依存する部分は Phase 3 同様
  deferred）
- `apm install` で `./cheaper`（extensionless）の exec bit 保存が
  apm 0.28.0 で保証されるか（`sh ./cheaper` で回避する判断を task 4
  の前提検証で行う）
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
- 前提: adversarial review 3 件の指摘（catch-all 必須化・timeout 復元・
  isFile 順序・launcher deploy 実測・powershell exe 直起動・shell
  fail-open 等）を File Map / Implementation Sequence / Risks に反映済み
- 着目点: 移植時の挙動不変性（fail-open/fail-closed・閾値・tokenizer）
  と hooks.json command の executable 表現、catch-all の網羅性
