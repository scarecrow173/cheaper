---
id: "ADR-034CpwNUNIDqrR92x7VaUR"
type: "adr"
status: "superseded"
title: "Remove the committed esbuild CLI payload and move the canonical version to Cargo.toml"
created: "2026-10-08"
updated: "2026-10-08"
owners: []
relations:
  source:
    - "../plans/phase-6-remove-typescript-node-pi.md"
    - "../plans/migrate-cheaper-runtime-to-rust-rig.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/phase-6-remove-typescript-node-pi.md"
  implemented-by:
    - "../tasks/phase-6-remove-typescript-node-pi.md"
  depends-on:
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  blocks: []
  supersedes:
    - "distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md"
    - "cli-package-json-as-canonical-version-with-check-enforcement.md"
  superseded-by: ["docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md"]
  related:
    - "version-pinned-launcher-acquisition.md"
    - "payload-untracking-migration.md"
    - "distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
    - "typescript-pnpm-monorepo.md"
    - "migrate-the-cheaper-cli-runtime-to-rust-and-rig.md"
  refines: []
  refined-by:
    - "../specs/unified-version-source-of-truth.md"
  derives-from:
    - "../plans/phase-6-remove-typescript-node-pi.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/rust-migration-distribution.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Remove the committed esbuild CLI payload and move the canonical version to Cargo.toml

## Context and Problem Statement

Rust 移行（ADR-034BbhP6hJV7DFtv7k7AGs）の Phase 6 で TypeScript package
（`packages/{cli,core,tool-runtime,runtime-pi,workflow}`）が撤去され、配布
executable は `cargo build --release -p cheaper-cli` の native binary に
確定した。これにより 2 つの既存決定が実態と矛盾する状態になった:

1. APM hook bundle が committed esbuild bundle（`cli/bin/cheaper.mjs`）を
   payload として保持する決定（ADR-034r05iHrZUE6W5HVbwRSz）。Rust binary が
   唯一の実行経路となったため、esbuild bundle は dead payload であり、
   version-pinned-launcher-acquisition（ADR-034zIOoBXZtluQdBMlJQge）が
   「payload/bundle 削除を行う ADR が supersede を宣言する」と規定した
   条件が本 ADR に相当する。
2. `packages/cli/package.json` を canonical version source とする決定
   （ADR-034r05iHrZUE6WgAk8Cpqq）。`packages/cli` の撤去により宣言先が
   消え、canonical を Rust workspace へ移す必要がある。

## Decision Drivers

* 実行経路の単一化: 配布・hook・dev の全経路が Rust binary を実行するため、
  JS fallback を残すと「動かない参照」を consumer doc と checker が抱える
* version 宣言の単一 source: `cheaper --version` の埋込元と配布 manifest
  の照合元が同一宣言に由来すること
* launcher・`hooks.json`・`cli/version.json`・Release asset 名・
  `SHA256SUMS` の外部 contract は不変を維持する

## Considered Options

* (1) esbuild payload を撤去し、canonical version を `Cargo.toml`
  `[workspace.package].version` へ移す（採用）
* (2) `cheaper.mjs` を unsupported fallback として残す
* (3) canonical version を独立した `version.json` / 専用 file に移す

## Decision Outcome

Chosen option: "(1)"。

確定した決定事項:

* **esbuild payload 廃止**: `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` と
  その生成経路（esbuild bundle、`scripts/build-executable.mjs`、
  `esbuild-bundle.mjs`）を撤去する。`node scripts/bundle-cli.mjs` は
  `version.json`・`workers/`・`policy.yaml` のみを
  `crates/cheaper-cli/resources/`（Rust binary の `include_dir!`/`include_str!`
  embedded resource と同一 source）から再生成する。platform executable は
  payload に含めず、`cli/bin/<target>/` の local output は引き続き
  Git ignore とする。Git 管理の payload を実体が無い JS fallback で
  埋めない。
* **canonical version = root `Cargo.toml`**: 製品 version の唯一の宣言は
  `[workspace.package].version`。`cheaper-cli` crate が
  `version.workspace = true` で継承し、build.rs が binary の
  `--version` 出力へ埋め込む。`packages/apm/apm.yml` と bundle 内
  `version.json` は一致必須の二次宣言（後者は `bundle-cli.mjs` の
  生成物）。`scripts/check-version.mjs`・`set-version.mjs`・
  `verifyReleaseVersion` の canonical は Cargo version を読む。
  library crate の `version` は publish しない内部 path 依存のため
  製品 version 宣言ではない。
* **TS/Node 依存の撤去範囲**: pnpm workspace・`tsconfig`・vitest 構成・
  Pi SDK（`@earendil-works/pi-ai`・`pi-coding-agent`）依存は削除済み。
  Node.js は `scripts/`・`benchmarks/`・doc-suite skills の repo tooling
  用途のみ toolchain に残り、product・distribution・hook 実行経路の
  依存は存在しない。

### Consequences

* Good, because APM payload が実際に使われる asset（launcher・
  `version.json`・`policy.yaml`・worker 定義）のみになり、`cheaper.mjs`
  への「存在しない fallback」の参照が consumer 向け文書と checker から
  消える。
* Good, because version の埋込元（build.rs ← `CARGO_PKG_VERSION`）と
  canonical 宣言が同一値になり、`--version` と manifest の drift が
  構造的に起きない。
* Good, because sibling・Release acquisition の無い環境でも動く
  「Node があれば動く」経路を明示的に捨て、launcher contract
  （fail closed + stderr 診断）に一本化できる。
* Bad, because Node.js のみが利用可能な環境（unsupported OS/arch で
  sibling も無い場合）では CLI を実行できない。対象外 OS は local cargo
  build の sibling 配置が唯一の経路になる。
* Bad, because `credentials.json`・`models.json` overlay 等の新 schema と
  併せて、旧 TS 資産に依存する外部 automation には移行作業が発生する。
* Neutral, because `cli/version.json`・launcher・`hooks.json`・
  `packages/apm/apm.yml`・Release asset 名・`SHA256SUMS` の外部
  contract は変更しない（本 ADR が変えるのは payload 内訳と version
  宣言先のみ）。

### Confirmation

* `mise run check` の `git diff --exit-code -- packages/apm/.apm/hooks/cli`
  （bundle freshness）と `check-apm-payload.mjs` が `cheaper.mjs` 不在の
  payload で pass する。
* `check-version.mjs` が `Cargo.toml` を canonical として `apm.yml`・
  `version.json` を照合し、`cheaper --version` が同値を返す。
* `git grep` で `cheaper.mjs`・`packages/{cli,core,tool-runtime,
  runtime-pi,workflow}`・pnpm の live 参照が working code・CI・
  guide に残らない（履歴 doc は除外）。

## Implementation Plan

* Affected paths: `packages/apm/.apm/hooks/cli/`（`bin/cheaper.mjs` 撤去、
  `version.json`/`workers/`/`policy.yaml` の再生成）、
  `scripts/{bundle-cli,check-apm-payload,check-version,set-version}.mjs`・
  `scripts/lib/{apm-bundle,release-artifacts}.mjs`、`Cargo.toml`、
  `AGENTS.md`・`README.md`・`CONTRIBUTING.md`・`docs/guides/*`・
  `packages/apm/.apm/skills/*/SKILL.md`
* Patterns to follow: launcher sibling 優先 → version-pinned Release
  acquisition（ADR-034zIOoBXZtluQdBMlJQge）の既存 contract を不変で維持。
  payload 複写元を `crates/cheaper-cli/resources/` に揃え、binary embedded
  resource と payload の単一 source を維持する
* Patterns to avoid: unsupported 環境向けの JS fallback・`node` 経路の
  復活。version 宣言の複数 canonical 化。`packages/cli` 由来の
  `version.ts` fallback 読み（binary は build.rs 埋込の単一経路）
* Dependencies or configuration: `scripts/set-version.mjs <X.Y.Z>` が
  `Cargo.toml` と `packages/apm/apm.yml` を書き換え、`bundle-cli.mjs` で
  `version.json` を再生成する既存の bump 手順を維持する

## Verification

* [x] `node scripts/check-version.mjs` が Cargo canonical で全宣言を照合
* [x] `mise run check` が `cheaper.mjs` 不在の payload で pass
  （`check-apm-payload.mjs` + bundle freshness を含む）
* [x] `build-executables` / `release-acquisition` workflow が cargo
  matrix build のみを参照し、pnpm/SEA/`cheaper.mjs` の live 参照が無い
* [ ] release E2E: `release-acquisition` の test tag dispatch で
  5 asset（raw3 + `SHA256SUMS` + APM archive）が publish され、
  `smoke-bundle` 全 OS が green（Phase 6 final gate）

## Pros and Cons of the Options

### Option 1: payload 撤去 + canonical を Cargo.toml へ — 採用

* Good, because 実行経路・version 宣言がともに単一 source へ収束し、
  dead reference と二重宣言が消える。
* Good, because version-pinned-launcher-acquisition が予約した
  「payload 削除 ADR」の条件を満たし、esbuild bundle ADR の supersede
  chain が閉じる。
* Bad, because Node-only 環境の実行経路が無くなる（unsupported OS は
  local build sibling のみ）。

### Option 2: `cheaper.mjs` を fallback として残す — 不採用

* Good, because Node.js があれば unsupported platform でも動く。
* Bad, because TS package 撤去後は bundle を再生成する source が存在せず、
  古い snapshot の JS bundle が「動くが stale」な状態で残る。
  保守されない fallback は誤用を招く。

### Option 3: canonical version を独立した専用 file へ移す — 不採用

* Good, because `Cargo.toml` 編集と version bump を分離できる。
* Bad, because `--version` 埋込元（`CARGO_PKG_VERSION`）と宣言先が
  分かれて drift の余地が残り、宣言を増やすだけで構造的利益がない。

## More Information

* supersede 対象:
  `distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md`
  （committed esbuild bundle payload の廃止）、
  `cli-package-json-as-canonical-version-with-check-enforcement.md`
  （canonical を `packages/cli/package.json` → `Cargo.toml` へ移行）。
  `check script で一致を強制する` という仕組み自体は `check-version.mjs`
  として継続し、宣言先だけが変わる。
* ADR-034zIOoBXZtluQdBMlJQge（version-pinned-launcher-acquisition）の
  「Historical ADR / spec reconciliation」が `cli/bin/cheaper.mjs` が
  同梱・参照される間は esbuild bundle ADR を有効とし、その supersede を
  payload 削除 ADR に委譲していた。本 ADR がその宣言 ADR に相当する。
* 上位の移行決定: `migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  （ADR-034BbhP6hJV7DFtv7k7AGs）。TS package 撤去・pnpm monorepo 解消・
  `<userConfigDir>/pi/` 廃止・js-tiktoken → `tiktoken-rs` 置換は同 ADR
  のスコープであり、本 ADR は payload と version canonical の収束に
  限定する。
