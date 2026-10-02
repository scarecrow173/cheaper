---
id: "PLAN-034z1M6zCAdKb12eiCWZRm"
type: "plan"
status: "draft"
title: "Implement Node 26 SEA standalone executable proof of concept"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "docs/discovery/standalone-cheaper-executable-distribution-without-node-js.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement Node 26 SEA standalone executable proof of concept Implementation Plan

## Goal

current host（Windows x64 開発機）で Node.js 非依存の `cheaper` standalone
executable を生成し、issue #71 Phase 1 の PoC として CLI 主要 command と
built-in resource 解決が VFS 経由で動くことを実証する。

## Scope

issue #71 の Phase 1（current-host SEA PoC）のみを 1 stream として扱う。
Phase 2 の cross-platform matrix / GitHub Release 接続、Phase 3 の APM
payload、Phase 4 の hook 移行は本 plan に含めず、別 plan で扱う
（independently verifiable なため分割）。

## Upstream Documents

- docs/specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md
- docs/adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md
- docs/discovery/standalone-cheaper-executable-distribution-without-node-js.md

## Design Inputs

- docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md

## File Map

- `scripts/lib/esbuild-bundle.mjs` - 新規。esbuild 呼び出し・
  createRequire banner 付与・shebang 保持を担う shared bundle helper。
  `scripts/bundle-cli.mjs` から切り出す
- `scripts/bundle-cli.mjs` - 変更。esbuild 部分を helper 呼び出しに置換。
  APM tree への copy 出力（`cheaper.mjs`・`version.json`・`workers/`・
  `policy.yaml`）は現行維持
- `scripts/build-executable.mjs` - 新規。`--target <platform>-<arch>`
  と `--out-dir` を受け取り、SEA config 生成・asset staging・
  `node --build-sea`・postject 注入（必要時）・artifact 出力・
  size 報告を行う
- `packages/cli/src/sea-main.ts` - 新規。`import.meta.dirname` を
  `runCli(..., { builtinDir })` に渡す SEA 用 entrypoint。
  `registerBundledOAuthFlows()` は `main.ts` と同じく先に呼ぶ
- `packages/cli/src/version.ts` - 変更。`packageVersion()` を
  `builtinDir` 基準の `version.json` 解決に揃える
  （design の resource-root 統一）
- `packages/cli/src/cli.ts` - 変更。`packageVersion()` 呼び出しに
  builtinDir を伝搬する
- `packages/cli/test/` - version 解決の unit test を追加
- `mise.lock` - 変更。`useVfs` が入った Node ≥26.9.0 に更新
  （`mise.toml` の `node = "26"` は変更不要）
- `dist/sea/` - 生成出力。`cheaper-windows-x64.exe`（current host）

## Implementation Sequence

- [ ] Node toolchain を ≥26.9.0 に更新（`mise install` → `mise.lock` 更新。
      `minimum_release_age = "7d"` のため 7 日以上前の 26.9+ が使われる）
- [ ] `scripts/lib/esbuild-bundle.mjs` に bundle 生成ロジックを切り出す
      （esbuild 起動・banner 前置・shebang 保持）
- [ ] `scripts/bundle-cli.mjs` を helper 呼び出しに書き換える
- [ ] `pnpm bundle` を実行し `packages/apm/.apm/hooks/cli/` の出力が
      リファクタ前と一致することを確認（byte 比較）
- [ ] `packageVersion` の builtinDir 解決テストを追加し失敗を確認
- [ ] `version.ts` / `cli.ts` を builtinDir 基準に変更しテストを pass
- [ ] `packages/cli/src/sea-main.ts` を追加し `pnpm build` で
      `dist/sea-main.js` が出ることを確認
- [ ] `scripts/build-executable.mjs` を実装（sea-config 生成・
      assets=`workers/` `policy.yaml` `version.json`・`--build-sea`・
      postject 注入・命名 `cheaper-<platform>-<arch>[.exe]`・size 出力）
- [ ] `node scripts/build-executable.mjs --target win32-x64` で current
      host executable を生成
- [ ] PATH から Node.js を外した環境で `--version` / `count-tokens` /
      `models` / `auth status` を実行
- [ ] fake/test runtime で built-in worker resolution・schema load が
      executable 内 resource から動くことを確認（tiktoken WASM の VFS
      内 load 可否をここで判定）
- [ ] stdout 1-envelope / stderr diagnostics / exit code の現行 CLI
      parity を確認
- [ ] executable size を記録

## Dependencies and Risks

- **Node ≥26.9.0 が必要**（`useVfs` 導入 version）。mise の
  `minimum_release_age = "7d"` で直近 release は弾かれるため、
  条件を満たす 26.9+ が存在しない場合はこの step で停止する
- **postject の追加依存**: `node --build-sea` 単体で注入まで完結しない
  場合は postject が必要。追加は AGENTS.md の「依存の追加・変更は確認」
  に従い実装時点で確認する
- **tiktoken / WASM が VFS から load できない可能性**（`.node` は不可、
  WASM は未検証）。PoC で `count-tokens` が失敗した場合は WASM を
  bundle から real FS へ退避させる方針に再設計し、design doc を
  amend する
- **Pi OAuth の dynamic import**: `registerBundledOAuthFlows()` を
  sea-main でも先に呼ぶことで bundle 内解決を維持する
- **breaking-change risk は低い**: bundle-cli.mjs の公開出力は不変を
  検証する。version.ts 変更は `version.json` 探索規則の収斂のみ
- **data-loss risk: なし**。読み取り専用の配布 artifact 生成のみ

## Task Breakdown

上記 Implementation Sequence を task-doc の粒度に分割する際は、
「リファクタ（出力不変確認）」「version 解決（TDD）」「SEA entrypoint」
「build script」「PoC smoke + 記録」の 5 タスクに分解する。

## Verification

- [ ] `pnpm bundle` 後の `packages/apm/.apm/hooks/cli/` が git diff 上
      変化しない（リファクタ無影響）
- [ ] `pnpm test` の version 解決テストが pass
- [ ] `node scripts/build-executable.mjs --target win32-x64` が
      `dist/sea/cheaper-windows-x64.exe` を生成し size を出力
- [ ] `env -i` 相当（PATH に Node なし）で
      `cheaper-windows-x64.exe --version` が package version を出力
- [ ] 同条件で `count-tokens <file>` / `models` / `auth status` が
      動作し exit code が現行 CLI と一致
- [ ] fake runtime で worker definition / JSON schema が VFS 内 resource
      から解決される
- [ ] executable size（MiB）を本 plan の記録または issue #71 へ記録

## Open Questions / Gaps

- postject が必要かは `--build-sea` の実挙動で確定する（未確認）
- `useVfs` VFS mount の `import.meta.dirname` が実際に `builtinDir`
  として機能するかは PoC で初めて確定する。機能しない場合は
  `node:sea` の `getAsset` API 経由の resolver に切り替える
  （design の deferred concern に記録済み）
- Node ≥26.9.0 が `minimum_release_age` を満たすかは実行時点依存

## Rollback Strategy

- 全変更は additive または出力不変の refactor であり `git revert` で
  戻せる。migration や不可逆変更は含まない
- Trigger: `pnpm bundle` 出力が変化する / version 解決が壊れる
- Procedure: `git revert <commit>` + `pnpm bundle && pnpm test`
- Verification: `mise run check` が pass し APM bundle が復元される

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（SEA PoC は初回検証が主眼であり、
  逐次の実機確認を要するため delegated dispatch より inline が適切）
- Independent work streams: なし（直列の critical path）
- User approval required before dispatch: yes

## Review Handoff

- 前提: Node ≥26.9.0 が mise で解決可能であること（要確認）
- 前提: `node --build-sea` が postject なしで完結するか、postject の
  依存追加をユーザー承認するか（実装時に確定）
- 参照: spec/ADR/discovery/design の 4 doc と issue #71 Phase 1
  チェックリストが upstream。Phase 2 以降は本 plan の対象外
