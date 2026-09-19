---
id: "IMPL-034rC4j788MZLcUE8xHHP2"
type: "impl"
status: "completed"
title: "Worker workload profile と auto tier classifier の core 実装"
created: "2026-09-19"
updated: "2026-09-19"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/core/src/model/classify.ts"
      - type: "file"
        path: "packages/core/src/model/resolve.ts"
      - type: "file"
        path: "packages/core/src/worker/validate.ts"
      - type: "file"
        path: "packages/core/test/model/classify.test.ts"
      - type: "file"
        path: "packages/core/test/model/resolve.test.ts"
      - type: "file"
        path: "packages/core/test/worker/validate.test.ts"
      - type: "file"
        path: "packages/cli/workers/bulk-reader.md"
      - type: "file"
        path: "packages/cli/workers/code-writer.md"
      - type: "file"
        path: "packages/cli/workers/doc-writer.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-worker-workload-profile-and-auto-tier-classifier.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/auto-tier-classification.md"
    - "docs/designs/auto-tier-classification.md"
    - "docs/adr/price-based-percentile-auto-tier-classification.md"
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

# Worker workload profile と auto tier classifier の core 実装

## Summary

issue #46 の core 部分を実装した。worker frontmatter に
`cheaper.model.workload.outputPerInput`（平均 output token / input
token 比、省略時 `1.0`）を追加し、`classify.ts` の interim
output-price heuristic を、`effectiveCost = input + output *
outputPerInput` の母集団相対 percentile ranking による正式な auto
分類に置き換えた。

## Context

- task: `docs/tasks/add-worker-workload-profile-and-auto-tier-classifier.md`
- spec: `docs/specs/auto-tier-classification.md`
- design: `docs/designs/auto-tier-classification.md`
- ADR: `docs/adr/price-based-percentile-auto-tier-classification.md`

## Implementation

### Changes

- `worker/validate.ts`: `cheaper.model.workload.outputPerInput` を
  schema に追加。有限・非負の number、unknown key 拒否、省略時は
  `DEFAULT_OUTPUT_PER_INPUT = 1.0`。
- `model/classify.ts`:
  - `effectiveCost(pricing, outputPerInput)` = `input + output * opi`。
  - `buildAutoTierContext(models, { outputPerInput? })`: pricing を
    解決し、unpriced / input+output=0 を除外した ranking 母集団を構築。
    explicit override / curated exception 対象 model も母集団には含める
    （分類 precedence は別途保つ）。
  - percentile は midrank（同額は同一 percentile・同一 tier）。
    `score = 3 * (2*below + equal)` の整数比較で `<2n → low`、
    `<4n → medium`、それ以外 `high`。浮動小数点境界を避ける。
  - `classifyModels` で共有 context による batch 分類を追加。
    `classifyModel` は `autoContext` 供給時のみ auto を実行し、
    `AutoClassificationDetail`（effectiveCost / percentile /
    outputPerInput）を返す。
- `model/resolve.ts`: `resolveModel` / `resolveExplicitModel` が
  `outputPerInput` を受け取り、exclude 適用後の eligible 集合で
  共有 context を構築。`ResolvedModel.auto` に分類根拠を保持。
  `--model` 経路も候補選択を bypass したまま policy 用分類だけ行う。
- built-in worker の初期 profile（暫定推定値）: `bulk-reader` 0.05、
  `code-writer` 0.3、`doc-writer` 1.0。

### Decisions During Implementation

- priced model が 1 つのみの集合は percentile 0.5 → `medium` とした。
- 全件同額なら全員 `medium`（midrank で 0.5）。
- `input + output === 0` の model は ranking 母集団から除外し、
  #45 の「価格ゼロは low にしない」不変を維持した。
- override / exception 対象 model は母集団に含めて分布に寄与させるが、
  自身の tier は precedence どおり override / exception で確定する。

## Related Experiments

### Adopted

なし（exp は作成していない）。

### Rejected

なし。

## Validation

- `pnpm --filter @cheaper/core test`: 23 files / 288 tests pass
  （classify 19・resolve 19・worker validate 18 を更新・追加）。
- effectiveCost formula、workload 差異、percentile 境界、tie、
  1/2/3 件集合、missing/zero pricing、override/curated precedence、
  exclude を網羅。

## Risks

- built-in `outputPerInput` は fixture と worker 意味論からの暫定推定。
  実 usage 分布との乖離があり得る。

## Follow-ups

- issue #44 の usage telemetry で実 `outputPerInput` を計測し、built-in
  初期値を検証・補正する（follow-up）。
