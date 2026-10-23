---
id: "DISC-Fad7xt2qdtfpcCHhjMhkVy"
type: "discovery"
status: "capturing"
title: "opt-in security.execDenyPatterns profile — A9 の降格実装（#164）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/164"
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
  related:
    - "exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
  defers: []
  deferred-by: []
---

# opt-in security.execDenyPatterns profile（#164）

## Seed（briefing 前の最小記録）

Issue #164（#151 実装分解 2/7）。旧 builtin `command_denied_executable_re`
（A9 — #163 で mandatory 層から削除済み）を operator opt-in の
deterministic deny `security.execDenyPatterns` として再配置する。

### 要件骨子（issue 本文より）

- policy schema に `security` 節を新設、`execDenyPatterns: []`（既定空）
- authoring は trusted 層（builtin/user）のみ — invocation 層は
  keys ⊆ `{version, command.grants}` guard で構造的に拒否、
  project partial schema も受け付けない
- deny は `resolve_command_policy`（declared basename）・
  `pin_command_grant`（resolved basename）・`verify_pinned_executable`
  （spawn 前再検証）の既存 3 適用点で効き、layer 由来を持たない
  resolved map 全体に及ぶ（invocation 合成 grant も対象）
- 表現は「deterministic executable deny-list」—「保護」「sandbox」
  「containment」とは呼ばない
- 旧 builtin 一覧を docs に「旧既定値」として記録 + migration seed 例

### 前提

#163 で builtin regex + predicate machinery は完全削除済み
（ADR Option A・YAGNI — builtin は operator pattern と再利用不可能）。
本 issue は新規 schema 節 + operator 定義の deny predicate +
3 適用点への再挿入 + docs。

詳細は briefing で確定する。
