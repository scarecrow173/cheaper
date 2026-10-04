---
id: "IMPL-034zUfCug3Fz5ZzcrI1yhg"
type: "impl"
status: "completed"
title: "Workflow resource inference compilation"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
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
    - "docs/tasks/integrate-workflow-resource-inference.md"
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

# Workflow resource inference compilation

## Summary
compiler の inference gate を resources port 注入へ変更し、全 job の manifest を検査して保持する。conflicts/diagnostics を deep clone で公開し、inferred edges は既存 compileDag に union する。

## Context
承認済み plan slice 2 / TASK-034zTTjWYMNKiiVb4ZZ6LV。共有 ADR D4/D5 の shape と未注入 compiler の既存挙動を保持する。

## Implementation
### Changes
compiler.ts、compiler.test.ts。IR は編集前に in-progress で作成した。
### Decisions During Implementation
未注入かつ非空 resources では INVALID_REQUEST を維持する。注入された port の失敗・manifest 欠落は省略-only workflow でも伝播する。optional field は exactOptionalPropertyTypes を守り undefined property を組み立てない。

## Related Experiments
なし。

## Validation
- RED: compiler contract 新規 case の 3 failure。all-omitted port calls=0、missing manifest が resolve、diagnostics=undefined を確認。
- compiler 初回 GREEN: workflow suite pass（2026-10-04 14:31 JST）。
- needs + $from + producer edge dedupe、inferred cycle rejection、既存 needs があっても duplicate producer rejection、metadata known consumer read、result deep clone を test で固定。
- pnpm lint: exit 0。
- pnpm typecheck: optional property construction の TS2375 を修正後 exit 0。
- pnpm build: 同修正後 exit 0。
- tool-runtime suite 18 tests / 3 files pass。
- canonical check deferred to coordinator merge run。

## Risks
fallback safety claim による conservative conflicts、Worker ancestor mkdir の root exclusive safety、core literal symlink/hard-link residual risk を継承する。engine/scheduler/ResourceManager/process executor は実装しない。

## Follow-ups
implementation review は APPROVED。残り P2 は非ブロッキング robustness/test-depth のため後述の deferred 分類で終端化する。

## 自己点検での source boundary 修正
- Worker target src/*.ts が一般 write normalization で root subtree に拡大し、無関係 output ownership を受理していた。新規 test は promise resolved の RED。authority は core ownership normalization（produces bucket）で exact literal を検査し、root mkdir safety は normalizeResourceClaim で別に作る。
- 非 process job の process 宣言が tool/Worker の実 source を置き換えていた。新規 test は promise resolved の RED。process declaration は kind:tool/tool:process のみ受理する。
- process config 自体が explicit source なので resources-omitted fallback diagnostic を付けない。
- source authority 修正後 workflow suite GREEN、pnpm lint/typecheck/build exit 0（2026-10-04 14:47 JST）。固定 core 型と approved design の authoritative exact target / process seam に合わせる source-layer correction。レビュー対象 commit に追加する。

process outputs の subtree glob でも同じ safety/authority 分離を確認した。writes の保守的 root 拡大に基づく authority が追加 outside output を許す regression RED を確認し、process authority を ownership-normalized produces に変更。workflow suite 109 tests / 5 files GREEN、lint/typecheck/build exit 0。

## Implementation review
msg_fb4b2821f57e は APPROVED: implementation review。最終検証は workflow 109 / tool-runtime 18、lint/typecheck/build pass。非ブロッキング P2: canonical mutation predicate の共有（coordinator の直接指示に基づく最小 core export で対応済み）、直接 port に不正 nullable resources を渡す時の TypeError 余地、fallback diagnostics 粒度、catalog descriptor の直接 unit tests（workflow catalog contract tests は存在）。IR count は実行履歴と最終値を区別して整合する。

## P2-1 mutation capability predicate 共有
coordinator の直接指示に沿い、packages/core/src/worker/validate.ts の既存集合に委譲する isMutatingTool(name: ToolName): boolean を export した。既存 barrel が star export のため index の変更は不要。workflow は worker.tools.some(isMutatingTool) を使う。core 型と既存 accounting 判定の挙動は変更しない。core suite exit 0、workflow 109 tests / 5 files、lint/typecheck/build exit 0（2026-10-04 15:05 JST）。

## Followup triage（分類完了）
- mutation predicate 重複: 本 chain 内修正済み。core export 例外は coordinator 最新直接指示を根拠とする。
- IR test count: 現行 suite 109 / tool-runtime 18 に統一済み。RED failure の個数と段階別 checkpoints は suite 実数と区別する。
- direct port malformed nullable payload: deferred robustness。WorkflowCompiler の schema boundary が通常経路で拒否する contract 外入力。将来 #59 の port producer integration 時に boundary 検査の責務を確認する。現 slice の acceptance blocker ではない。
- unknown-access diagnostics 重複/粒度: deferred optional diagnostic refinement。scheduler の順序を変更せず current evidence は保持する。
- catalog descriptor isolated unit tests: deferred optional test-depth。実 catalog descriptor を利用する workflow source/consumer 契約 tests は GREEN。
- core literal symlink/hard-link residual risk、process executor、scheduler/ResourceManager/engine: inherited/out-of-scope。現在の chain に未分類 followup、未達 acceptance、未完 task はない。

## Graph checkpoint
承認済み implementation を二 tasks done / 二 IR completed / plan completed として確定する。taskBudgetCount=2、maxHops=11、commitWaived=false。doc-maintenance plan→apply、IR index は approved exception の reused utility method で再生成する。global blockers は scope 外として記録し、chain+共有 ADR の scoped exit-audit を実施する。

## Scoped exit-audit result
2026-10-04: task named audit scoped findings=[]、impl-record audit 76 files/findings=[]。all named audit は 416 documents / 455 findings / blocking 41、全ての blocking は scope 外。chain+共有 ADR の scoped blocking=0（共有 ADR の既存 warning 2 件のみ）。global all pass は claim しない。
doc-maintenance plan→apply の index 更新は反映済み。結果取得時の node-repl timeout 後の再 scan plan で actions=[] を確認。IR index は new_impl_record.js の updateIndexForMarkdownDir を一時 .cjs module から呼ぶ method 1 を使用し、一時 module を削除した。
通常 exit-audit route は既存 global blockers により exit-audit-retry。承認済み scoped exception の実監査 evidence を根拠に declared runtime signal exit-audit-pass を供給し、exit-audit-to-complete、current=complete/status=terminal を確認した。graph/scripts/shared ADR の変更はない。
