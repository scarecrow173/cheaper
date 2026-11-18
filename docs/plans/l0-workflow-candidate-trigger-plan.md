---
id: "PLAN-034ZufPdNasDwkcklErIMy"
type: "plan"
status: "approved"
test-spec-skip: "Documentation/skill-metadata-only change; verified by doc audit, scoped grep for the L0 section and description clause, diff review of the three edited files, and mise run check — no executable test-spec artifact."
title: "L0 workflow candidate trigger implementation plan"
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
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
  implemented-by:
    - "docs/tasks/l0-workflow-candidate-trigger-implementation.md"
  depends-on:
    - "docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
    - "docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
    - "docs/discovery/l0-workflow-candidate-trigger-issue-202.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
    - "docs/designs/l0-workflow-candidate-trigger-distribution.md"
    - "docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
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
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# L0 workflow candidate trigger implementation plan Implementation Plan

## Goal

`cheaper-usage` skill metadata（`description` 発火句 + 本文末尾の L0 節）
に L0 判定規約を搭載し、consumer 向け guide（`parent-agent-integration.md`
の L0 節 = matrix・fallback・opt-in template、`workflow-usage.md` の相互
参照 1 行）で補完して issue #202 を実装する。

## Scope

単一 implementation stream。skill 1 file + guide 2 file の小さな
doc/metadata 変更のみで、code・`apm.yml`・hook package・他 skill への
変更は一切ない。task は 1 件（3 file の編集は 1 つの論理変更）。

## Upstream Documents

- `docs/specs/l0-workflow-candidate-trigger-issue-202.md`（SPEC-034Zk1kCap2wKUvTt21Sil）
- `docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md`（ADR-034Zk1CgFRRcnAwV6sZWqQ）
- `docs/discovery/l0-workflow-candidate-trigger-issue-202.md`（DISC-034Zk080rxFtqiKnk4VQHE — target discovery 実測の evidence 源）

## Design Inputs

- `docs/designs/l0-workflow-candidate-trigger-distribution.md`（DESIGN-034ZuedUsEd6RnxHqV4FL1 — 4 component boundary 確定済み）

## File Map

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — (a) front-matter
  `description` 末尾への発火句追加、(b) 本文末尾への
  `## Workflow candidate check (L0)` 節追加（規約の正本。英語、~45 行以内）
- `docs/guides/parent-agent-integration.md` — 「自動 vs 明示的な委譲」
  節の直後へ `## Workflow 候補の軽量判定（L0）` 節追加（contract 要約・
  per-target matrix・fallback・opt-in template。日本語）
- `docs/guides/workflow-usage.md` — 「選択基準」節末尾へ相互参照 1 行追加
  （L0/L1 が提案しても本節基準で Workflow 不適と判定され得る旨）

## Implementation Sequence

- [x] `cheaper-usage/SKILL.md`: `description` 末尾に発火句 1 文を追加
      （repeated / fan-out / fan-in / long intermediate outputs の
      シグナル語彙を含め、`cheaper-workflow-discover` を名前としてのみ
      言及 — 即時実行を促さない）
- [x] `cheaper-usage/SKILL.md`: 本文末尾（`## Delegation guidance` の後）
      に `## Workflow candidate check (L0)` 節を追加 — 目的（L1 を読むか
      だけを決める）・4 candidate signals・not-a-candidate・suppression
      rules（session 中 1 回・拒否済み再提案禁止・per-turn scan 禁止・
      route hook との区別）・条件付き L1 handoff（install 済みなら読む、
      未 install なら観測結果をユーザーへ報告）
- [x] `parent-agent-integration.md`: 「自動 vs 明示的な委譲」の直後に
      `## Workflow 候補の軽量判定（L0）` 節を追加 — contract 要約 +
      skill 節への pointer、per-target matrix（design §3 の表）、共通の
      発火限界、明示呼出し fallback、opt-in 最小指示 template（英文
      block、"if installed" 条件付き）
- [x] `workflow-usage.md`: 「選択基準」節末尾に相互参照 1 行を追加

## Dependencies and Risks

- `cheaper-workflow-discover`（#203）は未実装 — 全記述を "if installed"
  条件付きにし、未配布 version では観測結果の報告へ倒す。skill 名が
  #203 で変わった場合は L0 節・guide・template の 3 箇所を同期する
  （DISC open question に記録済み）。
- runtime 実機での implicit firing は未検証 — matrix は公式 doc と
  dogfood 実測（Devin CLI agent-skills）の verified 事実のみ記録し、
  保証できない発火は fallback で補う。#205 が横断 E2E を担う。
- `description` を長くしすぎると truncating host で L0 句が切れる —
  発火句は末尾 1 文に留め、既存 CLI 発火契約を前置きで損なわない。

## Task Breakdown

- `docs/tasks/l0-workflow-candidate-trigger-implementation.md` — 上記
  Implementation Sequence 全 4 step を 1 task で実施（3 file 1 論理変更）

## Verification

- [x] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → blocking 0
- [x] `mise run check` → 全 step pass（flaky timing test のみ間欠失敗 —
      impl record 参照）
- [x] `git diff --check` → whitespace error 0
- [x] `git diff --stat` が 3 file + doc chain のみを含む
      （crates/・hook package・`apm.yml`・`.agents/skills/` deploy 写し
      に差分なし）
- [x] scoped grep: `cheaper-usage/SKILL.md` に発火句と L0 節、
      `parent-agent-integration.md` に matrix・fallback・template、
      `workflow-usage.md` に相互参照が存在
- [x] guide と skill 本文に verbatim 複写がないこと（目視）
- [x] "if installed" 条件が全言及箇所で一貫していること（目視）

## Open Questions / Gaps

- #203 で `cheaper-workflow-discover` の skill 名が確定・変更された
  場合、L0 節・guide・template の 3 箇所の名前参照を同期する必要が
  ある（DISC open question 記録済み、issue への確認事項として報告）。
- runtime 実機 E2E（implicit firing の実測）は本 issue 範囲外。#205 で
  扱う。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution（3 file の小さな doc/metadata 変更 —
  delegation は不要）
- Independent work streams: none
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- reviewer focus: (1) L0 節が「L1 を読むか」のみに限定されているか、
  (2) suppression rules が issue #202 の禁止事項（per-turn scan・常駐
  monitor・hook 毎回 LLM 推論）を全て明示的に除外しているか、
  (3) matrix が verified 事実と fallback を分離し全 target 自動発火を
  謳っていないか、(4) `cheaper-workflow-discover` 言及が全て
  "if installed" 条件付きか。
