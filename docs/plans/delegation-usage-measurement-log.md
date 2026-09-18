---
id: "PLAN-034r05iHE1Fbkpd5tkg6Gs"
type: "plan"
status: "completed"
title: "Delegation usage measurement log — implementation plan"
test-spec-skip: "Verified by unit tests and e2e assertions inside the implementation tasks; no separate test-spec artifact needed."
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/delegation-usage-measurement-log.md"
    - "../designs/delegation-usage-measurement-log.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../designs/delegation-usage-measurement-log.md"
  implemented-by:
    - "../tasks/implement-usage-log-writer-and-cli-wiring.md"
    - "../tasks/usage-log-e2e-docs-and-final-verification.md"
  depends-on:
    - "../adr/usage-jsonl-in-user-config-dir-best-effort-append.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../designs/delegation-usage-measurement-log.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  plan:
    effort: "small"
    verification-policy: "task-verified"
---

# Delegation usage measurement log — implementation plan

## File Map

| Path | Change | Reason |
|---|---|---|
| `packages/cli/src/usage-log.ts` | add | record 構築 + JSONL append（DESIGN-034r05iHy2gdnoXu6h71XL） |
| `packages/cli/src/cli.ts` | modify | run・count-tokens 経路から recordUsage 呼出し、root の out-param 追加 |
| `packages/cli/test/usage-log.test.ts` | add | record shape・append・swallow の unit test |
| `packages/cli/test/e2e/exec.test.ts` | modify | fake-stream run + count-tokens で log append assert |
| `packages/cli/test/e2e/real-provider-ollama.test.ts` | modify | run record の実測 assert 追加 |
| `AGENTS.md` | modify | 計測 log の場所・record shape の記載 |
| `packages/apm/.apm/hooks/cli/bin/cheaper.mjs` | generated | cli 変更に伴う bundle 再生成 |

## Tasks

- **TASK-034r05iHQeQpDqJKnzydzW**: `usage-log.ts` + cli.ts 配線 + unit test（TDD）
- **TASK-034r05iHQeQpDqKGWioCow**: e2e assert 追加（fake-stream・real-provider）・
  AGENTS.md 記載・bundle 再生成・status 同期・全量検証

## Dependency Graph

```
TASK-034r05iHQeQpDqJKnzydzW (writer + wiring + unit tests)
   └─> TASK-034r05iHQeQpDqKGWioCow (e2e + docs + bundle + final verification)
```

## Risk Register

- envelope 型の拡張ミス: record は CliEnvelope からの読み取りのみ
  （schema 不変）。typecheck で担保。
- 書き込み失敗で stdout 汚染: 呼出しは try/catch + stderr のみ。
  unit test で swallow を直接検証。
- bundle freshness 忘れ: `mise run check` の freshness gate で
  検出される。TASK-034r05iHQeQpDqKGWioCow で再生成してから最終検証。

## Rollback

- `usage-log.ts`・test・cli.ts の呼出し 3 箇所の revert で完全
  復帰。envelope schema・既存挙動への変更が無いため影響は限定的。

## Verification Matrix

| Requirement | Verification |
|---|---|
| run record append | unit test + fake-stream e2e assert |
| count record append | unit test + count-tokens e2e assert |
| best-effort swallow | unit test（書込不可 path）+ exit/stdout 不変 |
| 実測連携 | real-provider e2e で provider=ollama・usage.input>0 の record assert |
| 隔離 | 全 test で CHEAPER_CONFIG_DIR を tmpdir に向ける |
| 回帰 | pnpm test 全件・typecheck・lint・audit・freshness |
