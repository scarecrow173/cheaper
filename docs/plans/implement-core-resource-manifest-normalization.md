---
id: "PLAN-034zQqWCWWNpushz8Itm5d"
type: "plan"
status: "completed"
title: "Implement core resource manifest normalization"
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
    - "docs/specs/core-resource-manifest-normalization.md"
  implemented-by:
    - "../tasks/core-resource-manifest-tests.md"
    - "../tasks/core-resource-manifest-implementation.md"
    - "../tasks/core-resource-manifest-verification.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/core-resource-manifest-normalization.md"
    - "docs/designs/core-resource-manifest-normalization.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/core-resource-manifest-normalization.md"
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---


# Implement core resource manifest normalization

## Goal

共有 ADR D4/D5 の固定型を維持し、core に identity 正規化と pure conflict 判定を一元化する。

## Scope

core resources の単一 stream。lease、producer inference、共有型、paths.ts、policy、依存、hot file は変更しない。

## Upstream Documents

- docs/specs/core-resource-manifest-normalization.md（approved）
- docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md（accepted、D4/D5）

## Design Inputs

docs/designs/core-resource-manifest-normalization.md（approved、msg_51dda51d3eb5）

## File Map

| Path | Responsibility |
| --- | --- |
| packages/core/src/resources/normalization.ts | ScopeDeclaration / manifest input、strict validation、boundary real 解決、glob safety、ownership、fallback、安定 dedup |
| packages/core/src/resources/predicates.ts | resourceKey、resourceContains、resourcesOverlap、claimContains、claimsConflict、manifestsConflict の IO-free 判定 |
| packages/core/src/resources/index.ts | 固定型と API export |
| packages/core/src/index.ts | resources export 行のみ変更 |
| packages/core/test/resources.test.ts | AC1–5 / AC4a/4b の契約テスト（下記 Coverage の named case を全て含む） |
| docs/test-specs/core-resource-manifest-normalization.md | identity / safety / ownership / conflict 保証の意図 |
| docs/tasks/core-resource-manifest-*.md、docs/impl/ir/core-resource-manifest-*.md | 実行状態、RED/GREEN/check 証拠と graph checkpoint |

## Coverage

`docs/test-specs/core-resource-manifest-normalization.md` と `test/resources.test.ts` は AC1–5 / AC4a/4b に加え、spec Requirements と design §Data and Control Flow / §Symlink Safety Rule / §Canonical Manifest Admission の全規則を named case として列挙する。最低限:

- (a) bucket 別 omitted / `null` / `[]` / `resources` 省略・`{}` matrix（`produces: null` 拒否、`reads: []` = declared-none）
- (b) mode-hint・未知 field・declaration/claim 混在拒否、canonical claim の bucket-mode 不整合拒否
- (c) within-workspace `dir/*.json` → root subtree / deny → literal ancestor subtree / produces `dir/**` 非拡大
- (d) POSIX `d\ir/*.json` → root subtree fail-closed / Windows `dir\*.json` → dir subtree（deny）
- (e) `dir/**`+scope=subtree 受理、partial glob（`dir/**/file`・scope=exact `dir/**`）・glob tail `..` 拒否、leading `!` → root subtree
- (f) canonical idempotence（正規化済み manifest の再正規化が恒等）
- (g) 入力順非依存の deterministic sort+dedup
- (h) WorkspaceEscapeError は非 wrap で伝播、その他契約違反は `CheaperError(INVALID_REQUEST)`

## Implementation Sequence

- [x] A: 承認後 test-spec と依存順 tasks を作成する。
- [x] B: AC1–5 / AC4a/4b の failing tests を書く。
- [x] C: focused test を実行して未実装 API による RED を確認する。
- [x] D: 正規化、predicates、export を実装する。
- [x] E: focused test と core build を実行し GREEN を確認する。
- [x] F: mise run check を実行する。
- [x] G: terminal status、IR、doc-maintenance index を更新する。
- [x] H: focused exit-audit を実行し、論理境界で commit する。

## Dependency Graph

| Step | Depends on | Blocks |
| --- | --- | --- |
| A | plan approval | B |
| B | A | C |
| C | B | D |
| D | C | E |
| E | D | F |
| F | E | G |
| G | F | H |

critical path は A→H。単一契約の RED/GREEN 証拠を共有するため inline の順次実行。

## Task Breakdown

- core-resource-manifest-tests: tests を書き RED を確認する。
- core-resource-manifest-implementation: 前 task に依存して実装、focused GREEN、core build。
- core-resource-manifest-verification: 前 task に依存して canonical check、index、exit-audit。
各 task の作業項目と完了条件を明示し、最初の source/test 編集前に対応 IR を開く。

## Risk Register

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| データ消失 | low | low | 正規化は read-only boundary のみ、fixture cleanup は既存 helper |
| breaking change | low | medium | types.ts は変更せず API を追加、barrel の既存 export を保持 |
| external dependency availability | low | medium | 依存追加なし、setup 済み toolchain。Ollama smoke 実行条件を report |
| symlink under-claim | medium | high | within-workspace glob は root 拡大、producer 非拡大、literal subtree の限界を明記 |
| glob escape under-claim | medium | high | POSIX backslash meta、Windows separator、glob tail .. 拒否 |
| filesystem-root boundary | low | medium | paths.ts 欠陥は scope 外、coordinator が別 issue で追跡 |

## Rollback Strategy

永続 migration / irreversible step なし。追加 API の不具合を trigger とし、統合者が実装 commit を git revert して core build と mise run check を再実行する。履歴書換えは行わない。

## Verification Matrix

| Step | Verification | Pass condition |
| --- | --- | --- |
| A | task graph projection | cycle / unresolved relation なし |
| B,C | mise exec -- pnpm --filter @cheaper/core exec vitest run test/resources.test.ts | 未実装 module/export を理由に RED（baseline 291 pass / 2 skip） |
| D,E | 上記 focused command と mise exec -- pnpm --filter @cheaper/core build | AC1–5 / AC4a/4b pass、exit 0 |
| F | focused GREEN、lint/typecheck/build、失敗 CLI file 単独再実行 | coordinator 指示 msg_5463b4cce5a1 により full canonical は deferred-to-coordinator。初回 check と単独再実行の実測結果を保存 |
| G | doc-maintenance plan → apply、doc id check | 自 chain index / ID 整合 |
| H | focused graph exit-audit、git diff --check | exit-audit pass、空白エラーなし |

Windows 条件は現在 host で実行、POSIX 条件は skip として report。symlink fixture は既存 test と同じ junction / permission skip を使用。

## Open Questions / Gaps

未回答の実装判断なし。literal exact/subtree 内 symlink / hard-link / 外部 symlink 差替えの到達集合保証、filesystem-root boundary 欠陥は既存限界。共有 ADR owner への exclusion 引継ぎは coordinator に報告する。

## Self-Review

- [x] 全 spec requirement を tests / implementation / verification に割り当てた。
- [x] placeholder なし。
- [x] path / relation / status / API 名は承認済み design と一致。

## Implementation Handoff

inline execution。独立 stream なし、追加 worker を起動せず依存順に実行。coordinator の plan 承認後 test-spec / tasks / IR を作成する。
graph focus-required 例外は plan 作成前のみ coordinator 承認済み。今後は本 plan を focus に router 再開。

## Review Handoff

mode 導出と canonical claim 不整合拒否、policy 別 glob widening / producer 非拡大、POSIX escape、literal symlink residual-risk を重点確認。新規 dependency と hot file 変更なし。

## Verification Amendment 2026-10-04

coordinator 指示 msg_5463b4cce5a1: 4 worker の full check 並走による CLI host contention のため、worker の full canonical 再試行を停止。focused suite GREEN、失敗 file の単独再実行結果、lint/typecheck/build を worker の完了証拠とし、full canonical は統合時に coordinator が逐次実行する。bundle / APM freshness stage は worker で未到達。共有 ADR marker 3件の表示修正は追加の user pre-authorization に従う。AC2 / AC4b の POSIX 2件は Windows host 条件 skip として記録する。
