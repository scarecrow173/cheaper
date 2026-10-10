---
id: "IMPL-034Dl4CmoLtZTP6cs7v1Q8"
type: "impl"
status: "completed"
title: "cheaper-usage SKILL.md hook path section rewrite"
created: "2026-10-10"
updated: "2026-10-10"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/87"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/rewrite-cheaper-usage-skill-hook-path-section.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/cheaper-usage-skill-hook-path-rewrite.md"
    - "docs/designs/cheaper-usage-skill-hook-path-section-rewrite.md"
    - "docs/specs/cheaper-usage-skill-hook-path-examples.md"
    - "docs/adr/document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/cheaper-usage-skill-hook-path-rewrite.md"
  derived-by: []
  verifies:
    - "docs/specs/cheaper-usage-skill-hook-path-examples.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# cheaper-usage SKILL.md hook path section rewrite

## Summary

`cheaper-usage` SKILL.md の "Locating the CLI" 節を、APM の実
deploy contract（`<hooks_root>/<dep-name>/`）に一致する記述へ
改訂した。deploy dir 名を package 名 `cheaper` と仮定した旧例示
`.windsurf/hooks/cheaper/cheaper` を排除し、規則説明・canonical
install 実例（`hooks/apm/`）・5 target の hooks_root 表・
discovery 手順を追加した（issue #87）。

## Context

- plan: `docs/plans/cheaper-usage-skill-hook-path-rewrite.md`
  （approved、test-spec-skip の docs-only plan）
- design: `docs/designs/
  cheaper-usage-skill-hook-path-section-rewrite.md`（item 1-8 の
  節構成を定義）
- spec: `docs/specs/cheaper-usage-skill-hook-path-examples.md`
  （R1-R5 + AC）
- ADR: `docs/adr/
  document-apm-dep-name-deploy-rule-in-cheaper-usage-skill.md`
  （Option A: 規則＋実例＋discovery 手順）

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — 節内 path
  例示ブロック（旧 :44-73）を書換え。節内 bullets（旧 :74-100）
  ・他 section・front matter は未変更。
  - 導入を target 非依存へ一般化し、"standalone executables"
    記述を sibling/first-run acquisition の実態へ訂正
  - `<hooks_root>/<dep-name>/` 規則と dep 名由来（path basename
    / virtual ref の末尾 path segment）を説明
  - canonical install 例 `.windsurf/hooks/apm/cheaper`・
    `powershell .../cheaper.ps1`（stock Windows の powershell
    5.1 で動く方を例示）+ copilot `scripts/apm/` 例
  - 5 target の hooks_root 表（windsurf/claude/codex/cursor/
    copilot、APM 0.28.0 実測値）
  - discovery 手順（generated hook config の `command` 参照。
    file 名は target 依存 + `apm-hooks.json` duplicate +
    claude `${CLAUDE_PROJECT_DIR}` 注記。または hooks_root
    配下探索）
  - install 名読み替え注記は規則説明へ昇格済みのため削除、
    "either form" 橋渡し文は維持

### Decisions During Implementation

- **Windows 例示を `pwsh` → `powershell` に変更**: review で
  stock Windows（PowerShell 5.1 のみ同梱）で `pwsh` が存在しない
  指摘を受け、generated hooks.json と同じ `powershell` 起動に
  揃えた。
- **virtual ref の dep 名を実測確定**: briefing review で「path
  dep の basename 規則しか実測されていない」指摘を受け、temp
  consumer で `scarecrow173/cheaper/packages/apm#main`（git
  clone）を install して `.windsurf/hooks/apm/` を実測確定。
- **copilot の config 表記を `<dep>` → `<dep-name>` に統一**:
  placeholder 命名 drift の指摘を反映。

## Related Experiments

### Adopted

なし（experiment log は作成していない）。

### Rejected

なし。

## Validation

- `grep -rn "hooks/cheaper" packages/apm/.apm/` → 0 件
- `mise run check` 全 pass（biome / fmt / clippy / version /
  doc-id / cargo test --workspace / node test / bundle・payload /
  freshness）
- temp consumer install（APM 0.28.0）実測: path dep basename
  `apm` → `.windsurf/hooks/apm/cheaper`・`cheaper.ps1`・`cli/`、
  virtual ref `scarecrow173/cheaper/packages/apm#main` →
  `.windsurf/hooks/apm/`（いずれも記載と一致）
- review-protocol: 協調・adversarial 両 reviewer が ACCEPT-
  WITH-FIXES。confirmed 指摘（pwsh→powershell、`<dep>` 正規化、
  `apm-hooks.json` 補足、"last segment"→"last path segment"）は
  全て反映済み

## Risks

- registry 公開後の registry ref 名で `<dep-name>` が `apm`
  以外になる可能性（git ref は実測済み。spec/design の
  Deferred Concern、#85 後に再確認。規則説明で吸収可能）

## Follow-ups

- issue #85（release v1.0.0 未公開）解消後、registry ref での
  install で dep 解決名を再実測する
