---
id: "PLAN-034zTiB05IwVw2pDB34sLk"
type: "plan"
status: "completed"
title: "Workflow loader と公開 interface CLI を接続する"
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
    - "../specs/workflow-file-interface.md"
  implemented-by: ["../tasks/implement-workflow-loader.md", "../tasks/wire-workflow-interface.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/workflow-file-interface.md"
    - "../designs/workflow-file-interface.md"
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/workflow-loader-contract.md", "../test-specs/workflow-public-interface.md"]
  references: []
  defers: []
  deferred-by: []
---

# implement-workflow-file-interface Implementation Plan

## Goal

#69-A の明示 Workflow JSON loader と #70 の compact interface CLI を既存 compiler 経路へ接続する。

## Scope

loader → CLI の単一 critical path・同一 release intent のため一 plan とする。run/engine、bundle、依存/config/hot file の追加変更は対象外。cli.ts/args.ts は承認済み interface 追加の最小差分のみ。

## Upstream Documents

- SPEC-034zSVZlWjFAMpwmhsLTkx と共有 ADR-034zGZpjoQlppwkkOs5gur D1/D5/D6。

## Design Inputs

- DESIGN-034zT2MINVpakJb3AhS0m5（coordinator APPROVED）。metadata error は catalog.get まで保留して canonical 検証優先順位を維持する。

## File Map

- coordinator 特例許可の追加修正: `packages/core/src/workspace/deny-paths.ts` と `packages/core/test/workspace/deny-paths.test.ts`。Windows casing 回避を matcher 所有境界で修正し、loader の規則複製を防ぐ。

- `packages/workflow/src/loader.ts`: boundary/deny/regular-file/JSON parse と compiler factory/validateDefinition 委譲。WorkspaceEscapeError/deny/読込失敗をここで INVALID_REQUEST に変換する。
- `packages/workflow/src/compiler.ts`: validateInputs の公開、既存 invocationInputs を共有。
- `packages/workflow/src/index.ts`: loader と型の barrel export。
- `packages/workflow/test/loader.test.ts`: file/dynamic 同一 compiler 経路、error/deny/boundary/input API の契約。
- `packages/cli/src/args.ts`: interface kind、option token による限定受理。
- `packages/cli/src/cli.ts`: policy/metadata composition、projection/utility envelope、HELP/error format path。
- `packages/cli/test/workflow-interface.test.ts`: command/parser/envelope/metadata precedence/非実行。
- `packages/cli/test/e2e/workflow-interface.test.ts`: build 済み CLI の process exit/stdout/text/error。

## Implementation Sequence

1. loader/input validation の契約テストを書く。
2. focused suite の failing-first を確認する。
3. loader と validateInputs/export を実装する。
4. workflow focused suite を実行する。
5. CLI の parser/utility/error/non-execution テストを書く。
6. CLI test 単独 failing-first を確認する。
7. args と CLI 接続を実装する。
8. CLI unit/e2e の対象 file を各単独実行する。
9. lint/typecheck/build を実行し、実装記録と scoped audit を更新する。
10. reviewable な loader/API と CLI の論理変更を各 Conventional Commit にまとめる。

## Dependencies and Risks

| Step | Depends on | Blocks |
| --- | --- | --- |
| loader/API | #64 統合済み | CLI |
| CLI | loader/API | 統合検証 |
| 統合検証 | CLI | exit-audit |

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| data loss | low | low | 読込専用。tool/Worker を実行しない |
| breaking change | low | medium | 既存 compiler 型/CLI command を維持、focused 契約テスト |
| external dependency availability | low | low | model/network 不要、依存変更なし |
| metadata error が structural error に先行 | medium | medium | error を catalog.get まで保存、priority テスト |

## Rollback Strategy

誤った公開契約または CLI regression を検出したら該当論理 commit を revert し、workflow/CLI 対象 suite と build を再実行する。migration/永続 data 変更はない。

## Task Breakdown

- Task A: loader と invocation API、契約テスト、IR を一つの coherent slice とする。
- Task B: A に依存する interface CLI 接続と unit/e2e、IR。

## Verification

| Step | Command | Pass condition |
| --- | --- | --- |
| A | `mise exec -- pnpm exec vitest run packages/workflow/test` | loader/API と既存 workflow suite GREEN |
| B | `mise exec -- pnpm exec vitest run packages/cli/test/workflow-interface.test.ts` | 対象単独 GREEN |
| B | `mise exec -- pnpm exec vitest run --config packages/cli/vitest.config.ts packages/cli/test/e2e/workflow-interface.test.ts` | built CLI の単独 GREEN |
| 統合 | `mise exec -- pnpm lint` / `pnpm typecheck` / `pnpm build` | 各 exit 0 |
| 文書 | doc-maintenance plan/apply、chain + shared ADR scoped exit-audit | scoped blocking findings 0 |

canonical check deferred to coordinator merge run。bundle は coordinator が refresh。

## Open Questions / Gaps

- 実装 blocker なし。run/resource inference は downstream issue の scope。

## Self-Review

- [x] AC1/2/5/8 → B、AC3/4/6/7 → A と B。
- [x] placeholders なし。型名・path・D6 error table が spec/design と一致。

## Implementation Handoff

- Recommended mode: inline execution（supervised worker 内）
- Independent work streams: なし。B は A に依存する。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- metadata precedence、option --input - の reject、deny/escape code、private 非公開を重点レビューする。
- 承認済み graph 運用例外: spec/design scoped audit + coordinator 承認を維持し、この plan の focus で graph を再開する。router/shared ADR は変更しない。

2026-10-04: coordinator APPROVED: plan approval（P0/P1 なし）。path filter と error 変換層の記載を反映。

## Completion Evidence

独立 review で Windows deny casing 回避を検出し、一時 script で blocked は拒否・Blocked は受理と再現した。coordinator ask msg_69bef75f648c の回答で core の責務と判定され、core matcher の局所修正を特例許可。Task A/IR を再開し、implementation-retry で repair を実施した。修正後 core package 24 files/370 passed/4 skipped、workflow 5 files/85 tests、CLI unit 26/e2e 9 が GREEN、lint/typecheck/build pass。独立再レビューは P0/P1・minor なし。core 資源型・共有 ADR 等は変更していない。

2026-10-04: Task A/B done、IR completed。workflow 5 files/84 tests、CLI unit 26、e2e 9、既存 args 31/main 22 が GREEN。lint/typecheck/build pass。doc-maintenance の type 別 plan/apply と impl-doc の updateIndexForMarkdownDir 再生成を実施。IR audit findings 0、chain + 共有 ADR の scoped blocking findings 0（共有 ADR の既存 reciprocal warning 2 件のみ）。global は 416 documents/455 findings/41 blocking で、別 chain の既存 findings は ownership 外。

Graph trace: implementation-to-followup-triage → followup-triage-terminal → exit-audit。followup は対象 slice 内に新規 blocker なしと分類し、#62 の run/engine と bundle refresh/canonical verification は既定 downstream/coordinator の責務として維持する。canonical check deferred to coordinator merge run。
