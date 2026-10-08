---
id: "PLAN-034Beq1KSUNeA8ZTnJ4cM4"
type: "plan"
status: "approved"
title: "Phase 6 remove TypeScript Node Pi"
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
    - "docs/plans/phase-5-switch-distribution-to-rust-binary.md"
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
  references: []
  defers: []
  deferred-by: []
---

# Phase 6 remove TypeScript Node Pi Implementation Plan

## Goal

`packages/` 配下の TypeScript 実装(`cli`/`core`/`tool-runtime`/`runtime-pi`/
`workflow`)と Pi SDK 依存(`@earendil-works/pi-ai`・`pi-coding-agent` 0.85.1)、
Node SEA ビルド経路・pnpm workspace 構成を撤去し、リポジトリを Rust workspace
単独構成へ収束させる。配布 contract(envelope・exit code・usage.jsonl・hook・
launcher・Release asset 名・SHA256SUMS)は Phase 5 で切替済みであり、本 phase
が消すのは「もうどの実行経路にも存在しない」TS/Node 側資産のみである。

## Scope

単一の implementation stream として扱う。境界:

- **In**: `packages/*` の TS 実装撤去(`packages/apm` の APM package 自体は
  残存対象)、Node SEA pipeline(`scripts/build-executable.mjs` + esbuild)
  撤去、pnpm workspace/root package.json/TS toolchain 設定の撤去、
  `packages/apm/.apm/hooks/cli/` の bundled CLI 経路再設計(`cheaper.mjs`
  廃止)、`mise.toml`/`mise run setup`/`mise run check` の Rust toolchain
  対応、CI workflow に残った TS/package.json 参照の除去、`benchmarks/` の
  native binary retarget、docs(README・guides・AGENTS.md・開発者向け手順)と
  ADR status/relations の更新、Deferred 判断 3 件の確定。
- **Out**: launcher(`cheaper`/`cheaper.ps1`)・`hooks.json`・skills 構造・
  `packages/apm/apm.yml` の変更(contract 不変)。cargo workspace 内の crate
  実装変更(Phase 1-5 で完了済み前提)。darwin-* の Release asset 追加
  (Deferred 維持)。機能追加・worker 追加。`docs/adr`/`specs`/`designs`/
  `tasks`/`impl` の履歴本文の書き換え(旧記述は履歴として保持し、status/
  relations のみ更新)。

**前提条件(未充足なら着手不可)**: Phase 5 完了 — Rust binary が 3 target
(linux-x64 / linux-arm64 / win32-x64)+ darwin-arm64 sibling 検証で Release
を組み、無改修 launcher が Rust binary を取得・検証・exec することを
`smoke-bundle` で確認済み、differential golden 20-40 case 全パス、親 plan
の「並走保守期間経過」判断が下りている。

### Deferred 判断の確定

設計書/spec の Deferred Design Concerns のうち Phase 6 時点の判断を以下に
確定する:

1. **version canonical source**: `packages/cli/package.json` → workspace root
   `Cargo.toml` の `[workspace.package].version` に移す。Phase 5 が
   「package.json canonical + Cargo.toml 照合追加」の二重経路としたのは
   TS 並走期間の措置であり、`packages/cli` 撤去と同時に canonical を
   Cargo.toml へ反転する。全 crate は `version.workspace = true` で参照する
   ため宣言点は 1 箇所に収束し、`check-version.mjs` は
   `Cargo.toml` ↔ `packages/apm/apm.yml` ↔ bundled `version.json` の
   3 者照合に縮小する。`--version` 出力契約(`CARGO_PKG_VERSION` 由来の
   bare semver 1 行)は不変。
2. **darwin-* target**: 本 phase でも Release asset に追加しない
   (設計書 Deferred を維持)。launcher の darwin acquisition 分岐解放と
   セットで再検討する必要があるため、移行完了後の別 spec で扱う。
   Phase 5 で導入した darwin-arm64 の sibling-only smoke(検証専用 build、
   Release asset/manifest 非混入)はそのまま維持する。
3. **APM bundle 同梱 `cheaper.mjs`(JS fallback)**: 廃止する。launcher
   (sh/ps1)・`hooks.json` のどの経路も `cheaper.mjs` を参照しておらず、
   実行経路は sibling `cli/bin/<target>/cheaper[.exe]` → version 固定
   Release acquisition に既に一本化されている。offline は同 tag の
   asset + SHA256SUMS pair の preseed または release archive の local
   install で代替可能であり、Node 前提経路を残す利益がない。
   `pnpm bundle` 検証経路は `node scripts/bundle-cli.mjs` の軽量コピー
   経路(`version.json` を Cargo workspace version から生成 +
   `workers/`/`policy.yaml` を Rust 側 bundled resource source から複写)
   に置き換え、bundle freshness(`git diff --exit-code --
   packages/apm/.apm/hooks/cli`)は `mise run check` 内で維持する。

補足の判断: **Node は repository tooling の実行基盤として残す**
(`scripts/*.mjs` の release/payload/doc 検証、`node --test`、`benchmarks`)。
製品・consumer 配布経路の Node 依存は完全に消えるが、
`scripts/lib/release-artifacts.mjs` 等の security-sensitive な検証ロジックを
別 runtime へ移植するコスト・リスクは製品上の利益に見合わない。`pnpm` は
npm 依存が全滅するため toolchain から除去する。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (§Out of Scope の APM 構造不変・Deferred Design Concerns・Acceptance
  Criteria の配布/互換項目)
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  (段階移行・並走/撤去戦略、More Information の旧 ADR status 見直し指示)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md` (親 plan、Phase 6 依存:
  Phase 5 + 並走保守期間経過)
- `docs/plans/phase-5-switch-distribution-to-rust-binary.md` (直前 phase。
  canonical version の二重経路・darwin sibling smoke・`cheaper.mjs` 維持の
  中間状態を本 phase が収束させる)

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md` §配布
  (cargo matrix・asset 名不変・version canonical source 移行の是非)、
  §段階計画との対応(Phase 6 行)、§Deferred Design Concerns(version
  canonical・`cheaper.mjs`・darwin-*)、§Verification Notes(launcher・
  asset・新旧 record 互換の確認項目)
- `docs/research/rig-agent-runtime/migration-plan.md` §7(APM bundle の
  同梱物判断: `cheaper.mjs` のみ廃止、`version.json`/`workers/`/
  `policy.yaml` の同梱継続)
- `docs/designs/version-pinned-launcher-acquisition.md`(launcher が
  `cli/version.json` と sibling/Release acquisition にのみ依存することの
  根拠。`cheaper.mjs` 非参照の確認源)

## File Map

以下 `RUST_WS` は Phase 1-5 で確定した cargo workspace root(設計書候補:
`rust/`;phase plan 間で `rust/crates/` と `crates/` の表記揺れがあるため
着手時に確定値へ読み替える)を指す。

### 撤去

| path | responsibility / 撤去理由 |
| --- | --- |
| `packages/cli/` | TS CLI 実装全体(src/bin/test/workers/policy.yaml/package.json/tsconfig/vitest.config)。`crates/cheaper-cli` へ移植済み(Phase 4)。builtin resource は `include_dir!` 埋込済みのため workers/policy.yaml の source of truth は Rust 側 |
| `packages/core/` | TS core ロジック。`cheaper-core` へ移植済み(Phase 1) |
| `packages/tool-runtime/` | TS tool runtime。`cheaper-tools` へ移植済み(Phase 2) |
| `packages/runtime-pi/` | Pi SDK 接着・auth・catalog・devin provider。`@earendil-works/pi-ai`/`pi-coding-agent` 0.85.1 依存はこの package.json と共に消える |
| `packages/workflow/` | TS workflow engine。`cheaper-workflow` へ移植済み(Phase 4) |
| `scripts/build-executable.mjs` | Node SEA pipeline(`node --build-sea`)。Phase 5 で CI 参照は除去済み前提 |
| `scripts/lib/esbuild-bundle.mjs` | esbuild 単一 bundle 生成。SEA と `cheaper.mjs` の両方が不要になる |
| `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` | generated JS fallback(Deferred 判断 3 で廃止確定) |
| `package.json`(root) | pnpm workspace scripts/devDeps(biome/typescript/vitest/esbuild/@types/node)。全て TS 実装と共に消える |
| `pnpm-workspace.yaml` / `pnpm-lock.yaml` | pnpm workspace 構成。`packages/*` が `packages/apm` のみ(=pnpm package 非含有)になり workspace が空になる |
| `tsconfig.json` / `tsconfig.base.json` / `vitest.workspace.ts` | TS toolchain 設定 |
| `benchmarks/fixtures/fake-worker-stream.mjs` | `packages/runtime-pi/dist/testsupport.js` 依存の fake stream fixture。Rust fake seam の JSON fixture DSL へ置換 |

### 変更

| path | responsibility |
| --- | --- |
| `mise.toml` | `[tools]` から `pnpm` 除去・`biome` を aqua backend(npm 非経由)で追加・`rust` toolchain の存在確認(Phase 5 で導入済み前提、未導入なら locked version で追加)。`node` は scripts/benchmarks 実行のため残す。`[tasks.setup]`/`[tasks.check]` を Rust 手順へ書き換え(後述) |
| `mise.lock` | tool 構成変更分の再生成(`mise lock`) |
| `biome.json` | 残す。lint 対象が `scripts/`・`benchmarks/` の `.mjs` と JSON に縮小 |
| `scripts/check-version.mjs` | canonical を `RUST_WS/Cargo.toml` `[workspace.package].version` に変更。`packages/*/package.json` 照合を除去し、照合対象は `packages/apm/apm.yml` + bundled `version.json` に縮小 |
| `scripts/set-version.mjs` | bump 対象を `Cargo.toml` workspace version + `packages/apm/apm.yml` に変更(`version.json` は生成物で bundle 再生成が追従) |
| `scripts/lib/release-artifacts.mjs` | `verifyReleaseVersion` の canonical 参照を `packages/cli/package.json` → `Cargo.toml` workspace version へ変更。`releaseTargets`・manifest 規則・snapshot 構造は無改変 |
| `scripts/lib/apm-bundle.mjs` / `scripts/bundle-cli.mjs` | `buildEsmBundle` 呼出しを除去。`version.json` を Cargo workspace version から生成し、`workers/`・`policy.yaml` を `cheaper-cli` crate の bundled resource source dir(Phase 4 の `include_dir!` 対象)から複写する。`bin/cheaper.mjs` を出力しない |
| `scripts/check-apm-payload.mjs` | asset 一覧から `cli/bin/cheaper.mjs` を除去し、canonical version の読み取り先を `Cargo.toml` に変更 |
| `scripts/test/apm-payload.test.mjs` / `release-artifacts.test.mjs` / `release-workflow.test.mjs` | 新 asset 一覧・Cargo.toml canonical・workflow 構造に合わせて更新 |
| `.github/workflows/build-executables.yml` | expected_version gate 等の `packages/cli/package.json` 参照を `Cargo.toml` 読みへ変更(Phase 5 の cargo matrix 化済み前提) |
| `.github/workflows/release-acquisition.yml` | `verifyReleaseVersion` 経由の canonical 参照が Cargo.toml になることを確認(呼出し側変更は原則不要、残存参照があれば除去) |
| `benchmarks/lib/cli.mjs` / `benchmarks/run.mjs` 等 | `node packages/cli/dist/main.js` spawn を `cargo build` 出力の native binary 起動へ retarget。`CHEAPER_TEST_FAKE_STREAM` は Rust seam の JSON fixture を指す形に、`PI_OFFLINE`・`CHEAPER_BENCH_*` 等の TS 固有 env を整理 |
| `benchmarks/README.md` | 実行手順・前提の更新 |
| `AGENTS.md` | setup/check contract、`pnpm bundle` 記述、Ollama e2e の対象(Rust binary 側 test)、Pi `ModelRuntime`/`<userConfigDir>/pi/` 節の新 credential store への書き換え、version canonical source、Node は repo tooling 用途のみの明記 |
| `README.md` / `CONTRIBUTING.md` | Pi runtime 記述・pnpm 手順・リポジトリ構造・test 手順の更新 |
| `docs/guides/*.md` | `release.md`(canonical source・bump 手順・SEA 記述)・`distribution.md`(dev local build = cargo、`cheaper.mjs` 言及除去)・`providers.md`/`configuration.md`(Pi models.json/`<userConfigDir>/pi/` → 新 credential store/catalog)・`parent-agent-integration.md`(`cheaper.mjs` 言及)・`managed-process.md`(SEA/`process.execPath` 記述 → `node` task 明示 error)・`usage.md`/`workers.md`/`workflow-usage.md` の drift 点検 |
| `packages/apm/.apm/skills/*/SKILL.md` | `cli/bin/cheaper.mjs` の手動実行経路の記述を除去(`cheaper-usage` 等) |
| `docs/adr/<新規>.md` | bundled esbuild payload 廃止 + version canonical 移行を宣言する supersede ADR。`version-pinned-launcher-acquisition` ADR が「payload/bundle 削除を行う ADR が supersede を宣言する」条項を持つため、撤去判断の記録として起票する |
| 関連 ADR の front matter | `agent-runtime-pi-sdk-*`(superseded-by: migration ADR)・`typescript-pnpm-monorepo`・`distribute-cheaper-as-node-26-sea-*`・`distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-*`・`cli-package-json-as-canonical-version-*`・`pi-persistent-state-userconfigdir-pi-*`・`canonical-tokenizer-js-tiktoken` の status/relations 更新 |

### 残存(無改変・確認対象)

| path | 理由 |
| --- | --- |
| `packages/apm/.apm/hooks/cheaper` / `cheaper.ps1` / `hooks.json` | launcher・hook routing は contract 不変。diff が生じたら defect |
| `packages/apm/apm.yml` / `.apm/skills/` 構造 | APM package 構造は変更しない(SKILL.md 本文の言及更新のみ) |
| `packages/apm/.apm/hooks/cli/version.json` | launcher acquisition が依存する version metadata。生成元が Cargo.toml に変わるだけで形式・配置は不変 |
| `packages/apm/.apm/hooks/cli/workers/` / `cli/policy.yaml` | payload 同梱を継続(migration-plan §7)。供給元が `packages/cli` → Rust resource dir に変わるのみ |
| `packages/apm/.apm/hooks/cli/bin/<target>/` | sibling executable 配置(gitignore 継続) |
| `scripts/generate-release-checksums.mjs` / `assemble-release-bundle.mjs` / `smoke-release.mjs` / `build-apm-bundle.mjs` / `check-doc-ids.mjs` / `check-binary-size.mjs`(Phase 5 新設) | asset 名・contract 不変のため流用。`build-apm-bundle.mjs --exe-dir` は Rust binary をそのまま sibling 化できる |
| `docker-compose.yml` | Ollama 実 provider e2e 用。Rust binary でも検証対象は同一 |
| `apm.yml` / `apm.lock.yaml`(root) | development-only dev project。製品 version とは独立 |
| `.gitignore` | `cli/bin/*/` entry 等は維持。TS/pnpm 固有 entry の整理のみ |
| `RUST_WS` 内の全 crate | Phase 1-5 の成果物。本 phase で実装変更しない |
| `docs/adr`/`specs`/`designs`/`tasks`/`impl` の履歴本文 | Pi/SEA 時代の記述は履歴として保持。status/relations のみ更新 |

## Implementation Sequence

commit 単位は revert 可能な粒度で分割する。A→B→C→D→E の順で land。

### Commit A: version canonical 反転 + payload 生成経路の再設計

- [ ] `scripts/check-version.mjs` を書き換える: canonical source を
      `RUST_WS/Cargo.toml` の `[workspace.package].version` とし、
      `packages/*/package.json` 照合を除去(照合対象は `apm.yml` +
      bundled `version.json`)
- [ ] `scripts/set-version.mjs` を書き換える: bump 対象を `Cargo.toml`
      workspace version + `packages/apm/apm.yml` に変更
- [ ] `scripts/lib/release-artifacts.mjs` の `verifyReleaseVersion` を
      `Cargo.toml` 読みへ変更する
- [ ] `scripts/lib/apm-bundle.mjs` / `scripts/bundle-cli.mjs` を書き換える:
      esbuild 呼出しを除去し、`version.json` を Cargo workspace version から
      生成、`workers/`・`policy.yaml` を `cheaper-cli` crate の bundled
      resource source dir から複写、`bin/cheaper.mjs` を出力しない
- [ ] `scripts/check-apm-payload.mjs` を更新する: asset 一覧から
      `cli/bin/cheaper.mjs` を除去し canonical 参照を `Cargo.toml` へ変更
- [ ] `scripts/test/apm-payload.test.mjs`・`release-artifacts.test.mjs`・
      `release-workflow.test.mjs` を新 contract に合わせて更新する
- [ ] `node scripts/bundle-cli.mjs` を実行し、`cheaper.mjs` 削除と
      `version.json`/`workers/`/`policy.yaml` 再生成を同一 commit に含める
      (生成物と生成経路の変更を分離しない)
- [ ] `scripts/build-executable.mjs`・`scripts/lib/esbuild-bundle.mjs` を
      削除し、`git grep` で残存参照が無いことを確認する

### Commit B: toolchain / workspace 構成の撤去

- [ ] `mise.toml` を更新する: `[tools]` から `pnpm` を除去、`biome` を
      aqua backend で追加、`rust` toolchain の存在を確認(未導入なら
      locked version で追加)
- [ ] `[tasks.setup]` を `apm install` → `cargo fetch` →
      `cargo build --workspace --all-targets` へ書き換える
- [ ] `[tasks.check]` を `biome check` → `cargo fmt --all -- --check` →
      `cargo clippy --workspace --all-targets -- -D warnings` →
      `node scripts/check-version.mjs` → `node scripts/check-doc-ids.mjs` →
      `cargo test --workspace` → `node --test scripts/test/` →
      `node scripts/bundle-cli.mjs` → `node scripts/check-apm-payload.mjs` →
      `git diff --exit-code -- packages/apm/.apm/hooks/cli` へ書き換える
- [ ] `mise.lock` を再生成する(`mise lock` / `mise install` で locked
      tool 解決を更新)
- [ ] root `package.json`・`pnpm-workspace.yaml`・`pnpm-lock.yaml` を削除する
- [ ] `tsconfig.json`・`tsconfig.base.json`・`vitest.workspace.ts` を削除し、
      `.gitignore` の TS/pnpm 固有 dead entry(`*.tsbuildinfo`・
      `.pnpm-store` 等)を整理する

### Commit C: TS 実装の撤去

- [ ] `packages/cli`・`packages/core`・`packages/tool-runtime`・
      `packages/runtime-pi`・`packages/workflow` を `git rm` で削除する
      (Pi 依存を含む全 TS 実装。単一の revert 可能 commit)
- [ ] TS 側にのみ存在する検証を棚卸しし、存続分を移植先へ移す:
      launcher acquisition stub・`apm-install-targets`・bundled
      `cheaper.mjs` 経由の hook contract 等、`cargo test`・
      `smoke-release.mjs`・`scripts/test/*.mjs`(node:test)のいずれかに
      相当物が既に存在するかを確認し、欠落分だけ `scripts/test/` の
      node:test として移植する
- [ ] `crates/cheaper-cli/tests/differential/` の処遇を確定する:
      harness は TS binary(`node cheaper.mjs`/SEA)を oracle に要求するため
      TS 撤去後は成立しない。撤去直前に最終 differential golden を取得し、
      fixture+期待出力を `crates/cheaper-cli/tests/regression/` の
      snapshot test(Rust binary のみで replay、normalize 適用済み期待値を
      固定)へ変換してから harness を削除する。`cargo test --workspace` が
      oracle 不在で恒久的 red になることを防止
- [ ] `scripts/dump-core-golden.mjs`・`scripts/dump-tool-fixtures.mjs` を
      削除する(両者とも撤去対象の `@cheaper/*` を import する dead script
      化が確定。fixture の再生成経路は以後 Rust 側 test が正とする旨を
      残存 doc に明記)
- [ ] `benchmarks/test/*.test.mjs` 4 件の新 check への配線を決める:
      `node --test benchmarks/test/` を `[tasks.check]` に含めるか、
      現行どおり手動実行経路とするかを tasks.check 書き換え時に明記

### Commit D: benchmarks retarget

- [ ] `benchmarks/lib/cli.mjs` の spawn 対象を `cargo build` 出力の
      native binary へ retarget する
- [ ] `fake-worker-stream.mjs` を Rust fake seam の JSON fixture DSL へ
      変換し、`CHEAPER_TEST_FAKE_STREAM` の指し先を差し替える
      (`PI_OFFLINE`・`CHEAPER_BENCH_*` 等の TS 固有 env は整理)
- [ ] `benchmarks/README.md` を更新し、benchmark harness が動くことを
      1 scenario で実測する

### Commit E: CI workflow 残存参照 + docs/ADR

- [ ] `.github/workflows/build-executables.yml`・`release-acquisition.yml`
      に残る `packages/cli/package.json`/pnpm/SEA 参照を除去する
      (Phase 5 で置換済みなら `git grep` 確認のみ)
- [ ] `AGENTS.md` を更新する(setup/check contract・`pnpm bundle` →
      `node scripts/bundle-cli.mjs`・Ollama e2e の Rust 側 test 参照・
      Pi `ModelRuntime`/`<userConfigDir>/pi/` 節の新 credential store への
      書き換え・version canonical = `Cargo.toml`・Node は repo tooling
      用途のみの明記)
- [ ] `README.md`・`CONTRIBUTING.md` を更新する(Pi runtime 記述・pnpm
      手順・リポジトリ構造・test 手順)
- [ ] `docs/guides/` を更新する(`release.md`・`distribution.md`・
      `providers.md`・`configuration.md`・`parent-agent-integration.md`・
      `managed-process.md` を中心に、`usage.md`/`workers.md`/
      `workflow-usage.md` の drift を点検)
- [ ] `packages/apm/.apm/skills/*/SKILL.md` から `cheaper.mjs` 手動実行経路の
      記述を除去する
- [ ] bundled esbuild payload 廃止 + version canonical 移行を宣言する
      supersede ADR を起票し、対象 ADR の status/relations
      (`superseded-by`)を更新する
- [ ] `docs/README.md`/`docs/plans/README.md` 等の索引が実態と整合して
      いることを確認・更新する

### Final gate

- [ ] fresh worktree で `mise run setup` → `mise run check` が exit 0
- [ ] release E2E smoke を実施する(§Verification)

## Dependencies and Risks

- **着手条件**: Phase 5 completed が必須依存(Rust binary 配布経路・
  differential golden 全パス・並走保守期間経過)。`depends-on` で参照。
- **撤去しすぎ risk**: `cli/version.json` は launcher acquisition の必須
  contract、`cli/workers/`・`cli/policy.yaml` は payload 同梱継続、
  `cli/bin/<target>/` の gitignore も維持。`check-apm-payload.mjs` が
  asset 存在を guard するため、payload 縮小は `bin/cheaper.mjs` のみに
  限定する。
- **apm 配布破壊 risk**: `cheaper.mjs` 削除で「Node があれば動く」fallback
  が消え、sibling も Release も無い環境では即失敗になる。Copilot JSON
  deploy gap(version.json が届かない制約)は従来から sibling 必須で
  変わらない。`distribution.md` の offline preseed・local archive 手順で
  代替経路が閉じることを確認し、`smoke-bundle`(3 OS + darwin sibling)
  が実証する。
- **CI 破壊 risk**: `mise.toml` tasks の書き換えで `ci.yml` が fail する
  可能性。`ci.yml` は `mise run setup`/`mise run check` の task 呼出し
  のみなので自動追従するが、`mise.lock` 再生成漏れがあると
  `install_args: "--locked"` で install が fail する。lock 更新を同じ
  commit に含める。
- **version drift risk**: canonical を Cargo.toml に反転する際、
  `version.json`/`apm.yml`/binary 埋込 version の何れかが stale のまま
  残ると `check-version`/`verifyReleaseVersion` で検出される。全宣言を
  同一 commit で更新する。
- **TS 側検証の消失 risk**: `packages/cli/test/e2e/` 配下にしか存在しない
  coverage(launcher stub・APM install target matrix 等)が Rust 側
  equivalent なしに消える。Commit C の棚卸しで欠落分を `scripts/test/` へ
  移植してから削除を完了とする。
- **二重保守期間の drift risk**: 並走期間中に TS 側へ入った修正が Rust 側へ
  未反映のまま撤去する危険。Phase 5 gate の differential golden で検出済み
  前提だが、撤去直前に最終 golden 全件を再実行する。
- **benchmark 破壊 risk**: `fake-worker-stream.mjs` が `CHEAPER_BENCH_*`
  パラメータ化(response profiles/delay/tool call path)を担っており、
  Rust fake seam の fixture DSL で表現不能な場合は対象 scenario を縮小して
  別 task 化する(完了条件から外さず plan 内で扱う)。
- **Node scripts 残留の誤解 risk**: 製品・配布経路の Node 依存は消えるが、
  repo tooling として `scripts/*.mjs`/`benchmarks` は残る。「Node.js
  ランタイム依存の除去」が製品 contract の話であり dev tooling は対象外
  であることを AGENTS.md/CONTRIBUTING.md で明記する。

## Task Breakdown

| task 候補 | 対応 commit | 概要 |
| --- | --- | --- |
| T1 | Commit A | version canonical 反転 + payload 生成経路の再設計 |
| T2 | Commit B | toolchain(mise/lock)・pnpm workspace・TS 設定撤去 |
| T3 | Commit C | `packages/*` TS 実装撤去 + TS 固有検証の棚卸し/移植 |
| T4 | Commit D | benchmarks の native binary retarget |
| T5 | Commit E | CI workflow 残存参照除去 + docs/ADR status 更新 |
| T6 | Final gate | fresh worktree setup/check + release E2E smoke |

T1→T2→T3 は直列依存(scripts が参照する source が存在する前提のため)。
T4 は `packages/runtime-pi/dist/testsupport.js` を参照する fixture を消す
性質上 T3 の前に完了させるか、T3 と並行して進められる
(benchmarks の変更は `packages/*` の存在に依存しない)。
T5 は Commit B 以降はどこでも進められるが、最終整合確認は全撤去後。
T6 は最後。

## Verification

- [ ] fresh worktree で `mise run setup` → `mise run check` が exit 0
      (toolchain lock・cargo build/test・bundle freshness を含む)
- [ ] `mise exec -- node --test scripts/test/` が全パス
      (apm-payload・release-artifacts・release-workflow・binary-size)
- [ ] `git grep` で以下が working code/CI/scripts から消失していることを
      確認する(docs 履歴・ADR 記述は除外):
      `@earendil-works|pi-coding-agent|vitest|tsconfig|pnpm |
      build-executable|esbuild|cheaper\.mjs|packages/(cli|core|tool-runtime|
      runtime-pi|workflow)`
- [ ] `cargo build --release` で 3 target(linux-x64・linux-arm64・
      win-x64)+ darwin-arm64 sibling 検証 build が成功し、
      `check-binary-size.mjs` で strip 後 ≤20MiB を維持する
      (Phase 5 の CI gate が引き続き機能することを確認)
- [ ] **release E2E**: `release-acquisition` を test tag で 1 dispatch し、
      5 asset(raw3 + `SHA256SUMS` + `cheaper-apm-v<X.Y.Z>.tar.gz`)の
      publish と `smoke-bundle` 全 OS(windows/ubuntu/arm/macos sibling)
      が green であることを確認する。これが hook 配布物の smoke を兼ねる
- [ ] ローカル検証: `cargo build --release` の出力を
      `build-apm-bundle.mjs --exe-dir` で sibling 化し、
      `smoke-release.mjs` で Node-free launcher(sh + ps1)経由の
      `--version`/`models` が通ることを確認する
- [ ] hook 配布物 smoke: consumer へ `apm install` 後、launcher 経由で
      `cheaper hook <event> --target <agent>` が Rust binary で実行され
      fail-open 契約(copilot/cursor は非 0 を返さない)が維持されること
- [ ] Ollama 実 provider e2e が Rust binary で通ること
      (`docker compose up -d` + model pull、Rust 側の e2e 経路で実施)
- [ ] `mise run check` の bundle freshness
      (`git diff --exit-code -- packages/apm/.apm/hooks/cli`)が
      `cheaper.mjs` 削除後も green であること
- [ ] version bump 手順: `node scripts/set-version.mjs <X.Y.Z>` →
      `node scripts/bundle-cli.mjs` → `mise run check` で全宣言
      (Cargo.toml・apm.yml・version.json・binary `--version`)が
      一致することを確認する

## Open Questions / Gaps

- cargo workspace root の確定 path。phase plan 間で
  `rust/crates/`(Phase 3)と `crates/`(Phase 2/4/5)の表記揺れがある
  ため、着手時に Phase 1-5 で確定した実パスを確認し、本 plan の `RUST_WS`
  を読み替える。
- Rust bundled resource source dir(`cheaper-cli` crate 内の
  `workers/`/`policy.yaml`/`schemas` の実配置)の確定値。Phase 4 の
  `embedded.rs`(`include_dir!` 対象)実装に依存し、bundle script の
  複写元として確定させる。
- benchmarks の `CHEAPER_BENCH_*` パラメータ化(response profiles・
  delay・tool call path)を Rust fake seam の fixture DSL で表現できるか。
  表現不能な場合は対象 scenario を縮小して別 task 化する。
- `biome` の aqua backend version 入手可能性。取得不能なら `scripts/`・
  `benchmarks/` の JS lint を廃止し `node --test` + review に縮退させる
  判断を実装時に確定する(代替として `mise.toml` への pin 不成立時の
  fallback)。
- darwin-* の Release asset 化は Deferred 維持。launcher の darwin
  acquisition 分岐解放とセットで扱う follow-up spec の起票主体・時期は
  本 plan の完了後に別途判断する。
- TS 側にのみ存在する検証(launcher stub・`apm-install-targets` 等)の
  うち Rust 側 equivalent が Phase 4/5 で既に提供されているかの最終確認。
  棚卸し時に欠落が判明した分だけ `scripts/test/` へ移植する。
- 旧 Pi/SEA 系 ADR の本文は履歴として保持し status/relations のみ更新する
  方針。supersede 宣言の単位(専用 ADR 1 本で esbuild bundle + canonical
  version + SEA をまとめて宣言するか)は reviewer 合意で確定する。

## Self-Review

- [ ] Every spec requirement maps to a task.
- [ ] No placeholders remain.
- [ ] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution(scripts/config/docs/CI の機械的置換が
  中心で crate 実装を含まず、依存関係が概ね直列)
- Independent work streams: T4(benchmarks retarget)のみ Commit B 完了後は
  T3/T5 と並行可能
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー重点: (a) launcher contract(`cli/version.json` 必須・
  `cli/bin/<target>/` sibling 優先・version 固定 Release acquisition)を
  壊していないか、(b) `cheaper.mjs` 削除後に残る実行経路が sibling +
  Release acquisition で閉じているか(hooks.json/launcher が `cheaper.mjs`
  を参照しないこと)、(c) version canonical 反転の全参照点
  (`check-version`/`set-version`/`release-artifacts`/`verifyReleaseVersion`/
  workflows)が漏れなく Cargo.toml へ向いているか、(d) docs/ADR の履歴
  改ざん(旧記述の書き換え)と status/relations 更新の線引きが適切か、
  (e) Node を repo tooling として残す判断が製品 contract と混同されて
  いないか、(f) TS 側にのみ存在する検証が Rust 側 equivalent なしに
  消えていないか。
- 前提: Phase 5 未完了のまま着手した場合、本 plan は進行不可として扱う。
  `RUST_WS` の実パス・bundled resource dir・fake seam env 名は Phase 1-5
  の成果物に依存するため、着手時に確定値を確認してから Commit A に入る。
- CI-only 検証領域(cargo matrix build・3 OS smoke・実公開)は local で
  再現できないため、構造 test・dry-run・実 dispatch の 3 層で evidence を
  分離して報告すること(現行 `release.md` の「発火前の検証範囲」と同じ
  規範)。
