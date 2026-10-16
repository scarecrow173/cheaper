---
id: "ADR-eb2ghgGK2Cd8w7pro1qe18"
type: "adr"
status: "accepted"
title: "Document bulk-reader output limits in the consumer skill rather than a runtime surface"
created: "2026-10-17"
updated: "2026-10-17"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/108"
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
  related:
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "../discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/bulk-reader-output-limits-and-invalid-output-documentation.md"
    - "../discovery/bulk-reader-output-limits-and-invalid-output-guidance.md"
  derived-by:
    - "../designs/bulk-reader-output-limits-and-invalid-output-skill-docs.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# Document bulk-reader output limits in the consumer skill rather than a runtime surface

## Context and Problem Statement

Issue #108: `bulk-reader` が `INVALID_OUTPUT`（exit 6）で失敗したとき、
親 agent は出力フィールド上限（`excerpt` ≤600 文字等）を文書から
知る術がなく、`error.details.rawOutput` を検証済み result と
誤認する危険がある。上限値の正本は embedded schema
（`crates/cheaper-cli/resources/workers/schemas/bulk-reader.output.schema.json`）
であり、worker schema を dump する公式の CLI 手段は存在しない
（embedded resource は実行時に temp dir へ実体化されるが、undocumented
な内部経路であり consumer 向けの参照手段ではない）。

## Decision Drivers

* SPEC-7GjKOMzR6m3HfBdpPmzSIe — issue は文書化を要求し、runtime 挙動
  変更を明示的に除外している。
* 親 agent が実際に読む場所は `cheaper-bulk-read` skill である。
* skill 文に値を写すと schema との drift リスクが生じる — だが参照手段が
  無い以上、現状の唯一の伝達経路である。drift は schema 値と skill 文を
  照合する自動チェック（node test）で緩和する。
* YAGNI: discovery コマンド新設は本件の要求を超える。

## Considered Options

* **Option A（採用）**: `cheaper-bulk-read` skill に出力上限表と
  INVALID_OUTPUT 回復指針を記し、`cheaper-usage` の exit code 節に
  `error.details`（`errors`/`rawOutput`）の簡潔な補足を追加する。
  正本スキーマへの path を記し、skill 記載値と schema JSON を照合する
  node test で drift を検知する。`.agents/skills/` は untracked な
  deploy 領域のため対象外。
* **Option B**: `cheaper-usage` のみに INVALID_OUTPUT 節を追加し、
  bulk-read skill は現状のまま — 却下: 失敗する worker の skill が
  読者の第一参照先であり、汎用 usage 文書への集約では発見性が低い。
* **Option C**: `cheaper run <worker> --describe` のような schema dump
  コマンドを追加し、skill はそのコマンドを案内する — 却下:
  issue が文書化のみを要求し runtime 変更を除外している。将来的な
  改善候補として記録に留める。

## Decision Outcome

Option A を採用する。理由:

1. 親 agent が参照するのは bulk-read skill であり、上限と回復指針を
   そこに置くのが最も発見性が高い。
2. `cheaper-usage` の exit code 表は既に exit 6 を汎用説明しており、
   `error.details` の各フィールドの意味を補足するのが自然な責務分担。
3. runtime 変更なしに本件の受入基準をすべて満たせる。

## Consequences

* 良い結果: 親 agent は失敗前に上限を知り、失敗後は `rawOutput` を
  result と誤認せず bounded な再試行を選べる。
* 負の結果: skill 文の上限値は schema と二重管理になるが、schema JSON と
  skill 表を照合する node test（`scripts/test/`）で drift を CI 検知する。
* 残課題: schema dump コマンド（Option C）と他 worker（tool-operator 等）
  への同様の上限記載は別 issue として検討余地あり。
