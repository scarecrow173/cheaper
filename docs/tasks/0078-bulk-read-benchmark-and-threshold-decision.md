---
id: "TASK-0078"
type: "task"
status: "done"
title: "bulk-read delegation benchmark and read threshold decision"
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
    - "docs/specs/0008-apm-delegation-skills-and-read-hook.md"
  implemented-by: []
  depends-on:
    - "docs/tasks/0077-delegation-loop-e2e.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/adr/0012-default-large-read-threshold.md"
    - "docs/discovery/0007-bulk-read-delegation-benchmark.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# bulk-read delegation benchmark and read threshold decision

## Work

`pre_read_code` hook の provisional threshold (8192) を実測で確定する。

- repo file token 分布の計測（`countFileTokens`、`.apm/hooks/cli` 除外）
- Ollama `qwen3.5:4b` での bulk-read 実委譲測定（file tokens vs
  parent 受信 result tokens vs worker usage vs wall time）
- cost break-even と非コスト要因（latency, false positive）の分析
- DISC-0007・ADR-0012・EXP-0003 に記録し、DESIGN-0013 の provisional
  注記を更新

## Acceptance Criteria

- [x] 分布計測: 293 files / 417k tokens、percentile・閾値別集計あり
- [x] 実委譲 2 件: 3013→~1230 tok (59%), 6267→~228 tok (96%)
- [x] ADR-0012 で default 8192 を確定、根拠と trade-off を記録
- [x] `pnpm lint` / audit script が clean

## Done

- DISC-0007 (benchmark), ADR-0012 (decision), EXP-0003 (experiment log)
- threshold 値自体は変更なし（provisional 8192 を確定）のためコード差分なし
