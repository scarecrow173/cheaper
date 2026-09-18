---
id: "TASK-034r05iHQeQpDqeuofmk64"
type: "task"
status: "done"
title: "Hook fast-path and fail-closed FILE_TOO_LARGE block"
created: "2026-09-17"
updated: "2026-09-17"
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
    - "docs/plans/huge-file-byte-cap-guard-and-fail-closed-hook.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/cli-count-tokens-env-override-and-file-too-large-envelope.md"
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
---

# Hook fast-path and fail-closed FILE_TOO_LARGE block

## Work

`.apm/hooks/pre-read-code.js` に 2 変更:

1. fast-path: `statSync(resolved).size <= threshold()` なら CLI spawn
   せず `exit(0)`（`tokens ≤ bytes` の不変条件）
2. envelope 判定: stdout が parse できたら
   `envelope.error.code === "FILE_TOO_LARGE"` を先に検査し block
   （exit 2 + byte size を含む委譲 guidance）。その他の失敗は従来
   どおり fail-open

完了後 `pnpm bundle` で `.apm/hooks/cli/bin/cheaper.mjs` を再生成し
commit する（CI freshness 対象）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/huge-file-byte-cap-guard-and-fail-closed-hook.md`
- `docs/designs/huge-file-byte-cap-guard-and-fail-closed-hook.md`

## Dependencies

- `docs/tasks/cli-count-tokens-env-override-and-file-too-large-envelope.md`

## Affected Files

- `.apm/hooks/pre-read-code.js`
- `packages/cli/test/e2e/pre-read-hook.test.ts`
- `.apm/hooks/cli/bin/cheaper.mjs`（再生成）

## Done When

- [ ] `FILE_TOO_LARGE` envelope で exit 2 + 委譲 guidance
      （workspace-relative path 含む・非実行型テキストのみ）
- [ ] bytes ≤ threshold の file で spawn せず exit 0
- [ ] 既存の fail-open ケース（file 不在・workspace 外・infra 障害）が
      維持される
- [ ] bundle 再生成済み

## Verification

- [ ] `pnpm --filter cheaper test`（hook e2e 含む）が pass
- [ ] `pnpm bundle` 再生成との一致（freshness）
