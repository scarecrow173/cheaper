---
id: "IMPL-034MeFMKLoLlMXEH8Ujgfi"
type: "impl"
status: "completed"
title: "Pin usage route error contract tests issue 184 T1"
created: "2026-10-25"
updated: "2026-10-26"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_report.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "docs/tasks/pin-usage-route-error-contract-tests.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/pin-usage-route-error-contract-tests.md"
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
  references:
    - "docs/plans/implement-usage-route-error-analysis-issue-184.md"
    - "docs/designs/usage-grouped-route-error-analysis-issue-184.md"
    - "docs/test-specs/usage-route-error-analysis-contract-issue-184.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Pin usage route error contract tests issue 184 T1

## Summary

T1 の compile-only seam と契約 pin 34 件を追加した。
承認済み expected-red slice であり、T2/T3/T4 の動作実装は行わない。
指定検証済み、独立レビュー/commit/最終closeoutは親へ引き継ぐため in-progress。

## Context

承認済み #184 plan / design / spec / test-spec と T1 に従う。
#192 amendment の五つの consumer tolerance を含むが、ID mint、
History store/schema、Usage writer は対象外。
graph は `task-graph-to-implementation` / `tasks-runnable`、
T1 のみ runnable、issues/blockers は空（exit 0）。

## Implementation

### Changes

- `UsageGroupBy::{Route,Error}`・`UsageFilters.route`・未使用
  `_group_by` 引数を公開。既存 src/test caller は `None` へ機械的に追従。
- fixture builder 3 種と route/error/window/filter/CLI/text/privacy/
  canonical-number/非有限/legacy/fake の report pin 26 件、
  #192 consumer tolerance 5 件、args guard 2 件、
  `usage_log_report_callers` 1 件を既存 integration module 内へ追加。
- route row の加重圧縮率、field順、error sort/内訳key順、recorded-run
  母数、first-wins logRef join、window-wide orphan/unkeyed を検証対象とした。
- task と index を in-progress に更新し、Done When の個別証拠、
  正確な expected-red 31 件（owner/最初の失敗）を task 本文へ記録した。
- Cargo/dependency、writer、History/ID 発行/妥当性、T2/T3/T4 の本体、
  既存 assertion と regression golden は変更していない。

### Decisions During Implementation

`UsageGroupBy` と `UsageFilters.route` は未使用の公開 API seam。
既存 caller は `None` を渡し、挙動・既存 assertion・golden は変更しない。
既知の contract-first 作業なので Experiment Log は作らない。
profile default は incremental-implementation / task-doc /
test-driven-development / impl-doc、tooling は git-commit、
review は requesting-code-review。外部モデル委譲はユーザー指示で使用しない。
build/fmt/clippy と既存 test の green を維持する一方、新規 pin を green に
する実装は明示的に保留する（承認済み T1 の expected-red 契約）。

## Related Experiments

### Adopted

なし（既知の contract-first 作業）。

### Rejected

なし。

## Validation

正確な command・exit・test名別の最初の失敗・owner は
`docs/tasks/pin-usage-route-error-contract-tests.md` の
「T1 implementation evidence」を参照。

- `mise exec -- cargo fmt --all` / `cargo fmt --all --check`: 各 exit 0。
- `mise exec -- cargo build -p cheaper-cli --all-targets
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`: exit 0。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings`: exit 0。
- 以下 test は `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` に続く指定:
  - `--test integration usage_report::`: exit 101、
    36 pass（既存34 + 新規2）/ 29 expected-red / 0 ignored / 271 filtered。
  - `--test integration usage_log::`: exit 0、
    21 pass（既存20 + 新規1）/ 0 fail / 0 ignored / 315 filtered。
  - `--test integration args::`: exit 101、
    45 pass（全て既存）/ 新規2 expected-red / 0 ignored / 289 filtered。
  - `--test regression usage`: exit 0 だが 0 tests / 1 filtered
    なので証明に使わず、全 replay を再実行した。
  - `--test regression -- --nocapture`: exit 0、
    `regression_golden` 1 pass、cases=61 pass=61 fail=0。
- `git diff --exit-code -- crates/cheaper-cli/tests/regression`: exit 0、
  capture 未設定で実行、golden 再生成なし。
- consumer tolerance のうち `usage_optional_ids_preserve_report` と
  `usage_report_history_independent` が green。前者は ID 追加前後の
  出力同一性（section emit の証明ではない）、後者は外部情報非参照の
  既存契約を pin。残り3件は T2/T3 の集計・emit 未実装で expected-red。
- `mise run check`・workspace 全体の test は未実行。全面 green は主張しない。
- `mise exec -- node .agents/skills/impl-doc/scripts/audit_impl_record.js
  --json`: exit 0、181 files / findings 0。
- `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type
  task --json`: exit 0、401 files / 143 non-blocking warnings /
  blocking 0、T1 自身の findings 0（他 task の reciprocal relation 警告）。
- `mise exec -- node scripts/check-doc-ids.mjs`: exit 0、1238 markdown files。
- `git diff --check` および Cargo manifest/lock の
  `git diff --quiet -- Cargo.toml Cargo.lock crates/cheaper-cli/Cargo.toml`:
  各 exit 0。

### Review checkpoint

独立 cooperative/adversarial review を実施。cooperative review は
NO CANDIDATES。adversarial review の候補を照合し、refetch join test で
`parentExecutionId` が owner の `executionId` と一致していて ID join の誤実装
でも通る点を確認。fixture を不一致値へ修正し、指定 test を再実行して
引き続き未実装 route aggregation の assertion で expected-red となることを確認。
`route_filter_scopes_route_sections_only` の route 名・全 route metric をより
強く区別する候補は P2 と判定。本 test の明示した route-scope/run 集計分離
契約を越えるため変更せず、T2 の scoped aggregation pins で扱う。

判定: ACCEPT WITH NON-BLOCKING NOTES。production seam は compile-only のまま、
T1 の所有範囲と既存 assertion/golden は維持。レビュー後に task/IR を完了化し、
論理コミットを作成する。

## Risks

31 件の red は意図した後続 task 所有。T2 が15件、T3 が9件
（一部T2前提）、T4 が7件。仕様を満たすまで tests を緩めないこと。
ID/store/writer の #192 acceptance をこの consumer tolerance で代替しない。

## Follow-ups

親: 独立レビュー、docs audit 証跡の確認、scope-specific staging と
`test(usage): ...` conventional commit、T1/IR/index の完了化。
その後に graph を再評価する。本委譲では T2 を開始しない。
