---
id: "TASK-034EDQnyEmsl4J7YHNNQFw"
type: "task"
status: "done"
title: "T05: runtime ctor consolidation, dev alias, provider error conversion, adapter visibility"
created: "2026-10-12"
updated: "2026-10-12"
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
    - "docs/plans/execute-post-feature-structural-refactoring.md"
  implemented-by: []
  depends-on:
    - "docs/plans/execute-post-feature-structural-refactoring.md"
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
    - "docs/test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# T05: runtime ctor consolidation, dev alias, provider error conversion, adapter visibility

## Work

runtime/provider/adapter の Phase 0 項目を実施する。

- runtime: `model_from_json` vs `extension_models` の共通 ctor 化 +
  コメント drift の機械的修正（実装意図の確認は spec Deferred のまま）
- runtime: `dev::{fixture,fake_provider}` alias 解消 — workspace 内 caller
  （cli run.rs/fake_stream.rs・dev-seam 系 tests）を `dev::` path へ統一し
  `lib.rs` の compat re-export を除去（dev-seam gate のため consumer API 不変）
- provider: `CliError→CheaperError` 変換を `From`/helper へ集約
  （plugin.rs・models.rs の 2 箇所）
- adapter: error.rs の外部未使用 pub fn を `pub(crate)` 化。
  `tests/adapter.rs` が使う `classify`/`ErrorClass`/
  `abort_reason_to_cheaper` は公開維持。`RetryPolicy` export の要否確認

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/execute-post-feature-structural-refactoring.md`
- `docs/specs/post-feature-structural-refactoring-scope.md`
- `docs/designs/post-feature-structural-refactoring.md`

## Dependencies

- なし

## Affected Files

- `crates/cheaper-runtime/src/{catalog/mod.rs,dev/,lib.rs}`
- `crates/cheaper-cli/src/{run.rs,fake_stream.rs}`・dev-seam 系 tests
  （alias caller 更新）
- `crates/cheaper-provider/src/{plugin.rs,models.rs}`
- `crates/cheaper-rig-adapter/src/{error.rs,lib.rs}`

## Done When

- [ ] 各重複が一権威化されている
- [ ] `dev-seam` feature 付き・無し両方で build/test が pass

## Verification

- [ ] `mise exec -- cargo test -p cheaper-runtime -p cheaper-provider -p cheaper-rig-adapter`（dev-seam 付き）が pass
