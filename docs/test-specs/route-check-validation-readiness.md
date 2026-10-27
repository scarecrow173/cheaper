---
id: "TSPEC-034N7wJlXboc0jNQpjf27C"
type: "test-spec"
status: "proposed"
title: "Route check validation readiness"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check validation readiness

## Purpose

workflow・worker・schema・grant・pin・renderer の readiness 判定が、実行なしに shared validation contract と policy 依存性の区別を保って報告されることを保証する。validation/readiness 規約が変更された場合にのみ廃止する。

## Feature

Feature: `route check` は実行せずに静的 readiness を評価し、policy 依存の項目を `unknown`/`unverified`、環境依存の項目を `environment-dependent` として報告する。

## Rules

- worker 定義本体と `schema: {file: ...}` の validity は policy 非依存であり、policy が無くても評価・報告する。
- tool capability、workflow の `validate_definition`、grant eligibility・`pin_exec_grant`・`check_exec_execution`、viability guard、codex `route.rewrite.approvalBypass` の enabled 判定は policy-dependent であり、policy 不在時は `unknown`/`unverified` とする。
- workflow validation は route の宣言 layer から document を取得して shared `validate_definition`・`validate_routed_convention` を通し、policy-guarded fs read と compiler catalog は policy がある場合にのみ実行する。policy 不在時は workflow validation status を `unknown` にする。
- worker/schema lookup は project → user → builtin precedence を同じ source-aware resolver で維持し、embedded builtin candidate も同一 validation pipeline に渡す。`WorkerNotFound` は compile port の error ではなく route diagnostic として報告する。
- grant/pin 判定は `route.grant`・`grants`・`substitute.grant` の有効投影 set を対象に既存 predicate を使い、diagnostics 専用 predicate を複製しない。
- executable readiness は `PATH` 探索と metadata read を許可するが spawn せず、ambient `PATH` に依存する結果は `environment-dependent` と明示する。ambient `PATH` で解決した pin は spawn success を意味しない。
- renderer capability は `hook::targets` が所有する静的 descriptor から `HookTarget::ALL` について報告し、host registration/approval/execution は常に `unverified` とする。

## Examples

- Given policy が load できない構成と invalid な worker 定義、when readiness を評価すると、worker validity は policy 非依存として報告され、tool capability のみ `unknown` になる。
- Given builtin `cargo-test` route の no-grant 状態、when readiness を評価すると、grant 欠如が `route resolve` と同じ policy 結果で報告される。
- Given grant-enabled の `cargo-test` fixture、when readiness を評価すると、`route resolve` と同じ grant 判定になる。
- Given ambient `PATH` で解決不能な executable、when pin/readiness を評価すると、spawn せず `environment-dependent` な未解決として報告される。
- Given `builtin_dir: None` の production path、when builtin worker と `{file:}` schema を評価すると、embedded catalog 経由で同一 validation pipeline に渡る。
- Given codex target と policy 不在の構成、when renderer を評価すると、静的 rewrite capability は報告され approval-bypass のみ `unknown`、host registration/approval/execution は `unverified` のままになる。

## Guarantees

- policy 非依存の定義妥当性と policy 依存の capability/grant/pin 判定が分離され、policy 不在で後者が `unknown`/`unverified` になることを固定する。
- builtin `cargo-test` の no-grant/grant-enabled が `route resolve` と parity であることを固定する。
- worker/schema/workflow 判定が shared validation を通り、diagnostics 専用 predicate の複製がないことを固定する。
- executable readiness が spawn せず `environment-dependent` 表示を伴うことを固定する。

## Non-goals

- spawn・network・auth の否定 probe 自体は invocation-boundary spec が担う。
- sample 由来の viability 優先順と matched/selected 意味は sample-resolution spec が担う。
- `unverified`/`environment-dependent` の envelope・exit severity への写像は output contract spec が担う。

## Risk

これらの保証が欠けると、policy 不在時に readiness が過剰確定または過剰 unknown となり、diagnostics が runtime の grant/viability 判定と drift し、静的 capability と未検証の host 状態が混同される。
