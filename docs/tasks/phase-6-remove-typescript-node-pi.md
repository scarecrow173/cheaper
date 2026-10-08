---
id: "TASK-034BgKlddxEi2ozLgZMFi3"
type: "task"
status: "todo"
title: "Phase 6: remove TypeScript Node Pi"
created: "2026-10-06"
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
    - "docs/plans/phase-6-remove-typescript-node-pi.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-6-remove-typescript-node-pi.md"
    - "docs/tasks/phase-5-switch-distribution-to-rust-binary.md"
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

# Phase 6: remove TypeScript Node Pi

## Work

plan `docs/plans/phase-6-remove-typescript-node-pi.md`
(PLAN-034Beq1KSUNeA8ZTnJ4cM4)の実行単位。`packages/` 配下の TypeScript
実装(cli/core/tool-runtime/runtime-pi/workflow)と Pi SDK 依存
(`@earendil-works/pi-ai`・`pi-coding-agent` 0.85.1)、Node SEA ビルド経路・
pnpm workspace 構成を撤去し、リポジトリを Rust workspace 単独構成へ収束
させる。配布 contract(envelope・exit code・usage.jsonl・hook・launcher・
Release asset 名・SHA256SUMS)は Phase 5 で切替済みであり、本 task が消す
のは「もうどの実行経路にも存在しない」TS/Node 側資産のみ。

plan の Task Breakdown T1-T6(commit A-E + Final gate)が本 task の作業
単位。commit は revert 可能な粒度で A→B→C→D→E の順に land する:

- T1(Commit A): version canonical 反転 + payload 生成経路再設計。
  `check-version.mjs`/`set-version.mjs`/`verifyReleaseVersion` の
  canonical を `Cargo.toml` `[workspace.package].version` へ、
  `bundle-cli.mjs`/`apm-bundle.mjs` から esbuild を除去し `version.json`
  を Cargo version から生成、`bin/cheaper.mjs` を出力しない、
  `check-apm-payload.mjs` 更新、`bundle-cli.mjs` 再生成と同一 commit で
  生成物を更新、`build-executable.mjs`/`esbuild-bundle.mjs` 削除
- T2(Commit B): toolchain/workspace 構成撤去。`mise.toml` から `pnpm`
  除去・`biome` を aqua backend で追加・`rust` toolchain 確認、
  `[tasks.setup]`=`apm install`→`cargo fetch`→`cargo build --workspace
  --all-targets`、`[tasks.check]`=biome→fmt→clippy→check-version→
  check-doc-ids→`cargo test --workspace`→`node --test scripts/test/`→
  bundle→check-apm-payload→bundle freshness、`mise.lock` 再生成、root
  `package.json`・`pnpm-workspace.yaml`・`pnpm-lock.yaml`・tsconfig・
  `vitest.workspace.ts` 削除
- T3(Commit C): `packages/cli`/`core`/`tool-runtime`/`runtime-pi`/
  `workflow` を `git rm`(単一 revert 可能 commit)。TS 側のみの検証
  (launcher stub・apm-install-targets 等)を棚卸しし欠落分を
  `scripts/test/` の node:test へ移植。differential harness は最終 golden
  を `tests/regression/` snapshot test へ変換してから削除。
  `dump-core-golden.mjs`・`dump-tool-fixtures.mjs` 削除
- T4(Commit D): benchmarks retarget。`benchmarks/lib/cli.mjs` の spawn
  を `cargo build` 出力 binary へ、`fake-worker-stream.mjs` を Rust fake
  seam の JSON fixture DSL へ変換、TS 固有 env 整理、README 更新 + 1
  scenario 実測
- T5(Commit E): CI workflow 残存参照除去、AGENTS.md/README/
  CONTRIBUTING/guides/SKILL.md 更新、supersede ADR 起票 + 対象 ADR の
  status/relations 更新、docs 索引整合
- T6(Final gate): fresh worktree `mise run setup`→`mise run check` +
  release E2E smoke

Deferred 判断 3 件の確定を含む: version canonical を `Cargo.toml` へ反転、
darwin-* は Release asset 非追加を維持(sibling smoke は継続)、
`cheaper.mjs` JS fallback 廃止。Node は repo tooling(scripts/benchmarks)
として残す。launcher・`hooks.json`・`packages/apm/apm.yml`・APM 構造・
履歴 doc 本文は無改変。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-6-remove-typescript-node-pi.md`(本 task が implement
  する plan。File Map・Commit A-E・Verification の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、Phase 6
  依存: Phase 5 + 並走保守期間経過)
- `docs/plans/phase-5-switch-distribution-to-rust-binary.md`(直前 phase。
  canonical version 二重経路・darwin sibling smoke・`cheaper.mjs` 維持の
  中間状態を本 task が収束させる)
- `docs/designs/rust-rig-runtime-migration-architecture.md` §配布・
  §Deferred Design Concerns
- `docs/designs/version-pinned-launcher-acquisition.md`(launcher が
  `cheaper.mjs` を参照しないことの根拠)
- `docs/research/rig-agent-runtime/migration-plan.md` §7(APM bundle
  同梱物判断)
- `docs/test-specs/rust-migration-distribution.md`(本 task を検証する
  spec)

## Dependencies

- `docs/tasks/phase-5-switch-distribution-to-rust-binary.md` — front
  matter の `depends-on` は「Phase 5 完了(Rust binary が 3 target +
  darwin-arm64 sibling 検証で Release を組み、無改修 launcher が Rust
  binary を取得・検証・exec することを `smoke-bundle` で確認済み、
  differential golden 全パス、並走保守期間経過の判断)が本 task の着手
  条件」であることを意味する。未充足なら着手不可
- `docs/plans/phase-6-remove-typescript-node-pi.md` — 承認済み plan への
  従属
- `RUST_WS`(cargo workspace root)・bundled resource source dir・fake
  seam env 名は Phase 1-5 の成果物に依存し、着手時に確定値を確認してから
  Commit A に入る

## Affected Files

- 撤去: `packages/{cli,core,tool-runtime,runtime-pi,workflow}/`・
  `scripts/build-executable.mjs`・`scripts/lib/esbuild-bundle.mjs`・
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`・root `package.json`・
  `pnpm-workspace.yaml`・`pnpm-lock.yaml`・`tsconfig*.json`・
  `vitest.workspace.ts`・`benchmarks/fixtures/fake-worker-stream.mjs`・
  `scripts/dump-core-golden.mjs`・`scripts/dump-tool-fixtures.mjs`・
  `crates/cheaper-cli/tests/differential/`(snapshot test へ変換後)
- 変更: `mise.toml`/`mise.lock`・`scripts/{check-version,set-version,
  check-apm-payload,bundle-cli}.mjs`・`scripts/lib/{release-artifacts,
  apm-bundle}.mjs`・`scripts/test/*.mjs`・`.github/workflows/*.yml`・
  `benchmarks/lib/cli.mjs` 等・`AGENTS.md`/`README.md`/`CONTRIBUTING.md`・
  `docs/guides/*`・`packages/apm/.apm/skills/*/SKILL.md`・関連 ADR の
  front matter
- 無改変(確認対象): launcher(`cheaper`/`cheaper.ps1`)・`hooks.json`・
  `packages/apm/apm.yml`・`cli/version.json`・`cli/workers/`・
  `cli/policy.yaml`・`cli/bin/<target>/` の gitignore

## Done When

- [ ] T1(Commit A): version canonical が `Cargo.toml` に反転し、payload
      生成経路から esbuild/`cheaper.mjs` が除去される
- [ ] T2(Commit B): mise tasks が Rust 手順へ書き換えられ、pnpm
      workspace・TS toolchain 設定が撤去される
- [ ] T3(Commit C): `packages/*` TS 実装が撤去され、TS 固有検証の欠落
      分が `scripts/test/` へ移植され、differential harness が regression
      snapshot test へ変換されてから削除される
- [ ] T4(Commit D): benchmarks が native binary + Rust fake seam fixture
      で動作する
- [x] T5(Commit E): CI workflow 残存参照除去・docs/ADR status 更新・
      supersede ADR 起票が完了する
- [ ] T6(Final gate): fresh worktree の `mise run setup`→`mise run check`
      と release E2E smoke が pass する

## Verification

- [ ] fresh worktree で `mise run setup` → `mise run check` が exit 0
      (toolchain lock・cargo build/test・bundle freshness を含む)
- [ ] `mise exec -- node --test scripts/test/` が全パス(apm-payload・
      release-artifacts・release-workflow・binary-size)
- [ ] `git grep -E "@earendil-works|pi-coding-agent|vitest|tsconfig|pnpm |
      build-executable|esbuild|cheaper\.mjs|packages/(cli|core|tool-runtime|
      runtime-pi|workflow)"` が working code/CI/scripts でヒットしない
      (docs 履歴・ADR 記述は除外)
- [ ] `mise exec -- cargo build --release` で 3 target(linux-x64・
      linux-arm64・win-x64)+ darwin-arm64 sibling 検証 build が成功し、
      `node scripts/check-binary-size.mjs --file <binary>` で strip 後
      ≤20MiB を維持する
- [ ] release E2E: `release-acquisition` を test tag で 1 dispatch し、
      5 asset(raw3 + `SHA256SUMS` + `cheaper-apm-v<X.Y.Z>.tar.gz`)の
      publish と `smoke-bundle` 全 OS が green である
- [ ] `cargo build --release` の出力を `node scripts/build-apm-bundle.mjs
      --exe-dir <dir>` で sibling 化し、`node scripts/smoke-release.mjs`
      で Node-free launcher(sh + ps1)経由の `--version`/`models` が
      通る
- [ ] consumer への `apm install` 後、launcher 経由で `cheaper hook
      <event> --target <agent>` が Rust binary で実行され fail-open 契約
      が維持される(hook 配布物 smoke、release E2E と兼用可)
- [ ] `docker compose up -d` + model pull 済み Ollama で Rust binary の
      実 provider e2e が通る
- [ ] `git diff --exit-code -- packages/apm/.apm/hooks/cli` が
      `cheaper.mjs` 削除後も green である(`mise run check` 内)
- [ ] `node scripts/set-version.mjs <X.Y.Z>` → `node
      scripts/bundle-cli.mjs` → `mise run check` で全宣言(Cargo.toml・
      apm.yml・version.json・binary `--version`)が一致する
