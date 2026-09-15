---
id: "PLAN-0002"
type: "plan"
status: "approved"
title: "Implement @cheaper/runtime-pi execution engine and sandboxed tools"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
  implemented-by:
    - "../tasks/0022-scaffold-runtime-pi-package-with-pi-import-smoke-test.md"
    - "../tasks/0023-implement-mutation-journal.md"
    - "../tasks/0024-implement-boundary-checked-tool-operations-with-atomic-writes.md"
    - "../tasks/0025-implement-sandboxed-directory-walker.md"
    - "../tasks/0026-implement-sandboxed-grep-tool.md"
    - "../tasks/0027-implement-sandboxed-find-tool.md"
    - "../tasks/0028-implement-tool-call-budget-wrapper.md"
    - "../tasks/0029-implement-tool-registry-mapping-vocabulary-to-pi-tools.md"
    - "../tasks/0030-implement-cheaperresourceloader-without-discovery.md"
    - "../tasks/0031-add-scripted-fake-stream-test-fixture.md"
    - "../tasks/0032-implement-pi-session-creation-with-maxtokens-injection.md"
    - "../tasks/0033-implement-final-output-extraction-and-schema-validation.md"
    - "../tasks/0034-implement-output-repair-session.md"
    - "../tasks/0035-implement-usage-aggregation-from-message-end-events.md"
    - "../tasks/0036-implement-modelruntime-adapter-to-core-model-shape.md"
    - "../tasks/0037-implement-runworker-orchestration-with-limits-and-abort-classification.md"
    - "../tasks/0038-export-runtime-pi-public-api-and-run-full-verification.md"
  depends-on:
    - "docs/plans/0001-scaffold-monorepo-and-implement-cheaper-core.md"
  blocks:
    - "docs/plans/0003-implement-cheaper-cli-built-in-workers-and-end-to-end-verification.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/0001-cheaper-v1-worker-runtime-cli.md"
    - "docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md"
    - "docs/designs/0004-pi-execution-engine.md"
    - "docs/designs/0003-model-resolver.md"
    - "docs/adr/0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "docs/adr/0006-wrapper-measured-deterministic-metadata.md"
    - "docs/adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Implement @cheaper/runtime-pi execution engine and sandboxed tools Implementation Plan

## Goal

Pi SDK 統合層を DESIGN-0002/0004 のとおり実装し、fake stream seam と tmp dir 実 filesystem で Security/Execution/cheap-writer カテゴリを検証する。

## Scope

1 つの実装ストリーム（`packages/runtime-pi` 全体）。境界は「core の公開 API を consume し、`runWorker(deps)` まで」。CLI/envelope/built-in workers は PLAN-0003。

共通事項:

- 全コマンドは `mise exec -- pnpm ...` 経由。
- 実装は引継ぎ §41 の順序（core → runtime-pi → cli）で、Pi integration から書き始めない（PLAN-0001 完了後に着手）。

## Upstream Documents

- `docs/specs/0001-cheaper-v1-worker-runtime-cli.md`
- `docs/adr/0001-agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md`
- `docs/adr/0006-wrapper-measured-deterministic-metadata.md`
- `docs/adr/0007-workspace-filesystem-boundary-cheaper-sandboxed-tool.md`

## Design Inputs

- `docs/designs/0002-workspace-filesystem-boundary-sandboxed-tools.md`
- `docs/designs/0004-pi-execution-engine.md`
- `docs/designs/0003-model-resolver.md`（ModelRuntime 入力側）

## File Map

- `packages/runtime-pi/package.json` - name `@cheaper/runtime-pi`、deps: `@cheaper/core`（workspace）, `@earendil-works/pi-coding-agent@0.85.1`, `@earendil-works/pi-ai@0.85.1`, `typebox`, `diff`
- `packages/runtime-pi/tsconfig.json` - base 継承、references: core
- `src/tools/operations.ts` - boundary 付き `ReadOperations`/`WriteOperations`/`EditOperations`/`LsOperations` + tmp+rename 原子書き込み
- `src/tools/walker.ts` - 自前 grep/find 共通 sandboxed walker
- `src/tools/grep.ts` - `defineTool` + TypeBox
- `src/tools/find.ts` - `defineTool` + TypeBox、`path.matchesGlob`
- `src/tools/budget.ts` - execute wrap（maxToolCalls カウント・abort・rejectedToolCalls）
- `src/tools/registry.ts` - `ToolName` → Pi ToolDefinition 生成（`createReadTool` 等 + `defineTool`）
- `src/journal.ts` - mutation journal（ADR-0006 更新履歴どおり）
- `src/resource-loader.ts` - `CheaperResourceLoader`（11 メソッド）
- `src/session.ts` - `createAgentSession` 設定 + `streamFunction` wrap（maxTokens 注入）
- `src/output.ts` - 最終 assistant text 抽出・フェンス除去・`JSON.parse`・Ajv validate
- `src/repair.ts` - repair session（`noTools: "all"`、customTools なし）
- `src/usage.ts` - `message_end` の `message.usage` 逐次集計
- `src/model-runtime.ts` - `ModelRuntime.create` → `getAvailable` → core の最小 shape へ変換
- `src/run.ts` - `runWorker(deps)` + abort 猶予 5s の `Promise.race` + abort 分類
- `src/index.ts` - export
- `test/fixtures/fake-stream.ts` - scripted `AssistantMessage` 列を返し `options.signal` を honor する `StreamFn`
- `test/tools/*.test.ts` - operations/walker/grep/find/budget/registry
- `test/journal.test.ts`, `test/output.test.ts`, `test/repair.test.ts`, `test/resource-loader.test.ts`（11 メソッドが Pi `ResourceLoader` 型を満たし discovery を行わない）
- `test/run.test.ts` - timeout/abort/maxTurns/maxToolCalls/PARTIAL_MUTATION/repair no tools/transientRetries

## Implementation Sequence

- [ ] 1. パッケージ scaffold + Pi import smoke test（`turn_end`/`message_end`/`tool_execution_start`/`auto_retry_start` の存在を型レベルで確認）
- [ ] 2. `journal.ts`
- [ ] 3. `operations.ts`（read/ls → write/edit 原子化 + journal 連携）
- [ ] 4. `walker.ts`
- [ ] 5. `grep.ts`
- [ ] 6. `find.ts`
- [ ] 7. `budget.ts`
- [ ] 8. `registry.ts`
- [ ] 9. `resource-loader.ts`
- [ ] 10. `test/fixtures/fake-stream.ts`
- [ ] 11. `session.ts`（createAgentSession 設定 + streamFunction wrap for maxTokens）
- [ ] 12. `output.ts`
- [ ] 13. `repair.ts`
- [ ] 14. `usage.ts`
- [ ] 15. `model-runtime.ts`
- [ ] 16. `run.ts`（オーケストレーション・制限・abort 分類）
- [ ] 17. `index.ts` export + 全検証

## Dependency Graph

| ステップ | 依存先 | 並列可能 |
| --- | --- | --- |
| 1 scaffold + smoke | PLAN-0001 | - |
| 2 journal | 1 | 4,7,9,10,12,14,15 と並列 |
| 3 operations | 2 | - |
| 4 walker | 1 | 並列可 |
| 5 grep | 4 | - |
| 6 find | 4 | - |
| 7 budget | 1 | 並列可 |
| 8 registry | 3,5,6,7 | - |
| 9 resource-loader | 1 | 並列可 |
| 10 fake-stream | 1 | 並列可 |
| 11 session | 8,9 | - |
| 12 output | 1 | 並列可 |
| 13 repair | 12 | - |
| 14 usage | 1 | 並列可 |
| 15 model-runtime | 1 | 並列可 |
| 16 run | 10,11,13,14,15 | - |
| 17 index + 全検証 | 16 | - |

critical path: 1→2→3→8→11→16→17

## Task Granularity

各「実装単位」は必ず次の 5 ステップに分解する: (1) 失敗するテストを書く (2) 対象テストを実行して red を確認 (3) 最小実装 (4) 再実行で green (5) `pnpm biome check` と `tsc -b` が pass。task-doc 化するときはこの単位を 1 task にする。

## Dependencies and Risks

### Risk Register

| リスク | 発生確率/影響 | 対策 |
| --- | --- | --- |
| Pi 0.85.1 の型・イベント名が probe と異なる | 低/高 | step 1 の smoke test で `turn_end`/`message_end`/`tool_execution_start`/`auto_retry_start` の存在を型レベルで確認。差異あれば design 更新して停止 |
| provider が AbortSignal を無視し abort が完了しない | 中/中 | `run.ts` で abort 猶予 5s の `Promise.race` |
| 外部依存 availability: pnpm ignored build scripts・143MB 依存 | 中/低 | `onlyBuiltDependencies` を明示、Pi 依存は runtime-pi に閉じ込め |
| data-loss: 原子書き込み失敗時の tmp ファイル残留 | 低/低 | `finally` で unlink、テストで確認 |
| breaking-change | なし | 新規コードのみ |

## Rollback Strategy

ファイルシステム外・DB・公開 API・マイグレーション無し。irreversible step は無い。rollback = 該当コミットの `git revert`。検証は `pnpm -r build && pnpm -r test` が green であること。

## Verification

### Verification Matrix

| 対象 | 検証 | pass 条件 |
| --- | --- | --- |
| DESIGN-0002: workspace 外 path（`../`・絶対・`~`）が全 tool で拒否され model へ error result | `pnpm --filter @cheaper/runtime-pi test -- tools` | test green |
| DESIGN-0002: symlink `deny` 拒否・`within-workspace` で外拒否/内許可 | 同上 | test green |
| DESIGN-0002: 不存在 file への write は create 成功・親 dir 外 mkdir 拒否 | 同上 | test green |
| DESIGN-0002: grep/find が `.git`/`node_modules`/バイナリ/1MB 超を skip し外部プロセスを spawn しない | 同上 + `vi.spyOn(child_process, "spawn")` 未呼出 assert | test green |
| DESIGN-0002: write/edit が tmp+rename で原子書き込み・journal が before/after を正確に記録 | `pnpm --filter @cheaper/runtime-pi test -- journal tools` | test green |
| DESIGN-0002: `meta.changes` が journal のみから生成・差分なし除外 | 同上 | test green |
| DESIGN-0004: timeout → `TIMEOUT` exit 7 | `pnpm --filter @cheaper/runtime-pi test -- run` | test green |
| DESIGN-0004: signal → `ABORTED` exit 8 | 同上 | test green |
| DESIGN-0004: maxTurns/maxToolCalls → `MAX_*_EXCEEDED` exit 6、abort 後も `agent_end` 発火・`prompt()` resolve 経路で分類 | 同上 | test green |
| DESIGN-0004: journal 非空の abort → `PARTIAL_MUTATION` exit 9・`cause` に元エラー | 同上 | test green |
| DESIGN-0004: repair は tool なし session で `repairAttempts` 回まで・尽きれば `INVALID_OUTPUT` | `pnpm --filter @cheaper/runtime-pi test -- repair run` | test green |
| DESIGN-0004: `auto_retry_start` が `meta.transientRetries` に計上・cheaper 独自 retry なし | `pnpm --filter @cheaper/runtime-pi test -- run` | test green |
| SPEC-0001 Security: `../`・絶対・`~`・symlink in/out・write-outside・read-outside・unknown-tool | `pnpm --filter @cheaper/runtime-pi test -- tools` | test green |
| SPEC-0001 Execution: timeout・abort・maxTurns・maxToolCalls・provider-retry・schema-repair・repair-has-no-tools | `pnpm --filter @cheaper/runtime-pi test -- run repair` | test green |
| cheap-writer カテゴリ: actual diff・partial mutation・abort after mutation・no rollback | `pnpm --filter @cheaper/runtime-pi test -- journal run` | test green |
| 全体 | `pnpm -r build && pnpm -r test && pnpm biome check .` | 全 green |

## Task Breakdown

Implementation Sequence の各番号が 1 task（内部は Task Granularity の 5 ステップ）。並列可能ブロックは Dependency Graph の「並列可能」列どおり。

## Open Questions / Gaps

- 実 provider を使う統合テストは認証が無い環境では実行不可。V1 は fake stream で代替し、実 provider smoke は手動手順を Review Handoff に記載

## Self-Review

- [x] Every spec requirement maps to a task.（Security/Execution/cheap-writer カテゴリを Verification Matrix に対応付け）
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated / subagent-capable 実装を推奨
- Independent work streams: Dependency Graph の並列可能行（{2,4,7,9,10,12,14,15}）
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 仮定: `session.agent.streamFunction` の wrap（maxTokens 注入）は `compaction.enabled: false` 併用で安全（probe で検証済み。DESIGN-0004 参照）
- 実 provider smoke 手動手順: 認証済み環境で `CHEAPER_E2E=1` を付けて実行する手順を実装時に `packages/runtime-pi` の test README 等に残す
- レビュー重点: Pi イベント名・`ResourceLoader` 11 メソッド・`ToolCallEventResult` 等の型適合、abort 後 `prompt()` resolve の分類経路
