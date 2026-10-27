---
id: "TASK-034N7D9MYpblZB6oi3FOuw"
type: "task"
status: "done"
title: "Add embedded route check source supply"
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
    - "docs/test-specs/route-check-layer-and-policy-diagnostics.md"
  references: []
  defers: []
  deferred-by: []
---

# Add embedded route check source supply

## Work

approved plan の step 3 と step 5 に対応する implementation slice。embedded builtin resource の read-only accessor、`RouteLayerSource` abstraction、check 専用 supply builder、policy の document 入力 API を実装する。

- `embedded.rs` に `EmbeddedResources` accessor を追加する。synthetic root `/builtin/` を唯一の builtin logical root とし、`/` 正規化された logical relative key（例: `routes.yaml`、`policy.yaml`、`workflows/**`、`workers/**`、`workers/schemas/**`）に対して `read`/`contains` で byte/text document を返す。`builtin_dir()`/`materialize_builtin_dir()` は呼ばず、`cheaper-builtin-*` temp dir を一切作らない。
- `route/supply.rs` に `RouteLayerSource`（`Filesystem { routes_file, workflow_root, worker_base_dir }` / `Embedded { catalog }`）と check 専用 supply builder を追加する。production builtin は `Embedded`、dev/test で `CliHooks.builtin_dir` 指定時のみ `Filesystem`、user/project は常に `Filesystem` とする。`worker_base_dir` は layer base dir とし、resolver が `workers/<name>.md` を join する既存規約を維持する。
- `cheaper-core/src/policy/load.rs` に document 入力 API（`load_policy_documents` 相当）を追加し、builtin/user/project の policy document を filesystem load と同一の validation/merge/normalize pipeline に通す。`RoutePolicyReport.layers` が per-layer `absent|loaded|failed` を保持できるよう、per-layer 失敗と post-merge validation/default-resolution/denylist 失敗を区別して報告できる error shape にする。
- 実行経路の `RouteSupply::load` は変更しない。本 slice は strictly read-only で、embedded resource の temp materialization・filesystem write を含め一切の副作用を持たない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- なし（plan の dependency graph 上、本 slice は invocation detection/gating slice と並行して進められる）。

## Affected Files

- `crates/cheaper-cli/src/embedded.rs` — `EmbeddedResources` read-only accessor（`/builtin/` logical root、`/` 正規化 key）
- `crates/cheaper-cli/src/route/supply.rs` — `RouteLayerSource` と check 専用 supply builder
- `crates/cheaper-core/src/policy/load.rs` — `load_policy_documents` 相当の document 入力 API

## Done When

- [ ] pre-existing builtin cache が無い clean-temp 環境で、embedded catalog から builtin `routes.yaml`/`policy.yaml`/workflow/worker/`{file:}` schema が読め、`cheaper-builtin-*` temp dir が生成されない。
- [ ] `EmbeddedResources` の key が `/` 正規化された logical relative path として扱われ、`read`/`contains` が `builtin_dir()`/`materialize_builtin_dir()` を呼ばない。
- [ ] check 専用 supply builder が `CliHooks.builtin_dir: None` の production path で builtin を `Embedded`、user/project を `Filesystem` として構築し、`worker_base_dir` の join 規約を維持する。
- [ ] `load_policy_documents` 相当の API が builtin/user/project document を同一 validation/merge/normalize pipeline に通し、`RoutePolicyReport.layers` の per-layer `absent|loaded|failed` と pseudo-layer `merged`/`policy:effective` attribution を区別できる。
- [ ] 実行経路の `RouteSupply::load` と既存 caller の挙動が変わらない。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam embedded::` — `EmbeddedResources` の key 正規化・`read`/`contains`・no-materialization が pass する。
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — clean-temp `builtin_dir: None` production-path fixture で builtin diagnostics が embedded catalog 経由で成立し temp dir を作らないこと、policy document input API が per-layer `absent|loaded|failed` を保持することを確認する。
