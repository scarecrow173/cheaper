---
id: "PLAN-034AEwZy3GcNygp87aXrL7"
type: "plan"
status: "completed"
title: "Implement managed process lifecycle hardening"
created: "2026-10-05"
updated: "2026-10-06"
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
    - "../specs/managed-process-lifecycle-hardening.md"
  implemented-by: ["../tasks/implement-process-lifecycle-policy.md", "../tasks/implement-process-tree-lifecycle.md", "../tasks/integrate-process-runtime-disposal.md"]
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/managed-process-lifecycle-hardening.md"
    - "../designs/managed-process-lifecycle-hardening.md"
    - "../adr/managed-process-bounded-lifecycle-policy.md"
  derived-by: []
  verifies: []
  verified-by: ["../test-specs/managed-process-bounded-lifecycle-guarantees.md"]
  references: ["../adr/managed-process-named-task-lifecycle.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Implement managed process lifecycle hardening Implementation Plan

## Goal

canonical managed process を、spawn 起点 timeout、configurable bounded capture、tree 終了 escalation、診断専用 heartbeat と実 settle を待つ dispose で強化する。

## Scope

policy → executor/backend → registry drain は一つの lifecycle 契約と critical path を共有するため一つの plan とする。既存 OperationManager、scheduler/process pool、resource ownership を再利用する。新しい pool、依存、native Job Objects、streaming Artifact Store writer、polling/LLM wake は追加しない。共通 brief に従い inline 実行し、nested dispatch は作らない。

## Upstream Documents

- spec: `docs/specs/managed-process-lifecycle-hardening.md`（approved、AC1–AC9）。
- ADR: `docs/adr/managed-process-bounded-lifecycle-policy.md`（accepted）。shared ADR D1/D2/D3/D5/D7 と #59 named-task 認可を維持する。

## Design Inputs

- `docs/designs/managed-process-lifecycle-hardening.md`（approved）。gate evidence: msg_6cc7f6088615、required W1 edits 反映済み。
- Windows 初回 identity snapshot 前に中間親が終了し ancestry を失う子孫は宣言済み limitation。adversarial containment escape と区別し、通常 root exit で追跡可能な child/grandchild は実 tree で検証する。
- policy.yaml ×2 defaults、16MiB core ceiling、heartbeat.interval project-min は同 verdict で許可済み。

## File Map

| Path | Responsibility | Notes |
| --- | --- | --- |
| `packages/core/src/policy/process.ts` | raw/resolved policy、defaults、duration/cap boundary と ceiling | top-level process.*、per-task override 拒否 |
| `packages/core/src/policy/schema.ts` | shared property schema と overflow 検証 | legacy complete omission と partial inherit を維持 |
| `packages/core/src/policy/merge.ts` | project-min の duration/byte 合成 | trusted replace を維持、元 duration 表現を選ぶ |
| `packages/core/src/policy/load.ts` | trusted → defaults → project restrict の同 resolver 利用 | 必要な型整合のみ |
| `packages/cli/policy.yaml`, `packages/apm/.apm/hooks/cli/policy.yaml` | 承認済み lifecycle defaults | bundled CLI を再生成しない |
| `packages/tool-runtime/src/process-tree.ts`（新規） | POSIX group と bounded Windows identity/taskkill backend | helper の cap/deadline/listener 回収 |
| `packages/tool-runtime/src/process.ts` | spawn/exit/abort timers、combined capture、idempotent cleanup と診断 | root exit から cleanup を開始し close を待つ。現行271–276行の cleanupError が signal.reason を覆う判定順を修正し、first-cause を先に返す |
| `packages/tool-runtime/src/registry.ts` | scoped onTimeout、managed promises の actual-settle drain | generic tool detach を変更しない |
| `packages/core/test/policy/process.test.ts`, `packages/core/test/policy/load.test.ts` | defaults/layering/invalid/per-task/cap 契約 | builtin assets の defaults と canonical default snapshot を確認 |
| `packages/tool-runtime/test/process.test.ts`, `packages/tool-runtime/test/fixtures/process-child.mjs` | timeout、queue、output、実 child/grandchild、grace、heartbeat、dispose | backend の専用 focused test が必要なら同 test directory に追加 |
| `packages/runtime-pi/test/process.test.ts`, `packages/runtime-pi/test/tool-runtime-contract.test.ts`, `packages/runtime-pi/test/run.test.ts`, `packages/runtime-pi/test/session.test.ts` | adapter の canonical policy/result/state と drain/settle の波及を検証 | canonical executor を再実装しない |
| `docs/guides/managed-process.md` | defaults、platform limitation、no-polling、dispose/diagnostic 契約 | managed dispose は abortGraceMs を超え得るが bounded |
| 自分の chain / generated indexes | tasks/test-spec/IR と evidence | maintenance plan → apply |

## Implementation Sequence

各 slice 開始時に task の in-progress IR を作り、変更と evidence を追記する。

1. policy default/validation/layering と asset defaults の回帰 test を記述する。
2. policy focused test の意味のある RED を確認する。
3. core policy/resolver/merge と承認済み asset defaults を実装する。
4. core focused test の GREEN を確認する。
5. policy の論理変更を commit する。
6. 実 tree fixture と capture/timeout/grace/terminal cleanup の回帰 test を記述する。
7. executor focused test の意味のある RED を確認する。
8. 小さい platform backend と executor-owned lifecycle を実装する。
9. executor focused test の GREEN を確認する。
10. scoped TIMEOUT/state、queue wait 除外、owner dispose、capacity と heartbeat isolation の回帰 test を記述する。
11. invocation focused test の意味のある RED を確認する。
12. registry の timeout cancel seam と managed invocation promise tracking を実装する。
13. runtime/adapter focused test の GREEN を確認する。
14. guide に承認済み platform/default/no-polling/dispose 契約を記述する。
15. focused package suites と lint/typecheck/build、関連 CLI test を実行する。
16. coordinator の implementation review を ask する。
17. required findings を修正して必要な検証を再実行する。
18. chain status と index を実測 evidence に合わせて更新する。
19. scoped exit-audit を実行する。
20. logical commits を完了し worker report と worker_done を送る。

### Commit Checkpoints

P は step 5、E は step 9 の GREEN 後、R は step 13 の GREEN 後に論理変更を commit する。guide と最終検証・review・status/index は step 20 で別 commit にする。未解決の failure を GREEN と記録しない。

## Dependencies and Risks

### Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| P: policy lifecycle contract | approved plan | E, R |
| E: executor/backend cleanup | P | R, final verification |
| R: runtime lifetime integration + guide | E | final review/audit |

critical path は P → E → R → review → audit。fixture/test preparation は独立して読めるが、同じ executor 契約を検証するため inline 順次実装する。

### Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| child が明示した outputs に不完全な file を残す data loss | low | medium | I/O 認可と scope は既存のまま。cleanup は process/pipe を対象とし workspace file を削除しない |
| lifecycle defaults の breaking behavior | medium | medium | legacy omission を補完、trusted override と project restriction を test、guide へ記載 |
| Windows helper の到達不能・deadline 超過 | medium | high | OS 標準 path、bounded helper、failure を明示し成功を偽装しない |
| Windows ancestry-loss W1 | medium | high | 宣言済み limitation を保持。通常 exit/abort の real child/grandchild coverage を落とさない |
| root exit 後の継承 pipe で close が停止 | medium | high | exit で cleanup を開始。force/failure の pipe teardown を finally で扱う |
| dispose が generic abortGraceMs より長い | medium | medium | managed promises を actual settle まで await、killGrace と helper deadline の bounded 処理、guide 明記 |
| Linux 実行環境が本 Windows checkout に無い | high | medium | Linux test を platform-aware に備え、未実行を report。canonical merge run に defer |

## Task Breakdown

- P: `implement-process-lifecycle-policy` — steps 1–5。core/asset 一つの policy boundary を変更する。
- E: `implement-process-tree-lifecycle` — steps 6–9。backend/capture/diagnostic timers と terminal cleanup。
- R: `integrate-process-runtime-disposal` — steps 10–20。OperationManager state/pool/dispose、adapter/guide と最終 review。
- 各 task の中の test 作成、RED、実装、GREEN、commit を別 checkbox にする。plan approval 後、まず behavior test-spec を作り tasks の verified-by にリンクする。

## Verification

### Verification Matrix

| Steps | Command / criteria | Pass condition |
| --- | --- | --- |
| 1–4 | `mise exec -- pnpm exec vitest run packages/core/test/policy/process.test.ts` | 新 test が RED → GREEN、defaults/merge/invalid/cap と AC7 |
| 6–9 | `mise exec -- pnpm exec vitest run packages/tool-runtime/test/process.test.ts` | RED → GREEN、AC2–AC6 の real tree/capture/diagnostic coverage |
| 10–14 | `mise exec -- pnpm exec vitest run packages/tool-runtime/test/process.test.ts packages/runtime-pi/test/process.test.ts` | AC1/AC5/AC6/AC9、queue timeout 除外、first-cause、owner isolation、実 settle まで pool 保持 |
| 15 | `mise exec -- pnpm exec vitest run packages/core/test/policy packages/tool-runtime/test packages/runtime-pi/test/process.test.ts packages/runtime-pi/test/tool-runtime-contract.test.ts packages/runtime-pi/test/run.test.ts packages/runtime-pi/test/session.test.ts` | affected focused suites exit 0、registry drain/settle の Pi adapter 波及を検証 |
| 15 | `mise exec -- pnpm exec vitest run packages/cli/test/hook/hook-contract.test.ts` | 関連 CLI focused check exit 0 |
| 15 | `mise exec -- pnpm lint` / `mise exec -- pnpm typecheck` / `mise exec -- pnpm build` | 各 exit 0 |
| 16–17 | coordinator implementation review | P0/P1 未解決なし、必要な再検証を evidence に保存 |
| 18–19 | `mise exec -- node .agents/skills/doc-status/scripts/audit_docs.js --type all --json` を own chain + shared ADR に scope | blocking 0、既存 global findings は変更しない |
| 5/20 | `git diff --check` と status/commit review | whitespace error なし、無関係変更/push/PR なし |

AC8: Windows の native tree tests を本 checkout で実行。POSIX graceful/no-force と Linux group tests は対応 platform でのみ実行し、skip/未実行を明記する。full canonical check は実行せず **canonical check deferred to coordinator merge run** と report に記載。

## Rollback Strategy

DB migration/data transformation/依存追加はない。変更は logical commits へ分割する。default incompatibility や cleanup regression が判明した場合、coordinator が該当 commit の通常 revert を選択し、上記 focused suite と typecheck/build を再実行する。履歴書き換え・branch 削除はしない。公開 lifecycle fields は削除の互換影響を評価してから revert する。

## Open Questions / Gaps

設計上の未決事項はない。W1、asset defaults、ceiling、heartbeat restriction と plan は承認済み。Linux の実行 evidence は本 checkout では未取得で、report の verification scope として扱う。

## Self-Review

- [x] AC1/AC9 は R、AC2–AC6 は E/R、AC7 は P、AC8 は E/R の platform-aware verification へ対応する。
- [x] placeholder を残さず、full-output artifact/native containment は明示的な non-goal とする。
- [x] paths、raw duration と ms 比較、top-level fields、declared limitation と status を upstream に一致させた。

## Implementation Handoff

- Recommended mode: inline execution。共通 brief の nested dispatch 禁止を適用する。
- Independent work streams: なし。policy/backend/runtime の critical path を順次扱う。
- review は coordinator ask、implementation record は task 開始時から更新する。

## Review Handoff

- scope/default/W1 は approved design の通り。新 native dependency を導入しない。
- review focus: timeout の spawn 起点と first-cause、root exit 後の close/descendant cleanup、capacity と owned dispose、diagnostic の非混入。
- plan approval 後に linked test-spec/tasks/IR を作成する。spec/ADR/design の承認 evidence は msg_6cc7f6088615。

## Approval

coordinator direct verdict（2026-10-05、settle により closed の ask msg_6d66657cbe77 への回答）: APPROVED plan gate、P0/P1 なし。required P2 の runtime-pi focused scope 拡大と P3 の first-cause reorder / per-slice commit checkpoints を反映した。

coordinator verdict（question msg_b41a1970f2fd、2026-10-06）: APPROVED implementation review + test-spec acceptance、P0/P1 なし。P/E/R を実装し focused 329 passed /1 POSIX-only skip、root lint/typecheck/build を確認。CLI の stale bundle parity failure 1件は coordinator merge 時の pnpm bundle + mise run check に defer することが承認されたため worker で bundle を再生成しない。canonical check deferred to coordinator merge run。
