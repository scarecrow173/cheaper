---
id: "TSPEC-034Jg5K8URUPa2X0HSK0mN"
type: "test-spec"
status: "draft"
title: "PreToolUse route verdict rewrite and target rendering"
created: "2026-10-20"
updated: "2026-10-21"
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
  implements: []
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
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# PreToolUse route verdict rewrite and target rendering

## Purpose

evaluate_shell の route arm と Verdict::Rewrite{command,fallback} が
spec R24-R40 を満たし、target 別 render(claude/devin updatedInput、
codex 等 fallback)が正しいことを pin する。

## Feature

PreToolUse hook が対象 argv を resolve し、runnable なら
`cheaper route run --payload <b64url>` へ rewrite する。

## Rules

- payload は `route::payload` 単一 owner で base64url 符号化され、
  command・cwd・allowlist-subset env(ambient capture)を含む。
- rewrite command は `current_exe()` で anchor し、解決不能時は
  bare `cheaper` に fallback する。
- claude/devin target は hookSpecificOutput.updatedInput に rewrite を
  載せ、codex を含む他 target は fallback verdict のみ render する。
- fallback verdict は read-candidate 評価の生 verdict(Allow/Deny)を
  保持し、non-runnable route は従来評価へ fallthrough する。
- HookTarget::Codex は独自 variant で apm-hook-codex が登録する。
- Rewrite verdict は dedup cache に persist/serve しない。
- Claude / Devin の Rewrite は
  `hookSpecificOutput:{hookEventName:"PreToolUse",updatedInput:{command}}`
  を使う。`decision` / `permissionDecision` は付けず、adapter が
  入力の書換えを根拠に承認まで決定することを防ぐ。
- hook firing 1 回につき effective routes + command policy を 1 回
  load し HookDeps 経由で供給する。

## Examples

- grant pin 済み `git diff` → claude へ updatedInput.command =
  `"/path/cheaper" route run --payload eyJ...` + fallback Allow。
- deny grant の command → fallback Deny を保持し rewrite しない。
- codex target → 従来通り fallback verdict のみ返す。

## Guarantees

- rewrite 後 command が token 列として安定し、payload decode で元
  command/cwd/env-subset が復元される。

## Non-goals

- windsurf/codex/cursor/copilot への rewrite 発行。

## Risk

- fallback 喪失は deny 素通しを生む。payload schema の二重管理は
  decode skew を生む。
