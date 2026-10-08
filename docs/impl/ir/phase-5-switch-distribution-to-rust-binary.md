---
id: "IMPL-tCVQX1QfT8lGHPsrMuqvEZ"
type: "impl"
status: "completed"
title: "Phase 5: switch distribution to Rust binary"
created: "2026-10-08"
updated: "2026-10-08"
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
    - "docs/tasks/phase-5-switch-distribution-to-rust-binary.md"
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

# Phase 5: switch distribution to Rust binary

## Summary

Release 配布物の中身を Node SEA executable から Rust + Rig の release
binary へ切り替えた。asset 名(`cheaper-linux-x64` /
`cheaper-linux-arm64` / `cheaper-windows-x64.exe`)・`SHA256SUMS`
形式・`--version` probe・無改修 launcher(sh/ps1)による取得・検証・
exec の contract は一切変更せず、変わるのは CI の build 手順(cargo
target triple matrix)と binary の実装のみ。`releaseTargets`
テーブル・manifest 規則・`packages/apm/.apm/hooks/cheaper` /
`cheaper.ps1` / `hooks.json` は無改修(`git diff main...HEAD` で
0 行差分を確認)。

実装は 5 commit で構成: 1c33da3(Slice 1: size gate + version 照合)→
919ed8a(Slice 2: cargo matrix)→ aa578b2(Slice 3: acquisition 整合 +
macOS smoke)→ fa138b9(Slice 4: launcher 検証 + docs)→
0d797c0(review fix)。

## Context

TASK-034BgKkC4U7NTKprsORKCH
(`docs/tasks/phase-5-switch-distribution-to-rust-binary.md`)/ plan
`docs/plans/phase-5-switch-distribution-to-rust-binary.md`、親 plan
`docs/plans/migrate-cheaper-runtime-to-rust-rig.md`。関連:
`docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`、
`docs/designs/rust-rig-runtime-migration-architecture.md` §配布、
`docs/designs/version-pinned-launcher-acquisition.md`(無改修
launcher の acquisition contract)、`docs/test-specs/
rust-migration-distribution.md`。前提は Phase 4 で配線済みの
`crates/cheaper-cli` binary(`cheaper`)。

## Implementation

実装は 5 commit(4 slice + review fix)で構成される。各 slice の
Scope・Files・検証は以下の節。CI 実証に委譲した runner 依存の
検証は `## Follow-ups` の `### CI-deferred verification` に集約。

### T1 — release profile・size gate・version 宣言照合(Slice 1)

#### Scope

workspace root `Cargo.toml` の `[profile.release]` を確定し、配布
binary の version 宣言を canonical version 照合の対象に加え、CI gate
となる size assert script を新設した。

#### Files

- `Cargo.toml` — `[profile.release]` に `lto = true`・
  `opt-level = "s"`・`strip = true`・`codegen-units = 1`・
  `panic = "unwind"` を設定。**`panic = "abort"` は設定しない**:
  `cheaper-cli` の command 境界(`main.rs` の `catch_unwind` →
  `INTERNAL_ERROR`(70)envelope)と hook の fail-open 経路
  (`catch_run_panic` → allow + exit 0)は panic を捕捉して canonical
  な失敗形へ畳む設計であり、abort にすると envelope 無し終了・hook
  deny 化で外部 contract が壊れる。
  `[workspace.package].version = "1.0.0"` を宣言。canonical source
  は引き続き `packages/cli/package.json` で、本宣言は drift 検出専用
  (check-version/verifyReleaseVersion が照合する)。
- `crates/cheaper-cli/Cargo.toml` — `version.workspace = true` で
  workspace 宣言を継承。配布 binary はこの crate のみ。library
  crate(core/tools/runtime/workflow 等)は publish しない内部 path
  依存のため `0.1.0` のまま。
- `scripts/check-binary-size.mjs` — stdlib-only の ≤20MiB gate。
  `--file <path> [--max-mb <n>]`。missing/non-regular/oversize/不正
  引数は exit 1。`build-executables.yml` で build 後・upload 前に
  挟む。
- `scripts/test/binary-size.test.mjs` — boundary suite(7 case:
  under/at/over limit・default 20MiB・missing・directory・bad args)。
- `scripts/lib/release-artifacts.mjs` — `cargoWorkspaceVersion`
  (`[workspace.package]` section 内の `version` 行のみを読む。
  TOML library は使わず section 切り出し + 行 regex)、
  `setCargoWorkspaceVersion`(match offset での index-based splice
  + literal 置換)、`apmPackageVersion`(`version:` 行の一意抽出)、
  `verifyReleaseVersion` への Cargo.toml 照合追加。
- `scripts/check-version.mjs` — 宣言照合表に
  `["Cargo.toml", cargoWorkspaceVersion]` を追加(package.json /
  apm.yml / version.json と同一 gate)。
- `scripts/set-version.mjs` — bump 時に `setCargoWorkspaceVersion`
  で Cargo.toml も更新し、末尾の `check-version.mjs` 再実行で全宣言
  を検証。
- `mise.toml` / `mise.lock` — `rust = "1.99.0"` を pin(rig の
  rust-version 1.95 要件を満たす固定 version。locked-toolchain
  contract — 当初 `stable` 指定だったものを review fix で固定化)。

#### 検証

- `mise exec -- cargo build --release --target
  x86_64-pc-windows-msvc -p cheaper-cli` — 成功。
- `node scripts/check-binary-size.mjs --file
  target/x86_64-pc-windows-msvc/release/cheaper.exe` — **実測
  18.9MiB**(gate 20MiB)で pass。
- `node --test scripts/test/binary-size.test.mjs` — 7 case 全 pass。
- `node scripts/check-version.mjs` — package.json / core /
  tool-runtime / workflow / runtime-pi / apm.yml / version.json /
  Cargo.toml の全宣言が 1.0.0 で一致。
- `mise exec -- cargo run --release -p cheaper-cli -- --version` /
  `models` — `--version` = 1.0.0、`models` が `ok:true` envelope を
  返すことを local で確認。

### T2 — `build-executables.yml` の cargo matrix 化(Slice 2)

#### Scope

SEA build を置き換え、4 target の cargo release matrix で reusable
workflow を構成した。trigger(`workflow_dispatch` + `workflow_call`)・
asset 名・permissions は不変。

#### Files

- `.github/workflows/build-executables.yml` — matrix `include` に
  `os`/`target`/`cargo_target`/`artifact` の 4 列を定義:
  `windows-latest`→`x86_64-pc-windows-msvc`→
  `cheaper-windows-x64.exe`、`ubuntu-latest`→
  `x86_64-unknown-linux-musl`→`cheaper-linux-x64`、
  `ubuntu-24.04-arm`→`aarch64-unknown-linux-musl`→
  `cheaper-linux-arm64`、`macos-latest`→`aarch64-apple-darwin`→
  `cheaper-darwin-arm64`(Release 非公開の検証 target)。
  step 列: checkout(固定 ref)→ mise-action(`--locked`)→
  `expected_version` 与件時の package.json 照合 →
  `mise exec -- rustup target add`(mise 管理 toolchain へ
  `RUSTUP_TOOLCHAIN` を伝える)→ musl target のみ `musl-tools`
  install(aws-lc-sys / ring が rustls 経由で要求する C toolchain)→
  `mise exec -- cargo build --release --locked --target
  <triple> -p cheaper-cli` → `dist/bin/<asset 名>` へ rename stage →
  `check-binary-size.mjs`(build 後・upload 前の fail 伝播 gate)→
  `--version` が package.json version と完全一致 + `models` exit 0
  smoke → `bin-<target>` 名で artifact upload。
- `scripts/test/release-workflow.test.mjs` — matrix が 4 target の
  cargo triple を持ち、size assert が build と upload の間に位置し、
  `bin-*` artifact 名を使う wiring を構造 test で pin
  (`build-executables builds the cargo release matrix behind a size
  gate`)。

#### 検証

- wiring は `release-workflow.test.mjs` で pin 済み。matrix 実 build
  は runner 依存(musl-tools・rustup target・各 OS toolchain)のため
  CI 実証に委譲 — local で win-msvc のみ build+size+smoke 実測済み
  (T1)。

### T3 — `release-acquisition.yml` 整合 + macOS sibling smoke(Slice 3)

#### Scope

downstream job が `sea-*` artifact / `dist/sea` を参照していたのを
`bin-*` / `dist/bin` へ揃え、`smoke-bundle` matrix に macOS の
sibling-only leg を追加した。

#### Files

- `.github/workflows/release-acquisition.yml` —
  - `resolve-release` — `verifyReleaseVersion`(Cargo.toml 照合込み)
    + focused suites(`binary-size.test.mjs`・
    `release-artifacts.test.mjs`・`release-workflow.test.mjs`)実行 +
    固定 SHA 出力。後段は変更なし。
  - `assemble-release` / `publish-release` — artifact pattern を
    `bin-*`・download path を `dist/bin` へ。manifest 生成・
    `SHA256SUMS --verify`・draft→public の 5 asset 列挙
    (`cheaper-linux-x64`・`cheaper-linux-arm64`・
    `cheaper-windows-x64.exe`・`SHA256SUMS`・`cheaper-apm-v*.tar.gz`)
    は不変。
  - `smoke-bundle` — matrix に `macos-latest` を追加。macOS leg のみ
    `bin-darwin-arm64` artifact を download し、展開済み package の
    `.apm/hooks/cli/bin/darwin-arm64/cheaper` へ 0755 で sibling
    配置してから共通 smoke(`smoke-release.mjs`)を実行する。
    darwin-arm64 は `releaseTargets` に含まれず acquisition は
    試みない。
- `scripts/smoke-release.mjs` — sibling-only 経路を追加。
  `process.platform` が `releaseTargets` 非対象(darwin-arm64)の
  場合、deploy 前に staged sibling の存在を要求し hash を採取、APM
  install 後に deploy 先 sibling の sha256 が staged 側と一致する
  ことを照合(manifest 対象外なので package 内 hash が基準)。
  従来の「非対象 platform = fail」を sibling 必須検査へ置換。
  実行検証(`--version`/`models` の Node-free launcher smoke)は
  共通経路が担う。
- `scripts/test/release-artifacts.test.mjs` /
  `release-workflow.test.mjs` — `releaseTargets` が 3 非 darwin
  asset のままであること・macOS leg が sibling を stage して
  acquisition しないことの pin を追加。

#### 検証

- 構造は test pin で確認。dispatch 実証(実 runner での sibling
  配置・deploy 後 hash 一致・launcher exec)は CI に委譲。

### T4 — launcher 検証・bundle 整合・docs・最終 check(Slice 4)

#### Scope

無改修 launcher が Rust binary を取得・検証・実行することを実
binary で確認し、bundle 経路の整合と guide 更新、最終 `mise run
check` を実施した。

#### Files

- `packages/cli/test/e2e/launcher-acquisition.test.ts` — Rust binary
  variant を追加(`acquires a cargo-built Rust release binary through
  the unmodified launcher`)。`target/release/<exe>` があれば stub
  Release からの download → `SHA256SUMS` 照合 → `--version` probe →
  exec を無改修 launcher で実施し、続いて検証済み cache から
  `hook pre-read-code --target windsurf` を実行して exit 2 +
  bulk-read 誘導を確認。binary 不在の環境(CI の Node-only lane)は
  `t.skip`。sh・pwsh 両 leg で実施。
- `scripts/smoke-release.mjs` — T3 の sibling hash 照合(本 slice で
  deploy 後同一性 assert まで完成)。
- `docs/guides/release.md` — version 宣言表に Cargo.toml 行を追加、
  job 記述を cargo matrix + size gate + `bin-*` artifact + 4 OS
  smoke(macOS sibling 経路)へ更新、asset 表を Rust binary 表記へ。
- `docs/guides/distribution.md` — pipeline が Rust binary へ切替
  済み・初回 Rust release は未公開である旨の注記、macOS sibling
  smoke の記述、dev consumer は `cargo build --release -p
  cheaper-cli` 出力(並走期間中は SEA 出力も可)を sibling 配置する
  旨を追記。

#### 検証

- Rust binary variant test: sh・pwsh 両 leg で `target/release/
  cheaper.exe` 実測 pass(stub Release download → SHA256 → probe →
  exec → hook exit 2)。
- `pnpm bundle` + `node scripts/check-apm-payload.mjs` +
  `git diff --exit-code -- packages/apm/.apm/hooks/cli` — pass。
  `build-apm-bundle.mjs --exe-dir` に rename 済み cargo binary を
  渡し、sibling 配置・launcher sibling 実行も確認(同 script は無改修
  で Rust binary を受理 — exe-dir の中身を問わない contract)。
- `mise run check` — lint / check-version(Cargo.toml 照合込み)/
  doc-ids / typecheck / bundle / payload / git diff は全 pass。
  `pnpm test` 内の `cross-agent-routing-adapters.test.ts` が負荷
  flake(20s timeout)で 1 件 fail したが isolate 再実行で 32/32
  pass(既知 flake、本 Phase の変更と無関係)。

### Review fix(0d797c0)— finding と解決

review で confirm された 6 finding を修正し、CI 実証が必要な 3
item はユーザー判断で延期した。

- R1 `binary-size.test.mjs` が release pre-gate から抜けていた —
  `resolve-release` の focused suites 行へ追加し、
  `release-workflow.test.mjs` の pin を同時更新。
- R2 rust toolchain が `stable` 浮動 pin だった — locked-toolchain
  contract 違反のため `rust = "1.99.0"` を `mise.toml`/`mise.lock`
  に固定(rig の rust-version 1.95 要件を満たす最小固定)。
- R3 `setCargoWorkspaceVersion` の rewrite バグ 2 件 — (a)
  `text.replace(section, …)` が index 0 から同一文字列を探し前方の
  出現を書き換え得る + section 内 `$&` 等が replacement pattern
  として展開される問題 → match offset ベースの splice + function
  replacement 化。(b) version 行 regex の `\s` が `\n` を食い次行の
  comment 行を match に巻き込み rewrite で消す問題 → `[ \t]` +
  `\r?` に限定。regression test 2 件追加
  (`release-artifacts.test.mjs` の splice/no-pattern-expansion・
  section-only rewrite)。
- R4 `apm.yml` の version 抽出が `check-version.mjs` と
  `verifyReleaseVersion` で別実装だった — `apmPackageVersion`
  (section 非依存の `version:` 行の一意抽出)へ統一。
- R5 launcher e2e Rust variant の guard 不足 — `spawnSync` に
  timeout 追加・status assert・`--version` 出力の semver regex を
  anchor 化。
- R6 `distribution.md` の記述が pipeline 切替と初回 release 未公開
  を曖昧にしていた — 明確化。

CI 実証 item(延期 — 理由は末尾 §CI-deferred verification):

- IC-1 `mise exec -- rustup target add` が runner の mise/rustup
  解決で `RUSTUP_TOOLCHAIN` を正しく伝えるか。
- IC-2 `musl-tools` install 後に aws-lc-sys / ring 依存が
  musl triple で link できるか。
- IC-3 artifact download → staging → deploy 経路で Linux binary の
  exec bit(0755)が保持されるか。

## Related Experiments

### Adopted

experiment log への正式参照は無い。採用判断(`panic = "unwind"`
維持・temp dir 無しの dist/bin stage・macOS sibling-only smoke・
`cargoWorkspaceVersion` の stdlib 行 regex 照合等)は各 slice の
Files・Review fix に記録済み。

### Rejected

`[profile.release] panic = "abort"` — command 境界の
`catch_unwind` → `INTERNAL_ERROR`(70)envelope と hook の
fail-open(allow + exit 0)を破壊するため不採用(### T1 Files 参照)。

## Validation

local で実測可能な項目は各 slice の検証節と以下の
`### Verification matrix` を参照。要点: win-msvc release build
成功・実測 **18.9MiB**(≤20MiB gate)・`--version`/`models` smoke・
無改修 launcher 経由の acquisition + hook fail-open(sh/pwsh 両
leg 実 binary)・`pnpm bundle` + `check-apm-payload` + hooks/cli
git diff・`mise run check` の lint/version/doc-ids/typecheck/
bundle/diff 全 pass(`pnpm test` の既知 flake 1 件は isolate 再実行
32/32 pass)。runner 依存の残検証は `## Follow-ups` の
`### CI-deferred verification` に集約。

### Verification matrix(task doc §Verification との対応)

| item | local 結果 | 状態 |
| --- | --- | --- |
| 4 target `cargo build --release` | win-msvc のみ build 成功(`target/x86_64-pc-windows-msvc/release/cheaper.exe`) | 残 3 target は CI |
| size gate ≤20MiB | win-msvc 実測 **18.9MiB** pass | 残 3 target は matrix job の gate |
| `--version` / `models` smoke | `--version`=1.0.0・`models` ok:true を実 binary で確認 | 全 target 分は CI |
| focused stdlib suites(`binary-size`・`release-artifacts`・`release-workflow`) | release-artifacts の既知 bsdtar `F:` flake 1 件を除き全 pass(clean HEAD でも同 fail) | 完全 exit 0 は CI |
| `build-executables` workflow_dispatch green | 構造 test pin のみ | CI |
| `release-acquisition` 1 dispatch で 5 asset 公開 | asset 名・列挙の構造確認のみ | CI |
| `smoke-bundle` 4 OS | wiring test pin + smoke-release.mjs sibling 経路実装 | CI |
| launcher acquisition(Rust binary variant) | sh/pwsh 両 leg で実 binary pass | 済 |
| hooks.json 経路の `cheaper hook` fail-open | Rust binary を launcher 経由で `pre-read-code --target windsurf` 実行 → exit 2 + bulk-read 誘導 | 済 |
| `pnpm bundle` + `check-apm-payload` + hooks/cli git diff | pass(`build-apm-bundle.mjs --exe-dir` sibling 配置も実測) | 済 |
| `mise run check` | lint/version/doc-ids/typecheck/bundle/payload/diff 全 pass、test は既知 flake 1 件(isolate 再実行で 32/32 pass) | 完全 exit 0 は CI |
| 旧 tag Release asset 不変(rollback) | asset 名・tag immutability の構造確認済み | 実 Release 観測は公開後 |

## Risks

release binary の実測は win-msvc のみで、musl/darwin の size・
link・exec bit は CI で初めて判明する。`panic = "abort"` を採ら
ない release profile は panic 捕捉経路(envelope・hook fail-open)
に依存するため、将来 profile を変更する場合は T1 の判断記録を
参照。詳細は以下。

### 既知の留意点

- TS SEA pipeline(`scripts/build-executable.mjs`)は Phase 5 では
  撤去しない(Phase 6 の範囲)。`distribution.md` には並走期間中の
  dev sibling 配置元として両者を記載。
- darwin-arm64 binary は検証専用。Release asset・manifest・
  `releaseTargets` への混入は structure test(`release targets
  stay the three published non-darwin assets`)が拒否する。
- 実測 18.9MiB は win-msvc。musl/darwin は未計測で 20MiB gate との
  余裕は CI で初めて判明する(musl static は size 増の方向性がある
  ため初回 matrix run で注視)。
- canonical version source は `packages/cli/package.json` のまま。
  Cargo.toml 宣言は drift 検出対象であり、version bump は
  `set-version.mjs` が両者を同期する。

## Follow-ups

runner 依存の検証は全て CI へ延期(下記 `### CI-deferred
verification`)。TS SEA pipeline(`scripts/build-executable.mjs`)の
撤去は Phase 6 の範囲。

### CI-deferred verification(ユーザー判断で延期)

branch 未 push・workflow_dispatch 未実行のため、runner 依存の
検証は全て CI へ延期。local で確認可能な項目は上表のとおり実測済み。

- `x86_64-unknown-linux-musl` / `aarch64-unknown-linux-musl` /
  `aarch64-apple-darwin` の release build(各 runner の toolchain・
  musl-tools・rustup target add が必要 — IC-1/IC-2)。
- 3 target の size gate・`--version`/`models` smoke(matrix job
  内で fail 伝播する wiring は test pin 済み、実 binary の計測は
  runner 生成物が必要)。
- `build-executables` / `release-acquisition` の workflow_dispatch
  green(branch push が前提)。
- `smoke-bundle` 4 OS(win/ubuntu/arm64 は acquisition 経路、
  macOS は sibling 経路。exec bit 保持を含む — IC-3)。
- `release-acquisition` 実 dispatch による 5 asset 公開と旧 tag
  asset 不変の実観測(rollback 経路。初回 Rust release 自体が
  対象)。
