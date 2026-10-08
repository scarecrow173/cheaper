---
id: "IMPL-TXm8SCjDLk10XvQdp1sPnY"
type: "impl"
status: "completed"
title: "Phase 6: remove TypeScript Node Pi"
created: "2026-10-09"
updated: "2026-10-09"
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
    - "docs/tasks/phase-6-remove-typescript-node-pi.md"
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

# Phase 6: remove TypeScript Node Pi

`packages/` 配下の TypeScript 実装(cli/core/tool-runtime/runtime-pi/
workflow)と Pi SDK 依存(`@earendil-works/pi-ai`・`pi-coding-agent`)、
Node SEA ビルド経路、pnpm workspace 構成を撤去し、リポジトリを Rust
workspace 単独構成へ収束させた。配布 contract(envelope・exit code・
usage.jsonl・hook・launcher・Release asset 名・SHA256SUMS)は Phase 5
で切替済みであり、本 Phase が消したのは「もうどの実行経路にも存在
しない」TS/Node 側資産のみ。launcher(`cheaper`/`cheaper.ps1`)・
`hooks.json`・`packages/apm/apm.yml`・APM 構造は無改変(ps1 は後述の
defect 修正を除く)。

実装は commit A-E + gate 記録 + normalize 修正の 7 commit で構成:
36bb74a(A: version canonical 反転 + esbuild 除去)→ b203035(B:
toolchain/workspace 撤去)→ 6e2e49d(C: packages 撤去 + resource 移設
+ regression snapshot 化)→ 461dc04(D: benchmarks retarget)→
d68b4d5(E: docs/ADR)→ a118f12(final-gate 結果記録)→
d4f5429(snapshot separator 正規化修正)。

## T1 — Commit A: version canonical 反転 + esbuild payload 除去(36bb74a)

### Scope

canonical version source を `packages/cli/package.json` から
`Cargo.toml` `[workspace.package].version` へ反転し、payload 生成
経路から esbuild と committed `cheaper.mjs` を除去した(deferred
decision #1・#3 の確定)。

### Files

- `Cargo.toml` / `crates/cheaper-cli/Cargo.toml` — workspace version
  が単一の canonical source。`build.rs` が `CARGO_PKG_VERSION` から
  `version.json` を生成(旧: package.json 読み取り)。
- `scripts/check-version.mjs` / `set-version.mjs` /
  `scripts/lib/release-artifacts.mjs`(`verifyReleaseVersion`)/
  `build-executables.yml` の `expected_version` gate — 照合・bump・
  CI gate の全てが Cargo.toml を読む。package.json 行は Commit B で
  ファイルごと消えるため本 commit で宣言表から除去。
- `scripts/lib/apm-bundle.mjs` / `bundle-cli.mjs` /
  `check-apm-payload.mjs` — esbuild 経路を削除し、workers /
  `policy.yaml` を crate の `include_dir!` source からコピーする
  経路へ。`bin/cheaper.mjs` を出力しなくなり、committed payload
  `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`(2641 行)を同 commit
  で削除。再生成物も同一 commit に同梱。
- `scripts/build-executable.mjs`・`scripts/lib/esbuild-bundle.mjs`
  — 削除(SEA pipeline 本体)。
- 注記: この時点では crates がまだ `include_str!`/`include_dir!` で
  `packages/cli` を参照しており、resource 移設は Commit C で実施。

## T2 — Commit B: toolchain/workspace 構成撤去(b203035)

### Scope

pnpm workspace・TS toolchain 設定を撤去し、mise tasks を Rust 手順へ
書き換えた。

### Files

- 削除: root `package.json`・`pnpm-workspace.yaml`・
  `pnpm-lock.yaml`(2566 行)・`tsconfig.json`・`tsconfig.base.json`・
  `vitest.workspace.ts`。
- `mise.toml` / `mise.lock` — tools から `pnpm` を除去し `biome` を
  aqua backend で追加(node・rust は継続)。`[tasks.setup]` =
  `apm install` → `cargo fetch` → `cargo build --workspace
  --all-targets`。`[tasks.check]` = biome → `cargo fmt --check` →
  clippy → check-version → check-doc-ids → `cargo test --workspace`
  → `node --test scripts/test/ + benchmarks/test/` → bundle →
  check-apm-payload → bundle freshness(`git diff --exit-code`)。
- `biome.json` — lint scope を `scripts/`・`benchmarks/` の mjs へ
  retarget(従来 lint 対象外だったファイル群の bulk reformat を含む)。
- `scripts/lib/release-artifacts.mjs` — tar 呼び出しを relative path
  化し、`node --test` が Windows/MSYS でも通るようにした。

## T3 — Commit C: packages 撤去 + resource 移設 + regression snapshot 化(6e2e49d)

### Scope

5 つの TS package を単一 revert 可能 commit で `git rm` し、bundled
resource を crate 配下へ移設、differential harness を Rust-only の
regression snapshot test へ変換してから削除した。

### Files

- 削除: `packages/{cli,core,tool-runtime,runtime-pi,workflow}/` 全体
  (Pi SDK 依存を含む全 TS 実装)。
- `crates/cheaper-cli/resources/` — `policy.yaml`・`workers/`(worker
  定義 md + input/output schema)を `packages/cli` から移設。
  `include_dir!`/`include_str!` の source と `apm-bundle` の
  `resourceSrc` を更新し、再生成 payload が byte-identical である
  ことを確認済み。
- `crates/cheaper-cli/tests/regression/` — differential harness を
  55 case の snapshot test へ変換(`cases/` + `expected/` +
  `harness.rs`/`main.rs`/`normalize.rs`)。TS oracle との 3 件の
  pinned divergence は expectation として捕捉。旧
  `tests/differential/`(adapter.mjs・main.rs・README)と
  `scripts/ts-envelope-vectors.mjs`・`scripts/dump-core-golden.mjs`・
  `scripts/dump-tool-fixtures.mjs` を削除。
- `scripts/test/` — TS 側のみに存在した検証(launcher acquisition・
  apm install targets・package layout)を node:test suite へ移植。
- `packages/apm/.apm/hooks/cheaper.ps1` — **acquisition lock の
  defect 修正**(下記)。

### cheaper.ps1 atomic-lock 修正(contract 変更ではなく defect 修理)

長年の concurrent-download flake の root cause を修正した。
`New-Item -ItemType Directory` による lock 取得は
check-then-create で、既存パスに対して非原子に失敗するため、並行
する 2 process が双方 lock owner になり得た(結果として同一 cache
key への二重 download / staging 競合)。

修正は (a) 一意名 `.win32-x64.claim.<guid>` を `CreateDirectory` で
作り、同一ボリュームの `[IO.Directory]::Move` rename で排他 claim
する方式(mkdir(2) 相当の原子性)と、(b) publish 前に
`if (Test-Path $script:key) { throw }` の guard を追加し sh
launcher と同等の「cache key 出現時は上書き拒否」を実現するもの。
launcher の argv・exit code・エラー message の外部 contract は
不変であり、取得処理の内部実装のみの修正。

## T4 — Commit D: benchmarks retarget(461dc04)

### Scope

benchmarks が Rust binary で動作するように retarget した。

### Files

- `benchmarks/lib/cli.mjs` — spawn 先を `node packages/cli/dist`
  から `cargo build` 出力 binary へ(release 優先・debug fallback)。
- `benchmarks/fixtures/fake-worker-stream.mjs` — TS fake stream を
  Rust fake seam の `CHEAPER_TEST_FAKE_STREAM` JSON fixture DSL
  generator へ変換(`usage_delta`/`delay_ms`/`tool_calls` step)。
- `benchmarks/lib/measurements.mjs` 等 — `promptBytes`・
  `stream-concurrency` の JSONL dimension は Rust 側に対応物がなく
  `null` 報告へ。README 更新 + 1 scenario 実測済み。

## T5 — Commit E: docs/ADR(d68b4d5)

### Scope

CI workflow 残存参照除去と docs/ADR status の整合。

### Files

- `AGENTS.md` / `CONTRIBUTING.md` / `README.md` — Rust toolchain
  contract(cargo build/test・Node は repo tooling のみ・Cargo.toml
  が version canonical・credentials.json store)を記述。
- `docs/guides/*`(configuration/distribution/managed-process/
  parent-agent-integration/providers/release)と
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md` を新しい
  配布・認証の実態へ更新。
- 新規 ADR `ADR-034CpwNUNIDqrR92x7VaUR`(remove the committed
  esbuild CLI payload and move the canonical version to Cargo.toml)
  を起票し、committed-esbuild-payload ADR と
  cli-package-json-canonical ADR を supersede。Pi-era ADR
  (pi SDK・tokenizer・userConfigDir/pi・typescript-pnpm-monorepo)
  は migration ADR による superseded 表記へ。

## T6 — Final gate(a118f12 で結果を task doc へ記録)

fresh worktree `F:/repositry/cheaper-fresh-gate`(node_modules・
target・apm_modules 非存在)で `mise run setup` → `mise run check`
が exit 0(`cargo build --workspace --all-targets` 初回 7m26s、
worktree は検証後に remove 済み)。

- `node --test scripts/test/`: 132 tests / 126 pass /
  6 platform-gated skip / 0 fail。
- 撤去 contract の `git grep`(`@earendil-works|pi-coding-agent|
  vitest|tsconfig|pnpm |build-executable|esbuild|cheaper\.mjs|
  packages/(cli|core|tool-runtime|runtime-pi|workflow)`)は working
  code/CI/scripts で非 exempt ヒットなし(hit 全件を分類済み:
  port provenance comment・Cargo.toml description・NOTICE・
  benchmarks 履歴 JSON・`build-executables.yml` の workflow/job 名・
  release-workflow test の absence-pin 断言・`apm-bundle.mjs` の旧
  `cheaper.mjs` 掃除 rmSync・`process_executable.rs` fixture の
  `pnpm.cmd` shim 名)。
- local release build 実測 **19,860,480B = 18.94MiB**(gate 20MiB)。
- Node-free launcher smoke: `build-apm-bundle.mjs --exe-dir` で
  3 target sibling 配置 → `assemble-release-bundle` →
  `smoke-release.mjs` が
  `{"target":"win32-x64","version":"1.0.0","verified":3,
  "nodeFree":true}`。pwsh leg は smoke 経由、sh leg は Git Bash
  `PATH=/usr/bin:/bin`(node 不在)で deploy 済み launcher の
  `--version`/`models` を実測 pass。
- `launcher-acquisition.test.mjs` が check 内で
  windsurf/claude/codex/cursor への binary-free install + 実 binary
  hook 実行(exit 2 経路含む)を pass。
- Ollama 実 provider e2e: `CHEAPER_E2E_OLLAMA=1 cargo test -p
  cheaper-runtime --test ollama_e2e` が 3/3 pass(qwen3.5:9b。
  初回のみ doc-writer が provider 応答 truncation で transient
  fail、孤立再実行で pass の flake)。加えて `cheaper run
  bulk-reader --model ollama/qwen3.5:9b` を実 provider で実行し
  usage 実測 envelope を確認。
- version round-trip: `set-version.mjs 1.0.1` → `bundle-cli.mjs` →
  `check-version.mjs` pass・rebuild 後 binary が `1.0.1` を報告、
  1.0.0 へ revert + rebundle で `git status` clean・binary
  `1.0.0` に復帰。
- `git diff --exit-code -- packages/apm/.apm/hooks/cli` は
  `cheaper.mjs` 削除後も green。

## Review 修正(d4f5429)— POSIX snapshot separator

review で confirm された finding: regression snapshot
`cli-run-worker-file.json` が Windows path separator(`\`)を固定値
として凍結しており、POSIX CI runner で `cargo test --workspace`
が fail する状態だった。`tests/regression/normalize.rs` に
path-token 置換後の残余 separator を畳む正規化を追加(token-gated
で無関係な backslash は不変)。同 commit で `bundle-cli.mjs`・
`apm-bundle.mjs` の stale な pnpm-bundle comment も除去。

## Deferred 判断の確定

task の deferred 3 件 + Node 扱いは以下の通り確定した:

- version canonical → `Cargo.toml [workspace.package].version`
  (Commit A で反転、Commit B で package.json 自体を削除)。
- darwin-* は Release asset 非追加を維持(sibling-only。Phase 5 の
  `smoke-bundle` macOS leg は継続)。
- `cheaper.mjs` JS fallback は廃止(committed payload を Commit A
  で削除。launcher が参照しないことは version-pinned-launcher-
  acquisition design が根拠)。
- Node は repo tooling(scripts の node:test・benchmarks・mise
  管理 toolchain)として存続 — 配布物・実行経路からは完全に除去。

## Verification matrix(task doc §Verification との対応)

| item | local 結果 | 状態 |
| --- | --- | --- |
| fresh worktree `mise run setup` → `mise run check` | exit 0 実測(clean checkout から cargo build 7m26s) | 済 |
| `node --test scripts/test/` 全パス | 132 tests / 126 pass / 6 platform-gated skip / 0 fail | 済 |
| 撤去 contract `git grep` | 非 exempt ヒットなし(全件分類済み) | 済 |
| 3 target + darwin sibling release build・size gate ≤20MiB | win32-x64 local 実測 18.94MiB pass | 残 3 target + darwin は CI matrix |
| release E2E(`release-acquisition` 1 dispatch・5 asset・smoke-bundle 全 OS) | assemble→extract→smoke-release 同等経路を local 実測 | CI 延期 |
| Node-free launcher sibling smoke | `verified:3, nodeFree:true`、sh(node 不在 PATH)+ pwsh 両 leg 実測 | 済 |
| launcher 経由 hook fail-open | check 内 `launcher-acquisition.test.mjs` で 4 agent × binary-free install + hook exec(exit 2 経路含む)pass | 済 |
| Ollama 実 provider e2e | `ollama_e2e` 3/3 + `cheaper run bulk-reader` usage envelope 実測 | 済(1 transient flake は孤立再実行 pass) |
| payload freshness `git diff` | `cheaper.mjs` 削除後も green | 済 |
| version round-trip(set→bundle→check→revert) | 1.0.1 bump で全宣言一致・binary 報告一致、1.0.0 revert で clean | 済 |

## CI-deferred verification(ユーザー判断で延期)

Phase 5 と同じく branch 未 push・workflow_dispatch 未実行のため、
runner 依存の検証は CI へ延期。

- `release-acquisition` の test tag 実 dispatch による 5 asset
  (raw3 + `SHA256SUMS` + `cheaper-apm-v<X.Y.Z>.tar.gz`)publish と
  `smoke-bundle` 全 OS green の実観測。
- 3 target(linux-x64・linux-arm64・win-x64)の matrix release
  build + size gate・`--version`/`models` smoke、および
  darwin-arm64 sibling leg の runner 実証。
- 初回 Rust release の実 publish と旧 tag asset 不変(rollback
  経路)の実観測。

## 既知の留意点

- TS 実装・Pi SDK・pnpm workspace は完全に除去済み。以降の機能
  追加は `crates/` 配下の Rust workspace が唯一の実装面。
- Node は repo tooling として残るため `mise.toml` の tools からは
  除去していない(scripts/benchmarks の node:test 実行に必要)。
  「配布物・実行経路に Node 不在」と「repo tooling に Node あり」
  の区別は AGENTS.md/CONTRIBUTING.md に明記済み。
- benchmarks の `promptBytes`・`stream-concurrency` dimension は
  Rust 側に計測点がなく `null` 報告。復活させる場合は Rust 側に
  対応する計測/emit が必要。
- Ollama e2e の doc-writer transient truncation flake は provider
  応答側の不安定さに起因し、本 Phase の変更とは無関係(孤立再実行
  で pass)。
