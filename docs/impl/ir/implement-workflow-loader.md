---
id: "IMPL-034zTuWWp7bpifS4FoAiG2"
type: "impl"
status: "completed"
title: "implement-workflow-loader"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/workflow/src/loader.ts"
      - type: "file"
        path: "packages/workflow/test/loader.test.ts"
    modified:
      - type: "file"
        path: "packages/core/src/workspace/deny-paths.ts"
      - type: "file"
        path: "packages/core/test/workspace/deny-paths.test.ts"
      - type: "file"
        path: "packages/workflow/src/compiler.ts"
      - type: "file"
        path: "packages/workflow/src/index.ts"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/implement-workflow-loader.md"
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

# Workflow loader と invocation API

## Summary
承認済み Task A を開始。dispatch-specific override: supervised worker の明示 skill 指示に従い meta profile 探索を省略、inline RED/GREEN・focused verification・code review・Conventional Commit を実行する。既知の compiler adapter なので exp は不要。

## Context
承認済み workflow-file-interface spec/design/plan と共有 ADR D1/D5/D6 に従う。

独立 review で Windows deny casing 回避を再現し coordinator に ask した。2026-10-04 の回答で core の責務と判定され、loader 補正は NG、core deny-paths.ts の win32 のみ大小文字非依存化と focused test の最小修正を今回 slice の特例として許可された。core resource 型等の固定契約は変更しない。この追加修正前に Task A と IR を再開する。

## Implementation
実装前に IR と RED テストを作成し、workspace boundary/deny/symlink/regular-file/JSON の I/O 境界と compiler 委譲を実装した。validateInputs は compile と同じ validateDefinition/invocationInputs を使用し、barrel から loader と型を公開した。

## Related Experiments
なし（既知の compiler adapter）。

## Validation
baseline: workflow suite は 4 files / 75 tests GREEN。新規 loader test は未実装 export/API により RED。実装後 mise exec -- pnpm --filter @cheaper/workflow test は 5 files / 84 tests GREEN、mise exec -- pnpm typecheck は exit 0。差分 review で独自 schema validator がなく、同一 canonical compiler を返し、parse error に private JSON 本文が含まれないことを確認した（P0/P1 なし）。コード commit: 8417f2b。

## Risks
追加 core 修正は Windows/POSIX casing と実在 denied-file alias のテストを先に追加して RED を確認した。core の denyPatternToRegex に win32 限定の i flag を追加し、loader は既存 core port を使うまま。core workspace focused 46 passed/2 skipped、core package 24 files/370 passed/4 skipped、workflow package 5 files/85 tests、CLI unit 26/e2e 9 が GREEN。core dist rebuild 前の loader test は古い compiled dependency で失敗したが、pnpm build 後の再実行は pass。lint（既存 warnings 18）/typecheck/build pass。独立 read-only reviewer が追加差分を再レビューし P0/P1・minor なし。POSIX の比較・glob/anchor・公開 patterns 表現は維持した。コード commit: 6de1e1d。

run/resource inference と bundle は downstream/coordinator の責務。

## Follow-ups
#62 が loader と validateInputs を再利用する。canonical check deferred to coordinator merge run。
