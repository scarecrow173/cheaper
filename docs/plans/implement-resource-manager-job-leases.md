---
id: "PLAN-034AadQgwgheMAV59FAcIZ"
type: "plan"
status: "completed"
title: "ResourceManager job lease を実装する"
created: "2026-10-05"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/61"]
  verified-by: ["../test-specs/resource-manager-job-leases.md"]
  implements: ["../specs/resource-manager-job-leases.md", "../adr/resource-manager-job-leases.md"]
  derives-from: ["../designs/resource-manager-job-leases.md"]
---
# ResourceManager job lease を実装する

## Goal and Scope
承認済み spec/design の AC1–AC8 を tool-runtime と runtime-pi の一つの release stream として実装する。engine/process executor/OS tracer/cross-process locks は対象外。hot file・依存の変更はしない。

## File Map
| Path | Responsibility |
| --- | --- |
| packages/tool-runtime/src/resources.ts | opaque lease、all-or-none queue、borrow（foreign job token conflict は INVALID_REQUEST fail-fast）、stable admission、late release |
| packages/tool-runtime/src/access.ts | ResourceScopeOptions、scope normalization、writeScope/manifest 認可と mkdir predicate、bounded access snapshot |
| packages/tool-runtime/src/scheduler.ts | capacity/claims の atomic admission、queued preparation、original abort、settle release |
| packages/tool-runtime/src/invocation.ts | invocation-owned manager、acquireJobLease facade、parent validation |
| packages/tool-runtime/src/registry.ts | ready latch、prepared scope/observer、immutable submitted input、budget/meter glue |
| packages/tool-runtime/src/operation.ts | queued canonical resourceClaims 記録 seam、既存六状態維持 |
| packages/tool-runtime/src/index.ts | exports |
| packages/tool-runtime/src/read.ts, count-tokens.ts, walker.ts, grep.ts | actual I/O observer port（sniff/open 直前を含む） |
| packages/runtime-pi/src/run.ts | raw writeScope input extraction、session 前 ready、親借用維持 |
| packages/runtime-pi/src/tools/operations.ts, registry.ts | shared scope 認可/observer の配線、journal/deny 維持 |
| packages/tool-runtime/test/resources.test.ts | lease/scheduler/scope contract tests |
| packages/runtime-pi/test/resource-leases.test.ts | actual I/O、job lifetime、journal/deny 回帰 |
| docs/impl/ir/ の task 対応 record | red/green、review、scoped audit evidence |

## Dependency Graph
| Slice | Depends on | Blocks |
| --- | --- | --- |
| A: lease / scheduler / scope controller | approved spec-design | B |
| B: actual I/O / Pi wiring | A の focused suite、lint/typecheck/build | final review / exit audit |
同じ ResourceManager と observer contract が必要なので inline で順序実行する。

## Implementation Sequence
1. A の in-progress IR を作る。
2. independent mutation / 同一 file read-write 直列化 / borrow / all-or-none / abort / alias / early-release / record-only ready serialization / borrow-vs-foreign-token fail-fast / parent-blocked conflicting waiter bypass（job・operation 両方 + release 後 stable queue 復帰）/ readStrictness 継承・不一致 override 拒否 / unknown manifest→root exclusive fallback / process declared-IO lifetime を test として書く。
3. focused test を実行し missing behavior による RED を確認する。
4. core の canonical type/normalizer/predicate を使って A を実装する。
5. A の focused suite と lint/typecheck/build を実行し evidence を IR に記録する。
6. A の coherent logical change を commit する。
7. B の in-progress IR を作る。
8. Worker read→edit 間の親保護と boundary record/deny/journal を test として書く。
9. B の focused test を実行し RED を確認する。
10. actual boundary observer と runWorker ready を配線する。
11. tool-runtime/runtime-pi focused suite と lint/typecheck/build を実行し IR に記録する。
12. coordinator に implementation review を ask し、confirmed blocker があれば修正・再検証する。
13. B を commit、generated indexes を doc-maintenance plan/apply で更新、scoped exit-audit と graph terminal を確認する。

## Risk Register
| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| stale edit / scope 外 mutation | medium | high | job parent を session 前取得、canonical containment を I/O 前検証 |
| parent と child の循環待ち | medium | high | 全包含 borrow、全 foreign parent を取得判定、mixed FIFO + owned-parent bypass |
| late handler で早期 release | medium | high | actual settle まで子 token / 親 refcount を保持 |
| submitted input / manifest / Set が待機中に書換え | medium | high | capture 時 clone、normalized token/snapshot を immutable 化 |
| API/既存 fake-root fixture の breaking change | medium | medium | additive API、legacy global fast admission、runWorker 一引数不変 |
| 外部依存 availability | low | medium | 依存追加なし、locked setup 済み、実 provider は不要 |
| data loss | low | high | MutationJournal / atomic tmp / mkdir support の既存 contract を維持 |

## Rollback Strategy
schema migration・依存変更・外部 publish はない。regression があれば作業を止め coordinator に evidence と該当 logical commit を報告する。統合後の rollback は coordinator が該当 feature commit を git revert し、同じ focused suite と lint/typecheck/build を確認する。こちらから履歴の書換え・branch 削除・push はしない。

## Verification Matrix
| Slice | Command | Pass condition |
| --- | --- | --- |
| A RED | mise exec -- pnpm --filter @cheaper/tool-runtime test resources | 意図した missing behavior で失敗 |
| A GREEN | mise exec -- pnpm --filter @cheaper/tool-runtime test | 全対象 pass |
| B RED | mise exec -- pnpm --filter @cheaper/runtime-pi test resource-leases | 意図した I/O / lifetime behavior で失敗 |
| B GREEN | mise exec -- pnpm --filter @cheaper/runtime-pi test | 全 package pass |
| shared regression | mise exec -- pnpm --filter @cheaper/tool-runtime test | 全対象 pass |
| each slice | mise exec -- pnpm lint | exit 0 |
| each slice | mise exec -- pnpm typecheck | exit 0 |
| each slice | mise exec -- pnpm build | exit 0 |
| final | mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json | 自 chain + shared ADR に blocking finding 0、既存 global finding は報告のみ |

## Self-Review
AC1/3/4/5/6 は A、AC2/7/8 は B に対応。source imports は core に向け Pi を tool-runtime に持ち込まない。readStrictness は ScopeOptions の単一 resolver。CLI/root hot files に変更なし。

## Implementation Handoff
inline 実行（dispatch の明示指示: nested run/子 dispatch を作らない）。TDD と verification-before-completion を使い review は coordinator ask。profile/meta routing は prescribed chain を優先し、その例外を IR に記録する。canonical check deferred to coordinator merge run。

## Review Handoff
coordinator msg_b40bf1f1ee70: APPROVED: plan。追記された fail-fast / bypass / named contract test を含めて実装する。
spec-design は msg_84fd50c7a75a で承認。plan 承認後に linked test-spec と A/B task を作り、全証拠を durable report に保存する。post-implementation に新たな重大契約判断が必要なら先に coordinator を ask。
