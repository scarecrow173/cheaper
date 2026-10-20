---
id: "IMPL-034JjvBItf3G44b0tll2Sh"
type: "impl"
status: "completed"
title: "Add hook route arm and Rewrite verdict"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "docs/tasks/add-hook-route-rewrite.md"
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/eval_cache.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_eval_dedup.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-hook-route-rewrite.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/add-route-resolver-and-payload.md"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add hook route arm and Rewrite verdict

## Summary

`Verdict::Rewrite{command, fallback}` を追加し、`evaluate_shell` 先頭へ
route arm を組み込んだ。runnable match した shell command は
`"<current_exe>" route run --payload <b64url>` 形の単一 segment command
へ rewrite され、fallback には従来 read-candidate 評価の生 verdict
(Allow/Deny)が保持される。non-runnable・no-match・route 未供給・複数/
unsafe segment は従来評価へ fallthrough する。

## Context

- implements: `docs/tasks/add-hook-route-rewrite.md`
- derives-from: `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
  (§hook eval への Rewrite arm — R24-R40)
- test spec: `docs/test-specs/hook-route-rewrite.md`

## Implementation

### Changes

- `routing_policy.rs`
  - `Verdict::Rewrite{command, fallback: Box<Verdict>}` を追加。internally
    tagged serde で `{"decision":"rewrite","command":…,"fallback":{…}}`。
  - `RouteEvalInput`(`routes` + `command_policy` 参照面)を新設し、
    `EvaluateOptions.route: Option<RouteEvalInput>` として eval へ供給する
    経路を確立(A#10 — eval 内で file I/O を分散させない)。`None` では
    route arm が不活性のまま。
  - `evaluate_shell` を route arm(`evaluate_route_arm`)と従来評価
    (`evaluate_read_candidates`)へ分解。arm は単一 safe segment のみ
    `route::resolve` を呼び、`matched && Runnable` のとき payload を
    encode して `Verdict::Rewrite` を返す。
  - `build_rewrite_command` — `current_exe()` を `"…"` quote して anchor
    (space 含み path でも単一 token)。`passthroughEnv` 名だけ ambient
    env を capture し `route::payload` で符号化。encode 失敗・`"` 含み
    exe path は `None`(fail-safe で従来評価へ)。
- `route/resolve.rs` — `RouteResolution.passthrough_env` を追加(match した
  route の allowlist を encode 入力として運ぶ。wire shape は `route_cmd`
  の手動 field pick で非破壊)。`CommandPolicyView` に `Copy` derive。
- `targets.rs` — 全 render arm に `Verdict::Rewrite{fallback} → render
  (*fallback)` の降格を追加(C6 の安全側既定。claude/devin の
  `updatedInput` render は C4 で導入)。`verdict_name` に `"rewrite"`。
  `HookDeps::eval_opts` は `route: None`(load 配線は C1)。
- `eval_cache.rs` — `is_shareable` に `Verdict::Rewrite{..} => false` を
  追加(C3 で確立した exhaustive match が compile error で Rewrite を
  強制したとおりに解消 — env 値を含み得る verdict は persist/serve
  しない)。

### Decisions During Implementation

- route arm の評価は `EvaluateOptions.route` の供給有無で gate する設計を
  採用 — C1(hook entry loading)が未着手の間も arm は構造的に不活性で、
  既存 eval の挙動を変えない。`HookDeps::eval_opts` は `route: None`。
- render 層の `Rewrite` 降格は「fallback verdict を render」に統一 — C4
  の per-target `updatedInput` render 導入までは全 target が安全側に
  倒れる(deny を素通しさせない C6 既定)。

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `cargo check -p cheaper-cli --all-targets`: pass
- `cargo test -p cheaper-cli --lib route_arm`: 4 pass —
  `rewrite_serde_round_trip` / `runnable_route_produces_rewrite` /
  `rewrite_preserves_fallback_deny` / `non_runnable_falls_through`
- `cargo test -p cheaper-cli --lib`: 60 pass
- `cargo test -p cheaper-cli --test integration`(dev-seam): 229 pass
- `cargo fmt -p cheaper-cli` / `cargo clippy -p cheaper-cli --all-targets`:
  clean

## Risks

- route arm は `EvaluateOptions.route` が `None` の間不活性 — C1 の
  hook entry loading が配線されるまで本番経路では rewrite が発行
  されない(意図どおりの段階導入)。
- `Rewrite` render は現在全 target で fallback render — claude/devin
  の `updatedInput` は C4 依存。

## Follow-ups

- C1(`add-hook-entry-route-loading`)で hook entry の routes/policy load
  を `HookDeps`/`EvaluateOptions.route` へ配線する。
- C4(`add-hook-rewrite-target-rendering`)で claude/devin の
  `updatedInput` render と `HookTarget::Codex` 分離を導入する。
