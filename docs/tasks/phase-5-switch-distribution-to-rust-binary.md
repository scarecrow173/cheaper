---
id: "TASK-034BgKkC4U7NTKprsORKCH"
type: "task"
status: "todo"
title: "Phase 5: switch distribution to Rust binary"
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
    - "docs/plans/phase-5-switch-distribution-to-rust-binary.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-5-switch-distribution-to-rust-binary.md"
    - "docs/tasks/phase-4-port-cli-and-workflow.md"
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
    - "docs/test-specs/rust-migration-distribution.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 5: switch distribution to Rust binary

## Work

plan `docs/plans/phase-5-switch-distribution-to-rust-binary.md`
(PLAN-034Beq14bSKdODWJFTdn02)の実行単位。Release 配布物の中身を Node SEA
executable から Rust + Rig の release binary へ切り替える。asset 名
(`cheaper-linux-x64` / `cheaper-linux-arm64` / `cheaper-windows-x64.exe`)・
`SHA256SUMS` 形式・`--version` probe・無改修 launcher(sh/ps1)による
取得・検証・exec の contract は一切変えず、変わるのは CI の build 手順
(cargo matrix)と binary の実装のみ。

plan の Task Breakdown T1-T4 が本 task の作業単位(直列):

- T1(Slice 1): workspace `Cargo.toml` の `[profile.release]` 確定
  (`lto`/`opt-level="s"`/`strip="symbols"`/`codegen-units=1`、
  **`panic = "abort"` は設定しない**)、`[workspace.package].version` を
  canonical version と一致、`scripts/check-binary-size.mjs` +
  `scripts/test/binary-size.test.mjs` 新設、`check-version.mjs`/
  `set-version.mjs`/`verifyReleaseVersion` に Cargo.toml 照合追加、
  `mise.toml` Rust toolchain 確認
- T2(Slice 2): `build-executables.yml` を cargo target triple matrix 化
  (win-msvc / linux-musl x64 / linux-musl arm64 / darwin-arm64 は Release
  非公開の検証 target)、`cargo build --release --target <triple>` →
  `dist/bin/<asset 名>` rename → size assert ≤20MiB → `--version`/`models`
  smoke → `bin-<target>` artifact upload。`release-workflow.test.mjs` 更新
- T3(Slice 3): `release-acquisition.yml` の artifact 整合
  (`sea-*`→`bin-*`・`dist/sea`→`dist/bin`)+ `smoke-bundle` matrix に
  `macos-latest` を sibling-only smoke として追加、5 asset(raw3 +
  `SHA256SUMS` + tar.gz)不変の確認
- T4(Slice 4): `smoke-release.mjs` sibling-only 経路、
  `launcher-acquisition.test.ts` の Rust binary fixture variant 追加、
  `pnpm bundle` + `build-apm-bundle.mjs --exe-dir` の整合確認、
  `release.md`/`distribution.md` 更新、最終 `mise run check`

不変条件: `releaseTargets` テーブル・manifest 規則・asset 名・
`packages/apm/.apm/hooks/cheaper`/`cheaper.ps1`/`hooks.json` は無改修
(diff が生じたら本 task の defect)。darwin binary は Release asset・
manifest・bundle に混入させない。TS SEA pipeline(`build-executable.mjs`)
は Phase 5 では変更しない(撤去は Phase 6)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-5-switch-distribution-to-rust-binary.md`(本 task が
  implement する plan。File Map・Slice 1-4・Verification の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、Phase 5
  依存条件)
- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (§配布切替・不変 contract・サイズ目標)
- `docs/designs/rust-rig-runtime-migration-architecture.md` §配布
- `docs/designs/version-pinned-launcher-acquisition.md`(launcher contract、
  検証対象であり変更しない)
- `docs/designs/sea-executable-build-pipeline-and-embedded-resource-resolution.md`
  (置換対象の現行 pipeline)
- `docs/test-specs/rust-migration-distribution.md`(本 task を検証する
  spec)

## Dependencies

- `docs/tasks/phase-4-port-cli-and-workflow.md` — front matter の
  `depends-on` は「Phase 4 完了(Rust `cheaper-cli`/`cheaper-workflow` が
  envelope・hook・usage.jsonl 互換で動作)かつ differential golden
  20-40 case 全パスが本 task の着手条件」であることを意味する。
  未了のまま着手した場合、本 plan は進行不可として扱う
- `docs/plans/phase-5-switch-distribution-to-rust-binary.md` — 承認済み
  plan への従属
- cargo workspace root path・Linux musl 可否は Phase 0/1 の決定を引き継ぐ
  (musl 未確定なら Slice 2 着手前に runner 上で試行して確定)

## Affected Files

- `.github/workflows/build-executables.yml`(cargo matrix 化)
- `.github/workflows/release-acquisition.yml`(artifact 整合・macOS smoke)
- `Cargo.toml`(workspace root。`[workspace.package].version` +
  `[profile.release]`)
- `mise.toml` / `mise.lock`(Rust toolchain)
- `scripts/check-binary-size.mjs`・`scripts/test/binary-size.test.mjs`
  (新規)
- `scripts/check-version.mjs` / `set-version.mjs` /
  `scripts/lib/release-artifacts.mjs` / `scripts/smoke-release.mjs`
- `scripts/test/release-artifacts.test.mjs` / `release-workflow.test.mjs`
- `packages/cli/test/e2e/launcher-acquisition.test.ts` /
  `native-executable-fixture.ts`(Rust binary fixture variant 追加)
- `docs/guides/release.md` / `docs/guides/distribution.md`

## Done When

- [ ] T1: release profile・size gate script・version 宣言照合
      (Cargo.toml/apm.yml/version.json/package.json の drift 検出)が
      実装される
- [ ] T2: `build-executables.yml` が 4 target の cargo matrix で動き、
      size assert が build 後・upload 前に fail 伝播する wiring になる
- [ ] T3: `release-acquisition.yml` が `bin-*` artifact と整合し、
      `smoke-bundle` が macOS sibling 経路を含む 4 OS で検証する構成に
      なる
- [ ] T4: launcher smoke(Rust binary fixture)・bundle 整合・guide 更新・
      `mise run check` 全 pass が完了する
- [ ] 不変条件の確認: `releaseTargets`・manifest 規則・launcher・
      `hooks.json` に diff が無い

## Verification

- [ ] `mise exec -- cargo build --release --target
      x86_64-pc-windows-msvc` / `--target x86_64-unknown-linux-musl` /
      `--target aarch64-unknown-linux-musl` / `--target
      aarch64-apple-darwin` が全 4 target で成功する
- [ ] 各 release binary(strip 後)が `node scripts/check-binary-size.mjs
      --file <binary>` で ≤20MiB と判定される
- [ ] 各 binary の `--version` が `packages/cli/package.json` の version
      と完全一致し、`models` が exit 0 を返す(CI build smoke)
- [ ] `mise exec -- node --test scripts/test/release-artifacts.test.mjs
      scripts/test/release-workflow.test.mjs scripts/test/binary-size.test.mjs`
      が exit 0
- [ ] `build-executables` の workflow_dispatch が全 matrix job で green
      (CI 実証。local では構造 test のみ)
- [ ] `release-acquisition` 1 dispatch で 5 asset
      (`cheaper-linux-x64`・`cheaper-linux-arm64`・
      `cheaper-windows-x64.exe`・`SHA256SUMS`・`cheaper-apm-v<X.Y.Z>.tar.gz`)
      が Rust binary 入りで公開される
- [ ] `smoke-bundle` が windows-latest / ubuntu-latest /
      ubuntu-24.04-arm / macos-latest で pass する(macOS は sibling
      経路・他 3 OS は Node-free launcher acquisition 経路)
- [ ] `launcher-acquisition.test.ts` の Rust binary variant が stub
      Release からの download → SHA256 照合 → `--version` probe → exec を
      無改修 launcher で pass する
- [ ] hooks.json 経路の `cheaper hook <event> --target <agent>`
      invocation が Rust binary で実行され fail-open 契約が維持される
- [ ] `pnpm bundle` 後の `node scripts/check-apm-payload.mjs` と
      `git diff --exit-code -- packages/apm/.apm/hooks/cli` が pass する
- [ ] `mise run check` 全体が exit 0(check-version の Cargo.toml 照合
      を含む)
- [ ] 旧 tag の Release asset が変更されず、旧 version.json pin の
      consumer が従来通り SEA binary を取得できる(rollback 経路の確認)
