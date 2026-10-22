---
id: "TASK-034KsKPPDTLAGrCkK8k17Y"
type: "task"
status: "done"
title: "Add codex flag-gated approval-bypass rewrite emit"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  implemented-by:
    - "docs/impl/ir/implement-per-target-hook-rewrite-enablement-and-approval-bypass-issue-150.md"
  depends-on:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md"
  blocks:
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/codex-hook-rewrite-emit.md"
  references:
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
  defers: []
  deferred-by: []
---

# Add codex flag-gated approval-bypass rewrite emit

## Work

plan `docs/plans/implement-per-target-hook-rewrite-enablement.md`
(PLAN-034KsKo6nYOBOpIpkNoviY)の T2。`targets.rs` の render 層に
`route.rewrite.approvalBypass` flag の供給経路と codex bypassing
emit 分岐を追加する。**`Verdict` enum・`RouteEvalInput`/
`EvaluateOptions`・`evaluate_route_arm`・dedup 規約は不変** —
flag は render 出力のみに効く(R14)。

### 1. flag 供給経路 — `evaluate_with_routes` 戻り値拡張

- `evaluate_with_routes`(:709)の戻り値を `Verdict` から
  `(Verdict, Option<RouteSupply>)` へ拡張し、load 済み
  `RouteSupply` を呼び出し側へ返す(load は firing あたり 1 回の
  既存経路のまま — `opts.route` 設定・eval は不変、戻り値に
  `supply` を載せるだけ)。
- `handle_hook`(:720)の各 arm でタプルを受け、**codex arm のみ**
  `bypass = supply.as_ref().map_or(false, |s|
  s.policy.route.rewrite.approval_bypass_enabled("codex"))` で flag
  を読み `codex_render(verdict, bypass)` へ渡す。`load_or_none`
  → `None` は flag 未供給 = bypass 無効(conservative default、
  R15)。他 arm(pretooluse/devin/windsurf/cursor/copilot)は supply
  を読まない。
- codex arm の `cheaper.hook` 完了 debug event(:778-783
  `hook response`)に `bypass: bool` field を追加 —
  approval-bypass emit は silent exit 0 で stderr 痕跡を持たない
  ため debug event が唯一の観測面。field は codex arm でのみ
  `true` になり得る(他 arm は `false` を emit)。

### 2. bypassing emit — `hook_specific_output_rewrite` + `codex_render`

- `hook_specific_output_rewrite`(:538)に `approval_bypass: bool`
  引数を追加 — `true` のとき `hookEventName:"PreToolUse"` と
  `updatedInput` の間に `permissionDecision:"allow"` を挿入する
  (field 挿入順は probe 観測順、`stringify_ordered` byte 互換規約)。
- `pretooluse_render`(:560)・`devin_render`(:624)は
  `hook_specific_output_rewrite(&command, false)` に揃える —
  emit byte は現行と完全一致(`permissionDecision` 不在、共有
  emit 点を 1 箇所に保つ)。
- `codex_render`(:572)に `approval_bypass: bool` 引数を追加し
  design の形に分岐:

  ```rust
  fn codex_render(verdict: Verdict, approval_bypass: bool) -> HookResponse {
      match verdict {
          Verdict::Deny { reason } => hook_specific_output_deny(&reason),
          Verdict::Allow => silent_allow(),
          Verdict::Rewrite { command, .. } if approval_bypass =>
              hook_specific_output_rewrite(&command, true),
          Verdict::Rewrite { fallback, .. } =>
              codex_render(*fallback, approval_bypass),
      }
  }
  ```

- `updatedInput` 単独・`permissionDecision:"ask"` 併記の emit は
  **いかなる flag 状態でも発行しない**(codex が両者を無視する
  silent no-op、実機確認済み — R7/D2)。emit shape は上記の
  `allow` 併記形のみ。

### 3. unit test(targets.rs tests)

- `codex_render(rewrite, true)` が `hookSpecificOutput` 内に
  `hookEventName:"PreToolUse"` + `permissionDecision:"allow"` +
  `updatedInput.command` を emit し exit 0 — field 集合を byte
  レベルに近い精度で pin。
- `codex_render(rewrite, false)` は従来
  `codex_rewrite_renders_fallback_only`(:871)の assert を維持
  (signature 変更のみ)。
- 負 shape test: `updatedInput` を `permissionDecision` 無し・
  `"ask"` 併記で emit する経路が存在しないことを固定
  (`codex_render` の全 verdict 分岐で `permissionDecision:"ask"`
  が出ないこと、flag 無しでは `updatedInput` が出ないこと)。
- pretooluse/devin の emit が flag 非対称で不変 —
  `hook_specific_output_rewrite(…, false)` の出力が
  `permissionDecision` を含まず現行 byte と同一。
- cursor/copilot render が Rewrite verdict に対し引き続き
  `*fallback` を render する(flag 有無にかかわらず — emit gate、
  R10/R12)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-per-target-hook-rewrite-enablement.md` — 本
  task が implement する plan(File Map・Dependency Graph の T2)
- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1)— `codex_render` signature・
  emit field 順序・debug field・供給経路の AUTHORITATIVE 定義
- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy)— R6-R8・R14-R17
- `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
  (ADR-034KrYGRHZuvOIx1d0S0nE)— D2・D5 の根拠

## Dependencies

- `docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md`
  (T1) — `RuntimePolicy.route`・`approval_bypass_enabled` が flag
  の供給元。

## Affected Files

- `crates/cheaper-cli/src/hook/targets.rs`(modify —
  `evaluate_with_routes`・`handle_hook`・`hook_specific_output_rewrite`・
  `codex_render`・`pretooluse_render`・`devin_render`・debug event)

## Done When

- [ ] `evaluate_with_routes` が `(Verdict, Option<RouteSupply>)` を
      返し、`handle_hook` の codex arm が
      `approval_bypass_enabled("codex")` で flag を読む
- [ ] flag 有りの codex Rewrite が
      `permissionDecision:"allow"` + `updatedInput.command` を
      `hookSpecificOutput` 内へ emit し exit 0
- [ ] flag 無しの codex Rewrite が従来どおり fallback verdict を
      render し `updatedInput` を emit しない
- [ ] `updatedInput` 単独・`"ask"` 併記の emit がいかなる flag
      状態でも発行されない
- [ ] pretooluse/devin の emit byte が不変(`permissionDecision`
      不在)
- [ ] codex `hook response` debug event に `bypass` field が載る
- [ ] `Verdict`/`evaluate_route_arm`/`RouteEvalInput`/dedup に
      差分が無い(render 層のみ)

## Out of Scope

- `route` section schema・`RoutePolicy`/`RewritePolicy`(T1)
- cursor hooks.json・`cursor_normalize`(T3 — 本 task は
  `cursor_normalize` に触れない)
- cursor/copilot の rewrite emit 本体(実機検証 task — flag の有無に
  かかわらず emit しない構造のみ)
- integration test(T4)
- `route/supply.rs`・`hook/mod.rs`(変更不要)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib` exits 0(targets.rs unit test 全 pass)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0(既存 hook contract の非回帰)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0

## Done

2026-10-23 実施。render 層に flag 供給経路と codex bypassing emit
分岐を追加した(`Verdict`/`RouteEvalInput`/`evaluate_route_arm`/
dedup は不変)。

- `evaluate_with_routes` が `(Verdict, Option<RouteSupply>)` を返す
  — `load_or_none` の load 済み supply を呼び出し側へそのまま載せる
  (`opts.route` 設定・eval は不変)。
- `handle_hook` の codex arm のみ
  `supply.policy.route.rewrite.approval_bypass_enabled("codex")` で
  flag を読み `codex_render(verdict, bypass)` へ渡す。`None` →
  `false`(conservative default)。他 arm は supply を読まない。
- `hook_specific_output_rewrite(command, approval_bypass)` —
  `true` のとき `hookEventName` と `updatedInput` の間に
  `permissionDecision:"allow"` を挿入(probe 観測順)。
  `pretooluse_render`/`devin_render` は `false` に揃え emit byte
  不変。
- `codex_render(verdict, approval_bypass)` — `Rewrite if
  approval_bypass` → `hook_specific_output_rewrite(&command, true)`、
  それ以外は fallback 再帰 render。`updatedInput` 単独・`"ask"`
  併記はいかなる flag 状態でも発行しない。
- `hook response` debug event に `bypass: bool` field を追加
  (全 arm emit、codex flag-on のみ `true` になり得る)。
- unit test — `codex_rewrite_with_approval_bypass_emits_allow_and_
  updated_input`(field 順 byte pin)、`codex_rewrite_never_emits_
  unapplied_shapes`(負 shape)、既存
  `codex_rewrite_renders_fallback_only`(signature 追随)等。
- 検証: `cargo test -p cheaper-cli --features cheaper-cli/dev-seam
  --lib` 89 pass、`--test integration hook::` 26 pass、`cargo
  clippy -p cheaper-cli --all-targets --features
  cheaper-cli/dev-seam -- -D warnings` exits 0、`cargo fmt`
  適用済み。
