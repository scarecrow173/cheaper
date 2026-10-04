---
id: "IDEA-034A8p4vQ9wEUp8M2aBepC"
type: "idea"
status: "promoted"
title: "managed process を canonical tool として提供する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/59"
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
  derived-by: ["../discovery/managed-process-canonical-tool.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# managed process を canonical tool として提供する

## Summary
policy に登録した named task を、Pi と Workflow が同じ `process` tool で呼び出せるようにする。

## Problem and Motivation

- shell を公開せずに test/build などの外部 process を実行する canonical tool が必要。

## Expected Value

- Worker と Workflow の利用者が、認可済み task の bounded 結果を受け取れる。

## Open Questions

- scheduler pool、Windows executable、cancel cleanup の契約を discovery で整理する。

## Next Action

- [x] discovery-doc へ進める（coordinator: APPROVED chain seed）。
