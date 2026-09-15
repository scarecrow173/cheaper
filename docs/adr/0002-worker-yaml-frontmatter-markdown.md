---
id: "ADR-0002"
type: "adr"
status: "proposed"
title: "Worker を YAML frontmatter + Markdown の宣言的定義にする"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0001-cheaper-v1-worker-runtime-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "0003-worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "0004-one-shot-json-first-cli.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/0001-cheaper-v1-pi-sdk.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# 2. Worker を YAML frontmatter + Markdown の宣言的定義にする

## Context and Problem Statement

Worker は「何をしたいか」を宣言する単位であり、built-in と user-defined が同じ仕組みである必要がある（§2.3, §44: 新 Worker 追加で Runtime ソースを変更しない）。定義形式を決める必要がある。

## Decision Drivers

* §2.3: Worker をコードへハードコードしない、宣言的定義
* §44: Worker Definition + Output Schema の追加だけで動く構造
* 他 Coding Agent のスキル形式（SKILL.md 等の frontmatter+Markdown）との親和性

## Considered Options

* TypeScript モジュールとして Worker を定義する
* JSON だけで Worker を定義する
* YAML frontmatter + Markdown body で定義する

## Decision Outcome

Chosen option: "YAML frontmatter + Markdown の宣言的定義", because メタデータ（structured）と instruction（自然言語）を1ファイルで宣言でき、他 Agent のスキル形式と一致し、built-in/user-defined が同一形式で配布できるから。

### Consequences

* Good, because Worker は単一 `.md` ファイルで配布・shadowing・versioning できる。
* Good, because Markdown body がそのまま system/instruction prompt になる。
* Bad, because frontmatter parser と validator を cheaper 側で実装する必要がある。

### Confirmation

* parser/validator の単体テスト（§42 Worker validation カテゴリ）で必須 field・name 規則・namespace strictness・schema exactly-one が検証されること。

## Implementation Plan

* Affected paths: `packages/core/`（予定 path、ADR-0008）— Worker Definition parser/validator、`packages/cli/` — built-in workers 同梱
* Patterns to follow: frontmatter + Markdown 形式（SKILL.md 系）
* Patterns to avoid: Worker を TS コード・decorator・plugin API で定義すること、Worker 継承/合成/merge（§39 non-goal）
* Dependencies or configuration: YAML parser（依存追加は別途承認制、ADR-0008）

確定内容（§8–16）:

* 必須 field: `name`, `description`, `tools`, `cheaper.model.tier`, `cheaper.output.schema`, Markdown body（全て non-empty）
* `name`: `^[a-z0-9][a-z0-9-]*$`、ファイル名と一致必須
* `cheaper:` namespace は strict。V1 で許可は `cheaper.model` / `cheaper.output` のみ。timeout/maxTurns/retry/filesystem policy/concrete model/provider/auth/network/shell を Worker Definition に置かない（§12）
* top-level の未知 field は他 Agent 共存のため許容し ignore（cheaper 設定とは解釈しない）（§16）
* output schema は `file` / `inline` の exactly one。相対 path は cwd ではなく Worker Definition ファイルのディレクトリ基準。`.json`/`.yaml`/`.yml` のみ、remote schema 対象外（§15）

## Verification

* [ ] 必須 field 欠落・空値・未知 `cheaper:` field・未知 tool が `INVALID_WORKER_DEFINITION`/`UNKNOWN_TOOL` となる単体テスト
* [ ] `file`/`inline` schema の exactly-one 違反が拒否される単体テスト
* [ ] top-level 未知 field が警告なく ignore される単体テスト
* [ ] ファイル名と `name` の不一致が `WORKER_NAME_MISMATCH` となる単体テスト

## Pros and Cons

### TypeScript モジュール

* Good, because 型安全で IDE 補完がある。
* Bad, because Worker 追加がコード変更になり §2.3/§44 に反し、user-defined Worker の配布も難しい。

### JSON のみ

* Good, because 厳密に機械処理できる。
* Bad, because 長い instruction を JSON string で書くのは非現実的で、他 Agent スキル形式と異なる。

### YAML frontmatter + Markdown

* Good, because structured metadata と自由記述 instruction が同居し、既存エコシステムと親和する。
* Bad, because frontmatter の strict 検証を自前で持つ必要がある。

## More Information

* 引継ぎ §2.3, §8–§16, §44

## 更新履歴

### 2026-09-15: `cheaper.input.schema` の追加

* ユーザー承認により引継ぎ §12 を拡張し、`cheaper:` 配下に任意項目 `input.schema`（`{file}` / `{inline}` の exactly one）を追加した。ある場合は実行前に入力 JSON を検証し、不一致は `INVALID_INPUT`（exit 2）。frontmatter 厳格性（`cheaper:` 配下は `model`/`input`/`output` のみ）は維持。
