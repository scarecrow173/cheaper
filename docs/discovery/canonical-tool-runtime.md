---
id: "DISC-034zIL9lJBa7f3clZmUHin"
type: "discovery"
status: "resolved"
title: "Canonical Tool Runtime 抽出"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  derives-from: ["../ideas/canonical-tool-runtime.md"]
  derived-by: ["../specs/canonical-tool-runtime.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Canonical Tool Runtime 抽出

## Exploration Goal

Pi に依存せず既存安全性を保存する抽出単位を特定する。

## Evidence

- registry.ts: factory と budget/meter wrapper を一箇所で構築。
- read/grep/find/count-tokens.ts: filesystem 実装と TypeBox schema は cheaper 所有、defineTool のみが Pi 依存。
- operations.ts: write/edit/ls の boundary/deny/writeScope と journal を所有。
- run.ts: journal は run ごと、timeout/abort と session finally が既存 ownership 境界。
- budget.ts: counter を await 前に増加し sibling admission race を防いでいる。

## Alternatives

| 案 | 評価 |
| --- | --- |
| Workflow handler を複製 | schema、安全性、計測が二重化するため不採用 |
| 全 helper を自前実装 | scope 超過と mutation semantics の回帰リスク |
| pure handler 抽出と helper 注入 | D1/Triage に一致、既存 semantics を保持 |

## Conclusion

共有 ADR D1/D7 を採用するため新しい ADR は不要。canonical 検証と budget/meter は
唯一の入口に置く。scheduler/operation/resource service は後続 issue の責務。

## Open Questions

scaffolding の環境 blocker は解消済み。coordinator 指示で local main から取得。
design 承認と graph focus の確認は未回答のため、実装開始の gate は未解消。

## Entry Decision

A-4: issue の acceptance と accepted ADR が具体的であり、dispatch 指示に従い
idea/discovery/spec/design を直接作成する（briefing-flow の SUBAGENT-STOP）。

## Graph Execution Checkpoint

mode は run-until-yield。probe は focus 無しで focus-required、idea focus 指定後は
probe-to-briefing を選択した。spec は coordinator の P1 指摘を修正し承認済み。
共有 ADR を正しく derives-from に結んだ後は、既存複数 plan を含む lineage により
spec/design focus が focus-required となることを router の実装と出力で確認した。
この blocked route を pass と見なさず、design/plan approval 後に own plan focus で
再投影する進め方の可否を coordinator に質問中（msg_8b25c7d87944）。
design draft の承認も質問中（msg_a52bfdd0e21d）。本体実装は未着手。

baseline 検証: 関連 tools/sibling/no-execution-mode/dependency guard の
14 test files、184 tests が pass。mise run setup は main の scaffolding 取り込み後も exit 0。

2026-10-04 の待機診断: coordinator terminal の最後の出力は design review subagent
完了後の Quota exhausted であり、質問への承認返答は得られていない。
承認を代行せず、文書の partial checkpoint を保存して failed と報告する。
doc id check は 464 markdown files で pass。index は chain 未完了のため未生成であり、
design の index-missing-entry は未解消。mise run check / exit-audit は未実行。
