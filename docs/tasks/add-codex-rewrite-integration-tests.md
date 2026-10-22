---
id: "TASK-034KsKXLwLr4ZYjemKg6Hr"
type: "task"
status: "done"
title: "Add codex rewrite emit and project-layer flag rejection integration tests"
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
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
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
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
  blocks:
    - "docs/tasks/document-approval-bypass-policy-flag.md"
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
    - "docs/test-specs/route-rewrite-approval-bypass-policy-flag.md"
    - "docs/test-specs/codex-hook-rewrite-emit.md"
  references:
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "crates/cheaper-cli/tests/integration/route_run.rs"
  defers: []
  deferred-by: []
---

# Add codex rewrite emit and project-layer flag rejection integration tests

## Work

plan `docs/plans/implement-per-target-hook-rewrite-enablement.md`
(PLAN-034KsKo6nYOBOpIpkNoviY)の T4。`route_run.rs` に codex の
flag-gated rewrite emit・flag 不在 fallback・project 層 `route` key
fail-closed の end-to-end contract test を追加する。precedent は
`devin_hook_rewrite_emits_nested_updated_input`(:1519)と
`reject_fx`/`ROUTES_YAML` fixture(ユーザー層 `policy.yaml` +
`routes.yaml` + grant を `f.cfg` へ書く構造)。

### 1. flag 有り — codex bypassing emit

- user policy `route.rewrite.approvalBypass: [codex]` + `child-run`
  grant で `--target codex` の routed shell firing
  (`{"tool_name":"Bash","tool_input":{"command":"routed ..."}}`
  payload、`pretooluse_normalize` 経路)が exit 0 で
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"allow","updatedInput":{"command":...}}}`
  を emit する(R6)。
- assert: `hook.pointer("/hookSpecificOutput/permissionDecision")
  == "allow"`、`/hookSpecificOutput/hookEventName == "PreToolUse"`、
  `/hookSpecificOutput/updatedInput/command` が
  ` route run --payload ` を含む、top-level `decision`/
  `updatedInput` は無い、marker file は実行されない
  (`devin_hook_rewrite` の assert 形に揃える)。
- fixture 構成は `reject_fx` の policy.yaml 生成に
  `route:\n  rewrite:\n    approvalBypass: [codex]\n` を加える形
  (新 helper 化するか `grant_yaml_tail` 同様の追記引数で —
  実装時に最小の形を選ぶ)。

### 2. flag 無し — fallback render の維持

- 同一 firing で flag 未設定(および `approvalBypass: []`・
  `[cursor]` 等の codex 非含有ケース)のとき、codex render は
  fallback verdict のみを返し `hookSpecificOutput` に
  `updatedInput` を含まない(R7)。`builtin_cargo_test_route_
  registered_but_no_grant`(:1570)の「no-grant は rewrite しない」
  と対になる「flag 無しは rewrite しない」pin。

### 3. project 層 `route` key — fail-closed

- workspace `.cheaper/policy.yaml`(`f.ws` 側)に `route` key を
  持たせると project partial schema 違反で `load_policy` 全体が
  失敗 → `load_or_none` が `None` → route arm 不活性 + bypass
  無効となり、user 層 policy に flag があっても hook は rewrite
  を発行しない(R3/R15 — untrusted project が approval bypass を
  有効化できない構造保証の end-to-end 側)。
- assert: hook は exit 0 で `updatedInput` を emit しない
  (fallback render または従来評価へ fallthrough — load 失敗は
  hook を落とさない fail-open 規約)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-per-target-hook-rewrite-enablement.md` — 本
  task が implement する plan(File Map・Dependency Graph の T4)
- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1)— §Verification Notes の
  integration test 列挙
- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy)— AC「flag に codex がある場合の
  emit」「flag 未設定・空・codex 非含有での fallback」「project
  層 `route` key で load error」「route supply load 失敗で bypass
  無効」

## Dependencies

- `docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md`
  (T1) — flag の schema/load/normalize。
- `docs/tasks/add-codex-flag-gated-rewrite-emit.md`(T2) — codex
  bypassing emit 本体。本 task はその end-to-end pin。

## Affected Files

- `crates/cheaper-cli/tests/integration/route_run.rs`(modify —
  test case 追加。`tests/common/mod.rs` の Fixture は既存のまま
  利用)

## Done When

- [x] user flag + grant で `--target codex` firing が
      `permissionDecision:"allow"` + `updatedInput.command`
      (`route run --payload` 形)を emit し exit 0 の test が
      ある(R6)
- [x] flag 未設定・空・codex 非含有で `updatedInput` を emit
      しない test がある(R7)
- [x] project `.cheaper/policy.yaml` の `route` key で hook が
      rewrite を発行しない test がある(fail-closed、R3/R15)
- [x] 既存 `devin_hook_rewrite_*`・`builtin_cargo_test_*` test が
      非回帰

## Out of Scope

- 実装本体(T1・T2 — 本 task は test のみ)
- cursor `preToolUse` 経路の integration(T3 の unit test で
  カバー。cursor hook firing の e2e は emit contract 確定後)
- cursor/copilot の emit(実機検証 task)
- `hook response` debug event の e2e(T2 unit test で field 存在を
  pin — debug event の出力面 assert は observability 系 test の
  責務)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_run::` exits 0(新規 3 ケース + 既存 route_run contract 全 pass)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0
- [x] `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0(T1-T3 成果を含む全 contract — fixture regen 込み)

## Done

2026-10-23 実施。`route_run.rs` に codex flag-gated rewrite emit の
end-to-end contract test 3 件を追加した。fixture は
`reject_fx`(ROUTES_YAML + `child-run` grant)を使い、user 層
`policy.yaml` を `route:` section 付きで上書きする最小構成
(`devin_hook_rewrite_emits_nested_updated_input` 先例と同形)。

- `codex_hook_rewrite_with_approval_bypass_emits_allow_updated_input`
  — user policy `route.rewrite.approvalBypass: [codex]` + grant で
  `--target codex` の `Bash`/`tool_input.command` firing が exit 0
  + `hookSpecificOutput.{hookEventName:"PreToolUse",
  permissionDecision:"allow",updatedInput.command}`(" route run
  --payload " 含有)を emit。top-level `decision`/`updatedInput`
  無し・marker 未実行を assert(R6)。
- `codex_hook_rewrite_without_flag_renders_fallback_only` — flag
  未設定・`[]`・`[cursor]` の 3 subcase で `updatedInput` も
  `"permissionDecision":"allow"` も emit しない(fallback render
  維持、R7)。
- `codex_hook_project_route_key_fails_closed` — user 層に flag +
  grant を正しく置いた上で `.cheaper/policy.yaml` に `route` key
  を置くと project partial schema 違反で load 全体が失敗し、
  `load_or_none` → `None` で route arm 不活性 + bypass 無効。
  hook は exit 0 を維持しつつ rewrite を発行しない(R3/R15 の
  e2e 側)。
- 検証: `route_run::` 28 pass / `hook::` 26 pass / workspace 全
  50 suite pass・0 failed。既存 `devin_hook_rewrite_*`・
  `builtin_cargo_test_*` 非回帰。
