---
id: "DISC-034A8pIq50dPYXsDhld9st"
type: "discovery"
status: "resolved"
title: "managed process の認可と lifecycle を整理する"
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
  derives-from: ["../ideas/managed-process-canonical-tool.md"]
  derived-by: ["../specs/managed-process-canonical-tool.md", "../adr/managed-process-named-task-lifecycle.md"]
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# managed process の認可と lifecycle を整理する

## Exploration Goal

issue #59 の named task 認可、resource 宣言、shell-free spawn と cancel cleanup を、統合済み canonical runtime に接続する。Entry Decision は A-4（brief と共有 ADR に acceptance が明示済み）。dispatch-specific override: briefing-flow の SUBAGENT-STOP に従い、明示 dispatch の文書スキルを inline で使い、profile 生成と nested dispatch は行わない。

## Key Issues and Assumptions

- 確認済み: `tool-runtime/src/registry.ts` が入力検証・budget/meter・scheduler の唯一の入口。
- 確認済み: scheduler は invocation 共通だが process sublimit は未実装。OperationDeclaration に pool を追加する必要がある。
- 確認済み: Workflow の ProcessResourceDeclaration は存在するが compiler は埋めない。policy-aware ToolCatalog の resources を fallback source として接続する。
- 宣言 I/O を信用する scheduling であり、実アクセスの tracing/sandbox は提供しない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| canonical named task + native spawn | 既存の lifecycle/accounting を共有 | executable 対応範囲を明記する必要 | 採用候補 |
| Pi 独自 shell tool | shim を動かしやすい | shell-free と canonical 契約に違反 | 却下 |
| process 独立 job kind | 専用 API を作れる | D5 の tool job と責任が重複 | 却下 |

## Tentative Conclusions and Hypotheses

確認済み: D1/D2/D4/D5/D7 の既存責任境界で実装可能。task 名のみを入力として受け、policy argv を解釈せず実行し、readonly task は shared、output を宣言する task は exclusive とする。

## Open Questions

- 実装前 blocker: spec/ADR/design/plan の coordinator 承認、および core vocabulary/policy と builtin policy 二箇所の hot-file 承認。

## Promotion Candidates

- [x] spec-doc: managed-process-canonical-tool。
- [x] adr-doc: managed-process-named-task-lifecycle。

## Sources and Relations

source は issue #59 とその Triage 2026-10-04。共有 ADR は references のみ。変更前の scheduler/resource-inference/concurrency suite: 57 tests pass。
