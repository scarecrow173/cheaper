---
id: "PLAN-034Dz3LX6Jz18cbOR0KtFd"
type: "plan"
status: "approved"
title: "Execute post-migration structural cleanup"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/post-migration-structural-cleanup-scope.md"
    - "../discovery/post-migration-structural-inventory-and-cleanup-policy.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/post-migration-structural-cleanup-scope.md"
  implemented-by:
    - "../tasks/remove-js-json-shim.md"
    - "../tasks/remove-poc-rig-crate.md"
    - "../tasks/move-devin-provider-tests-to-tests-dir.md"
    - "../tasks/consolidate-js16-into-cheaper-core.md"
    - "../tasks/reorganize-cheaper-tools-modules.md"
    - "../tasks/reorganize-cheaper-runtime-run-module.md"
    - "../tasks/rename-provider-crate-and-modularize-devin.md"
    - "../tasks/gate-dev-seams-behind-dev-seam-feature.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/post-migration-structural-cleanup-reorganization.md"
    - "../test-specs/dev-seam-feature-build-mode-contract.md"
    - "../adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/post-migration-structural-cleanup-scope.md"
    - "../designs/post-migration-structural-cleanup-reorganization.md"
    - "../adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md"
    - "../adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/dev-seam-feature-build-mode-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Execute post-migration structural cleanup Implementation Plan

## Goal

spec の 6 stream(F1-F8)を、各段階で `cargo test --workspace` green を保つ
behavior-preserving な段階的再配置として実行する。

## Scope

単一 plan として実行する。stream 間で同一 workspace・同一ファイル
(`lib.rs`・`Cargo.toml`)を触る critical path が共有されており、かつ
単一の release intent(構造整理の一括適用)を持つため分割しない。
順序は spec の risk-oriented order(残滓除去 → core 集約 → 内部再配置 →
provider rename → dev-seam 化)に従う。

## Upstream Documents

- `docs/specs/post-migration-structural-cleanup-scope.md`(approved)
- `docs/adr/gate-dev-test-seams-behind-a-dev-seam-cargo-feature.md`(accepted)
- `docs/adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md`(accepted)

## Design Inputs

- `docs/designs/post-migration-structural-cleanup-reorganization.md`(approved)
  — 各 stream の target layout は design の Components and Boundaries に従う

## File Map

### T1: `js_json` shim 削除(F3)

- `crates/cheaper-rig-adapter/src/js_json.rs` — 削除(re-export shim のみ)
- `crates/cheaper-rig-adapter/src/lib.rs` — `pub mod js_json;` 除去
- `crates/cheaper-rig-adapter/src/agent.rs` — `crate::js_json::stringify*` →
  `cheaper_core::json::stringify::*` へ import 変更(2 箇所)
- `crates/cheaper-rig-adapter/tests/adapter.rs` — 参照更新(関数名含め)
- `crates/cheaper-runtime/src/meta.rs` — `cheaper_rig_adapter::js_json::stringify`
  → `cheaper_core::json::stringify` へ変更
- `crates/cheaper-runtime/src/repair.rs` — 同上(stringify_pretty)

### T2: `js16` の core 集約(F4)

- `crates/cheaper-core/src/js16.rs` — `cheaper-tools/src/js16.rs` の実装を移設
- `crates/cheaper-core/src/lib.rs` — `pub mod js16;` 追加
- `crates/cheaper-tools/src/js16.rs` — `pub use cheaper_core::js16::*;`
  の thin re-export に置換(`cheaper_tools::js16` path 互換のため)
- caller(`crates/cheaper-tools/src/tools/{edit,find,grep}.rs`)は
  `crate::js16` 参照のまま変更不要の想定
- caller 調査済み: `crate::js16` 参照は tools/{edit,find,grep}.rs の
  3 file(edit.rs:365・find.rs:17・grep.rs:35)のみで外部参照ゼロ
  (repo-wide grep で検証済み)。spec の caller 調査 precondition を
  満たす

### T3: provider test 移動(F7)

- `crates/cheaper-provider-devin/src/tests/*.rs` — `crates/cheaper-provider-devin/tests/` へ移動
- `crates/cheaper-provider-devin/src/lib.rs` — `mod tests;` / `#[cfg(test)]` 宣言除去
- `crates/cheaper-provider-devin/src/tests/mod.rs` — 分割して各 test file へ
  統合 test 化(shared helper は `tests/common/` または各 file 内へ)
- 移動先の test は `cheaper_provider_devin::` の public API 経由に調整
  (crate 内部 `crate::` 参照は統合 test では使えない)
- visibility 決定(確定): 移動対象 test が `src/tests/` から参照している
  `pub(crate)` item は `jwt_expires_at`(src/jwt.rs:73)・
  `chat_status_message`/`chat_send_error`/`resolve_host`
  (src/rig_provider.rs:365/374/460)。解決方針は move 時に最小限の
  `pub` 化(intentional surface change として task-doc に記録)を基本とし、
  spec AC「全テストが `tests/` 配下で pass する」と整合させる。
  移動時に `pub(crate)` grep sweep を行い、追加の private 参照が
  見つかった場合も同方針で処理する

### T4: `poc-rig` 削除(F5)

- `crates/poc-rig/` — ディレクトリ削除
- `Cargo.toml` — `exclude = ["crates/poc-rig"]`(line 8)と poc-rig 説明
  コメント block(line 2-4)を除去。line 1 の manifest header
  (`# Repo-root virtual manifest ...`)は残す
- `.gitignore` — `crates/*/target/`(line ~185)を除去
  (直上 comment の "standalone spikes" 用で poc-rig 専用)
- `CONTRIBUTING.md` — workspace 構成説明の `crates/poc-rig` 項
  (:89-90)を除去
- 削除前に `poc-rig` の知見が移行設計書/ADR に取り込み済みか確認
- 参照調査: `poc` を名前に含む doc/script 参照の確認。adapter の
  doc comment 内 `crates/poc-rig/*` 参照は歴史的記述として残存許容
  (Verification の residual-grep allowlist 参照)

### T5: `cheaper-tools` 内部再配置(F1 / T-B)

- `crates/cheaper-tools/src/glob.rs` → `src/glob/mod.rs`
- `crates/cheaper-tools/src/glob_tables.rs` → `src/glob/tables.rs`
- `crates/cheaper-tools/src/tools/process.rs` → `src/tools/process/mod.rs`
- `crates/cheaper-tools/src/tools/process_executable.rs` → `src/tools/process/executable.rs`
- `crates/cheaper-tools/src/tools/process_tree.rs` → `src/tools/process/tree.rs`
- `crates/cheaper-tools/src/lib.rs`・`src/tools/mod.rs` — module 宣言追従
- glob/process 内部の相対参照(`crate::glob_tables` 等)を module path に追従

### T6: `cheaper-runtime` 内部再配置(F6 / R-B)

- `src/run.rs` → `src/run/mod.rs`
- `src/run_error.rs` → `src/run/error.rs`
- `src/abort.rs` → `src/run/abort.rs`
- `src/grace.rs` → `src/run/grace.rs`
- `src/repair.rs` → `src/run/repair.rs`
- `src/output.rs` → `src/run/output.rs`
- `src/meta.rs` → `src/run/meta.rs`
- `src/panic_boundary.rs` → `src/run/panic_boundary.rs`
- `src/lib.rs` — module 宣言追従 + 公開 path を `pub use` で維持
- module path 互換方針(確定): module alias re-export で維持し、
  consumer 書き換えは最小化する。`src/run/mod.rs` で
  `pub mod error; pub mod meta; pub mod panic_boundary;` 等を宣言し、
  `src/lib.rs` で `pub use run::error as run_error;`・`pub use run::meta;`・
  `pub use run::panic_boundary;` 等により
  `cheaper_runtime::{meta,run_error,panic_boundary}::*` の現行 path を
  維持する。known consumer: cli `tests/envelope.rs`:14,38、runtime
  `tests/run.rs`:38,560、`tests/run_fixture_test.rs`:33
- `auth/`・`catalog/`・`plugin.rs`・`runtime.rs`・`fake_provider.rs`・
  `fixture.rs` は T6 では現位置維持(dev/ 化は T8)

### T7: provider crate rename + devin module 化(F8 / PV-B)

- `crates/cheaper-provider-devin/` → `crates/cheaper-provider/`(git mv)
- `crates/cheaper-provider/Cargo.toml` — `name = "cheaper-provider"` へ変更
- `src/lib.rs` — `pub mod devin;` のみを公開する薄い root へ再編
- `src/*.rs`(cli/context_map/credentials/jwt/metadata/models/plugin/
  rig_provider/stream/wire)→ `src/devin/` 配下へ移動、
  `src/devin/mod.rs` で現行の公開 API(`DevinProviderPlugin` 等)を
  `pub use` で維持
- `crates/cheaper-cli/Cargo.toml` — dependency を `cheaper-provider` へ更新
- `use cheaper_provider_devin` の全 site を `cheaper_provider::devin` へ更新:
  - `crates/cheaper-cli/src/auth.rs`:34
  - `crates/cheaper-cli/src/usage_report.rs`:31
  - `crates/cheaper-cli/src/run.rs`:40
  - `crates/cheaper-cli/tests/auth.rs`:12
  - T3 で `tests/` へ移動済みの provider test file の
    `cheaper_provider_devin::` 参照
- code 外参照も洗い替え: `crates/cheaper-runtime/Cargo.toml`:34 の
  base64 pin comment、rig-adapter doc comment(`src/lib.rs`:11・
  `src/agent.rs`:44)、`CONTRIBUTING.md`:87、`AGENTS.md`:83、
  doc/script/benchmark 内の `cheaper-provider-devin`・
  `cheaper_provider_devin` 参照
- `Cargo.lock` 更新(`cargo build` で自動反映、committed lockfile check 通過)

### T8: `dev-seam` feature gate(F2 / S-B)

- `crates/cheaper-runtime/Cargo.toml` — `[features] dev-seam = []`
- `crates/cheaper-cli/Cargo.toml` — `[features] dev-seam = ["cheaper-runtime/dev-seam"]`
- `crates/cheaper-runtime/src/dev/{mod,fake_provider,fixture}.rs` —
  fake_provider.rs・fixture.rs を `dev/` 配下へ移動し
  `#[cfg(feature = "dev-seam")] pub mod dev;` で公開
- `crates/cheaper-runtime/src/lib.rs` — fake/fixture の mod 宣言を cfg gate
- module path 互換方針(確定): `#[cfg(feature = "dev-seam")]
  pub use dev::{fixture, fake_provider};` で
  `cheaper_runtime::{fixture,fake_provider}::*` の現行 module path を
  維持し、consumer 書き換えを最小化する。known consumer: runtime
  `tests/run.rs`:33,622,668,719・`tests/run_fixture_test.rs`:27-30、cli
  `tests/regression/harness.rs`:19・`src/fake_stream.rs`:19・
  `src/run.rs`:45。`fixture_default_policy`(lib.rs:69)は consumer ゼロ
  (repo-wide grep で検証済み)のため再 export せず drop する
- `crates/cheaper-cli/src/fake_stream.rs` — `#[cfg(feature = "dev-seam")]` で
  module 全体を gate(`src/lib.rs` の `pub mod fake_stream;` も)
- `crates/cheaper-cli/src/run.rs` — fixture plumbing を cfg gate(全 site):
  `load_test_stream` 呼出(:359)・`resolve_run_worker` への fixture
  param/`resolve_named_worker` fallback(:361)・
  `f.policy.apply_to(&mut policy)`(:376-381)・`create_runtime` への
  fixture 引数(:406)・`session_factory` 呼出(:414)・`explicit_model`
  fallback(:419-422)・fixture abort task(:432-443)・
  `abort_grace_ms` override(:457)。`create_runtime`(:105-135)内の
  env overlay・`cheaper-fake` plugin push・`auth_logins` scripted
  login も同様
- `crates/cheaper-cli/src/workflow_run.rs` — 同上: `load_test_stream`
  呼出(:166)・session_factory 分岐(:170-173)・`explicit_model`(:178)・
  `abort_grace_ms`(:179)
- seam-off 時の dead-code 対象(各々 `#[cfg]` gate または restructure):
  `SignalRelay::controller()`(run.rs:297、sole caller :433)、
  `Shared.env` field(workflow_run.rs:123、sole reader :166、構築
  :735-738)、`session_factory` signature の `RunFixture` 参照
  (run.rs:184)、`create_runtime` の fixture param(run.rs:105-107)。
  clippy `-D warnings` での dead-code fallout 解消を T8 の
  想定工数に含める
- test gating strategy(確定):
  - 丸ごと seam 依存の target → `[[test]] required-features =
    ["dev-seam"]`: `crates/cheaper-cli/tests/regression/`(55 case)、
    `crates/cheaper-runtime/tests/run_fixture_test.rs`
  - seam/non-seam 混在 target → file-split: cli `tests/run.rs`
    (seam 依存 case + non-seam case 混在)・cli `tests/workflow.rs`・
    runtime `tests/run.rs` は seam 依存 test を専用 target file
    (`tests/fake_stream.rs` 等)へ分離して required-features、
    non-seam test は現 file に残す
  - in-src `#[cfg(test)]` module で seam 型を使うもの →
    `#[cfg(all(test, feature = "dev-seam"))]`
  - non-seam test(`tests/envelope.rs` 等)は ungated のまま
- `grep_eval` の seam 分類(確定): gate 対象外。`__grep-eval`
  dispatch は production code(`crates/cheaper-cli/src/main.rs`:73-77。
  `EvalSpawn::current_exe` が production default — grep_eval.rs:106)。
  `test_support`(grep_eval.rs:599-)のみ既に `#[cfg(test)]` 済み
- `mise.toml` — benchmark/regression/e2e 関連 task は存在せず、実 touch
  point は `[tasks.setup]`・`[tasks.check]` の cargo コマンド:
  - `tasks.check` の `cargo test --workspace`(line ~46)→
    `cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
    へ更新(ADR-034DyGyITd0YaATg8fkld7 の prescribed form)
  - `tasks.check` の `cargo clippy --workspace --all-targets -- -D warnings`
    (line ~43)にも同 feature を付与 — 付けなければ gated code が
    `-D warnings` を逃れる
  - `tasks.setup` の `cargo build --workspace --all-targets`(line ~31)
    も同 feature で build する(推奨: 付与 — fresh checkout で seam
    test target も build 検証される)
  - 注記: regression(55 case)・Ollama e2e は `cargo test --workspace`
    内で動くため、上記 feature 付与で網羅される
- `benchmarks/lib/cli.mjs` — binary 解決 comment(:14-18)の build 手順を
  `cargo build --release -p cheaper-cli --features dev-seam` 前提に更新。
  `benchmarks/README.md` の前提 section(:9-15)の build 手順も同様に
  更新(`benchmarks/run.mjs` 自体に build step は存在しない)
- `CONTRIBUTING.md` — `mise run check`・`cargo test` 手順(:21-44)を
  feature 有効 form へ追従

## Implementation Sequence

- [ ] T1 `js_json` shim 削除(F3)
- [ ] T4 `poc-rig` 削除(F5)
- [ ] T3 provider test 移動(F7)
- [ ] T2 `js16` core 集約(F4)
- [ ] T5 `cheaper-tools` 内部再配置(F1)
- [ ] T6 `cheaper-runtime` 内部再配置(F6)
- [ ] T7 provider crate rename(F8)
- [ ] T8 `dev-seam` feature gate(F2)

各 task 完了時に `cargo build --workspace` + 対象 crate test を実行し、
最終 task 後に全 verification matrix を実行する。

## Dependencies and Risks

| Step | Depends on | Blocks |
| --- | --- | --- |
| T1 js_json | — | — |
| T4 poc-rig | — | — |
| T3 test move | — | T7 |
| T2 js16 | — | T5(同一 crate 触る順序付け) |
| T5 tools reorg | T2 | — |
| T6 runtime reorg | T1(runtime meta/repair の import 変更と競合回避) | T8 |
| T7 provider rename | T3 | T8(cli Cargo.toml/imports と共有) |
| T8 dev-seam | T6, T7 | — |

T1・T4 は他 stream と独立(並列化可能)。T2/T5/T6 は同一 crate 内の
順序依存。T8 は全ての reorg 後に実施して cfg 対象を確定させる。

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | module 移動で `crate::` 内部参照・`pub use` 公開 path が壊れる | med | med | 各 task で `cargo build --workspace` を実行し、公開 path 維持を `pub use` で担保 |
| 2 | provider test の `crate::` 参照が統合 test で使えずビルド失敗 | med | low | T3 で private helper は test 側へ移すか public 化の範囲を最小化して調整 |
| 3 | dev-seam gate 後に benchmark/regression/e2e が default binary で失敗 | med | high | feature 有効 binary 用の task/手順を同時更新。default build で seam 非含有を grep/サイズで確認 |
| 4 | crate rename で `Cargo.lock`・doc 参照の取りこぼし | low | low | `cheaper_provider_devin`/`cheaper-provider-devin` の残存 grep を acceptance criteria 化 |
| 5 | poc-rig 削除で未捕捉の参照が壊れる | low | low | 削除前に `poc` 参照を全域 grep、workspace exclude 外で参照されていないことを確認 |

## Task Breakdown

各 task は task-doc で独立文書化する。task 粒度は「1 task = 1 stream の
atomic な変更 + その verification」で、上記 T1-T8 に対応する。

## Verification

- [ ] 各 task 後: `cargo build --workspace` + `cargo test -p <対象 crate>`
- [ ] T8 後: `cargo build -p cheaper-cli`(default)で fake/fixture/fake_stream 非含有を確認
- [ ] T8 後: default-feature binary に `CHEAPER_TEST_FAKE_STREAM` +
      `NODE_ENV=test` を設定して実行し、seam が起動しないこと(fixture
      worker が `WORKER_NOT_FOUND` になること)を runtime smoke check で
      確認。precedent: cli `tests/run.rs`:323
      `run_fake_stream_disabled_without_node_env_test` — ただし
      default-feature binary 対象として別途実施する
- [ ] T8 後: feature 有効で `cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam`・regression 55 case・
      `benchmarks/run.mjs`・Ollama e2e(gate 有)が現行通り通る
- [ ] 最終: `mise run check` green
- [ ] 残存参照 grep: `js_json`・`cheaper_provider_devin`・`poc-rig` が
      意図した箇所以外に残存 0。intended-residue allowlist: adapter doc
      comments(`crates/cheaper-rig-adapter/src/hooks.rs`:4・
      `src/error.rs`:3・`src/accumulator.rs`:4・`src/agent.rs`:44・
      `tests/adapter.rs`:3)、`crates/cheaper-workflow/tests/no_pi_deps.rs`
      の denylist、`docs/{tasks,plans,research,impl}` 配下の歴史文書内
      `poc-rig` 記述は残存許容

## Open Questions / Gaps

- (解決済み)`grep_eval` の seam 分類: gate 対象外と確定。`__grep-eval`
  dispatch は production code(main.rs:73-77、`EvalSpawn::current_exe`
  が production default)で、`test_support` のみ `#[cfg(test)]` 済み
  — T8 file map に確定事項として記載
- (解決済み)`src/tests/` → `tests/` 移動時の private item 参照の
  公開範囲: 対象 item を列挙済み(`jwt_expires_at`・
  `chat_status_message`・`chat_send_error`・`resolve_host`)。
  最小限の `pub` 化を基本とする方針を T3 file map に確定事項として記載

## Self-Review

- [x] Every spec requirement maps to a task(F1-F8 → T5/T6/T1/T2/T3/T4/T7/T8)
- [x] No placeholders remain
- [x] File paths, relations, status values, and terminology are consistent

## Implementation Handoff

- Recommended mode: inline execution(task 間の共有ファイル競合が多く、
  各 task が順次依存する critical path のため逐次実行が安全)
- Independent work streams: T1・T4 のみ他 stream と独立だが、規模が小さいため
  逐次実行で十分(subagent dispatch は不要)
- User approval required before dispatch: yes
- If declined or unavailable: execute inline using the same dependency graph
  and verification matrix

## Review Handoff

- 上流 doc: spec(approved)+ 2 ADR(accepted)+ design(approved)で
  target layout・採否・順序は確定済み
- review で確定した判断(module path 互換・test gating・T3 公開範囲・
  grep_eval 非 gate 化)は T3/T6/T7/T8 file map と Verification に
  確定事項として記載済み。Open Questions に残る未決事項は無い
- 各 task 完了時の `cargo test -p` green を checkpoint とし、最後に
  `mise run check` + feature matrix で exit する
