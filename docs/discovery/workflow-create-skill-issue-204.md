---
id: "DISC-034Zu8VG1BDxLMjnX4WKTr"
type: "discovery"
status: "active"
title: "cheaper-workflow-create skill for issue 204"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/204"
    - "https://github.com/scarecrow173/cheaper/issues/200"
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
    - "../specs/workflow-file-interface.md"
    - "../specs/workflow-contract-compiler.md"
    - "../specs/workflow-catalog-issue-201.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper-workflow-create skill for issue 204

## Exploration Goal

親 Agent が「Workflow JSON v1 を設計・保存・検証・登録する」手順を、
新しい CLI surface を追加せず既存 contract(`workflow interface`・
#201 catalog manifest・`workflow run --input <file|->`)だけで
一貫して実行できるかを確かめる。成果物は APM skill
`packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` で、
skill の手順は現行 Rust 実装が受理する形だけを教える必要がある。

## Key Issues and Assumptions

- Confirmed: Workflow JSON v1 の schema 正本は
  `crates/cheaper-workflow/src/schema.rs` の `WORKFLOW_JSON_SCHEMA`。
  top-level は `schemaVersion`(const 1)・`inputs`・`jobs`・`outputs`
  の 4 field 必須・`additionalProperties:false`。job は
  `{kind:"tool"|"worker", <kind>:<name>, input, needs?, resources?,
  when?}` で、`when` は `exists`(`right` 禁止)または
  `eq|ne|gt|lt`(`right` 必須)。
- Confirmed: binding wrapper は `{"$input":<name>}` /
  `{"$from":<jobId>,"path":"<RFC6901>"}` / `{"$literal":<value>}` の
  exact shape のみ(`parse_binding`、余剰 key や `$from` 単独は
  `invalid binding wrapper`)。string interpolation 経路は存在しない。
- Confirmed(実測): `cheaper workflow interface <path>` は definition
  validation + public interface 投影のみで、valid fixture(minimal
  worker・tool→worker `when` chain・fan-in)は `ok:true` +
  `result.inputs[name]={schema,required}`/`outputs[name]={schema}`、
  exit 0。malformed JSON は `INVALID_INPUT`、unknown worker・
  unknown `$input`・impossible pointer・invalid wrapper・非対応
  schemaVersion・invalid default・cycle・top-level 余剰 key は
  `INVALID_REQUEST` + exit 2 で fail closed(部分 interface を
  返さない)。invocation input・resource inference・worker/tool/model
  実行は起きない。
- Confirmed(実測): catalog は `<workspace>/.cheaper/workflows/
  catalog.json` の schemaVersion 1 manifest(`{id,path,summary,tags?}`)。
  `cheaper workflow list` が登録状態を `status`(`ok`/`missing`/
  `invalid`/`duplicate`)+ `diagnostics` で報告する — 同一 `id` の
  2 entry は両方 `duplicate` + `duplicate-id` diagnostics、missing
  file は `missing` + `missing-path` diagnostics。書込み CLI は
  存在せず、登録は manifest への reviewable な file edit。
- Confirmed: 実在 worker は `bulk-reader`/`code-writer`/`condense`/
  `doc-writer`/`tool-operator`、canonical tool は `read`/`write`/
  `edit`/`ls`/`grep`/`find`/`count_tokens`/`process`/`command`
  (`ToolCatalog::NAMES`)。`interface` は両者の存在を検証するため、
  架空の job/tool/worker 名は静的検証で `unknown tool|worker:` に
  なる — ただし custom worker は consumer の `.cheaper/workers/` に
  置かれ得るため skill は「定義した worker file 名を使う」経路も
  認める。
- Confirmed: `CONDITION_SKIPPED` は dependents へ伝播しない
  (engine.rs `finish_job`)。`when` 不成立で skip された producer
  への `$from` は `null` に materialize され、`when` を持たない
  dependent/output はその `null` で進む — output schema・後続 input
  schema は nullable を許容するか、dependent 自身の `when` で守る
  必要がある。
- Confirmed: `$from` は input・`when`・`needs` の何れでも dep edge
  を積むため `needs` は binding を伴わない順序付け専用。
  `validate_pointer_schema` は closed schema で証明できる不可能性
  のみ拒否し、open schema(`anyOf`/`$ref`/型未宣言)は runtime 残す。
- Assumption: 副作用を伴う実行(`workflow run`・`process`/`command`
  grant・file write)は skill の責務外 — 定義の保存と検証・登録まで。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| skill が手順+template を教え、`workflow interface` で静的検証し、catalog は agent が manifest を編集して `workflow list` で確認 | 新規 code 不要。検証の正本が compiler 自身で drift しない。登録 diff が人に review 可能 | 手順の正しさは skill 文書の品質依存 | Yes |
| `cheaper workflow create`/`register` CLI を新設 | schema/登録が code path で強制 | issue #204 非目標(CLI generator 本体)。#201 が deferred した catalog 書込み CLI の先行決定になる | No |
| skill が standalone JSON schema file を配布し agent が手で検証 | editor 連携に使える | `WORKFLOW_JSON_SCHEMA` は Rust literal で export されておらず、skill 内 schema が正本と drift する温床 | No |
| Workflow 定義を inline `workflow run --input '{...}'` で即時 smoke | 実行確認まで一気に進む | `--input` は file|`-` 規約で inline JSON を受理しない。副作用 smoke は承認境界違反 | No |

## Tentative Conclusions and Hypotheses

Confirmed: 検証の正本は `cheaper workflow interface <path>` — skill は
「書く → interface で確かめる → 修正 → catalog manifest へ追記 →
`workflow list` で確認 → invocation 例を提示」の手順を教える。
invalid definition は完成品にしないが、`workflow run` の実行・
grant/policy/Route の変更は skill の責務外(承認が別途必要)。

Confirmed: skill 本文の例は全て `workflow interface` が accept する
形に揃え、pointer は producer の output schema に照らして確実な
path(`command` → `/details/outputTokens`/`/details/logRef`/
`/details/argv`、tool 共通 → `/content/0/text`、worker → output
schema の property)だけを使う。

Hypothesis: skill は単一 SKILL.md で足りる — template は verified
fixture を直接埋め込み、外部 reference file は要らない(acceptance
の「検証可能なテンプレート」は本文内 code block で満たす)。

## Open Questions

- なし(blocking な未確定点なし — issue 本文が手順と境界を規定済み)。

## Promotion Candidates

- [x] spec-doc needed: skill の発動・承認境界・authoring 手順・
  出力契約・受け入れ条件を issue #204 から formalize。
- [x] adr-doc needed: 「検証は `workflow interface` のみ・catalog
  登録は reviewable な manifest edit・実行は skill 責務外」の境界と、
  catalog 書込み CLI を作らない決定を記録。

## Sources and Relations

Issue #204(親 Epic #200、#201 依存)が product source。確認済み code:
`crates/cheaper-workflow/src/schema.rs`・`bindings.rs`・`compiler.rs`・
`engine.rs`・`interface.rs`・`catalog.rs`、
`crates/cheaper-tools/src/catalog.rs`・`tools/command/mod.rs`・
`tools/process/mod.rs`、`crates/cheaper-cli/src/workflow_run.rs`・
`workflow_list.rs`・`resources/workflows/routed-exec.workflow.json`・
`resources/workers/schemas/*.json`、`benchmarks/scenarios/saved-workflow.json`。
実測: `target/debug/cheaper.exe workflow interface|list` の pass/fail
fixture 検証(tmp/wf-fixtures・tmp/wf-ws)。
