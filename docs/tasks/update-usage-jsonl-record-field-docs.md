---
id: "TASK-034Kr8pszGdyClSDbgMU3Q"
type: "task"
status: "todo"
title: "Update usage.jsonl run record field documentation"
created: "2026-10-22"
updated: "2026-10-22"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/141"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "AGENTS.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-run-error-record-observability.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-run-error-record-observability.md"
    - "docs/tasks/add-usage-log-diagnostic-field-tests.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-run-error-record-observability.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/run-error-record-observability.md"
  defers: []
  deferred-by: []
---

# Update usage.jsonl run record field documentation

## Work

plan `docs/plans/implement-run-error-record-observability.md`
(PLAN-034Kr7PAaZlxMx7LjG34Ls)の T4。T1–T3 で確定した record field
contract を user-facing 文書へ追随させ、`mise run check` で plan を
閉じる。

### 1. `docs/guides/usage.md` — 「計測 log」節の record 記述

- 「run record には加えて `turns`・`toolCalls`・`savings`… が載る」
  の段落(:251-255 付近)に `version`・`repairs`・`execution`・
  error record の `error` object 拡張を追記する:
  - `version` — run を実行した binary の product version
    (workspace `Cargo.toml` の canonical version)。新旧版の識別用。
  - `execution` — run に適用された**実効** policy echo
    `{timeoutMs, maxTurns, maxToolCalls}`(`--timeout` narrow・
    policy 層 merge を通った解決後の値。`maxTurns` は `null`
    許容 — 未設定ではなく policy 上の上限なし)。
  - `repairs` — INVALID_OUTPUT の repair 消費数(non-null のみ)。
  - error record の `error` object — `code` に加えて条件付きで
    `reason`(abort 語彙のみ)・`message`(≤300 文字)・
    `schemaErrors`(≤5 件の `{instancePath ≤200 文字,
    message ≤120 文字}`)・`errorMessage`(≤300 文字)・
    `checkpoint`(`{path, observationsCount}`、best-effort で
    欠落しうる)。
- **`execution` の命名衝突を明示**: record の `execution` は
  policy echo であり、envelope `meta.execution`
  (`ExecutionMetrics` = 観測 metrics、record には現行も載らない)
  とは別概念 — 同名だが面が違うことを usage.md で明記する
  (design §record schema の要求)。
- 記録対象条件・best-effort 規約・`schemaVersion: 1`・
  additive-only 混在の記述は不変であることを確認する。

### 2. `AGENTS.md` — 「delegation 計測ログ」節の記述

- record field の列挙部分(`kind:"run"`(worker・model・usage・
  durationMs)…`turns`・`toolCalls`・`savings`)に `version`・
  `repairs`・`execution` echo・error record の診断 field を追随
  させる。簡潔な列挙の追記で十分(詳細は `docs/guides/usage.md`
  への参照を維持)。

### 3. 最終検証

- `mise run check` を実行して plan 全体の pipeline(biome・
  fmt・clippy・version/doc-id check・workspace test・
  lockfile/slim payload check)が pass することを確認する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-run-error-record-observability.md` — 本 task
  が implement する plan。File Map・Dependency Graph の T4
- `docs/designs/run-error-record-observability.md`
  (DESIGN-034KqI29dOTRXERRPTp5gV)— §record schema・命名衝突
  注記の要求
- `docs/specs/run-error-record-observability.md`
  (SPEC-G1ALq0d5lyFIi0h43tZsZG)— Req 5・6・7(既存 contract の
  不変性)

## Dependencies

- `docs/tasks/add-usage-log-diagnostic-field-tests.md`
  (TASK-034Kr8oUgcvTuprTQJwuyp)— verify 済みの contract を文書化
  する位置づけ。

## Affected Files

- `docs/guides/usage.md`(modify)
- `AGENTS.md`(modify)

## Done When

- [ ] `usage.md` の run record field 記述が `version`・`repairs`・
      `execution`・error `error` object 拡張を含む
- [ ] `execution` の命名衝突(record = policy echo vs envelope
      `meta.execution` = metrics)が明記されている
- [ ] `AGENTS.md` の計測ログ記述が新 field に追随している
- [ ] `mise run check` が pass する

## Out of Scope

- `cheaper usage` の error 内訳集計・`--error` filter の文書化
  (別 issue、spec Open Item)
- spec・ADR・design 自体の更新(upstream は確定済み)

## Verification

- [ ] `mise run check` exits 0
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `git grep -n "execution" docs/guides/usage.md` が record の
      `execution`(policy echo)と envelope `meta.execution`
      (metrics)の区別を含む記述を返す
