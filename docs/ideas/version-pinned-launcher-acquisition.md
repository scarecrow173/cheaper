---
id: "IDEA-034zINY5tKFy4kx4xFaveB"
type: "idea"
status: "promoted"
title: "Acquire version-pinned release binaries from platform launchers"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/73"
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
    - "../discovery/version-pinned-launcher-acquisition.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Version 固定 launcher acquisition

## Summary

APM の git install を維持し、sibling executable がない場合に package version と一致する Release executable を user cache に取得する。

## Problem and Motivation

LFS payload と release artifact の byte 一致運用は version bump ごとに runner 往復を要求する。consumer の Node 非依存 hook を保ちつつ、payload の取得を launcher に移せるか検討する。

## Target Users

git ref から APM package を install する親 Coding Agent と、offline/proxy 環境の利用者。

## Open Questions

公開 checksum、version identity、同時初回起動、移行の境界を discovery で確認する。

## Next Action

#73-A として discovery → spec/ADR → design → plan/tasks → implementation を進める。#73-B/C は別作業とする。
