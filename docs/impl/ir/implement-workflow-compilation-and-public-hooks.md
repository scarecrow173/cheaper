---
id: "IMPL-034zQvN42aaLRbRi0R2fI4"
type: "impl"
status: "completed"
title: "Implement workflow compilation and public hooks"
created: "2026-10-04"
updated: "2026-10-04"
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
  implements:
    - "docs/tasks/implement-workflow-compilation-and-public-hooks.md"
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
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Workflow compiler と public hooks の実装記録

## Summary

approved design/plan に従い compiler/projection を実装した。実行系は scope 外。

## Changes Made

default snapshot、non-scan、static/deferred validation、terminal output、resource manifest 保持を対象にする。

## Validation and Evidence

failing tests で未実装を確認してから実装し、最終 focused suite 75 件の pass を確認した。plan gate msg_54c265f472f0 は APPROVED: plan。

## Risks or Follow-ups

#62/#70 consumer が exported hooks を利用する。AC7 guidance は本 IR に記録する（plan review で IR のみも受理可）。

## Compact Contract Fixture / Token-efficiency Guidance

単独の単純処理は既存の Worker/tool 直接呼出しを使う。Workflow は fan-out/fan-in、連鎖、stored definition の反復で親 agent の中間 result 読み直しを減らせる場合に使う。public interface は schema/default/requiredness を投影し、private result は declared outputs だけに絞る。本 IR は parent agent が読む contract fixture/guidance も兼ねる（plan review の AC7 note に従う）。

```json
{
  "schemaVersion": 1,
  "inputs": { "path": { "type": "string", "default": "src/index.ts" } },
  "jobs": {
    "read": { "kind": "tool", "tool": "read", "input": { "path": { "$input": "path" } } },
    "analyze": { "kind": "worker", "worker": "bulk-reader", "input": { "task": { "$from": "read", "path": "/content/0/text" } } }
  },
  "outputs": { "summary": { "schema": { "type": "string" }, "value": { "$from": "analyze", "path": "/summary" } } }
}
```

fixture は compile/materialization contract 用であり provider 実行を主張しない。test は実在 bulk-reader.input.schema.json を core compileSchema で読み、解決された task の適合を確認する。read は fake ToolCatalog の canonical metadata を注入する。

## Progress Evidence

初回 red: compiler 未実装のため 40 failed / 23 passed。実装後 63 passed、static Pointer certainty/output readiness tests を追加後 74 passed。workflow build exit 0。biome の forEach callback error は再帰 scan を for-of にして修正、残る non-null assertion warnings は validation 済み map/test の確定 key を参照する箇所であり errors は 0。最終検証と canonical check の委譲は下記に記録した。

public input/default/private snapshot、tool/Worker metadata、core validateWorkerInput、単一/batch terminal outputs、async ResourceInference の全 job manifests/inferred edges を実装。省略/{} resources は manifest を emit せず、explicit empty arrays は port を要求する。

## Final Validation / Review

workflow: 4 files / 75 tests passed、workflow build exit 0。canonical run の lint/version/doc-id/typecheck と workflow guard/tests は pass。CLI e2e 3 files / 5 cases が既知の 20000ms timeout で失敗したため、指定された exception に従い各 file を単独で順に再実行した。

- copilot-pre-read-hook: 11 passed、62.81s。
- cross-agent-routing-adapters: 32 passed、225.59s。
- hook-contract: 16 passed / 1 existing skipped、191.08s。

未到達の bundle-cli/check-apm-payload/freshness を完了し bundled payload 差分なし。canonical 初回が exit 0 だったとは主張しない。report に全出力末尾と recovery 証跡を保存する。実 provider/engine/resource normalization の完成を主張しない。

source control/data flow を self-review し、private original template の単一解釈、compile 時 static schema、hook 時最終 schema、result-root Pointer、全 terminal output、ResourceManifest 保持、依存方向を確認した。新たな spec/ADR/plan 判断はない。

Coordinator policy note（2026-10-04）: host contention による CLI e2e timeouts は full canonical green を worker の blocking condition にしない。focused suite GREEN、失敗 file の単独再実行、lint/typecheck/build pass を必要証跡とし、full canonical は merge 時に coordinator が worktree ごとに serial 実行する。canonical check deferred to coordinator merge run。

## Context

approved design/plan と ADR D1/D4/D5/D6、coordinator の manifest/fallback 修正を実装する。

## Implementation

src/compiler.ts/interface.ts/index.ts と contract fixtures/tests を追加。public defaults、private snapshots、job/output hooks、resource port、public-only projection を consumer API として export した。

## Related Experiments

既知の設計を実装したため experiment log は作成していない。red/green/canonical timeout recovery は Validation and Evidence と Final Validation に記録した。

## Follow-ups

未分類 follow-up はない。canonical check deferred to coordinator merge run。global document findings は coordinator が merge 時に処理する（coordinator が msg_82c3f8e71629 に承認済み）。#62/#65-B/#70 は既存の別 issue scope であり本 issue の未完了 task としない。


## Graph Completion Evidence

run-until-yield、taskBudgetCount=2、maxHops=11。plan focus-required の既知 lineage 問題は resume instruction の承認済み例外で plan 作成まで手動 delegate 順を保持した。plan gate msg_54c265f472f0 は APPROVED: plan。planning-to-task-graph、task-graph-to-implementation、implementation-retry を実行し2 task を完了した。implementation-verified の evidence signal で implementation-to-followup-triage へ進んだ。

focused briefing の considered-options が fail した原因は、accepted shared ADR の numbered markers を router が数えないことだった。coordinator 承認 msg_57528f4c4d43 に従い3行の marker のみ presentation repair し独立 docs(adr) commit とした。D contracts/status/relations/script は変更していない。

IR index は coordinator 承認 msg_82c3f8e71629 に従い既存 impl-doc generator の updateIndexForMarkdownDir を一時 data module から実行した。tracked script/一時 file の変更はない。通常 indexes は doc-maintenance plan→apply を実行した。global document findings は owner 外なので focused audit の判断と分ける。

focused chain の doc-status findings は 0、IR audit findings は 0。global blocking findings 41 件は scope 外として報告する。followup-triage-terminal → exit-audit-to-complete の順に進み、complete の terminal route（blockers/taskGraph.issues ともに空）を確認した。route と監査 JSON は worker-64-report.md が参照する evidence files に保存した。
