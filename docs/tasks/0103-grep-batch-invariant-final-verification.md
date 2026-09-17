---
id: "TASK-0103"
type: "task"
status: "done"
title: "grep batch invariant final verification"
created: "2026-09-18"
updated: "2026-09-18"
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
    - "docs/plans/0018-grep-batch-max-bytes-invariant.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0018-grep-batch-max-bytes-invariant.md"
    - "docs/tasks/0102-enforce-grep-batch-max-bytes-invariant.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0023-grep-batch-max-bytes-invariant-enforcement.md"
    - "docs/specs/0018-grep-batch-max-bytes-line-payload-invariant.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# grep batch invariant final verification

## Work

TASK-0102 の実装に対し、PLAN-0018 の最終ゲートと SPEC-0018 の
acceptance criteria を検証し、結果を記録する。

### 1. 検証コマンド

```bash
mise exec -- pnpm typecheck
mise exec -- pnpm lint
mise exec -- pnpm --filter @cheaper/runtime-pi test
mise exec -- node .claude/skills/doc-status/scripts/audit_docs.js --type all
```

### 2. 構造確認

- `git diff` で `grep.ts` の batch 締めロジック・既定値・framer・
  `limits.ts` に変更がないことを確認する。
- `grep.ts` は bundled CLI に含まれないため bundle freshness は
  非影響のはず。`git diff -- .apm/hooks/cli` が空であることを
  確認する。

### 3. SPEC-0018 acceptance criteria の確認

- `batchMaxBytes` option 定義に `B >= 3 * L` contract と根拠が
  記載されている。
- 違反 option での構築が error になる test が存在し pass する。
- batch 締めの既存 test が invariant 適合 fixture で pass する。
- runtime-pi test suite 全件 pass。

## Acceptance

- [x] 上記全コマンドが pass し、結果が本 task に記録されている。
- [x] SPEC-0018 / PLAN-0018 の status 同期と index 更新が
      完了している。

## Result

検証済み（2026-09-18、`feature/grep-batch-max-bytes-invariant`）:

- `pnpm typecheck`: pass
- `pnpm lint`: pass（131 files checked）
- `pnpm --filter @cheaper/runtime-pi test`: 255 tests 全 pass
  （新規 reject 検証 + 適合 fixture の batch 締め test を含む）
- `audit_docs.js --type all`: 183 files, no findings
- `git diff` 構造確認: batch 締めロジック・既定値・framer・
  `limits.ts` に変更なし（batch 関連行 0 変更）。`.apm/hooks/cli`
  diff 空 — bundle 非影響（runtime-pi tool は bundled CLI 非含有）。

SPEC-0018 acceptance criteria 全項目充足。SPEC-0018 → implemented、
PLAN-0018 → completed に同期。
