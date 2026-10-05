---
id: "IMPL-034ARlQpkPs620j6TP15Rv"
type: "impl"
status: "in-progress"
title: "Seeded workflow benchmark fixtures と runner"
created: "2026-10-05"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
      - type: "file"
        path: "benchmarks/lib/result-schema.mjs"
      - type: "file"
        path: "benchmarks/lib/run-scenario.mjs"
      - type: "file"
        path: "benchmarks/run.mjs"
    modified:
      - type: "file"
        path: "benchmarks/scenarios/scenarios.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/build-seeded-workflow-benchmark-fixtures-and-runner.md"
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
  verified-by:
      - "docs/test-specs/workflow-benchmark-scenario-determinism.md"
  references:
    - "docs/adr/benchmark-harness-placement-and-accounting.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Seeded workflow benchmark fixtures と runner

## Summary

7 scenario family を fake Worker と built CLI で反復測定する root `benchmarks/` harness を実装した。

## Context

Issue #67 は tool-only 単発、fan-out/fan-in、tool/Worker chain、process output、manual sequence、dynamic/saved workflow、definition/interface の再現可能な比較を求める。親 token が測定できない場合は節約として扱わない。

## 実装

### 変更

- Seeded scenario manifest、saved workflow、fake stream、record validation、prompt/timing/critical-path 計測を追加した。
- built CLI を `NODE_ENV=test` と `CHEAPER_TEST_FAKE_STREAM` で subprocess 実行する runner を追加した。
- fan-out fixture を独立 read tool 2 件 + Worker fan-in 1 件にした。複数 fake Worker の concurrent model resolution は繰り返し `MODEL_TIER_UNAVAILABLE` となったため、この要件に沿う tool fan-out に変更した。
- interface size は envelope の `result` payload に限定し、可変 `durationMs` を除いた。
- process stdout は `/details/stdout/text` から Worker input に bind する。

### 実装時の判断

- direct invocation 数は親 turns の proxy にとどめた。
- tool-only workflow は workflow envelope の job metadata から集計し、`usage.jsonl` へは加算しない。

## 関連実験

### 採用

- なし。seeded harness の比較は本実装記録と集約 artifact に保存する。

### 却下

- concurrent fan-out fake Worker: 同一 fake provider の model resolution が反復中に空候補を返し、決定的な sweep にならなかったため、tool-job fan-out fixture に置き換えた。

## 検証と証拠

- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — pass。
- 全 7 scenario を各 arm warm-up 2 回 + measured 15 回で実行し、`benchmarks/results/e45d35b7-2fa6-416b-946a-70ca844b9c80.json` に統合した。
- canonical `mise run check` は coordinator merge run に defer。

## リスク

- 測定した CLI wall time は Windows のこの環境に限る。provider 一般の性能や親 agent の token savings を示さない。

## Follow-up

- scheduler/prompt/model/CLI 変更後に `mise exec -- node benchmarks/run.mjs` を再実行する。
