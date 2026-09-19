---
id: "PLAN-eDDZBS6ncoMf6TOdM7tDSW"
type: "plan"
status: "completed"
title: "implement auto tier classification"
created: "2026-09-20"
updated: "2026-09-20"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/46"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/auto-tier-classification.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/price-based-percentile-auto-tier-classification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/auto-tier-classification.md"
    - "docs/designs/auto-tier-classification.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/auto-tier-classification.md"
  references: []
  defers: []
  deferred-by: []
---

# implement auto tier classification Implementation Plan

## Goal

spec（SPEC-H9CuNdGONsV6HEHetvRCxh）と design
（DESIGN-C4VbdOatUn4aVt3blVe8pk）どおりに、Worker workload
profile・effectiveCost ranking・midrank percentile による auto tier
分類を実装し、interim heuristic を置き換える。

## Scope

単一の implementation stream。core（worker schema + classifier）→
resolver / runtime-pi / cli → guide 更新の依存順で 1 plan にまとめる。
cache pricing・boundary config 化・telemetry 検証はこの plan の外。

## Upstream Documents

- `docs/specs/auto-tier-classification.md`
- `docs/adr/price-based-percentile-auto-tier-classification.md`
- `docs/designs/auto-tier-classification.md`

## File Map

| File | Change |
| --- | --- |
| `packages/core/src/worker/validate.ts` | `cheaper.model.workload.outputPerInput` schema・`WorkerDefinition` 拡張 |
| `packages/core/src/model/classify.ts` | interim heuristic 削除、`DEFAULT_OUTPUT_PER_INPUT`・`AutoClassificationDetail`・`buildAutoTierContext`・`classifyModels` 追加 |
| `packages/core/src/model/resolve.ts` | `outputPerInput` opt・母集団分類・`ResolvedModel.auto` |
| `packages/runtime-pi/src/model-runtime.ts` | `outputPerInput` 伝播・`ResolvedPiModel.auto` |
| `packages/runtime-pi/src/run.ts` | worker workload 取得・`meta.model` explainability field |
| `packages/cli/src/args.ts` | `models --worker` option |
| `packages/cli/src/cli.ts` | `executeModels` の worker workload 適用・行への分類根拠 |
| `packages/cli/src/text-format.ts` | models 表示の拡張（必要最小限） |
| `packages/cli/workers/*.md`・`packages/apm/.apm/hooks/cli/workers/*.md` | built-in `outputPerInput` |
| `packages/core/test/model/classify.test.ts` 他 | ranking ベーステストへ置換・追加 |
| `docs/guides/workers.md`・`docs/guides/configuration.md`・`docs/guides/usage.md` | workload・`--worker`・meta field の説明 |

## Dependency Graph

```text
task: add-worker-workload-profile-and-auto-tier-classifier
  → task: wire-auto-classification-into-resolver-and-cli
    → task: update-guides-for-auto-tier-classification
```

## Risk Register

- `classifyModel` の signature 変更が既存呼出（cli models）に波及
  → batch/context API で後方互換を保つ
- boundary の float 誤差 → 整数比較で回避
- built-in `outputPerInput` は推定値 → IR に根拠と follow-up を記録

## Rollback

- 各 task は独立 commit。core classifier 差し戻しは task1 の revert、
  resolver/cli 差し戻しは task2 の revert で可能（task2 が task1 の
  API に依存するため順序逆では不可）。

## Verification Matrix

| Acceptance Criteria | 検証 |
| --- | --- |
| outputPerInput 設定・default fallback | worker validate test |
| effectiveCost 算出 | classify test |
| deterministic ranking・percentile | classify test |
| tie・small set | classify test |
| precedence（override/curated > auto） | classify/resolve test |
| pricing 欠落・価格ゼロ → unclassified | classify test |
| `--model` bypass | resolve/cli test |
| explainability metadata | run/models test |
| `mise run check` | canonical verification |

## Completion Evidence

- 全 task（3/3）done。Implementation Record:
  `docs/impl/ir/worker-workload-profile-auto-tier-classifier-core.md`、
  `docs/impl/ir/auto-resolver-runtime-pi-cli.md`、
  `docs/impl/ir/auto-tier.md`。
- `mise run check` pass（lint / version / doc-ids / typecheck /
  288+324+168 tests / bundle freshness）。Ollama 実 provider e2e 3 件も
  pass。
- 並列負荷時に apm install 系 e2e の `beforeEach` が 10s hookTimeout
  を超える既知 flake を確認。`VITEST_MAX_WORKERS=2` で全件 pass、
  単体実行でも pass。コード起因ではない。

## Follow-ups（graph 外に持ち越し）

- built-in worker の `outputPerInput` 初期値（0.05 / 0.3 / 1.0）は
  暫定推定。issue #44 の usage telemetry（`usage.jsonl` の
  input/output token 実測値）で検証・補正する follow-up を
  `IMPL-034rC4j788MZLcUE8xHHP2` に記録。本 issue の完了条件には
  含まれないため `followup-terminal` で triage。
