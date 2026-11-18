---
id: "TASK-034IUTJoHjZJipMihVLIMX"
type: "task"
status: "done"
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
    - "docs/plans/implement-workflow-discover-skill.md"
  implemented-by:
    - "../impl/ir/workflow-discover-skill-implementation-record.md"
  depends-on:
    - "docs/plans/implement-workflow-discover-skill.md"
    - "docs/designs/workflow-discover-skill-structure.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/workflow-discover-skill-issue-203.md"
    - "docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-workflow-discover-skill.md"
    - "docs/designs/workflow-discover-skill-structure.md"
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
---

# Implement workflow discover skill (L1)

## Work

issue #203 の実装 slice。3 file を 1 つの論理変更として編集する:

1. `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md`
   新設 — front matter（`name: cheaper-workflow-discover`、
   比較意図で発火する `description`、discovery-only 境界句）+
   本文 7 節:
   - Boundaries — write・`workflow run`・grant/policy/catalog 変更
     禁止、workflow 定義 file を inspect 目的で読まない、
     workspace scoped lookup のみ、親 context ≠ cheaper
     telemetry、最小収集
   - What to gather — 手順要約・反復（observed/user-stated/
     unknown 分離）・代表 I/O・中間結果・副作用/時間制約・
     既知比較対象。捏造禁止
   - Comparison procedure — 7 step（direct adequacy → workers →
     routes → `workflow list` → `workflow interface` → 同一基準
     比較 → economics）
   - Evidence labels — measured/estimated/unknown、parent token
     節約の断定禁止
   - Output contract — 決定語彙・alternatives・evidence・
     conceptual proposal summary（YAML 形）・免責文・user-facing
     例
   - Handoff to L2 — "if installed" 条件付き、未 install 時は
     manual authoring 経路
   - Non-goals
   英語で記述。
2. `docs/guides/parent-agent-integration.md` — skills 一覧へ
   `cheaper-workflow-discover` 追加（6 件化）、L0 節へ L1
   handoff 補足 1 段落（install 済みで読まれた後の振る舞い要約
   + canonical pointer）。日本語。
3. `docs/guides/workflow-usage.md` — discovery 相互参照節を追加
   （L1 が本 guide の選択基準・claim 境界を適用する評価層であり
   proposal ≠ 作成・実行である旨。pointer のみ）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-workflow-discover-skill.md`（PLAN-034Y9sqpRWawlMTaY1PIyP）
- `docs/designs/workflow-discover-skill-structure.md`（DESIGN-034ziRfYArWAlH3iiItcAR）
- `docs/specs/workflow-discover-skill-issue-203.md`（SPEC-034LNA8FqEjwFaDnqvIUc5）
- `docs/adr/l1-workflow-comparison-lives-in-discover-skill-body.md`（ADR-034aSc75jH9YL83FNQRuwH）

## Dependencies

- `cheaper-workflow-create`（#204）は未 merge — 全言及を
  "if installed" 条件付きに限定。
- #201 catalog（`workflow list` + manifest 規約）と #202 L0 規約は
  main にマージ済み — 本 task はその consumer。
- code・`apm.yml`・hook package・他 skill・deploy 写しへの依存
  なし。

## Affected Files

- `packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md` —
  新規（L1 規約の正本）
- `docs/guides/parent-agent-integration.md` — skills 一覧 + L0→L1
  handoff 補足（consumer 面の要約）
- `docs/guides/workflow-usage.md` — discovery 相互参照節

## Done When

- [x] `cheaper-workflow-discover/SKILL.md` が front matter + 7 節で
      存在し、決定語彙・reuse-first 順序・evidence labels・
      conceptual summary・免責文・条件付き L2 handoff を含む
- [x] `parent-agent-integration.md` の skills 一覧が 6 件で L1
      handoff 補足がある（skill 本文との verbatim 複写なし）
- [x] `workflow-usage.md` に discovery 相互参照節がある
- [x] `git diff --stat` が 3 file + doc chain のみを含み、
      crates/・hook package・`apm.yml`・`.agents/skills/` deploy
      写しに差分がない

## Verification

- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → blocking 0
- [x] `mise run check` → 全 step pass
- [x] `git diff --check` → whitespace error 0
- [x] scoped grep で必須節の存在と禁止事項（`workflow run` 指示・
      定義 file 読取・無条件 L2 参照・user-global lookup）の
      非存在を確認
- [x] skill が指示する全 command の実 binary spot-check — flag・
      出力形状が記述どおり（impl record の Validation 節に実測値）
- [x] `cheaper-workflow-create` 言及が全箇所 "if installed"
      条件付きであること（目視）
