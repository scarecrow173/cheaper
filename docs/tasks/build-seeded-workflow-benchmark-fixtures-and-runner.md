---
id: "TASK-034AR8FgMmq4ecbz4SExZy"
type: "task"
status: "done"
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
        path: "benchmarks/run.mjs"
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-workflow-orchestration-benchmarks.md"
  blocks:
    - "docs/tasks/aggregate-benchmark-evidence-and-publish-workflow-usage-guidance.md"
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
  references: []
  defers: []
  deferred-by: []
---

# Seeded workflow benchmark fixtures と runner

## 作業

7 scenario family の seeded JSON fixture、fake Worker stream、result schema、built CLI を subprocess で実行する runner を実装する。2 warm-up + 15 measured repetition を既定値とし、必要な arm・使用環境・source revision を保存する。

## 完了内容

- workflow JSON を生成する場合と saved JSON を読む場合、compact interface を読む場合を含む 7 scenario / 17 arm を追加した。
- 単純 read は同一 fixture を使う直接 filesystem 比較と fake Worker の単一 tool-call 比較を記録する。
- Fan-out は独立 read tool 2 件の後に 1 Worker が fan-in する DAG とし、direct arm は同じファイルを読んで 1 Worker を呼ぶ。
- process stdout の text binding、大きな中間出力、親 usage unavailable の provenance を含めて保存する。
- full run と scenario 単位の run が UUID result JSON を生成し、既存 artifact を上書きしない。

## 検証

- `mise exec -- node --test --test-reporter=spec benchmarks/test/*.test.mjs` — pass。
- `mise exec -- node benchmarks/run.mjs --scenario single-simple-tool-call --warmups 0 --repetitions 1` — 3 arm を実行。
- 7 scenario の各 arm を 2 warm-up + 15 measured repetitions で実行し、集約前の観測値を保存した。

## 制限

Fake Worker は seeded model response を使う。provider 一般の品質・latency を表さず、親 agent の実 turns/token telemetry は取得しない。
