---
id: "IMPL-034zCh1C1Qy6Pc2cvIFlOP"
type: "impl"
status: "completed"
title: "Add same-turn batching guidance to buildSystemPrompt and regenerate bundle"
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
    - "docs/tasks/add-same-turn-batching-guidance-to-buildsystemprompt-and-regenerate-bundle.md"
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

# Add same-turn batching guidance to buildSystemPrompt and regenerate bundle

## Summary

`buildSystemPrompt` を `export` 化し、Pi の sibling tool-call 並列実行を
活かす 5 本の exact 英文 guidance bullet（same-turn 発行・polling 禁止・
unsafe mutation batch 禁止・output token cap・final contract 不変）を
`Available tools:` と `Output schema:` の間に追加した（`toolNames` 非空
時のみ emit）。prompt 文言変更に伴い bundled CLI
（`packages/apm/.apm/hooks/cli/bin/cheaper.mjs`）を再生成した。

## Context

TASK-034zBJ55ACvdlADYOtaKKc（plan Phase B、T6–T13）。issue #55 /
SPEC-034zA513QKarQrr5SQtgp7 / DESIGN-034zAKcWuB8SuFkOePsubg に従う。
spec/ADR では runtime prompt guidance を同 issue の leverage 手段として
合意済み。

## Implementation

### Changes

- `packages/runtime-pi/src/run.ts`: `buildSystemPrompt` を `export` 化
  （signature・既存出力は不変）。`toolNames.length > 0` のとき design が
  定める 5 本の bullet を exact 文言どおり挿入。
- `packages/runtime-pi/test/run-prompt.test.ts`（新規）: 出力 contract・
  空 allowlist・5 bullet exact 文言・配置（Available tools 行と
  Output schema 間）・schema contract 後置を pin する 4 test。
- `packages/apm/.apm/hooks/cli/bin/cheaper.mjs`: `pnpm bundle` で再生成
  （guidance 文字列が bundle に含まれることを確認済み）。

### Decisions During Implementation

- bullet 文言は design（`DESIGN-034zAKcWuB8SuFkOePsubg`）の exact 英文を
  そのまま採用し、test 側も同じ文言を `toContain` で pin することで
  意図しない文言 drift を検出可能にした。
- `toolNames: []` で bullet 非 emit とし、「Available tools: none」と
  batching 指示の同居を構造的に防いだ（design 判断どおり）。

## Related Experiments

### Adopted

なし（known-solution task。Experiment Log は未作成）。

### Rejected

なし。

## Validation

- `mise exec -- pnpm --filter @cheaper/runtime-pi build` → exit 0
- `vitest run test/run-prompt.test.ts` → bullet 実装前に guidance 系
  2 test fail / baseline 2 pass（red 確認）、実装後 4 test 全 pass
- `mise exec -- pnpm --filter @cheaper/runtime-pi test` → 342 test 全 pass
- `mise exec -- pnpm typecheck` → exit 0
- `mise exec -- pnpm exec biome check --write`（対象 2 file）→ clean
- `mise exec -- pnpm bundle` → `cheaper.mjs` 再生成、
  `grep -c "same assistant turn"` = 1（guidance 含有確認）

## Risks

- guidance 文言と model の実挙動（実際に same-turn batch を組むか）の
  関連は本 IR では実測していない。spec/ADR で real-model adherence
  測定は acceptance criteria から外されている。
- `cheaper.mjs` は 12.3 MiB の bundled artifact で diff レビュー不可。
  生成は `scripts/bundle-cli.mjs` の deterministic output であり
  `mise run check` の `git diff --exit-code` が freshness を保証する。

## Follow-ups

- Task C（`docs/designs/pi-execution-engine.md` への section 追記）、
  Task D（`mise run check` + plan status 更新）へ進行。
