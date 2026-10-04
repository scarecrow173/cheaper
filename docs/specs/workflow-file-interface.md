---
id: "SPEC-034zSVZlWjFAMpwmhsLTkx"
type: "spec"
status: "approved"
title: "明示 workflow loader と公開 interface command の契約"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/69", "https://github.com/scarecrow173/cheaper/issues/70"]
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
  derives-from: ["../discovery/workflow-file-interface.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# 明示 workflow loader と公開 interface command の契約

## What Should Be Built

明示 path の reusable Workflow JSON v1 を読み込み、`cheaper workflow interface <path>` から公開契約だけを取得する。invocation 値は definition と分離し、将来の run が loader と invocation validation を再利用できる。

## Why Now

#64 の schema/compiler/projection が統合され、file の読込と親 agent 向け CLI が未接続。

## Users and Value

親 Coding Agent が private DAG/prompt を context に取り込まず、必要な input と期待する output を把握できる。

## Scope

### In Scope

- explicit-path load/parse、既存 compiler への validation/compile 委譲、invocation validation API。
- workflow interface の JSON/text utility envelope、canonical --cwd と boundary/deny policy。

### Out of Scope

- workflow run/engine（#62 依存）、named lookup、glob/discovery/watch/daemon、stdin definition、--values、bundle 更新。

## Requirements

- 共有 ADR D1 の依存方向、D5 の validation 段階/default/schema、D6 の utility envelope/CLI path/error を維持する。
- interface は definition validation のみを行う。invocation 値・resource inference・Worker/tool/process execution を要求しない。
- 公開 output の schema のみを出し、private binding を出さない。
- schema version や参照/DAG の不正を canonical compiler と同じ意味で reject する。

## Acceptance Criteria

- [x] AC1: result は WorkflowInterface 形の schemaVersion・inputs[name]={schema,required}・outputs[name]={schema} のみ。default/type/constraints は schema 内に保持し、一行 JSON envelope を返す。
- [x] AC2: private jobs/needs/prompt/tool parameter/resource/binding は出力に含まれず、model/Worker/tool は実行されない。
- [x] AC3: file と動的 definition は同じ compiler の definition validation と compile 経路を使い、parallel validator を持たない。
- [x] AC4: 不在 file・malformed JSON・invalid definition は error envelope と非 0 exit、部分 interface を返さない。
- [x] AC5: --format json/text・utility worker:null・error envelope/exit code は既存 CLI と一致する。
- [x] AC6: invocation defaults/unknown/missing/schema validation を run から再利用でき、compile と同じ規則で動く。
- [x] AC7: path は canonical --cwd 相対（workspace 内の絶対 path も可）、workspace escape/symlink policy/deny path を拒否する。
- [x] AC8: interface 専用受理 option は <path> と --cwd/--format/--quiet のみ。他 command 専用 option（--input - を含む）は INVALID_REQUEST。既存 global --help/--version は維持する。

## Deferred Design Concerns

- run engine・resource inference は #62/#65-B の統合時に供給する。interface で推論を偽装しない。

## Sources and Relations

issues #69/#70 の Triage 2026-10-04、共有 ADR-034zGZpjoQlppwkkOs5gur D1/D5/D6、discovery を参照。

2026-10-04: coordinator により spec approval APPROVED（P0/P1 なし）。AC1 の既存型と option 限定を反映。
