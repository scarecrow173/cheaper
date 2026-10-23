---
id: "DISC-034KNX4OP8iBFRKTREaigz"
type: "discovery"
status: "active"
title: "unified exec grant model — command.grants / process.tasks の統合（#165）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/165"
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
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/opt-in-exec-deny-patterns-issue-164.md"
  defers: []
  deferred-by: []
---

# unified exec grant model — command.grants / process.tasks の統合（issue #165）

## Exploration Goal（seed — briefing 前の最小記録）

issue #165（#151 実装分解 3/7）。`command.grants` と
`process.tasks` は同一概念（宣言済み exec の許可 + argv 契約 +
policy 管理）の二重実装であり、schema・resolve・pin・spawn・
test・文書が全て二重で不整合余地が構造的に存在する。#163 で
両側の内容審査分岐が除去された今、単一 exec grant モデルへ
統合する（Option D capability-grant 統合 — 親 design で採用
決定済み）。

### 統一 grant の保持情報（issue 本文）

`{name, executable, fixed?: argv固定, argv?: pattern, inputs?,
outputs?, cwd/env 宣言, lifecycle 宣言, scope: trusted|project}`

### Merge Fidelity 決定（全採用 — 親 design §Merge Fidelity）

- **M1** 統一 schema は optional `inputs`/`outputs` — 持つ grant は
  per-path manifest（`process.rs:392-396` の reads/writes/produces
  同等）+ declared-path deny・produces ownership・writeScope
  coverage 検査（`prepare.rs:71-122`）。持たない grant は従来の
  conservative root manifest（`command.rs:488-498`）。outputs 空
  = `ClaimMode::Shared`、非空 = `Exclusive`。
- **M2** fixed-argv は `fixed: true` marker — task 由来 argv のみ、
  model 入力は `{name}` のみ（args 不許可）。
- **M3** project 層の `process` 節を維持（deprecate しない）—
  project 宣言 tasks は provenance `project` の project-scoped
  grant namespace へ写像。route grant 参照（`route.grant`・
  `grants[]`・`substitute.grant`）は trusted-scoped のみ解決
  （A3 invariant 保全）。
- **M4** writeScope × exec — coverage check は **`fixed: true` かつ
  `outputs ⊆ writeScope` の grant のみ** per-call 適用。argv 供給
  grant は writeScope 下で拒否維持（C5）。runtime 構築時
  fail-closed（`registry.rs:1133-1142`）維持。
- **M5** `tool:"process"` workflow 互換 — canonical vocabulary に
  `process` を残し `input.task` を merged fixed grant 名へ解決
  する compat alias（UnknownTool fail-closed E7 に落とさない）。

### 統合対象

統一 grant schema（M1/M2）・`process.tasks` → grant loader 写像
（provenance scope — M3）・tool surface の `command` 統一 +
`tool:"process"` alias（M5）・writeScope per-call coverage（M4）・
pin/spawn/manifest/pool 機構の単一化。route の `project_grant`
投影・viability・record も統一モデルのみを見る（#166 と整合）。

### Acceptance Criteria（issue 本文）

- 統一 schema が `fixed`・optional `inputs`/`outputs`・
  provenance scope を保持（M1/M2）
- project `process.tasks` → `fixed:true` + provenance `project`
  grant 写像、project-scoped grant が route 参照を満たせない（M3）
- writeScope 下で `outputs ⊆ writeScope` の fixed grant は exec 可、
  outputs 超過 task・argv 供給 grant は拒否（M4）
- `tool:"process"` 既存 workflow が UnknownTool で落ちない（M5）
- 既存契約 test（pin・env_clear・stdin null・lifecycle・fs
  boundary・writeScope）が回帰なく pass

### seed の前提

- deps: #163（merged — PR #172）。#164 も完了済み（PR #173）で
  `security.execDenyPatterns` が grants に適用される — 統合後の
  grant モデルと deny の相互作用を briefing で確認。
- file:line 引用は issue 本文のもの（main `59088c2` 時点 —
  #163/#164 後の line drift を briefing で実測修正する）。

## Key Issues and Assumptions

- briefing で確定する。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| briefing で確定する | | | |

## Tentative Conclusions and Hypotheses

briefing で確定する。

## Open Questions

- briefing で確定する。

## Promotion Candidates

- [ ] spec-doc needed: yes（統一 schema・provenance・tool surface の intent）
- [ ] adr-doc needed: yes（schema 形状・`fixed` marker・scope 表現の決定記録）

## Sources and Relations

- issue #165・親 issue #151 chain（design/spec/ADR/discovery）
- #163 IR（内容審査除去後のコード状態）・#164 IR（execDenyPatterns）
