---
id: "IMPL-034NbOZbe7YzlSA1aVfJ8p"
type: "impl"
status: "completed"
title: "Share route sample matching and renderer capability"
created: "2026-10-27"
updated: "2026-10-27"
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
    - "docs/tasks/share-route-sample-matching-and-renderer-capability.md"
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

# Share route sample matching and renderer capability

## Summary

`route resolve` の match phase を diagnostics から再利用できる形へ分離し、`missing-target` 判定だけを注入可能にする。併せて `hook::targets` が所有する静的 renderer capability descriptor を追加する。

## Context

`TASK-034N7Db7xm6kehiQl3tj8P`。approved plan の step 7・step 8、design §sample/renderer capability、および `route-check-sample-resolution-semantics` / `route-check-validation-readiness` test spec に従う。最終 report orchestration は後続 task に委ねる。

## Implementation

### Changes

- `route/resolve.rs` に `match_command` を追加し、env-prefix strip・self-bypass・prefix match・longest-prefix winner・tie 判定・suppressed attribution を shared phase として返すようにした。normalized argv・executable、effective route ごとの `matched`/`selected` row、selected route、tied、suppressed name を `RouteCommandMatch` で提供する。
- `RouteMergeRow::command()` を追加し、effective row だけでなく invalid/suppressed/shadowed row も宣言 `match.command` を shared predicate へ渡せるようにした。
- `resolve_with_target_exists` を追加し、`missing-target` 判定だけを `Fn(&Route) -> bool` で注入可能にした。既存 `resolve()` は `route.target_path.is_file()` を使う compatibility wrapper として残し、`route run` も同じ filesystem predicate を明示的に渡す。
- `route run` は longest-match の再実装をやめ、`RouteResolutionWithMatch.matched.selected` を `resolution.route` と照合して実行 route を取得する。既存の no-match、route-mismatch、grant projection、environment allowlist の guard は変更していない。
- `RouteLayerSource::target_exists` と `RouteCheckSources::target_exists` を追加した。filesystem layer は `Path::is_file`、embedded builtin layer は `EmbeddedResources::contains` で logical `/builtin/` resource を判定し、temp materialization は行わない。
- `hook/targets.rs` に `TargetCapability`、`RewriteCapability`、`ApprovalBypassGate`、`ApprovalBypassStatus`、`HostVerification` を追加した。`TargetCapability::all()` は `HookTarget::ALL` 順に descriptor を返し、host registration/approval/execution は常に `unverified`、codex の approval bypass は `RewritePolicy` 注入で `unknown`/`enabled`/`disabled` を返す。
- `embedded::builtin_dir_call_count` の test 観測点を global atomic から thread-local counter へ変更した。unit test は同一 process 内で並列実行されるため、counter を同一 test thread 内の呼出しだけに限定して race を除去した。

### Decisions During Implementation

- `ResolveInput` 自体へ必須 field を増やすと全 caller/test を触るため、既存 `resolve()` は filesystem default の薄い wrapper、diagnostics 向けに `resolve_with_target_exists()` を提供する。
- `target_exists` は `&Path` ではなく `&Route` を受ける関数とし、route.layer に応じて filesystem/embedded source を切り替えられるようにした。
- `RouteCommandMatch.rows` は effective route の row-level sample 状態を保持し、non-effective row の declaration match は `matched_command(&row.command())` で同じ normalized argv/self-bypass 規約を再利用する。
- `selected` は merge row ごとに複製される `Route` clone に依存せず、`is_selected` が name/layer/command の identity で比較する。同一 layer duplicate は既存 schema が `Invalid` row へ quarantine するため effective route 内では一意である。
- codex renderer は approval-bypass flag が無い場合 fallback を返すが、route check の静的 rewrite capability は canonical codex renderer が emit 可能という意味で `supported` とする。policy-dependent な状態は `ApprovalBypassStatus` 側に分離した。
- report 本体未実装のため、後続 task の consumer になる型・method には targeted `#[allow(dead_code)]` を付けた。public API と既存 runtime 経路の挙動は変えていない。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `cargo fmt --all` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route:: --lib` — 66 tests pass。shared match の row-level state・declaration predicate、env-prefix strip、self-bypass、longest/tie、suppressed attribution、`target_exists` 注入、embedded source-aware viability、route run guard の回帰を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam embedded:: --lib` — 6 tests pass。logical catalog access が builtin materialization を呼ばないことを、thread-local call counter で同一 test thread 内で確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam hook::targets --lib` — 16 tests pass。capability descriptor と canonical rewrite render の一致、stable `as_str` vocabulary、host `unverified` 固定、codex approval-bypass の `unknown`/`enabled`/`disabled`/`not-applicable` を確認。
- `mise exec -- cargo clippy -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --all-targets -- -D warnings` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 1 test pass。production embedded source が `cheaper-builtin-*` temp cache を materialize しないことを確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` — 35 tests pass。shared match 移行後も payload/env-prefix/grant/stdin/no-match/no-grant/rewrite/run logging 経路が変わらないことを確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route::` — 10 tests pass。既存 `route resolve` の env-prefix、no-match、no-grant、substitute、member grants の公開契約が変わらないことを確認。
- `git diff --check` — pass。

## Risks

- `route check` の report orchestration は未実装のため、valid invocation は依然として layer evaluation 後の sanitized `INTERNAL_ERROR` を返す。
- shared match/capability seam は後続 report task が consumer のため、現時点では一部型・method が `#[allow(dead_code)]` を伴う staged seam である。
- `RouteCommandMatch::is_selected` は name/layer/command identity を使う。same-layer duplicate は schema 側で `Invalid` row になる前提であり、report task は invalid row を effective row へ混ぜない必要がある。
- renderer capability は static descriptor であり、host registration/approval/execution は意図的に `unverified` のままである。issue #162 の実機検証は本 task の範囲外。

## Follow-ups

- `implement-route-check-diagnostics-report` task で `RouteCommandMatch`、`RouteMergeRow::command()`、`RouteCheckSources::target_exists`、`TargetCapability` を report row/readiness/capability projection へ接続する。
- policy load failure 時の `unknown`/`unverified` 投影、stable diagnostic codes、deterministic JSON/text output、exit class 判定は report task で実装する。
