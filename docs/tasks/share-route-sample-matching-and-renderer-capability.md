---
id: "TASK-034N7Db7xm6kehiQl3tj8P"
type: "task"
status: "done"
title: "Share route sample matching and renderer capability"
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
  implements:
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
    - "docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md"
    - "docs/tasks/add-embedded-route-check-source-supply.md"
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
    - "docs/test-specs/route-check-sample-resolution-semantics.md"
    - "docs/test-specs/route-check-validation-readiness.md"
  references: []
  defers: []
  deferred-by: []
---

# Share route sample matching and renderer capability

## Work

approved plan の step 7 と step 8 に対応する implementation slice。`route resolve` の match/viability 規約を shared phase として分離し、`target_exists` predicate 注入と renderer の静的 capability descriptor を実装する。

- `route/resolve.rs` から env-prefix strip・self-bypass・prefix match・longest/tie・suppressed attribution を `match_command` 相当の shared phase へ分離する。`matched`（宣言 `match.command` の prefix match）と `selected`（effective route の longest-prefix winner）を row 単位で返せる shape にし、suppressed attribution は `RouteResolution.suppressed` と同じ route-name 値を再現する。
- viability 入力へ `target_exists` predicate を追加する。既定は `Path::is_file`、route check は layer source 解決 predicate（embedded builtin worker 等を catalog 経由で判定）を渡す。viability 優先順 unsafe-command → invalid-route → missing-target → no-grant → stdin-unsupported → runnable と `missing-target` の位置は変更しない。
- `hook/targets.rs` に `HookTarget::ALL` の静的 rewrite capability descriptor を追加し、codex の policy-dependent `route.rewrite.approvalBypass` 判定と静的 rewrite support を分離する。descriptor は render contract と同じ owner（`hook::targets`）に置き、host registration/approval/execution は常に `unverified` とする。
- 既存 caller は `Path::is_file`/filesystem default を維持する compatibility wrapper として残し、`route resolve`/`route run`/hook evaluation の挙動を変えない。diagnostics 専用 predicate の複製は作らない。本 slice は strictly read-only とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- `docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md`
- `docs/tasks/add-embedded-route-check-source-supply.md`（`target_exists` が layer source を解決するため）

## Affected Files

- `crates/cheaper-cli/src/route/resolve.rs` — `match_command` phase 分離と `target_exists` predicate 注入
- `crates/cheaper-cli/src/hook/targets.rs` — `HookTarget::ALL` の静的 rewrite capability descriptor、codex approval-bypass 判定 seam
- 既存 caller — filesystem/`Path::is_file` default を維持する compatibility wrapper のみ

## Done When

- [x] shared `match_command` phase が env-prefix strip・self-bypass・prefix match・longest/tie・suppressed attribution を `route resolve` と同一規約で返し、`matched`/`selected` を row 単位で提供できる。
- [x] `target_exists` predicate が注入可能で、既定 `Path::is_file`、route check は embedded catalog 対応の layer source predicate を渡せる。
- [x] viability 優先順と `missing-target` の位置が変わらず、`route resolve` との parity が保たれる。
- [x] `HookTarget::ALL` の静的 rewrite capability descriptor が canonical rewrite render と一致し、host registration/approval/execution が常に `unverified`、policy 不在の codex approval-bypass が `unknown` として扱える。
- [x] 既存 caller の挙動が変わらない。

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route:: --lib` — `match_command` parity・`target_exists` 注入・viability 優先順・route run guard の unit coverage が pass する。
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam hook::targets --lib` — capability descriptor と canonical rewrite render の一致、`unverified` 固定が pass する。
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — production embedded source が builtin temp cache を materialize しないことが pass する。
