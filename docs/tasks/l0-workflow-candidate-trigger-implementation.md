---
id: "TASK-034Zuh3LdPDyCROadypAO0"
type: "task"
status: "done"
title: "L0 workflow candidate trigger implementation"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/202"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/l0-workflow-candidate-trigger-plan.md"
  implemented-by:
    - "../impl/ir/l0-workflow-candidate-trigger-implementation-record.md"
  depends-on:
    - "docs/plans/l0-workflow-candidate-trigger-plan.md"
    - "docs/designs/l0-workflow-candidate-trigger-distribution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
    - "docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/l0-workflow-candidate-trigger-plan.md"
    - "docs/designs/l0-workflow-candidate-trigger-distribution.md"
  derived-by: []
  verifies:
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
  defers: []
  deferred-by: []
---

# L0 workflow candidate trigger implementation

## Work

issue #202 の実装 slice。3 file を 1 つの論理変更として編集する:

1. `packages/apm/.apm/skills/cheaper-usage/SKILL.md` —
   (a) front-matter `description` 末尾に発火句 1 文を追加（repeated /
   fan-out / fan-in / long intermediate outputs のシグナル語彙、
   `cheaper-workflow-discover` は名前としてのみ言及）。
   (b) 本文末尾に `## Workflow candidate check (L0)` 節を追加 —
   目的（L1 を読むかだけを決める）・4 candidate signals・
   not-a-candidate・suppression rules（session 中 1 回・拒否済み
   再提案禁止・per-turn catalog/`history`/`usage`/file scan 禁止・
   過去 session/他 agent の履歴を仮定しない・PreToolUse route hook
   との区別・hook から毎回 LLM 推論を起こさない）・条件付き L1
   handoff（install 済みなら読む、未 install なら観測結果をユーザー
   へ報告）。英語、~45 行以内。
2. `docs/guides/parent-agent-integration.md` — 「自動 vs 明示的な
   委譲」節の直後に `## Workflow 候補の軽量判定（L0）` 節を追加。
   contract 要約（skill 節を canonical として参照）・per-target
   matrix（deploy 先・runtime discovery・発火限界。design §3 の表）・
   全 target 共通の限界（description 一致は model 判定で非保証、
   `--legacy-skill-paths` の扱い）・明示呼出し fallback（skill 名の
   明示呼出しで同じ L0 判定が動く。未配布の `cheaper-workflow-discover`
   は呼ばず観測結果を報告）・opt-in 最小指示 template（英文、
   "if installed" 条件付き）。日本語で記述。
3. `docs/guides/workflow-usage.md` — 「選択基準」節末尾に相互参照
   1 行を追加（L0/L1 が提案しても本節の費用判断基準で Workflow 不適
   と判定され得る旨）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/l0-workflow-candidate-trigger-plan.md`（PLAN-034ZufPdNasDwkcklErIMy）
- `docs/designs/l0-workflow-candidate-trigger-distribution.md`（DESIGN-034ZuedUsEd6RnxHqV4FL1）
- `docs/specs/l0-workflow-candidate-trigger-issue-202.md`（SPEC-034Zk1kCap2wKUvTt21Sil）
- `docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md`（ADR-034Zk1CgFRRcnAwV6sZWqQ）

## Dependencies

- `cheaper-workflow-discover`（#203）は未実装 — 全記述を "if installed"
  条件付きに限定し、未配布 version では観測結果のユーザー報告へ倒す。
- code・`apm.yml`・hook package・他 skill・deploy 写しへの依存なし。

## Affected Files

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — description 発火句
  + L0 節（規約の正本）
- `docs/guides/parent-agent-integration.md` — L0 節（matrix・fallback・
  opt-in template。consumer 面の canonical）
- `docs/guides/workflow-usage.md` — 「選択基準」節への相互参照 1 行

## Done When

- [x] `cheaper-usage/SKILL.md` の `description` に発火句、本文末尾に
      `## Workflow candidate check (L0)` 節が存在し、4 signals・
      not-a-candidate・suppression rules・条件付き L1 handoff を含む
- [x] `parent-agent-integration.md` に `## Workflow 候補の軽量判定（L0）`
      節があり、per-target matrix・明示呼出し fallback・opt-in
      template を含む（skill 本文との verbatim 複写なし）
- [x] `workflow-usage.md`「選択基準」節に相互参照 1 行がある
- [x] `git diff --stat` が 3 file + doc chain のみを含み、
      crates/・hook package・`apm.yml`・`.agents/skills/` deploy 写し
      に差分がない

## Verification

- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → blocking 0
- [x] `mise run check` → 全 step pass。4 回の実行で flaky timing test が
      間欠失敗するのみ（`abort_kills_and_preserves_reason` は内部
      watchdog 20s を数 ms 超過 — main と同一 file・孤立実行 pass。
      詳細は impl record の Validation 節）
- [x] `git diff --check` → whitespace error 0
- [x] scoped grep で発火句・L0 節・matrix・fallback・template・
      相互参照の存在を確認
- [x] `cheaper-workflow-discover` 言及が全箇所 "if installed" 条件付き
      であること（目視）
