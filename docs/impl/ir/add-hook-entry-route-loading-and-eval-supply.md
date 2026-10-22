---
id: "IMPL-034JjGk8chOgCkQfzFETJY"
type: "impl"
status: "completed"
title: "Add hook entry route loading and eval supply"
created: "2026-10-20"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "docs/tasks/add-hook-entry-route-loading.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/supply.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-hook-entry-route-loading.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/add-hook-route-rewrite.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/discovery/devin-pretooluse-rewrite-runtime-verification.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add hook entry route loading and eval supply

## Summary

`route::supply` 共有 loader を新設し、hook entry(targets.rs)が
normalized request あたり 1 回だけ effective routes + command policy を
load して `EvaluateOptions.route`(`RouteEvalInput`)へ供給する配線を
確立した。`route_cmd` も同じ supply loader を使うよう refactor し、
route resolution の caller 側 load が単一経路に集約された。

## Context

- implements: `docs/tasks/add-hook-entry-route-loading.md`
- derives-from: `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
  (§hook eval の grants/routes 供給経路 — A#10 対応)
- depends-on: `add-hook-route-rewrite`(C2 — `RouteEvalInput` 供給面)

## Implementation

### Changes

- `route/supply.rs`(新設)
  - `RouteSupply` — canonical workspace root・builtin dir・config
    dirs・`EffectiveRoutes`・`RuntimePolicy`・`CommandPolicyView`・
    path denylist を所有する供給コンテナ。
  - `load(cwd, env, builtin_dir) -> Result<RouteSupply, CheaperError>`
    — CLI 経路(`route resolve`/将来の `route run`)の strict 版。
  - `load_or_none(...) -> Option<RouteSupply>` — hook 経路の
    fail-open 版。routes/policy load 失敗は `None` に写し、route
    arm 不活性(従来 read-candidate 評価へ継続)として hook 自体を
    block しない。
  - `eval_input() -> RouteEvalInput` — eval 供給への projection。
- `route/mod.rs` — `pub mod supply` を export。
- `targets.rs`
  - `request_cwd(&RoutingRequest)` — payload の cwd 抽出を集約。
  - `evaluate_with_routes(&RoutingRequest, &HookDeps)` — firing ごとに
    `load_or_none` を 1 回呼び、`opts.route =
    supply.as_ref().map(|s| s.eval_input())` して
    `eval_cache::evaluate_dedup` へ供給。5 箇所の target evaluation
    call site(windsurf/copilot/pretooluse/devin/cursor)を全てこの
    helper へ置換 — per-candidate reload を構造的に排除。
- `route_cmd.rs` — 独自の workspace layout / route load / policy load を
  `supply::load` へ置換し重複を解消。`supply.command_view()` と
  `supply.routes` を `resolve` へ渡す。
- `tests/integration/hook.rs`
  - `windsurf_shell()` payload helper を抽出(既存 inline 組立を共通化)。
  - `eval_store_dir()` / `eval_records()` — dedup store 観測 helper。
  - `routed_shell_fixture()` — user 層 routes.yaml + grant policy +
    workflow file を持つ route fixture。
  - `shell_route_arm_engages_via_firing_load` — C1 contract test。
    routed `cat big.txt` が `Rewrite{fallback: Deny}` を経由し
    windsurf で exit 2 deny 応答を返すこと、および Rewrite 非 persist
    により dedup record が残らないことを観測。control として
    routes 無し fixture の同一 firing が Deny record を publish
    することを対照検証。

### Decisions During Implementation

- load は `evaluate_with_routes` の 1 箇所へ集約し、`HandleRequest` の
  target 分岐(hook firing あたり 1 分岐)より前ではなく後に配置 —
  分岐ごとの helper 呼出は発生しないため実効的に 1 firing 1 load を
  保証しつつ、route 非対象 payload(read 等)で supply load を
  実行しない lazy さを維持する。
- hook 側は `load_or_none`(fail-open)を採用 — route/policy の
  corrupt や I/O 失敗で親 agent の command を block しないという
  hook 自体の契約を守りつつ、route arm だけを不活性化する。
- `route_cmd` を同じ supply へ統合したのは DRY — builtin/user/project
  の 3 層解決 + policy load の規約を caller ごとに複写させない。

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration -- shell_`:
  3 pass — `shell_route_arm_engages_via_firing_load` 含む
- `cargo test -p cheaper-cli --test integration -- hook:: route::`:
  30 pass(hook + route regression)
- `cargo test -p cheaper-cli --lib`: 60 pass
- `cargo fmt -p cheaper-cli`: clean
- `cargo clippy -p cheaper-cli --all-targets`(dev-seam): clean

## Risks

- `load_or_none` は routes/policy どちらの失敗でも `None` になるため、
  供給失敗時は hook が route arm 不活性で静かに従来評価へ戻る —
  診断面の signal は route log(B3)側で観測する前提。
- `evaluate_with_routes` は target 分岐内で firing ごとに呼ばれる —
  将来 1 firing で複数 target を逐次評価する形が増えた場合は
  load の単位を再確認する必要がある(現行 spec では 1 target/1
  firing)。

## Follow-ups

- C4(`add-hook-rewrite-target-rendering`)で claude/devin の
  `updatedInput` render と `HookTarget::Codex` 分離を導入する。
- D1(`add-route-run-delegated-exec`)で `route run` が同じ
  `supply::load` を使うことを確認する。
