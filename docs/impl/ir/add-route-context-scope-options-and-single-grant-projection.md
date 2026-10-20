---
id: "IMPL-034JiCcdU6kUQ9Hj4Sd44J"
type: "impl"
status: "completed"
title: "Add route context scope options and single-grant projection"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-context-scope-options.md"
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

# Add route context scope options and single-grant projection

## Summary

`RouteCommandContext`(`extra_env`/`exec_cwd`/
`accept_any_exit_code`/`log_tee`)を scope option として
`ToolRuntimeOptions` → registry `ToolCtx` → `ctx.command()` →
`CommandToolOptions` へ通し、command tool が route 注入 context
で cwd・env・exit 受理を切替える。さらに
`ResolvedCommandPolicy::project_grant` で resolved grants map を
対象 1 件へ投影する機構を追加(confused deputy 対策 C9)。

## Context

Task `add-route-context-scope-options`(plan Phase A3、design
§route context 注入、spec R18・C2・C9)。`route run` 由来の
invocation が payload `cwd`/env-prefix/exit mirror/log tee を
command tool へ届ける plumbing と、routed workflow が他 user
grant を呼べないよう resolved policy を単一 grant へ投影する
責務を分離実装する。

## Implementation

### Changes

- `RouteCommandContext` を `tools::command` に新設。
  `CommandToolOptions.log_tee` を `route_context` field へ統合
  (A2 で導入した平たい `log_tee` を context member へ移動 —
  route 注入が単一概念になるよう集約)。
- `command::execute`:
  - `exec_cwd` — `prepare_exec_cwd`(canonicalize + denylist +
    is_dir のみ、workspace containment 無し)へ切替え、
    grant.cwd 経路を bypass。
  - `extra_env` — `grant.env` map へ merge してから
    `minimal_env` へ渡す(正規化を一箇所に集約、
    `extra_env > grant.env`)。
  - `accept_any_exit_code` — `accepted_exit_codes: None` へ
    投影し任意の completed exit を result 化(C2)。
- `ToolRuntimeOptions.route_context` → `ToolCtx.route_context` →
  `ctx.command()` の plumbing(`create_worker_scope` は
  pass-through)。
- `ResolvedCommandPolicy::project_grant(name)` — grants map を
  対象 1 件へ置き換え。Shared/invocation 構築**前**の resolved
  policy に適用するため parity check は保たれる(影響範囲
  追加で core/policy/command.rs)。
- `prepare_exec_cwd` を proc 層に追加 — cwd 解決の所有
  boundary に置き、denylist/is_dir 規約を `prepare_cwd` と
  共有。

### Decisions During Implementation

- route 注入は `RouteCommandContext` 単一概念へ集約 — 平たい
  option 3 個 + log_tee を並べるより、route context という
  語のまま scope 構築層(D1)が注入する形にした
  (representation consistency)。
- `extra_env` の env-prefix > ambient 合成順は tool 側では
  持たない — 注入側(D1)が合成済み map を渡し、tool は
  `grant.env` への override としてのみ畳む。
- `exec_cwd` の拒否 error は `PolicyViolation` の
  `denied_declared_path` を共有し、"command cwd must be an
  existing directory" の InvalidRequest も `prepare_cwd` と
  同じ message 系に揃えた。
- artifact port 継承: `InvocationToolRuntimeOptions.artifacts`
  → `inner.artifacts` → scope `artifacts` の経路が既存のため
  `RouteLogReader` 注入に追加変更は不要(B3 で実装した
  `ArtifactReader` impl を D1 が `ArtifactPorts.reader` に
  載せるだけ)。

## Related Experiments

### Adopted

なし。

### Rejected

- `extra_env` を `minimal_env` の戻り Vec へ後付け merge —
  Windows の key 正規化(大文字化)が tool 側へ漏れるため
  却下。declared map 合成 → `minimal_env` 一箇所へ集約した。

## Validation

- `cargo test -p cheaper-tools --test proc_command` — 32 pass
  (新規 3: `route_context_extra_env_overrides_grant_env`・
  `route_context_exec_cwd_overrides_grant_cwd`・
  `route_context_accept_any_exit_code_mirrors_exit`)。
- `cargo test -p cheaper-tools --test proc_invocation` —
  `single_grant_projection_preserves_scope_parity` pass
  (投影 policy を両側へ供給し parity 成立 + 投影外 grant は
  `unknown or unauthorized command grant` で拒否)。
- `cargo test -p cheaper-core project_grant` — 1 pass
  (round-trip resolve parity 含む)。
- `cargo clippy --workspace --all-targets` — warning なし。

## Risks

- `exec_cwd` は workspace containment を課さない設計どおり —
  trusted route 宣言のみが到達する前提は D1 の layer trust
  gate が担保する。

## Follow-ups

- D1 `add-route-run-delegated-exec` が payload `cwd` →
  `exec_cwd`、env-prefix + ambient 合成 → `extra_env`、
  `accept_any_exit_code: true`、`RouteLogWriter` → `log_tee`
  を組立て、`policy.command.project_grant(route.grant)` を
  Shared/invocation 構築前に適用する。
