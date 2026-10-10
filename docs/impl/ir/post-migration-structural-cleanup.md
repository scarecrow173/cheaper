---
id: "IMPL-034DE3wewvurwfdPB2HrG6"
type: "impl"
status: "completed"
title: "Rust 移行後の構造整理"
created: "2026-10-10"
updated: "2026-10-10"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-core/src/js16.rs"
      - type: "file"
        path: "crates/cheaper-provider/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
    modified:
      - type: "file"
        path: ".gitignore"
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "CONTRIBUTING.md"
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "mise.toml"
      - type: "file"
        path: "benchmarks/README.md"
      - type: "file"
        path: "benchmarks/fixtures/fake-worker-stream.mjs"
      - type: "file"
        path: "benchmarks/lib/cli.mjs"
      - type: "file"
        path: "crates/cheaper-cli/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-cli/src/auth.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/models.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/workflow_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/auth.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/README.md"
      - type: "file"
        path: "crates/cheaper-cli/tests/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow.rs"
      - type: "file"
        path: "crates/cheaper-core/src/json/stringify.rs"
      - type: "file"
        path: "crates/cheaper-core/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/agent.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-rig-adapter/tests/adapter.rs"
      - type: "file"
        path: "crates/cheaper-runtime/Cargo.toml"
      - type: "file"
        path: "crates/cheaper-runtime/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/js16.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/mod.rs"
    deleted:
      - type: "dir"
        path: "crates/poc-rig/"
      - type: "file"
        path: "crates/cheaper-rig-adapter/src/js_json.rs"
      - type: "file"
        path: "crates/cheaper-provider-devin/src/tests/mod.rs"
    renamed: []
    moved:
      - type: "dir"
        path: "crates/cheaper-provider-devin/ -> crates/cheaper-provider/"
      - type: "dir"
        path: "crates/cheaper-provider-devin/src/ -> crates/cheaper-provider/src/devin/"
      - type: "file"
        path: "crates/cheaper-provider-devin/src/lib.rs -> crates/cheaper-provider/src/devin/mod.rs"
      - type: "dir"
        path: "crates/cheaper-provider-devin/tests/ -> crates/cheaper-provider/tests/"
      - type: "file"
        path: "crates/cheaper-tools/src/glob.rs -> crates/cheaper-tools/src/glob/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/glob_tables.rs -> crates/cheaper-tools/src/glob/tables.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process.rs -> crates/cheaper-tools/src/tools/process/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process_executable.rs -> crates/cheaper-tools/src/tools/process/executable.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process_tree.rs -> crates/cheaper-tools/src/tools/process/tree.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run.rs -> crates/cheaper-runtime/src/run/mod.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/abort.rs -> crates/cheaper-runtime/src/run/abort.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/run_error.rs -> crates/cheaper-runtime/src/run/error.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/grace.rs -> crates/cheaper-runtime/src/run/grace.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/meta.rs -> crates/cheaper-runtime/src/run/meta.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/output.rs -> crates/cheaper-runtime/src/run/output.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/panic_boundary.rs -> crates/cheaper-runtime/src/run/panic_boundary.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/repair.rs -> crates/cheaper-runtime/src/run/repair.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/fake_provider.rs -> crates/cheaper-runtime/src/dev/fake_provider.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/fixture.rs -> crates/cheaper-runtime/src/dev/fixture.rs"
    generated: []
  implements:
    - "docs/tasks/remove-js-json-shim.md"
    - "docs/tasks/consolidate-js16-into-cheaper-core.md"
    - "docs/tasks/move-devin-provider-tests-to-tests-dir.md"
    - "docs/tasks/remove-poc-rig-crate.md"
    - "docs/tasks/reorganize-cheaper-tools-modules.md"
    - "docs/tasks/reorganize-cheaper-runtime-run-module.md"
    - "docs/tasks/rename-provider-crate-and-modularize-devin.md"
    - "docs/tasks/gate-dev-seams-behind-dev-seam-feature.md"
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

# Rust 移行後の構造整理

## Summary

TypeScript → Rust 移行完了後に残った構造的な負債を一括整理した。js16 UTF-16 helper を `cheaper-core` へ集約し、`cheaper-rig-adapter` の `js_json` re-export shim を削除。完了済みの `poc-rig` spike crate を workspace から除去。`cheaper-provider-devin` を `cheaper-provider` へ rename し、Devin 実装を `src/devin/` module 配下へ集約、unit test を top-level `tests/` へ移動。`cheaper-tools` の glob / process 関連 module をサブディレクトリ化し、`cheaper-runtime` の run pipeline 諸 module を `run/` 配下へ集約。最後に fake provider / fixture 等の dev/test seam を `dev` module へ隔離し `dev-seam` cargo feature で gate した。

## Context

Rust 移行完了後の構造整理 task 群（TASK-034DzYbfe9r0G8Nd58FQjy, TASK-034DzYnno038qspspOYHv2, TASK-034DzYjQDNWcel1Oqxnlia, TASK-034DzYfbw07gFHdsAguwHp, TASK-034DzYs0gJRVZ5JJfKAIAn, TASK-034DzYxGyUC8loD0ruTCQN, TASK-034DzYC8wR9HwNbdMoF8NP, TASK-034DzYI5SJnPTX6KZeAvYU）に従う。dev/test seam の feature gating は ADR-034DyGyITd0YaATg8fkld7 および test-spec `docs/test-specs/dev-seam-feature-build-mode-contract.md` の contract に従う。移行で生じた一時的な shim・spike crate・平坦な module 配置を、責務ごとの境界へ修正する。

## Implementation

### Changes

- `crates/cheaper-core/src/js16.rs` — UTF-16 関連 helper を `cheaper-tools` 側の重複実装から core へ集約（T2）。`cheaper-tools/src/js16.rs` は core への委譲へ縮小。
- `crates/cheaper-rig-adapter/src/js_json.rs` 削除 — `cheaper-core::json` への re-export shim を外し、利用側（agent / runtime meta / repair）を core 直接参照へ（T1）。
- `crates/poc-rig/` 削除 — 移行フェーズの spike crate。workspace `Cargo.toml` メンバ、`.gitignore`、`CONTRIBUTING.md` の参照も除去（T4）。
- `crates/cheaper-provider-devin` → `crates/cheaper-provider` — crate rename + `src/*` を `src/devin/` module へ集約（`lib.rs` → `devin/mod.rs`、新規 `src/lib.rs` で plugin 登録面を再 export）、`src/tests/` 内の unit test を top-level `tests/` へ移動（T3 / T7）。`cheaper-cli`・`cheaper-runtime`・`rig-adapter` の依存名・参照を更新。
- `crates/cheaper-tools` — `glob.rs` / `glob_tables.rs` を `glob/` サブディレクトリへ、`tools/process*.rs` を `tools/process/` サブディレクトリへ再配置（T5）。
- `crates/cheaper-runtime` — `run.rs`・`abort.rs`・`run_error.rs`・`grace.rs`・`meta.rs`・`output.rs`・`panic_boundary.rs`・`repair.rs` を `run/` サブモジュールへ集約（T6）。
- `dev-seam` feature gating（T8）— `fake_provider.rs` / `fixture.rs` を `dev/` module へ移動し `cfg(feature = "dev-seam")` で gate。`cheaper-cli` 側にも同名 feature を追加して dev seam 依存コードを gate。seam 依存 test を `tests/run_dev_seam.rs` / `tests/workflow_dev_seam.rs`（cli）、`tests/run_dev_seam.rs`（runtime）へ分離し、feature 無し build で抜け落ちない構造にした。`mise.toml` の check/build 系 cargo コマンドと `benchmarks/lib/cli.mjs` に feature flag を付与し、AGENTS.md・CONTRIBUTING.md・benchmarks/README.md を更新。
- post-review fixup — `dev/fixture.rs` の import を `super::fake_provider` の canonical path 経由に修正、未使用の `fixture_default_policy` re-export alias を除去、`benchmarks/fixtures/fake-worker-stream.mjs` と regression README の stale な fixture path 参照を修正。

### Decisions During Implementation

- **crate 名を `cheaper-provider` に汎用化**: provider plugin は Devin 固有でなく `CheaperProviderPlugin` contract の bundled 実装を置く層のため、crate 名から `devin` を外し provider 固有実装を `devin` module へ閉じ込めた。
- **`src/tests/` → top-level `tests/`**: unit test を crate 標準の integration test 配置へ寄せ、`src/lib.rs` の `#[cfg(test)]` 経路を解消。
- **dev seam を feature で gate**: fake provider / fixture は production build に不要な test-only seam のため、default feature に含めず `dev-seam` opt-in とし、benchmarks / mise check 経路で明示的に有効化。
- **module 分割は move のみ**: 各再配置はファイル移動と `mod` / `use` 更新に限定し、公開 API surface（`lib.rs` の re-export）を維持して呼び出し側の変更を最小化。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise run check` — biome check → cargo fmt --check → cargo clippy → version/doc-id check → cargo test --workspace → node --test → lockfile / payload check が全て pass。
- dev-seam feature gate: `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 付きの build / clippy / test を canonical contract として `mise.toml` に反映済み。
- `docs/test-specs/dev-seam-feature-build-mode-contract.md` の contract に沿い、feature 無しでも dev seam 非依存 test が検証から抜け落ちないことを確認。

## Risks

- dev seam を使う新規 test を書く際、`dev-seam` feature flag 無しでは compile 対象外になるため、flag 付き実行が前提となる点に注意が必要（AGENTS.md に手順記載済み）。
- `cheaper-provider` への rename により external な crate 名参照がある場合は追随が必要（workspace 内の参照は全て更新済み）。

## Follow-ups

- なし。
