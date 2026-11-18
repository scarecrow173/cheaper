---
id: "PLAN-034Y9sqpRWawlMTaY1PIyP"
type: "plan"
status: "approved"
test-spec-skip: "Documentation/skill-only change; verified by doc audit, scoped grep for required sections and forbidden instructions, real-binary CLI spot-checks of every command the skill instructs, and mise run check — no executable test-spec artifact."
title: "Implement workflow discover skill (L1)"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/203"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/workflow-discover-skill-issue-203.md"
  implemented-by:
    - "docs/tasks/implement-workflow-discover-skill.md"
  depends-on:
    - "docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/workflow-discover-skill-issue-203.md"
    - "docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
    - "docs/discovery/workflow-discover-skill-issue-203.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-discover-skill-issue-203.md"
    - "docs/designs/workflow-discover-skill-structure.md"
    - "docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-discover-skill-issue-203.md"
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# Implement workflow discover skill (L1)

## Goal

`packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` を新設し、
L1 比較規約（inputs → 順序付き比較 → evidence labels → output
contract → 条件付き L2 handoff）を搭載する。guide 2 件は要約 +
pointer で補完する（skill 本文が canonical）。issue #203 を
実装する。

## Scope

単一 implementation stream。skill 新設 1 file + guide 2 file の
doc/metadata 変更のみで、code・`apm.yml`・hook package・他 skill
への変更は一切ない。task は 1 件。

## Upstream Documents

- `docs/specs/workflow-discover-skill-issue-203.md`（SPEC-034LNA8FqEjwFaDnqvIUc5）
- `docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md`（ADR-034aSc75jH9YL83FNQRuwH）
- `docs/discovery/workflow-discover-skill-issue-203.md`（DISC-0349RTCACbyKZHJy1Cvnpv — CLI surface の実 binary 検証済み evidence）

## Design Inputs

- `docs/designs/workflow-discover-skill-structure.md`（DESIGN-034ziRfYArWAlH3iiItcAR — front matter・7 節構成・guide 境界確定済み）

## File Map

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` —
  新規。front matter（`name` + 比較意図で発火する `description` +
  discovery-only 境界句）+ 本文 7 節（Boundaries・What to gather・
  Comparison procedure・Evidence labels・Output contract・Handoff
  to L2・Non-goals）。英語。
- `docs/guides/parent-agent-integration.md` — skills 一覧を 6 件化
  （`cheaper-workflow-discover` 追加）+ L0 節へ L1 handoff 補足
  1 段落。日本語。
- `docs/guides/workflow-usage.md` — discovery 相互参照節を追加
  （選択基準の権威を維持したまま L1 へ接続。pointer のみ）。

## Implementation Sequence

- [x] SKILL.md 新設 — front matter（発火語彙 + 境界句）・
      Boundaries・What to gather（observed/user-stated/unknown の
      分離・捏造禁止・最小収集）・Comparison procedure（direct
      adequacy → workers → routes → catalog → interface → 同一
      基準比較 → economics）・Evidence labels・Output contract
      （決定語彙・alternatives・evidence・conceptual summary・
      免責文・user-facing 例）・Handoff to L2（"if installed"
      条件付き）・Non-goals
- [x] `parent-agent-integration.md` — skills 一覧へ
      `cheaper-workflow-discover` 追加、L0 節へ L1 handoff 補足
- [x] `workflow-usage.md` — discovery 相互参照節追加
- [x] skill が指示する全 command を実 binary で spot-check
      （`workflow list` catalog 有/無・`workflow interface`・
      `route check`・`route resolve --command`・`usage`・
      `history list`・`count-tokens`・`models`）

## Dependencies and Risks

- `cheaper-workflow-create`（#204）は未 merge — 全言及を
  "if installed" 条件付きに限定し、未 install 時は manual
  authoring を経路として残す。
- `workflow interface` は workspace 内 path のみ受理（workspace 外は
  `INVALID_REQUEST`）— skill は catalog `path`（workspace 相対）を
  そのまま使う形で記述済み。
- `history list` は `--cwd` を受理しない — skill では `--cwd` を
  使う指示を history 系に出さない。
- skill 本文は英語・guide は日本語（各 file の既存言語に揃える）。

## Task Breakdown

- `docs/tasks/implement-workflow-discover-skill.md` — 上記
  Implementation Sequence 全 step を 1 task で実施（3 file 1 論理
  変更 + doc chain）

## Verification

- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → blocking 0
- [x] `mise run check` → 全 step pass
- [x] `git diff --check` → whitespace error 0
- [x] `git diff --stat` が SKILL.md + guide 2 file + doc chain のみ
      （crates/・hook package・`apm.yml`・`.agents/skills/` deploy
      写しに差分なし）
- [x] scoped grep: 必須節（Boundaries・Evidence・Output contract・
      Handoff）存在、`workflow run` 指示・定義 file 読取・無条件
      L2 参照・user-global lookup がないこと
- [x] 全指示 command の実 binary spot-check（DISC に記録）
- [x] guide と skill 本文に verbatim 複写がないこと（目視）
- [x] `cheaper-workflow-create` 言及が全箇所条件付きであること
      （目視）

## Open Questions / Gaps

- なし。#205 が横断 E2E（runtime 実機での implicit firing 実測）
  を担う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（skill 1 file + guide 2 file の
  doc 変更 — delegation は不要）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: (1) skill が discovery-only 境界（write・
  `workflow run`・grant/policy/catalog 変更の禁止）を冒頭と
  procedure の両方で保持しているか、(2) reuse-first の順序
  （direct adequacy → workers → routes → catalog → interface →
  比較）と negative recommendation が実装されているか、(3)
  evidence labels が measured/estimated/unknown に厳密で cheaper
  telemetry と親 context を混同していないか、(4) L2 言及が全て
  "if installed" 条件付きか、(5) 指示 command が実 binary の
  flag/出力と一致するか。
