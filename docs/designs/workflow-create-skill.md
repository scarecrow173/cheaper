---
id: "DESIGN-034ZubMMzFoN5Put3FrXWi"
type: "design"
status: "approved"
title: "cheaper-workflow-create skill authoring and validation flow"
created: "2026-11-18"
updated: "2026-11-18"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/204"
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
    - "workflow-catalog-manifest-and-read-only-list.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/workflow-create-skill-issue-204.md"
    - "../adr/workflow-create-skill-uses-interface-validation-and-reviewable-catalog-diff.md"
    - "../discovery/workflow-create-skill-issue-204.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# cheaper-workflow-create skill authoring and validation flow

## Context

Epic #200 の L2 create 層。親 Agent が user 承認のもと Workflow JSON
v1 を設計・保存・検証・登録する手順を、新規 CLI surface なしに
ADR-034Zub9WUEyiDT9ATk2WAg の境界(`workflow interface` 検証 +
reviewable catalog diff + 実行は責務外)で教える APM skill を設計する。
schema 正本は `crates/cheaper-workflow/src/schema.rs` の
`WORKFLOW_JSON_SCHEMA`、登録先は #201 の
`.cheaper/workflows/catalog.json` manifest。

## Scope

### In Scope

- `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` 単一
  file の構成(front matter・各節・template・例)。
- skill が教える手順: 契約確定 → catalog 比較 → JSON v1 組立 →
  path 決定 → `workflow interface` 検証 → catalog 登録 →
  invocation 例提示。
- `docs/guides/workflow-usage.md`・`parent-agent-integration.md`
  への最小導線。

### Out of Scope

- skill 内 reference file・script の追加(単一 SKILL.md で完結)。
- CLI/worker/tool/schema の code 変更、catalog 書込み CLI。
- `workflow run` 実行・副作用 smoke の手順化(承認が別途必要)。

## Components and Boundaries

skill 本文の節構成(既存 skill の語彙・構造に合わせる):

- front matter `name`/`description` — 発動条件(明示的作成依頼 or
  discover 提案の承認)を description に含める。
- `## When to use` — 発動条件と非発動条件(自動候補発見だけでは
  永続化しない、直接 tool/単一 worker で足りる場合は Workflow 化
  しない)。
- `## Approval boundary` — 定義保存と実行の分離、grant/policy/
  Route 非改変、catalog・既存 file 上書きは事前確認。
- `## Authoring procedure` — issue 本文の 9 手順を skill 向けに
  再構成(契約 → 比較 → 組立 → path → 検証 → 登録 → 例)。
- `## Workflow JSON v1 anatomy` — schema 要点(4 必須 field・job
  shape・`when` ops・binding 3 形・RFC 6901 pointer・default/
  requiredness・実在 worker/tool 名)。
- `## Templates` — 検証済みの完全 JSON 5 形態(最小 worker・tool→
  worker `when` chain・fan-in・defaults・skip 耐性)。
- `## Validate` — `workflow interface` 実行・pass/fail 判定・
  代表的 error の対処表。
- `## Register in the catalog` — manifest 追記手順・重複/競合の
  扱い・`workflow list` 確認。
- `## Report back` — 出力契約(path・inputs/outputs 要約・検証結果・
  登録状態・副作用・invocation 例)。
- `## Good vs bad` — 良い例/悪い例の対比表。

## Data and Control Flow

```text
user 承認(明示依頼 or discover 提案承認)
  -> 公開 input/output 契約の確定(必要な input・返す output・副作用)
  -> cheaper workflow list で既存候補を比較(再利用/改修を優先)
  -> JSON v1 組立(schemaVersion 1・inputs・jobs・outputs)
  -> 配置 path 決定(既定 .cheaper/workflows/<purpose>.json、
     canonical --cwd 境界内・上書きは事前確認)
  -> cheaper workflow interface <path> --cwd <workspace>
       ok:true  -> inputs/outputs の required/default/schema を確認
       ok:false -> error.code/message で修正して再検証
  -> .cheaper/workflows/catalog.json へ {id,path,summary,tags} 追記
     (id 重複は別名にするか user に確認 — 黙って上書きしない)
  -> cheaper workflow list --cwd <workspace> で status:"ok" を確認
  -> 結果報告(path・契約要約・検証結果・登録状態・
     workflow run <path> --input <file|-> の呼出し例)
```

## Non-Functional Constraints

- skill の例は全て現行 `workflow interface` が accept する形
  (実 fixture で検証済み)。pointer は producer の output schema で
  closed に確かめられる path のみ使う。
- `--input <file>|-`・`--cwd`・utility/run envelope の語彙は
  `cheaper-usage` skill( CLI 正本)に揃える。
- 高価な model 呼出しを不必要に連結しない — 中間出力が親へ
  漏れない公開 outputs を設計する指針を含める。
- `CONDITION_SKIPPED` は伝播しない — skippable job を参照する
  output/consumer は nullable schema か自身の `when` で守る旨を
  明記する。

## Risks and Trade-offs

- skill が教える手順は実行時にしか強制されない — 誤手順
  (inline `--input`・架空 worker 名)は `workflow interface` の
  fail closed が受け止めるため、skill 本文に「検証を省略しない」
  旨を明記する。
- catalog manifest は hand-edit — 構造違反は `workflow list` の
  error envelope で検出可能だが、skill は追記後の確認を必須手順
  にする。
- 単一 file に例を集約するため SKILL.md が長くなる — template は
  検証済み最小形に絞り、複合例は節を分けて reader load を抑える。

## Deferred Design Concerns

- catalog 書込み CLI | #201 deferred・file 契約で完結 | skill 運用で
  手順ミスが実害化した段階 | なしでも成立。
- #203 discover skill との相互参照 | #203 未着地 | discover skill
  公開時に description/関連節へ追記 | 片方向参照のみで不整合なし。

## Verification Notes

- [ ] skill 内 5 template が `workflow interface` を pass
  (実 fixture で検証済みの形をそのまま使う)。
- [ ] invalid case(unknown worker・未知 `$input`・impossible
  pointer・invalid wrapper・malformed・非対応 version・invalid
  default・cycle・余剰 key)が fail する対処表が実測と一致。
- [ ] catalog 追記 → `workflow list` の `status`/`diagnostics`
  確認手順が #201 契約と一致。
