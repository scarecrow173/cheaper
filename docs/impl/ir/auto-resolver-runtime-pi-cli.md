---
id: "IMPL-034rC4jywjUQYNL42ruw7Q"
type: "impl"
status: "completed"
title: "auto 分類の resolver / runtime-pi / CLI 統合"
created: "2026-09-19"
updated: "2026-09-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/runtime-pi/src/model-runtime.ts"
      - type: "file"
        path: "packages/runtime-pi/src/run.ts"
      - type: "file"
        path: "packages/runtime-pi/test/model-runtime.test.ts"
      - type: "file"
        path: "packages/runtime-pi/test/run.test.ts"
      - type: "file"
        path: "packages/cli/src/args.ts"
      - type: "file"
        path: "packages/cli/src/cli.ts"
      - type: "file"
        path: "packages/cli/src/text-format.ts"
      - type: "file"
        path: "packages/cli/test/main.test.ts"
      - type: "file"
        path: "packages/core/src/model/resolve.ts"
    deleted: []
    renamed: []
    moved: []
    generated:
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/bin/cheaper.mjs"
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/workers/bulk-reader.md"
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/workers/code-writer.md"
      - type: "file"
        path: "packages/apm/.apm/hooks/cli/workers/doc-writer.md"
  implements:
    - "docs/tasks/wire-auto-classification-into-resolver-and-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/auto-tier-classification.md"
    - "docs/designs/auto-tier-classification.md"
    - "docs/impl/ir/worker-workload-profile-auto-tier-classifier-core.md"
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

# auto 分類の resolver / runtime-pi / CLI 統合

## Summary

core の auto tier 分類を runtime-pi と CLI に配線し、worker の
`workload.outputPerInput` を model 解決へ伝播させた。選択 metadata
（`meta.model`・`cheaper models` 出力）に auto 分類根拠
（`effectiveCost`・`effectiveCostPercentile`・`workload`）を追加した。

## Context

- task: `docs/tasks/wire-auto-classification-into-resolver-and-cli.md`
- spec / design: `docs/specs/auto-tier-classification.md` /
  `docs/designs/auto-tier-classification.md`

## Implementation

### Changes

- `runtime-pi/model-runtime.ts`: `resolvePiModel` /
  `resolveExplicitPiModel` が `outputPerInput` を受け core へ転送。
  `ResolvedPiModel.auto` で `AutoClassificationDetail` を返す。
- `runtime-pi/run.ts`: `worker.cheaper.model.workload?.outputPerInput`
  を tier / explicit 両経路へ渡す。`WorkerRunMeta.model` に
  `effectiveCost`・`effectiveCostPercentile`・`workload` を追加
  （`tierSource === "auto"` のときのみ存在）。
- `cli/args.ts`: `cheaper models --worker <name>` を追加
  （worker name 検証は run と同一ルール）。
- `cli/cli.ts`: `executeModels` が `--worker` の workload を解決し、
  exclude 適用後の eligible 集合で `buildAutoTierContext` を構築して
  batch 分類。行に auto 詳細を出力。help に `--worker` を追記。
- `cli/text-format.ts`: text 行に `eff=$X/1M p=Y opi=Z` を表示。
- bundled CLI / bundled worker 定義を `pnpm bundle` で再生成。

### Decisions During Implementation

- `cheaper models` の ranking 母集団は exclude 適用後の集合とし、
  excluded 行の表示用分類は同じ context で行う（母集団には入れない）。
- `--model` 経路でも `outputPerInput` を渡すのは、policy の maxTier
  判定に使う分類を worker workload 基準に揃えるため（候補選択自体は
  bypass のまま）。
- 実 catalog は環境依存の model を含むため、run.test.ts の auto 分類
  検証は catalog を fake provider に限定して決定的にした。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise run check`: lint / version / doc-ids / typecheck / test /
  bundle freshness すべて pass。
- `pnpm --filter @cheaper/runtime-pi test`: 28 files / 324 tests pass。
- `pnpm --filter cheaper test`: 17 files / 168 tests pass
  （Ollama 実 provider e2e 3 件含む）。
- 並列負荷時に e2e `beforeEach`（apm install）が 10s hookTimeout を
  超える既知 flake があり、`VITEST_MAX_WORKERS=2` で回避して全件
  pass を確認（単体実行でも pass）。

## Risks

- 環境依存 catalog を持つ runtime では percentile が実行環境で変わる
  （設計上の相対 ranking の性質であり、deterministic は同一入力に
  対してのみ保証）。

## Follow-ups

- なし。
