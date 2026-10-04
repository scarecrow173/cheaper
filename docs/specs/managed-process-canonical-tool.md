---
id: "SPEC-034A8q6kmxcBsNF8zS9Cwn"
type: "spec"
status: "approved"
title: "policy-defined managed process を canonical tool として提供する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/59"]
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
  derives-from: ["../discovery/managed-process-canonical-tool.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# policy-defined managed process canonical tool

## What Should Be Built

`{"kind":"tool","tool":"process","input":{"task":"test"}}` が認可された named task を実行する。Pi でも同じ canonical executor を呼ぶ。

## Why Now

test/build のような process を、任意 shell を開放せず Workflow と Worker で利用するため。

## Users and Value

Workflow author と親 Coding Agent が、予測可能な認可、出力上限、cancel cleanup を持つ process 実行を利用する。

## Scope

### In Scope

- named task argv、workspace cwd、explicit env、declared I/O、operation/pool admission、bounded structured result、Pi/Workflow adapter。

### Out of Scope

- unrestricted shell、remote/container sandbox、OS tracing、job ID/polling/status/wake API、依存追加。

## Requirements

- 入力は task 名だけ。任意 command、追加 argv、shell executable/構文は拒否する。
- cwd は canonical root または検証済み相対 directory。親 process の秘密 env は継承しない。
- process は invocation-wide operation 上限と process sublimit の両方に従う。queued cancel は spawn しない。
- cancel は管理下の、re-parent/新 session 化しない process tree を終了させてから settle する。stdout/stderr 合計の capture を byte cap 内に収め、総 bytes と truncation を返す。
- declared inputs は reads、outputs は writes/produces。部分集合 output glob は D4 に従って拒否し、literal または `dir/**` の ownership を要求する。
- declared output は Worker writeScope に矛盾できない。実アクセスの観測・制約は宣言から推測しない。

## Acceptance Criteria

- [ ] Pi adapter と Workflow compiler/materialization → direct canonical runtime が同じ task を呼べる。
- [ ] unknown task、command 入力、shell argv、cwd escape、undeclared output scope を拒否する test が通る。
- [ ] env 継承なし、argv literal、非ゼロ exit の failure と bounded diagnostics を実 process で確認する。
- [ ] policy-aware ResourceManifest に inputs/outputs と producer edge が現れる。
- [ ] 複数 owner の readonly process が process/maxOperations 上限を超えず、mutation は排他となる。
- [ ] queued cancel は子を作らず、running cancel は管理下の実子と孫を終了し operation が cancelled になる。
- [ ] declared-I/O trust boundary、shim 対応範囲、OS tracing 不在、setsid/re-parent 孫と正常終了後の孫残留検査が範囲外であることを guide に記録する。
- [ ] project task が任意 executable を事前認可できる threat model と、.cheaper/** deny の適用限界を guide に記録する。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

この chain の外で新しい設計課題を起票しない。non-goals は実装しない。

## Sources and Relations

issue #59 と共有 ADR の D1/D2/D4/D5/D7 を正本とする。

coordinator 承認: msg_126e0d3e23d0、spec-approval / adr-acceptance / design-approval。P1/P2 の境界補足を設計へ反映する。
