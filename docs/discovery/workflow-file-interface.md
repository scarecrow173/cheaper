---
id: "DISC-034zSVnLYkkFBYQvt56QNJ"
type: "discovery"
status: "resolved"
title: "Workflow file と compiler の接続境界を調査する"
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
  derives-from: ["../ideas/workflow-file-interface.md"]
  derived-by: ["../specs/workflow-file-interface.md"]
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow file と compiler の接続境界を調査する

## Exploration Goal

明示 file と動的 definition が同じ検証意味論を持ち、interface 取得が invocation 値や execution を要求しない接続境界を調べる。
Entry Decision: A-4。dispatch の明確な受け入れ条件に基づき、指定された idea/discovery/spec/design chain を実行する（briefing meta-skill の profile 探索は dispatch-specific override）。

## Key Issues and Assumptions

- ADR-034zGZpjoQlppwkkOs5gur D1/D5/D6 は既に accepted。第二の schema/interface system を作らない。
- compiler.ts の compile は必須 invocation input を要求する。interface.ts は同じ schema.ts の definition validation のみを使う。
- core の resolveWorkspacePath と createPathDenylist に file boundary の責務がある。
- createToolCatalog と resolveWorker は metadata のみ取得し、Pi/model を初期化しない。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| compiler の既存 API を file adapter から呼ぶ | validation が一箇所 | metadata catalog を composition root が供給する必要 | yes |
| loader 専用 schema/JSON dump | file 表示は容易 | validation drift と private data 漏出 | no |

## Tentative Conclusions and Hypotheses

- confirmed: public projection は #64 に実装済み。compile を interface の preflight にすると必須 input を要求し、D5 段階分離を破る。
- hypothesis: file を JSON parse した後、compiler の definition/compile/project API へ委譲する adapter が最も小さい接続になる。

## Open Questions

- pre-implementation blocker はない。run engine は #62 の downstream 管理対象。

## Promotion Candidates

- [x] spec-doc: explicit-path loader と compact interface の受け入れ条件。
- [x] ADR は既存共有 ADR D1/D5/D6 を再利用。新しい契約判断は不要。

## Sources and Relations

調査対象: packages/workflow/src/{compiler,interface,schema}.ts、packages/tool-runtime/src/catalog.ts、packages/core/src/{worker/resolve,workspace/paths,workspace/deny-paths}.ts、packages/cli/src/{args,cli,envelope}.ts。
