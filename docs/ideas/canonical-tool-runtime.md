---
id: "IDEA-034zIKQIT4QevaIv4SeLB7"
type: "idea"
status: "promoted"
title: "Canonical Tool Runtime 抽出"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/63"]
  derived-by: ["../discovery/canonical-tool-runtime.md"]
---

# Canonical Tool Runtime 抽出

## Summary

Pi Worker と direct caller が deterministic tool の境界・結果・計測を共有する。

## Problem and Motivation

Workflow の tool job が model/session を必要とする現状を解消し、二重実装を避ける。

## Expected Value

同じ workspace に対する同じ入力が同じ結果・拒否となる。

## Open Questions

helper-backed mutation と invocation lifetime の分離境界。

## Next Action

- [x] discovery へ昇格する。
