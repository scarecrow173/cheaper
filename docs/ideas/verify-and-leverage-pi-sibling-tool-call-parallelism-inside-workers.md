---
id: "IDEA-034zzQkQLoqLftac2tPws5"
type: "idea"
status: "promoted"
title: "Verify and leverage Pi sibling tool-call parallelism inside Workers"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/55"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../discovery/pi-sibling-tool-call-parallelism-inside-workers.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Verify and leverage Pi sibling tool-call parallelism inside Workers

## Summary

Pi Worker が 1 assistant turn 内で独立した sibling tool call を並列発行できるかを
pinned Pi version で実測し、runtime が生成する Worker 指示に same-turn 並列呼び出しを
明示して Worker turn 数と反復入力コンテキストを削減する。

## Problem and Motivation

- serial な ReAct 型 tool/model/tool/model シーケンスは Worker turn と
  増大するコンテキストの反復送信を増やし、delegation のコスト削減効果を薄める。
- upstream の想定だけでなく pinned Pi version の決定的な runtime test で
  sibling tool-call 並列性を supported contract として確認したい。

## Expected Value

- Worker turn 数の削減により、同一作業で消費する反復入力トークンが減る
  （トークン単価ではなく turn/コンテキスト量の削減が効果源）。
- #63 canonical Tool Runtime / #57 ToolScheduler 設計への実測ベースの入力になる。

## Open Questions

- pinned Pi version で sibling tool call が本当に並列実行されるか、
  順序・barrier・部分失敗の semantics はどうなるか。
- runtime 指示のどの層（worker definition / execution engine / prompt
  template）に same-turn 並列呼び出しの指示を置くべきか。
- #57 scheduler limit と sibling call の並列度はどう整合させるか。

## Next Action

- [x] Promote to discovery-doc for deeper exploration
- [ ] Promote directly to spec-doc if requirements are clear
- [ ] Park for later reconsideration
- [ ] Discard — reason: <!-- why -->
