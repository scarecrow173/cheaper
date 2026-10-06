---
id: "PLAN-034Beq14bSKdODWJFTdn02"
type: "plan"
status: "approved"
title: "Phase 5 switch distribution to Rust binary"
created: "2026-10-06"
updated: "2026-10-06"
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
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-4-port-cli-and-workflow.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-distribution.md"
  references:
    - "docs/designs/version-pinned-launcher-acquisition.md"
    - "docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md"
  defers: []
  deferred-by: []
---

# Phase 5 switch distribution to Rust binary Implementation Plan

## Goal

Release 配布物の中身を Node SEA executable から Rust + Rig の release binary
へ切り替える。asset 名(`cheaper-linux-x64` / `cheaper-linux-arm64` /
`cheaper-windows-x64.exe`)・`SHA256SUMS` 形式・`--version` probe・無改修
launcher(sh/ps1)による取得・検証・exec の contract は一切変えず、変わるのは
CI の build 手順(cargo matrix)と binary の実装のみ。

## Scope

単一の implementation stream として扱う。境界:

- **In**: `build-executables.yml` の cargo build matrix 化(Windows x64 /
  Linux x64 / macOS + contract 必須の Linux arm64)、release profile(lto /
  opt-level / strip)確定、binary size assert(≤20MB)ゲート、
  `release-acquisition.yml` の artifact 整合と macOS sibling smoke 追加、
  version 宣言への Cargo.toml 組込み、`pnpm bundle` 経路との整合、
  release/distribution guide の更新。
- **Out**: TS 実装・SEA build 経路(`scripts/build-executable.mjs`)の撤去
  (Phase 6)、`cheaper.mjs` JS fallback の廃止判断(Phase 6 の Deferred
  concern)、darwin-* の Release asset 追加(設計書 Deferred を維持し、
  macOS binary は sibling smoke 検証用途のみ)、launcher/hooks.json/
  `packages/apm/` 構造の変更(無改修が contract)。

前提条件: Phase 4 完了(`cheaper-cli` / `cheaper-workflow` の Rust binary が
envelope・hook・usage.jsonl 互換で動作)かつ differential golden 20-40 case
全パス(親 plan の Phase 5 依存条件)。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (§配布切替・不変 contract・サイズ目標 ≤20MB / 目標 5-15MB)
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md` (親 plan、Phase 5 依存)

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md` §配布
  (`cargo build --release` + `lto` + `opt-level="s"` + strip、asset 名・
  SHA256SUMS・`--version` probe 不変、launcher 無改修、
  `build-executables.yml` を cargo matrix に置換、mise.toml に Rust
  toolchain)、§段階計画 Phase 5、ISS-21(CI matrix に size assert 追加)、
  Deferred(version canonical source は Phase 5 までに決定)
- `docs/designs/version-pinned-launcher-acquisition.md`(launcher の取得・
  検証 contract。本 phase は検証対象であり変更しない)
- `docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md`
  (置換対象の現行 pipeline)

## File Map

| path | responsibility |
| --- | --- |
| `.github/workflows/build-executables.yml` | cargo build matrix へ置換。checkout → mise → expected_version gate → `cargo build --release --target <triple>` → strip 確認 → asset 名へ rename → size assert → `--version`/`models` smoke → artifact upload |
| `.github/workflows/release-acquisition.yml` | download artifact pattern と `--asset-dir` の rename 追従、smoke-bundle matrix に `macos-latest`(darwin sibling smoke)追加。job DAG・permissions・publish 手順は不変 |
| `<cargo workspace>/Cargo.toml`(workspace root、Phase 1 で `rust/` または `packages-rs/` に確定) | `[workspace.package].version` = canonical version、`[profile.release]` に `lto = true`・`opt-level = "s"`・`strip = "symbols"`・`codegen-units = 1` を確定。**`panic = "abort"` は設定しない**(catch_unwind → INTERNAL_ERROR 境界が unwind を要求) |
| `mise.toml` | Rust toolchain entry(Phase 1 で導入済み前提。未導入なら `rust` tool を locked version で追加) |
| `scripts/check-binary-size.mjs`(新規) | release binary の byte サイズを上限(既定 20MiB)と比較する stdlib-only CLI。CI matrix 各 job の gate |
| `scripts/test/binary-size.test.mjs`(新規) | size gate の境界(超過拒否・ちょうど上限許容・不在 file 失敗)の node:test suite |
| `scripts/check-version.mjs` / `scripts/set-version.mjs` | Cargo.toml workspace version を宣言照合・bump 対象に追加(version drift を `mise run check` で検出) |
| `scripts/lib/release-artifacts.mjs` | `verifyReleaseVersion` に Cargo.toml workspace version の照合を追加。`releaseTargets`・manifest 規則・snapshot 構造は不変 |
| `scripts/smoke-release.mjs` | darwin 系 target を sibling-only smoke として扱う経路を追加(acquisition 非対象のまま)。既存 3 target の検証は不変 |
| `scripts/test/release-artifacts.test.mjs` / `release-workflow.test.mjs` | Cargo.toml 宣言照合・rename 後の workflow 構造・size gate wiring の test を更新 |
| `scripts/build-executable.mjs` / `scripts/lib/esbuild-bundle.mjs` | Phase 5 では変更しない(TS 版並走・differential golden の TS binary 供給に使用)。撤去は Phase 6 |
| `scripts/bundle-cli.mjs` / `scripts/lib/apm-bundle.mjs` / `scripts/build-apm-bundle.mjs` / `scripts/check-apm-payload.mjs` | 変更なしを維持する確認対象。`build-apm-bundle.mjs --exe-dir` は同名 asset を置けば Rust binary をそのまま sibling 化できる |
| `packages/apm/.apm/hooks/cheaper` / `cheaper.ps1` / `hooks.json` | **無改修**(contract)。diff が生じたら本 phase の defect として扱う |
| `docs/guides/release.md` / `docs/guides/distribution.md` | SEA → cargo binary の記述更新(5 asset・smoke 構成・musl static・macOS は sibling のみ) |
| `packages/cli/test/e2e/launcher-acquisition.test.ts` / `native-executable-fixture.ts` | launcher が Rust binary を取得・検証・exec する経路を、cargo 出力 fixture で回す variant を追加(既存 suite は TS binary 用に維持) |

## Implementation Sequence

### Slice 1: release profile と size gate

- [ ] workspace root `Cargo.toml` の `[profile.release]` に `lto = true`・`opt-level = "s"`・`strip = "symbols"`・`codegen-units = 1` を設定し、`panic` 未設定(unwind 維持)を確認する
- [ ] `[workspace.package].version` を `packages/cli/package.json` の canonical version と一致させる
- [ ] `scripts/check-binary-size.mjs` を実装する(`--file <path>` `[--max-mb <n>]`、byte ≤ 20MiB で exit 0、超過・不在・非 regular file は exit 1)
- [ ] `scripts/test/binary-size.test.mjs` で境界ケースを GREEN にする
- [ ] `check-version.mjs` / `set-version.mjs` / `release-artifacts.mjs` の `verifyReleaseVersion` に Cargo.toml workspace version を照合対象として追加する
- [ ] `mise.toml` に Rust toolchain が無ければ locked version で追加する

### Slice 2: build-executables.yml の cargo matrix 化

- [ ] matrix を cargo target triple へ再定義する: `windows-latest`→`x86_64-pc-windows-msvc`→`cheaper-windows-x64.exe`、`ubuntu-latest`→`x86_64-unknown-linux-musl`→`cheaper-linux-x64`、`ubuntu-24.04-arm`→`aarch64-unknown-linux-musl`→`cheaper-linux-arm64`、`macos-latest`→`aarch64-apple-darwin`→`cheaper-darwin-arm64`(Release 非公開の検証 target)
- [ ] `pnpm install`/`pnpm build`/`build-executable.mjs` step を `cargo build --release --target <triple>` に置換し、出力 binary を `dist/bin/<asset 名>` へ rename 配置する
- [ ] expected_version gate(`packages/cli/package.json` 照合)と smoke step(`--version` が canonical version と完全一致・`models` exit 0)を維持する
- [ ] `scripts/check-binary-size.mjs` による ≤20MiB assert step を build 後・upload 前に挿入する
- [ ] upload artifact 名を `bin-<target>` に rename する(artifact 内容は従来通り単一 executable)
- [ ] `release-workflow.test.mjs` を更新し、matrix/asset 名/permissions/size gate 挿入位置を pin する

### Slice 3: release-acquisition.yml の整合と macOS smoke

- [ ] `download-artifact` の `pattern: sea-*` → `bin-*`、`--asset-dir dist/sea` → `dist/bin` に追従する(assemble/smoke/publish の 3 job)
- [ ] `assemble-release` が変更なしで動くことを確認する(asset 名不変のため `generateManifest`/`copySnapshot`/`SHA256SUMS` 生成はそのまま Rust binary を hash する)
- [ ] `smoke-bundle` matrix に `macos-latest` を追加し、darwin artifact(`bin-darwin-arm64`)を download して展開済み package の `.apm/hooks/cli/bin/darwin-arm64/cheaper` に mode 0755 で sibling 配置してから `smoke-release.mjs` を実行する
- [ ] `publish-release` の `gh release create` asset 列挙が raw3 + `SHA256SUMS` + tar.gz の 5 asset のままであることを確認する(darwin binary は upload しない)
- [ ] `release-artifacts.test.mjs`/`release-workflow.test.mjs` を GREEN にし、`resolve-release` pre-gate の focused suite として引き続き動くことを確認する

### Slice 4: launcher smoke・bundle 整合・docs

- [ ] `smoke-release.mjs` に sibling-only smoke 経路を追加する(`process.platform-arch` が `releaseTargets` 外なら acquisition 検証を skip し、deploy 済み sibling の存在・sha256・`--version`/`models` を launcher 経由で検証)
- [ ] `launcher-acquisition.test.ts` に Rust binary fixture variant を追加し、無改修 launcher が stub Release から Rust binary を取得 → SHA256SUMS 照合 → `--version` probe → exec する経路を検証する
- [ ] `pnpm bundle`(`buildApmCliTree`)が `cli/bin/cheaper.mjs`・`version.json`・`workers/`・`policy.yaml` を従来通り生成し、`check-apm-payload.mjs` が pass することを確認する(Rust binary は builtin resource を `include_dir!` 埋込のため bundle 側の構造変更は不要。`cheaper.mjs` fallback は Phase 6 まで維持)
- [ ] `build-apm-bundle.mjs --exe-dir` に cargo 出力 rename 済み asset を渡して sibling 配置されることを確認する(hook からの実行経路 = launcher sibling 優先)
- [ ] `release.md` を更新する(3 target SEA → cargo matrix + size gate、artifact 名変更、macOS smoke 追加、5 asset 不変)
- [ ] `distribution.md` を更新する(binary の実装が Rust に変わる旨、darwin は sibling のみ・acquisition 非対象の維持、preseed/rollback 手順の不変)
- [ ] `mise run check` 全 pass(check-version の Cargo.toml 照合を含む)を確認する

## Dependencies and Risks

- **依存**: Phase 4 完了(Rust `cheaper-cli` が envelope/exit code/`--version`/
  `models`・hook 4 target・usage.jsonl を互換動作)と differential golden
  全パスが着手条件。cargo workspace root path(`rust/` か `packages-rs/`)、
  Linux target の musl 可否は Phase 0/1 の決定を引き継ぐ。
- **asset 名不一致 risk**: `releaseTargets` テーブルと manifest 規則を変更
  しないことが不変条件。`manifest requires each target exactly once`・
  `asset names match launcher resolution` の既存 named test を維持し、
  workflow test で asset 名を pin する。
- **size 超過 risk**: ≤20MB を超えたら size assert が CI を fail させる
  (ISS-21)。対処順序: 未使用 feature 削減 → `opt-level = "z"` 検討 → 依存
  見直し。20MB 超過での緩和・閾値引上げは行わず、親 plan へ escalate。
- **launcher 検証失敗 risk**: SHA256 mismatch(manifest と公開 binary の
  不整合)は assemble/publish の `verifyAssets` が防ぐ。`--version` 不一致は
  binary 埋込 version(workspace version)と canonical version の drift が
  原因なので、check-version/verifyReleaseVersion の Cargo.toml 照合 + build
  smoke の完全一致 probe で二重に防ぐ。Windows CRLF 許容は launcher 側で
  済んでおり、Rust 側 stdout は bare semver 1 行のみを出力する。
- **musl static build 失敗 risk**: C 依存を持つ crate が musl で link 失敗
  した場合は `x86_64-unknown-linux-gnu` への fallback を理由付きで記録し、
  asset 名・contract は変えない(spec の「musl static を検討対象」の範囲内)。
- **darwin asset 誤公開 risk**: darwin binary を Release asset・manifest・
  bundle の `releaseTargets` に混入させない(workflow test で 5 asset 固定を
  pin)。macOS smoke は sibling 経路のみで acquisition を試みない。
- **version 宣言 drift risk**: canonical source は Phase 5 時点で
  `packages/cli/package.json` を維持する決定とする(TS 並走期間中に Cargo.toml
  への移行は宣言経路の二重化を招く)。check-version 拡張で drift を機械検出。
- **rollback 前提 risk**: 旧 tag の Release(SEA asset)を上書き・削除しない
  (tag immutable・既存 Release 拒否が既存 contract)。同一 asset 名を維持
  するため、launcher が pin する旧 version tag は引き続き SEA binary を
  取得・検証・exec できる。

## Task Breakdown

| task 候補 | 対応 slice | 概要 |
| --- | --- | --- |
| T1 | Slice 1 | cargo release profile・size gate script・version 宣言照合 |
| T2 | Slice 2 | build-executables.yml cargo matrix 化 + workflow test 更新 |
| T3 | Slice 3 | release-acquisition.yml 整合 + macOS sibling smoke |
| T4 | Slice 4 | launcher smoke(Rust fixture)・bundle 整合確認・guide 更新・最終 `mise run check` |

T1→T2→T3→T4 は直列依存(workflow file 変更が後続を前提にする)。T4 内の
docs 更新のみ他と並行可能。

## Verification

- [ ] `mise exec -- cargo build --release --target <triple>` が全 4 target
      (win-msvc / linux-musl x64 / linux-musl arm64 / darwin-arm64)で成功する
- [ ] 各 release binary(strip 後)が `scripts/check-binary-size.mjs` で
      ≤20MiB と判定され、CI matrix の gate として fail 伝播する
- [ ] 各 binary の `--version` が `packages/cli/package.json` の version と
      完全一致し、`models` が exit 0 を返す(build smoke、CI のみ実証)
- [ ] `mise exec -- node --test scripts/test/release-artifacts.test.mjs scripts/test/release-workflow.test.mjs scripts/test/binary-size.test.mjs` が exit 0
- [ ] `build-executables` の workflow_dispatch が全 matrix job で green
      (CI 実証。local では構造 test のみ)
- [ ] `release-acquisition` 1 dispatch で 5 asset(`cheaper-linux-x64`・
      `cheaper-linux-arm64`・`cheaper-windows-x64.exe`・`SHA256SUMS`・
      `cheaper-apm-v<X.Y.Z>.tar.gz`)が Rust binary 入りで公開される
- [ ] `smoke-bundle` が windows-latest / ubuntu-latest / ubuntu-24.04-arm /
      macos-latest で pass し、macOS は sibling 経路・他 3 OS は Node-free
      launcher acquisition 経路で `--version`/`models` を検証する
- [ ] `launcher-acquisition.test.ts` の Rust binary variant が、stub
      Release からの download → SHA256 照合 → `--version` probe → exec を
      無改修 launcher で pass する
- [ ] hooks.json 経路: APM deploy 済み launcher への
      `cheaper hook <event> --target <agent>` invocation が Rust binary で
      実行され、fail-open 契約(非 0 を返さない/allow 変換)が維持される
- [ ] `pnpm bundle` 後の `check-apm-payload.mjs` と
      `git diff --exit-code -- packages/apm/.apm/hooks/cli` が pass
      (`mise run check` 内)
- [ ] `mise run check` 全体が exit 0(check-version の Cargo.toml 照合含む)
- [ ] 旧 tag の Release asset が変更されず、旧 version.json pin の
      consumer が従来通り SEA binary を取得できる(rollback 経路の確認)

## Rollback Strategy

- **配布切替前(merge 前)**: 全変更は workflow/script/docs のみ。`git revert`
  で SEA pipeline へ戻せる。
- **配布切替後**: launcher は `version.json` で tag pin 済みのため、rollback
  は「consumer の deploy を旧 version(旧 tag の APM package)へ戻す」操作。
  asset 名・`SHA256SUMS` 形式を不変に保ったため、旧 tag の SEA binary は
  そのまま取得・検証・exec できる。Rust binary に固有の defect が見つかった
  場合も fix-forward(新 immutable tag)で対処し、既存 Release は書き換え
  ない。
- **中途半端な公開**: `publish-release` の既存拒否・draft 公開・手動復旧
  手順は現行 contract を維持し、本 phase では変更しない。

## Open Questions / Gaps

- cargo workspace root の確定 path(`rust/` vs `packages-rs/`)は Phase 1
  出力依存。File Map の `<cargo workspace>/Cargo.toml` は着手時に確定値へ
  読み替える。
- Linux target の musl static 可否は Phase 0 PoC の実測結果を引き継ぐ。
  未確定なら Slice 2 着手前に runner 上で `cargo build --release --target
  x86_64-unknown-linux-musl` を試行して確定する(gnu fallback は contract
  非影響)。
- macOS binary を Release asset に含めるかは設計書 Deferred(darwin-* の
  Release 追加は launcher の darwin acquisition 分岐解放とセット)を維持。
  本 phase は build 検証 + sibling smoke のみ。
- version canonical source の Cargo.toml 移行は Phase 6(TS 撤去)時に再判定。
  Phase 5 は `packages/cli/package.json` canonical + Cargo.toml 照合追加。
- `x86_64-apple-darwin`(Intel Mac)は matrix に含めない(`macos-latest` は
  arm64)。Intel 対応が必要になった場合は Deferred として再検討。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(workflow/script/docs が中心で crate 実装
  を含まず、単一 stream で依存関係が直列)
- Independent work streams: なし(Slice 4 の docs 更新のみ部分的に並行可)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー重点: (a) asset 名・`releaseTargets`・manifest 規則・launcher が
  本当に無改変か、(b) darwin binary が Release asset/manifest/bundle に
  混入していないか、(c) size gate が build 後・upload 前に fail 伝播する
  wiring か、(d) `panic = "abort"` が release profile に混入していないか
  (unwind 必須)、(e) version 宣言照合が package.json/apm.yml/version.json/
  Cargo.toml の 4 者を網羅するか。
- 前提: Phase 4 完了・differential golden 全パスが親 plan の依存条件。
  未了のまま着手した場合、本 plan は進行不可として扱う。
- CI-only 検証領域(matrix build・3 OS smoke・実公開)は local で再現
  できないため、構造 test・dry-run・実 dispatch の 3 層で evidence を分離
  して報告すること(現行 `release.md` の「発火前の検証範囲」と同じ規範)。
