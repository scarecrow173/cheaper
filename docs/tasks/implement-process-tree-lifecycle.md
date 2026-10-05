---
id: "TASK-034AGiFXd7uNwKAYJMW2TP"
type: "task"
status: "done"
title: "Implement process tree lifecycle"
created: "2026-10-05"
updated: "2026-10-05"
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
    - "../plans/implement-managed-process-lifecycle-hardening.md"
  implemented-by:
    - "../impl/ir/implement-process-tree-lifecycle.md"
  depends-on:
    - "../plans/implement-managed-process-lifecycle-hardening.md"
    - "./implement-process-lifecycle-policy.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/managed-process-bounded-lifecycle-guarantees.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement process tree lifecycle

## Work

canonical executor に spawn 起点 timeout、bounded combined capture、runtime-only heartbeat と全 terminal path の idempotent cleanup を追加する。小さい POSIX group / Windows identity-taskkill backend に platform 差異を隔離する。

- [x] task の in-progress IR を作成する。
- [x] 実 parent/child/grandchild、normal/nonzero、grace/stubborn、cap/heartbeat の回帰 test を記述する。spawn failure は R の追加回帰で検証する。
- [x] focused test の意味のある RED を確認する。
- [x] backend と executor-owned timers/capture/cleanup を実装する。
- [x] focused test の GREEN と非実行 platform の skip を記録する。
- [x] executor logical change を commit する。

## Classification

- `normal-plan-task`

## Upstream Documents

- approved plan `../plans/implement-managed-process-lifecycle-hardening.md`、spec AC2–AC6/AC8 と accepted ADR。
- test-spec `../test-specs/managed-process-bounded-lifecycle-guarantees.md`。

## Dependencies

- P `./implement-process-lifecycle-policy.md` done が前提。R の state/dispose integration を block する。

## Affected Files

- `packages/tool-runtime/src/process.ts`, 新規 `process-tree.ts`。
- `packages/tool-runtime/test/process.test.ts`, `fixtures/process-child.mjs`、必要なら同 directory の backend focused test。
- root exit で cleanup を開始し、close と tree cleanup を待つ。signal.reason の判定を cleanupError より先にし、first-cause を保持する。
- 新 pool/native containment/dependency/artifact streaming API は追加しない。W1 は宣言済み limitation。

## Done When

- [ ] configurable cap/UTF-8/truncation、timeout/abort tree、normal/nonzero root exit の子孫、spawn failure の handles/timers cleanup を検証した。
- [ ] grace 内の exit は force-kill せず、stubborn tree は escalation 後に終了する。heartbeat は result/child stderr を汚染せず settle 後に停止する。
- [ ] Windows の実 tree evidence と POSIX-only skip/未実行を分けて IR に記録した。

## Verification

- [ ] `mise exec -- pnpm exec vitest run packages/tool-runtime/test/process.test.ts` exits 0。
- [ ] `git diff --check` exits 0。
