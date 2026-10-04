---
id: "IMPL-034zR9OQeNgEdCN2OMdt8I"
type: "impl"
status: "in-progress"
title: "Implement native launcher acquisition and hook routing"
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
    - "../../tasks/implement-native-launcher-acquisition-and-hook-routing.md"
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

# Native launcher acquisition
## Summary
取得/検証/公開/起動を実装中。
## Context
先行taskのredを確認済み。承認plan/TSPECをinlineで実行。
## Implementation
### Changes
sh/pwshとhooks.jsonを変更する。
### Decisions During Implementation
PS 5.1のargv/stdio保全にはProcess APIを使う。curl.exeがあればnative downloader、無い場合IWRのredirectを手動検証する。
## Related Experiments
### Adopted
なし。
### Rejected
なし。
## Validation
未実行。
## Risks
強制kill lockは自動回収しない。
## Follow-ups
公開manifestとCopilot sidecar補完は#73-B/C。
