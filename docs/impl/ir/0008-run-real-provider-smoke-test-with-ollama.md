---
id: "IMPL-0008"
type: "impl"
status: "done"
title: "Run real-provider smoke test with Ollama"
created: "2026-09-16"
updated: "2026-09-16"
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
    - "docs/tasks/0055-run-bulk-read-and-cheap-write-smoke-against-ollama.md"
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

# Run real-provider smoke test with Ollama

## Summary

`bulk-read` と `cheap-write` を実 provider（Docker Ollama `qwen3.5:9b`）経由で実行し、いずれも exit 0・単一 envelope・実測 usage・実測 changes（cheap-write は `src/farewell.ts` 作成 9 行）を確認した。ランタイムのコード変更は不要だったが、local 小モデル向けに models.json の `samplingParams`（`reasoning_effort: "none"`・`temperature: 0`）と user `policy.yaml`（timeout 10m・maxTurns 16・repairAttempts 4）の調整が必要だった。

## Context

TASK-0055（PLAN-0004 / ADR-0009）に従う。workspace は `$TEMP/cheaper-smoke-ws`（bulk-read）と `$TEMP/cheaper-smoke-ws2`（cheap-write）。

## Implementation

### Changes

- リポジトリ内ファイルの変更なし
- 環境側の変更:
  - Ollama に `qwen3.5:9b` を追加 pull（4b は出力 schema 適合に失敗したため）
  - `models.json` の両モデルに `samplingParams: {reasoning_effort: "none", temperature: 0}` を設定
  - `~/.config/cheaper/policy.yaml`（新規）: `execution.timeout: 10m`・`maxTurns: 16`・`output.repairAttempts: 4`
  - `~/.config/cheaper/models.yaml`: `ollama/qwen3.5:9b` を preferences.low 先頭に追加

### Decisions During Implementation

- **thinking 無効化**: qwen3.5 は thinking モデルで CPU 推論では `message.reasoning` への生成が遅く policy 既定 120s・延長 10m でも完了しなかった。Ollama OpenAI 互換 API は `reasoning_effort: "none"` で thinking を無効化できる（`think: false` は無視される）ことを確認し、`samplingParams` 経由で付与（pi-ai openai-completions は samplingParams を request params に Object.assign する）
- **4b → 9b**: `qwen3.5:4b` は non-thinking でも output schema の入れ子 item shape（findings.locations、relevantFiles の `reason` 必須等）を遵守できず、repair 4 回でも収束しなかった。`qwen3.5:9b` は repair で schema 適合に到達
- **`response_format` 棄却**: `json_object` は JSON 構造を強制するが tool 呼び出しを抑制する傾向（9b で toolCalls 0）があり、`json_schema` は試験的に tool call を完全抑制することを確認。worker の tool 使用を阻害するため採用しない
- **`temperature: 0`**: 再現性のため。ただし repair も deterministic になる副作用はあり（失敗時は同じ誤りを繰り返す）、今回は成功に寄与
- **policy 変更は user config 層**: リポジトリの built-in policy（120s/8 turns/repair 1）は変更せず、`~/.config/cheaper/policy.yaml` の partial layer で延長。ランタイムコード変更ゼロで要件を満たした

## Related Experiments

### Adopted

- `reasoning_effort: "none"` の samplingParams 注入（openai-completions → Ollama で thinking を無効化）

### Rejected

- `response_format: {type:"json_object"}` / `{type:"json_schema",...}`（tool call を抑制）
- `qwen3.5:4b`（output schema 適合不能）
- `think: false`（Ollama OpenAI 互換 API では無視される）

## Validation

- `bulk-read`: `{"task":"List the exported names in src/greeter.ts","paths":["src/greeter.ts"],"maxFindings":3}` → exit 0、`ok:true`、model `ollama/qwen3.5:9b` tier `low`、usage `{input:395,output:216,cacheRead:1736,total:2347}`、turns 4、toolCalls 1、repairs 2、changes 空、stdout は envelope 1 行
- `cheap-write`: `{"task":"Create a simple farewell module...","targets":[{path:"src/farewell.ts",intent:"create"}],...}` → exit 0、`ok:true`、usage `{input:1723,output:479,total:5374}`、repairs 3、`changes.files=[{path:"src/farewell.ts",kind:"created",linesAdded:9}]`、生成物は greeter ミラーの `createFarewell`/`DEFAULT_SUFFIX` を export する TypeScript
- 失敗記録: `qwen3.5:4b` thinking 有効で TIMEOUT（120s・600s 共）、non-thinking で INVALID_OUTPUT（repair 4 回尽き）

## Risks

- `qwen3.5:9b` でも初回出力は schema 不適合で repair に依存しており、`repairAttempts` が少ないと失敗しうる。local 小モデルの構造化出力は repair 回数前提
- `reasoning_effort` は Ollama 固有の拡張であり、他 provider では無視またはエラーになりうる

## Follow-ups

- TASK-0056: Experiment Log 記録と `pnpm -r test` 回帰確認
- 将来: provider 別 samplingParams のベストプラクティスを docs に残す価値あり
