---
id: "IMPL-034Zuk5KqJIYiOP5aFveMz"
type: "impl"
status: "completed"
title: "L0 workflow candidate trigger implementation record"
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
    - "docs/tasks/l0-workflow-candidate-trigger-implementation.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/l0-workflow-candidate-trigger-implementation.md"
    - "docs/plans/l0-workflow-candidate-trigger-plan.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/l0-workflow-candidate-trigger-issue-202.md"
    - "docs/designs/l0-workflow-candidate-trigger-distribution.md"
    - "docs/adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/tasks/l0-workflow-candidate-trigger-implementation.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# L0 workflow candidate trigger implementation record

## Summary

issue #202 の L0 判定規約を `cheaper-usage` skill metadata
（`description` 発火句 + 本文末尾の `## Workflow candidate check (L0)`
節）に搭載し、`docs/guides/parent-agent-integration.md` に L0 節
（contract 要約・per-target matrix・明示呼出し fallback・opt-in
template）、`docs/guides/workflow-usage.md` に相互参照 1 行を追加した。
code・`apm.yml`・hook package・他 skill・deploy 写しへの変更はゼロ。

## Context

TASK-034Zuh3LdPDyCROadypAO0 / PLAN-034ZufPdNasDwkcklErIMy /
DESIGN-034ZuedUsEd6RnxHqV4FL1 / SPEC-034Zk1kCap2wKUvTt21Sil /
ADR-034Zk1CgFRRcnAwV6sZWqQ の chain に従う。upstream はすべて
approved/accepted で、ADR の Option 1（`cheaper-usage` skill
metadata に搭載、guide・template で補完）をそのまま実装した。
前段の discovery（DISC-034Zk080rxFtqiKnk4VQHE、resolved）で確定した
per-target discovery 実測値を matrix の入力に使った。

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
  - `description` 末尾に 1 文追加: workflow-candidate signal（repeated
    identical procedures / manual fan-out/fan-in / long intermediate
    outputs）を示す通常作業でも読む対象であること、L0 節が
    `cheaper-workflow-discover`（L1）を読むかだけを決める旨を記述。
  - 本文末尾「Delegation guidance」の後に `## Workflow candidate
    check (L0)` 節を追加: 目的（L1 を読むかのみを決め、Workflow 化
    判断・重複探索・コスト分析・保存はしない。追加 CLI/LLM/file scan
    不要）・4 candidate signals・not-a-candidate（1 回きり単純作業・
    既存単一 worker で足りる・提案不要・context 不足）・suppression
    rules（session 中 1 回・拒否済み再提案禁止・catalog/`history`/
    `usage`/file scan 禁止・過去 session/他 agent 履歴を仮定しない・
    PreToolUse route hook との区別）・条件付き L1 handoff（install
    済みなら読む、未 install なら観測結果をユーザーへ報告）。
- `docs/guides/parent-agent-integration.md`
  - 「自動 vs 明示的な委譲」の直後に `## Workflow 候補の軽量判定
    （L0）` 節を追加。contract 要約 + skill 節を canonical とする
    pointer、per-target matrix（claude `.claude/skills/`・codex/
    cursor/copilot/windsurf/agent-skills `.agents/skills/` + 各
    runtime の discovery 様式 + description 一致は model 判定で
    非保証の限界列）、共通限界（`--legacy-skill-paths` の扱い、
    agent-skills は hook を持たない）、明示呼出し fallback、opt-in
    最小指示 template（英文 block、`cheaper-workflow-discover` は
    "if installed" 条件付き）。skill 本文との verbatim 複写なし。
- `docs/guides/workflow-usage.md`
  - 「選択基準」節末尾に 1 行追加: L0/L1 は候補検出の入口であり、
    Workflow 化の可否は本節の費用判断基準が正本である旨 +
    parent-agent 連携ガイド L0 節への link。

### Decisions During Implementation

- L0 節の言語は SKILL.md 本文と同じ英語（同 file の言語一貫性）。
  guide 側は既存本文と同じ日本語。
- `cheaper-workflow-discover` への言及は全箇所 "if installed" / 「install
  済みなら」条件付きに限定 — 未配布の L1 を即時実行させる記述は
  一切入れなかった。
- description 発火句は末尾 1 文に限定し、既存の CLI 発火契約
  （"Read this before calling any cheaper command"）を前置きで損なわ
  ない配置とした。
- guide の matrix は DISC の実測 + 公式 doc の verified 事実のみに
  限定し、「全 target 自動発火」を謳わず fallback で補う構成にした。
- opt-in template は guide にのみ配置（skill 節の写しではなく最小
  規約である旨を明記）し、drift 経路を guide→skill 参照一方向に
  絞った。

## Related Experiments

### Adopted

（experiment log なし — 3 file の直接編集で完結）

### Rejected

（experiment log なし）

## Validation

- `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` → `{"blocking":0,"findings":[]}`（1060 documents）
- `git diff --check` → whitespace error 0
- `mise run check` → 全 step 実測 pass。pipeline 内訳:
  - biome check / cargo fmt --check / cargo clippy -D warnings → 全 run pass
  - check-version / check-doc-ids → pass（version 1.0.0、1319 files）
  - `node --test scripts/test/` → 43/43 pass
  - check-lockfile / check-apm-payload → pass
  - `cargo test --workspace` → 全 test binary が少なくとも 1 回完走 pass。
    ただし 4 回の実行で 3 件の timing-boundary flaky test が並列負荷下で
    間欠失敗（いずれも本 change と無関係、test file は main と同一）:
    - `route_run::tiny_count_cap_partial_count`・
      `route_run::cli_run_emits_raw_text_exit_and_route_record`
      （1 回目のみ。孤立実行・2 回目以降 pass）
    - `abort_kills_and_preserves_reason`（内部 watchdog 20s を 2-7ms
      超過。fixture child は TerminateProcess で正常終了。孤立実行 pass、
      `abort_kills_managed_parent_and_descendant` も孤立・3 回目 pass）
- `git diff --stat` → 変更 file は上記 3 file + doc chain（discovery/
  spec/adr/design/plan/task/impl record + 各 README index）のみ。
  crates/・`packages/apm/apm.yml`・hook package・`.agents/skills/`
  deploy 写しに差分なし
- scoped grep: `cheaper-usage/SKILL.md` に発火句と `## Workflow
  candidate check (L0)` 節、`parent-agent-integration.md` に
  `## Workflow 候補の軽量判定（L0）` 節（matrix 6 行・fallback・
  template）、`workflow-usage.md` 選択基準節に相互参照を確認
- `cheaper-workflow-discover` 言及は skill 節・guide 節・template の
  全箇所で install 条件付き（目視確認）

## Risks

- `cheaper-workflow-discover`（#203）未実装の間、L0 の handoff は
  「install 済みなら読む」の条件分岐に留まる。#203 で skill 名が
  変わった場合、L0 節・guide 節・opt-in template の 3 箇所を同期
  する必要がある（DISC open question に記録済み）。
- runtime 実機での implicit firing（description 一致による間接発火）
  は未検証 — matrix は verified 事実のみ記録し、非保証の限界と
  fallback を明記した。#205 で横断 E2E を扱う。
- `description` が長くなり truncating host で L0 句が切れる可能性
  → design で受容済み（fallback の明示呼出しが残るため実害は限定的）。

## Follow-ups

- #203 `cheaper-workflow-discover`（L1 skill）実装時に L0 節・
  guide 節・template の名前参照を同期する。
- #205 横断 E2E で各 runtime の implicit firing を実測し、matrix の
  「発火限界」列を実測値で更新する。
- ユーザーへ: #203 で L1 skill 名が `cheaper-workflow-discover` で
  確定するか確認（rename の場合は本 3 箇所の 1 行同期が必要）。
