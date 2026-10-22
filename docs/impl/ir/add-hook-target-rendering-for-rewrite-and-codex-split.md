---
id: "IMPL-034JjMYRBAtmxiGp1639qW"
type: "impl"
status: "completed"
title: "Add hook target rendering for Rewrite and codex split"
created: "2026-10-20"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "docs/tasks/add-hook-rewrite-target-rendering.md"
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "packages/apm-hook-codex/.apm/hooks/hooks.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-hook-rewrite-target-rendering.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/add-hook-route-rewrite.md"
    - "docs/tasks/add-hook-entry-route-loading.md"
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

# Add hook target rendering for Rewrite and codex split

## Summary

`Verdict::Rewrite` の per-target render contract を実装した —
claude(pretooluse)は `hookSpecificOutput.updatedInput`、devin は
`{"decision":"allow","updatedInput":{…}}` envelope、codex/windsurf/
cursor/copilot は fallback verdict のみを render する。
`HookTarget::Codex` variant を追加し、`apm-hook-codex` の
hooks.json を `--target codex` へ分離して claude/codex を
discriminator として区別可能にした。

## Context

- implements: `docs/tasks/add-hook-rewrite-target-rendering.md`
- derives-from: `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
  (§render(targets.rs)と target 分離 — R36-R38/C7)
- test spec: `docs/test-specs/hook-route-rewrite.md`
- depends-on: C2(Rewrite variant)・C1(route supply — Rewrite が
  実際に発行される経路)

## Implementation

### Changes

- `targets.rs`
  - `HookTarget::Codex` variant を追加(`ALL` 6 値、`as_str() ==
    "codex"`)。`Pretooluse` は claude 専用の意味へ(C7 —
    discriminator は `tool_name` field ではなく `--target` 値)。
  - `hook_specific_output_deny(reason)` — claude/codex 共有の
    `hookSpecificOutput` deny 応答を helper 化(DRY)。
  - `updated_input(command)` — `{"command":…}` object の共有
    builder。
  - `pretooluse_render` — Rewrite →
    `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
    "updatedInput":{"command":…}}}` + exit 0。`permissionDecision`
    は emit しない(R37/D3 — `permissionDecision:"allow"` 併記は
    approval bypass のため不可)。
  - `codex_render` — Rewrite → fallback verdict を codex の応答
    規約(hookSpecificOutput deny / silent allow)で render。
    `updatedInput` を発行しないため構造的に rewrite が codex へ
    届かない(R38 — approval-bypass 検証前)。
  - `devin_render` — Rewrite → `{"decision":"allow",
    "updatedInput":{"command":…}}` + exit 0。
  - `handle_hook` — `HookTarget::Codex` arm は
    `pretooluse_normalize` を共有し `codex_render` で応答(payload
    shape は claude と同一)。
  - `#[cfg(test)] mod tests` — 6 render unit test(claude/devin/
    codex/windsurf/cursor/copilot の Rewrite 契約)。
- `packages/apm-hook-codex/.apm/hooks/hooks.json` — 登録 command を
  `--target pretooluse` → `--target codex` へ変更。
- `tests/integration/hook.rs`
  - `TARGETS_JSON_LINE` — `"codex"` を targets 配列へ追加。
  - `hook_target_parse_table` — codex/devin の parse arm を追加。
  - `routed_shell_per_target_render_contract` — routed shell
    request の end-to-end contract(claude → updatedInput、codex →
    fallback deny)を firing load 経由で観測。

### Decisions During Implementation

- codex の normalize は `pretooluse_normalize` をそのまま共有 —
  design のとおり payload shape は同一であり、分離が必要なのは
  render contract のみ。`source_tool` prefix も `"pretooluse:…"`
  のまま(telemetry の識別には target 値側で区別)。
- claude の Rewrite render は fallback 内容に関わらず
  `updatedInput` のみ emit — R37 が「render は target 別に
  `updatedInput` のみを発行し `permissionDecision` を emit
  しない」と明確化しており、fallback の保持は rewrite 非対応
  target の render 経路向けの verdict 構造。

## Related Experiments

### Adopted

なし

### Rejected

なし

## Validation

- `cargo test -p cheaper-cli --lib hook::targets`: 6 pass(render
  unit test)
- `cargo test -p cheaper-cli --features …/dev-seam --test
  integration -- hook::`: 26 pass —
  `routed_shell_per_target_render_contract`・`hook_target_parse_table`・
  `hook_targets_json_emits_capability_line_without_usage` 含む
- `node scripts/check-apm-payload.mjs`: OK(hooks.json ↔ launcher
  整合)
- `cargo fmt -p cheaper-cli` / `cargo clippy -p cheaper-cli
  --all-targets`: clean

## Risks

- codex は R38 の approval-bypass 検証が未了のため `updatedInput`
  を受け取らない — codex 上では routed command が従来評価の
  verdict のみを得る(allow なら原 command がそのまま走る)。
  codex の rewrite 対応は issue #150 の検証待ち。
- devin の `updatedInput` field 名は design の Deferred Concern —
  Devin hook docs で正式 contract を確認する必要がある(現行は
  design 指定どおり `updatedInput` object を `decision:"allow"`
  と併記)。

## Follow-ups

- issue #150 — codex/cursor/copilot の Rewrite approval-bypass
  検証と `updatedInput` 対応拡大。
- Devin `updatedInput` field 名の正式 contract 確認(Deferred
  Design Concern の解消)。
