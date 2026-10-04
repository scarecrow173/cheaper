---
id: "IDEA-034zSTT1wmfJi1KmZtrY98"
type: "idea"
status: "promoted"
title: "明示 workflow file の公開 interface を取得する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/69"
    - "https://github.com/scarecrow173/cheaper/issues/70"
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
  derived-by: ["../discovery/workflow-file-interface.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 明示 workflow file の公開 interface を取得する

## Summary

親 agent が reusable workflow の全文を読まず、呼び出しに必要な公開 input/output を取得できるようにする。

## Problem and Motivation

- private DAG、prompt、tool parameter の読込が親 context を消費する（#69・#70）。

## Expected Value

- 親 Coding Agent が小さい公開契約から invocation を組み立てられる。

## Open Questions

- 既存 compiler の definition validation と file の読込をどの境界で接続するか。

## Next Action

- [x] discovery-doc に昇格し、#64 の既存 API と CLI utility 契約を調査する。
