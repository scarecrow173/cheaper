---
id: "IMPL-034z57lgR0CQbaWKMcKP6v"
type: "impl"
status: "completed"
title: "remove legacy js adapters and final verify"
created: "2026-10-02"
updated: "2026-10-02"
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
    - "docs/tasks/remove-legacy-js-hook-adapters-and-run-final-verification.md"
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

# remove legacy js adapters and final verify

## Summary

Node ベースの JS hook adapter 6 file を削除し、`cheaper hook`
経路への移行を完了させた。hook payload から Node 依存が消え、
bundle を再生成して最終検証を実施した。

## Context

- task: docs/tasks/remove-legacy-js-hook-adapters-and-run-final-verification.md

## Implementation

### Changes

- `packages/apm/.apm/hooks/` から 6 file を削除:
  `pre-read-code.js`・`pre-run-command.js`・
  `pre-read-code-copilot.js`・`pre-read-code-pretooluse.js`・
  `pre-read-code-cursor.js`・`routing-policy.js`
- `packages/cli/test/e2e/apm-install-targets.test.ts`:
  bundle 内容 assert を adapter .js 群から launcher（`cheaper`・
  `cheaper.ps1`）と `cli/bin/win32-x64/cheaper.exe` の存在確認に
  置き換え
- `pnpm bundle` で `cli/bin/cheaper.mjs` を再生成（biome 修正済み
  source を反映する差分を含む）

### Decisions During Implementation

- `cheaper.mjs` は削除せず残置: platform executable が無い環境での
  手動 fallback として SKILL.md / parent-agent-integration に
  明記済み
- 30s timeout の復元（`AbortSignal.timeout(30_000)`）は
  routing-policy 側に実装済み — in-process 化で失う
  subprocess kill cap の等価復元として敵対的レビューの指摘を
  採用した経緯は task 1 の IR に記録
- `usage.jsonl` 非記録は意図的決定として `docs/guides/usage.md`
  と SKILL.md に明記

## Validation

- SEA exe（Node 非依存）で全 target の deny contract を実機確認:
  windsurf read/command → stderr + exit 2、copilot →
  `permissionDecision: deny` JSON + exit 0、pretooluse →
  `hookSpecificOutput` deny JSON + exit 0、cursor →
  `permission: deny` JSON + exit 0
- `mise run check`（lint・version・doc-id・typecheck・全 package
  test・bundle freshness）— 最終結果は下記 Follow-ups と照合

## Risks

- macOS executable は未同梱（issue #71 のスコープ外）。macOS
  consumer では launcher が platform 判定で失敗するため
  Copilot/Cursor 経路は shell 側 fail-open で許容、Windsurf は
  exit 2 伝播で block になる既知の制約（design で承認済み）
- remote install で LFS pointer が smudge されない場合の防御は
  launcher の pointer 検出で対応済み

## Follow-ups

- Phase 4 plan の status を completed に更新
- issue #71 全体の残スコープ（macOS matrix 等）は issue 側で管理
