---
id: "TASK-034DzYC8wR9HwNbdMoF8NP"
type: "task"
status: "todo"
title: "T7: provider crate rename + devin module 化 (F8)"
created: "2026-10-10"
updated: "2026-10-10"
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
    - "docs/plans/execute-post-migration-structural-cleanup.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-migration-structural-cleanup.md"
    - "docs/tasks/move-devin-provider-tests-to-tests-dir.md"
  blocks:
    - "docs/tasks/gate-dev-seams-behind-dev-seam-feature.md"
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
---

# T7: provider crate rename + devin module 化 (F8)

## Work

plan `docs/plans/execute-post-migration-structural-cleanup.md`
(PLAN-034Dz3LX6Jz18cbOR0KtFd)の T7 stream(F8 / PV-B)。
`cheaper-provider-devin` crate を `cheaper-provider` へ rename し、
provider 実装を `src/devin/` module 配下へ移動する
(`pub use` で現行公開 API を維持する薄い root + devin module)。
code 外参照(Cargo.toml comment・doc comment・CONTRIBUTING・AGENTS・
doc/script/benchmark)も洗い替えする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-migration-structural-cleanup.md` — 本 task が
  implement する plan。File Map「T7: provider crate rename + devin
  module 化(F8 / PV-B)」が作業の正
- `docs/specs/post-migration-structural-cleanup-scope.md` — F8 stream の
  acceptance criteria
- `docs/adr/consolidate-bundled-provider-implementations-into-a-single-cheaper-provider-crate.md`
  — bundled provider 実装を単一 `cheaper-provider` crate へ集約する
  判断根拠
- `docs/designs/post-migration-structural-cleanup-reorganization.md` —
  Components and Boundaries の cheaper-provider target layout

## Dependencies

- `docs/tasks/move-devin-provider-tests-to-tests-dir.md`(T3)— test が
  `tests/` へ移動済みであることが前提。T3 完了後に着手する(plan の
  Dependencies table: T7 depends on T3)
- `docs/tasks/gate-dev-seams-behind-dev-seam-feature.md`(T8)を block
  する — cli `Cargo.toml`・import を共有するため rename 確定後に T8 へ
  進む

## Affected Files

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
- `Cargo.lock` 更新(`cargo build` で自動反映、committed lockfile check
  通過)

## Done When

- [ ] crate dir が `crates/cheaper-provider/` で `name =
      "cheaper-provider"`、公開面は `pub mod devin;` の薄い root +
      `src/devin/` module 構成になっている
- [ ] `use cheaper_provider_devin` の全 site・code 外参照が
      `cheaper_provider::devin` / `cheaper-provider` へ更新されている
- [ ] `Cargo.lock` が更新され committed lockfile check が pass する
- [ ] `cargo build --workspace` と対象 crate の test が green

## Out of Scope

- devin provider 以外の bundled provider 実装の追加 — 本 task は
  rename + devin module 化のみ。`fake` provider plugin 登録の seam
  gate 化は T8 の範囲
- docs 履歴文書(`docs/{tasks,plans,research,impl}` 配下)内の旧名
  記述 — 残存許容(allowlist)

## Verification

- [ ] `mise exec -- cargo build --workspace` exits 0
- [ ] `mise exec -- cargo test -p cheaper-provider` exits 0
- [ ] `mise exec -- cargo test -p cheaper-cli` exits 0
- [ ] `test -d crates/cheaper-provider && test -f
      crates/cheaper-provider/src/devin/mod.rs && test ! -d
      crates/cheaper-provider-devin` exits 0
- [ ] `git grep -n "cheaper_provider_devin\|cheaper-provider-devin" --
      crates/ scripts/ benchmarks/ CONTRIBUTING.md AGENTS.md` が 0 件を
      返す(exit 1。docs 履歴文書は allowlist 対象)
- [ ] `mise exec -- node scripts/check-lockfile.mjs` exits 0
      (committed lockfile check)
