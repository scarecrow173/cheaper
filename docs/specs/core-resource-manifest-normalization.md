---
id: "SPEC-034zIOnRXiXsUtWd4Z6edT"
type: "spec"
status: "proposed"
title: "Core resource manifest normalization"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/65"
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
  derives-from:
    - "../discovery/core-resource-manifest-normalization.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  defers: []
  deferred-by: []
---

# Core resource manifest normalization

## What Should Be Built

共有 ADR-034zGZpjoQlppwkkOs5gur の D4 に従い、core/resources に workspace-relative ScopeDeclaration から ResourceClaim を正規化する境界、manifest dedup と unknown fallback、pure containment / conflict predicate を実装する。既存 types.ts は変更しない。

## Why Now

#59 / #61 / #65-B が identity や overlap ルールを重複実装しないため。

## Users and Value

Tool Runtime と Workflow が filesystem に依存しない同一 predicate を利用する。

## Scope

### In Scope

- resources/**、新規 core tests、core barrel の resources export 行。
- workspace paths.ts の canonical root と real 解決を再利用し、不存在 tail、symlink policy、workspace escape、platform case identity を扱う。

### Out of Scope

lease / queue / DAG edge / duplicate producer の compiler 検査、artifact namespace、hard-link alias、外部 symlink 差替え、共有型や boundary の変更。

## Requirements

- 入力 scope は exact / subtree の明示値。subtree は本体と descendants。類似 prefix sibling は別 resource。
- D5 の JSON 入力は ScopeDeclaration {path, scope}[]。mode は core の manifest 正規化が配列種別から導出する（reads=shared / writes,produces=exclusive）。core API は既存の canonical ResourceClaim[] も受理し、その場合は固定型の mode と bucket の不整合を拒否する。standalone claim 正規化は mode を独立引数で受ける。JSON protocol に ResourceClaim 表現や mode field は追加しない。
- manifest 入力の reads / writes が省略または null なら root subtree の shared / exclusive。空配列は既知の access なし。produces は省略で空、null は拒否し ownership を推測しない。
- resources オブジェクト自体の省略・空（{}）は D4 の conservative fallback（D5 の「省略 / 空」はこれを指す）。各配列の空配列（reads: [] 等）は「access が無いと宣言済み」の narrow manifest であり fallback ではない。未知 / broad access を表現する consumer（#64 compiler）は空配列ではなく key 自体を省略しなければならない。
- canonical path は paths.ts の絶対 real 解決を保持する。比較 key のみ Windows で小文字化し case-insensitive、POSIX は case-sensitive。配列は比較 key / scope / mode と実 path の安定順に duplicate を除去する。
- reads / writes の glob は literal ancestor subtree に近似する。produces の partial glob は拒否し、literal exact / subtree と scope=subtree の dir/** を受理する。glob tail の .. は近似が誤るため拒否する。
- 不正 mode / scope / path / symlink policy / manifest 配列を fail closed で拒否する。
- containment / overlap / conflict は IO を行わず component 境界で計算する。exclusive のみ shared を含めて覆える。

## Acceptance Criteria

- [ ] AC1: future exact / subtree、本体 / descendant / src/a と src/ab の分離。
- [ ] AC2: workspace escape、symlink deny / within-workspace alias、platform 条件付き Windows case identity と POSIX case distinction。
- [ ] AC3: mode 不整合拒否、unknown fallback、入力順に依存しない normalization / dedup。
- [ ] AC4: reads/writes glob の ancestor safety と produces ownership の分離、dir/*.json 拒否 / dir/** 受理、不正入力拒否。
- [ ] AC5: read/read 非 conflict、read/write と write/write conflict、ancestor overlap、独立 path の pure predicate 表。
- [ ] AC6: 関連 test と mise run check、focused exit-audit が pass。

## Deferred Design Concerns

#65-B の compiler inference は本 task の対象外。

## Sources and Relations

issue #65 Triage と共有 ADR D4 が authority。
