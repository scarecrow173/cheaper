---
id: "TSPEC-034Jg5XonLxlwll8pJ15V0"
type: "test-spec"
status: "draft"
title: "Hook to route run end-to-end delegation"
created: "2026-10-20"
updated: "2026-10-20"
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
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Hook to route run end-to-end delegation

## Purpose

PreToolUse firing → rewrite → `route run` → workflow exec →
exit mirror の e2e が spec R1-R40 の end state を満たすことを
pin する。

## Feature

対象 command が hook で rewrite され delegated workflow で実行される。

## Rules

- hook が発行する payload が direct CLI `route run --payload` と
  同一 contract で処理される。
- hook eval の resolve が `route resolve` CLI の出力と一致する。
- e2e で cwd・env-prefix・bounded output・exit code が保持される。
- fallback verdict を持つ non-rewrite target が従来挙動を維持する。
- 全 spec R と ADR verification item が少なくとも 1 test へ写像済み。

## Examples

- fixture grant pin command を hook eval → updatedInput.command →
  route run 実行 → 同一 exit・出力を得る。

## Guarantees

- hook 経由と direct CLI で resolve/run の乖離が無い。

## Non-goals

- GUI/実 provider E2E(ollama_e2e 相当)。

## Risk

- e2e 欠如は各層の単体 pass にも関わらず統合で破綻する。
