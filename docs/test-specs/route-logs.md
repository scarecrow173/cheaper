---
id: "TSPEC-034Jg5VLupBVGDPELPFf83"
type: "test-spec"
status: "draft"
title: "Route log store and route logs CLI"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source: []
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
  derived-by: []
  verifies:
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route log store and route logs CLI

## Purpose

route-logs store(runId 採番・tmp+rename・0600・sweep)と
`route logs <id>` の bounded read・route-refetch 記録を pin する。

## Feature

`cheaper route logs <runId> [--offset N]` が外部 log を返す。

## Rules

- runId は `^[0-9]+-[0-9]+-[0-9a-f]+$` を満たし、file は
  `<userConfigDir>/route-logs/<runId>.log`。
- 書き込みは `<runId>.log.tmp` streaming → 完了時 rename、Unix
  mode 0600。
- retention sweep は TTL 30 日・最大 200 file・最大 256 MiB で、
  `*.tmp` を除外する。
- RouteLogReader は共有 ArtifactReader bounded-read contract を実装し、
  `route logs` と worker artifact read が同一 runId 検証を使う。
- offset/eof semantics は既存 ArtifactReader 規約に従う。

## Examples

- `route logs 123-4-abcdef` → bounded output + eof flag。
- 不正 runId `../x` → 検証拒否。

## Guarantees

- in-flight `.tmp` file を sweep が消さない。

## Non-goals

- log 内容の structured parsing。

## Risk

- ref 検証漏れは arbitrary file read を許す。
