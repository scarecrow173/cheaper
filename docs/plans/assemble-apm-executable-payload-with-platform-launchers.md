---
id: "PLAN-034z2ET4udxbq0djqYM2kl"
type: "plan"
status: "approved"
title: "Assemble APM executable payload with platform launchers"
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
    - "docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md"
    - "docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/apm-executable-payload-layout-and-platform-launchers.md"
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/apm-executable-payload-layout-and-launcher-dispatch-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Assemble APM executable payload with platform launchers Implementation Plan

## Goal

issue #71 Phase 3 として、APM package（`packages/apm/`）に platform
別 SEA executable と薄い platform launcher を同梱し、consumer 環境に
Node.js がなくても launcher 経由で `cheaper` executable を起動できる
payload layout を組み立てる。

## Scope

issue #71 の Phase 3（APM payload layout + launcher + bundle 組み立て
script）のみを 1 stream として扱う。Phase 4 の `cheaper hook`
subcommand 化と `hooks.json` 書き換えは別 plan（次の plan）で扱う。
そのため本 plan では `hooks.json`・既存 JS adapter は**変更しない**
（launcher と executable は同梱されるが、hook エントリはまだ
`node ./pre-*.js` を指す transitional 状態）。

対象 platform は Phase 2 matrix と同じ 3 target（`win32-x64`・
`linux-x64`・`linux-arm64`）。macOS は需要が出るまで対象外。

## Upstream Documents

- docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md
- docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md
- docs/plans/implement-node-26-sea-standalone-executable-proof-of-concept.md
- docs/plans/integrate-sea-executables-into-tag-triggered-github-release-workflow.md

## Design Inputs

- docs/designs/apm-executable-payload-layout-and-platform-launchers.md

## File Map

- `packages/apm/.apm/hooks/cheaper` - 新規。POSIX launcher。
  `uname -s`/`uname -m` で `cli/bin/<platform>/cheaper` に `exec`
- `packages/apm/.apm/hooks/cheaper.ps1` - 新規。PowerShell launcher。
  `cli/bin/win32-x64/cheaper.exe` を起動し `$LASTEXITCODE` を伝播
- `packages/apm/.apm/hooks/cli/bin/<target>/cheaper[.exe]` - 生成物。
  platform 別 executable の配置先
- `scripts/build-apm-bundle.mjs` - 新規。`pnpm bundle` 出力 +
  `--exe-dir` で与えた executable 群を APM distribution tree に
  組み立てる
- `.gitattributes` - 新規または変更。`cli/bin/<platform>/` 配下の
  executable を `filter=lfs diff=lfs merge=lfs -text` で LFS 管理
  に指定（追跡方針: コミットするが LFS。下記 Open Questions 参照）
- `packages/apm/.apm/skills/cheaper-usage/` - 変更。CLI location
  guidance を executable 基準に更新

## Implementation Sequence

- [ ] launcher 2 本を実装（POSIX `cheaper` + `cheaper.ps1`）。
      POSIX 版は `uname -s`→`darwin|linux`、`uname -m`→`x64|arm64`
      の対応表で `<target>` を決定し `exec`。未対応 platform では
      stderr に非対応を出して非 0 exit。PowerShell 版は
      `win32-x64` 固定で sibling exe を `&` 起動し
      `exit $LASTEXITCODE`
- [ ] `scripts/build-apm-bundle.mjs` を実装: `bundle-cli.mjs`
      相当の canonical bundle 出力（既存 helper 再利用）に加えて、
      `--exe-dir <dir>` の `cheaper-<platform>-<arch>[.exe]` を
      `cli/bin/<platform>/` へリネームコピー。`--exe-dir` 未指定時は
      executable を配置しない（host だけの部分 bundle を許容し、
      存在する platform のみ配置）
- [ ] `.gitattributes` に executable パスの LFS 指定を追加し、
      `git lfs install` 済み環境で add したとき pointer file 化を
      確認（`git lfs status` / `git cat-file` で pointer 判定）
- [ ] `mise run check` の bundle freshness step が新しい tree
      構成（launcher・executable）を壊さないことを確認、必要なら
      freshness 対象を調整
- [ ] launcher の動作確認: POSIX 版は Git Bash で `cheaper
      --version` が win32-x64 exe に委譲されること、PowerShell 版は
      `pwsh cheaper.ps1 --version` で同様に
- [ ] `cheaper-usage` skill の CLI location guidance を executable
      基準に更新
- [ ] APM install 後の path/mode 検証は local `apm install` が
      可能な範囲で実施（POSIX bit・launcher 解決）

## Dependencies and Risks

- **executable の git 追跡（決定済み: LFS）**: `packages/apm/.apm/
  hooks/cli/` は現状 git 追跡済み（`cheaper.mjs`・`version.json`・
  `workers/` が commit され `mise run check` が freshness diff を
  持つ）。executable は `.gitattributes` の LFS rule で追跡するため、
  repo 本体に入るのは pointer のみ、実体は LFS object store。
  **前提条件: dev / CI 双方で `git lfs` が利用可能であること**
  （checkout での smudge・freshness diff が pointer 比較になる）
- **host 以外の executable の入手**: current host で build できるのは
  `win32-x64` のみ。linux 系は release workflow の artifact を
  `gh release download` 等で取得して `--exe-dir` に渡す想定。
  full bundle の組み立ては release 時の作業になる
- **hooks.json は不変**: 移行は Phase 4。本 plan で payload を
  同梱しても hook エントリはまだ Node adapter を呼ぶ（design の
  transitional 状態と一致）
- **launcher の薄さ**: 引数・exit code の伝播のみ。誤判定時は
  明確に fail（fail-open にはしない — hook が黙って no-op に
  なるより fail して気づける方がよい）
- **APM package size**: payload が 100MB 級になる。install 時間・
  deploy 検証への影響は対象 platform 数で線形に増える
- **breaking-change risk**: 新規ファイル中心。`hooks.json`・adapter
  JS・`bundle-cli.mjs` の既存出力は変えない
- **data-loss risk: なし**

## Task Breakdown

「launcher 実装」「build-apm-bundle.mjs」「git 追跡方針 + freshness
調整」「skill guidance 更新」「local 検証 + IR 記録」の 5 タスクを
目安とする（launcher と build script は直列、guidance 更新は独立）。

## Verification

- [ ] Git Bash で `hooks/cheaper --version` が
      `cli/bin/win32-x64/cheaper.exe` に委譲され version を出力
- [ ] `pwsh .apm/hooks/cheaper.ps1 --version` が同様に動作し
      exit code を伝播
- [ ] `node scripts/build-apm-bundle.mjs --exe-dir dist/sea` が
      executable を `cli/bin/win32-x64/` に配置
- [ ] `git lfs` が環境で利用可能で、add した executable が
      pointer file として commit される
- [ ] `pnpm bundle`（または `build-apm-bundle`）後の tree が
      freshness check と整合
- [ ] `mise run check` pass
- [ ] （可能なら）`apm install` 後の target dir で launcher 経由
      `--version` が動く

## Open Questions / Gaps

- **executable の追跡方針（決定）**: コミットするが **Git LFS**
  で管理する。`.gitattributes` に
  `packages/apm/.apm/hooks/cli/bin/** filter=lfs diff=lfs merge=lfs
  -text` を指定し、pointer のみ履歴に残す。freshness diff は
  pointer 比較として機能する。
  - 前提: dev 環境に `git lfs install` が必要（CI は
    `actions/checkout` の `lfs: true` で対応）
  - design の「binary freshness を CI が検証」は pointer レベルの
    diff で維持される
- `cheaper.ps1` のみで Windows を賄うか、`cheaper.cmd`/`.bat` が
  必要か（apm が Windows hook をどう起動するかに依存。
  hooks.json の現行 `powershell` キー構造から `.ps1` で足りる想定）
- `cheaper-usage` skill の更新範囲（executable path の記述のみか、
  bundle 参照の全面置き換えか）

## Rollback Strategy

- 新規ファイル + guide 更新中心。`git revert` で戻せる
- Trigger: launcher の誤動作 / tree 構成が freshness check を壊す
- Procedure: `git revert <commit>`。hooks.json を触っていないため
  現行 Node adapter 経路は常に健在
- Verification: `mise run check` pass

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（launcher の実機確認と bundle
  tree の検証が逐次必要）
- Independent work streams: skill guidance 更新のみ独立
- User approval required before dispatch: yes

## Review Handoff

- 前提: Phase 1（host executable）・Phase 2（manual release
  workflow + 3 platform matrix）が完了済み
- 前提: executable の git 追跡方針が approve 時に確定すること
  （Open Questions の (a)/(b)）
- 着目点: hooks.json を触らない transitional 構成、launcher の
  fail 挙動、executable サイズの repo 影響
