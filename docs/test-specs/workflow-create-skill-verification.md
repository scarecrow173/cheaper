---
id: "TSPEC-034ZubPv1oKQMgmITu1e4y"
type: "test-spec"
status: "proposed"
title: "workflow create skill verification contract"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
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
    - "../test-specs/workflow-catalog-manifest-and-list-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-create-skill-issue-204.md"
    - "docs/designs/workflow-create-skill.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-create-skill-issue-204.md"
    - "docs/designs/workflow-create-skill.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# workflow create skill verification contract

## Purpose

`cheaper-workflow-create` skill が教える検証・登録手順が、現行
`cheaper workflow interface` / `workflow list` の実契約と一致する
ことを保証する。compiler schema や catalog manifest 契約が変わった
場合にのみ改訂する。

## Feature

Feature: skill の template は `workflow interface` が accept する
Workflow JSON v1 であり、skill の invalid 対処表は実際の error
envelope(`INVALID_REQUEST`/`INVALID_INPUT` + exit 2)と一致する。
catalog 登録手順は `{id,path,summary,tags}` 追記 + `workflow list`
の `status`/`diagnostics` 確認で完結し、書込み CLI・`workflow run`・
権限変更を要求しない。

## Rules

- skill 内の各 template JSON は `schemaVersion:1`・`inputs`・`jobs`・
  `outputs` の 4 必須 field を持ち、`workflow interface <path>` が
  `ok:true` + exit 0 で `result.inputs`/`result.outputs` を返す。
- binding は `{"$input":<name>}` / `{"$from":<jobId>,"path":<ptr>}` /
  `{"$literal":<v>}` の exact shape のみ。string interpolation・
  `{$from:x}`(path 欠落)のような不完全 wrapper は invalid。
- `$from` の `path` は RFC 6901 pointer で、closed output schema で
  不可能と証明できる path(primitive の子・closed object の未知
  property・範囲外 array index)は `workflow interface` が拒否する。
- `when` は `{left,op:"exists"}` または `{left,op:eq|ne|gt|lt,right}`。
  skip された producer への `$from` は `null` に materialize される
  ため、それを参照する output schema は nullable を許容する。
- catalog 登録は `.cheaper/workflows/catalog.json` への entry 追記
  で、重複 `id` は両 entry が `status:"duplicate"` となることを
  `workflow list` で確認する。黙った上書きをしない。
- invocation 例は `cheaper workflow interface <path>` と
  `cheaper workflow run <path> --input <file|->` のみ。
  `--input '{...}'` inline JSON・`--model`(run に存在しない)・
  stdin 定義・YAML を skill が提案しない。
- 承認なしに定義 file・catalog・grant・policy・Route を書き換えず、
  `workflow run` を起動しない。

## Examples

- Given 最小 worker template(bulk-reader 1 job)、when
  `workflow interface` を実行すると、`ok:true` で
  `result.outputs.summary.schema` が `{type:"string"}` になる。
- Given tool→worker `when` chain(command→condense、
  `when:{left:$from exec /details/outputTokens, op:gt, right:$input
  tokenThreshold}`)、when interface すると、`ok:true` で
  `tokenThreshold` が `required:false`(default あり)になる。
- Given fan-in template(grep+find→tool-operator)、when interface
  すると、`ok:true` で両 producer が `needs`/`$from` で fan-in する。
- Given unknown worker `nonexistent-worker` の定義、when interface
  すると、`INVALID_REQUEST: unknown worker` + exit 2。
- Given `$from` pointer が primitive の子(`/summary/deep`)、when
  interface すると、`INVALID_REQUEST: primitive output has no
  child` + exit 2。
- Given `$input` が未宣言 input を参照、when interface すると、
  `INVALID_REQUEST: unknown input reference` + exit 2。
- Given `{$from:"x"}`(path 欠落)、when interface すると、
  `INVALID_REQUEST: invalid binding wrapper` + exit 2。
- Given catalog に同一 `id` の 2 entry、when `workflow list` を
  実行すると、両 entry が `status:"duplicate"` + `duplicate-id`
  diagnostics。
- Given missing path の entry、when `workflow list` すると、
  `status:"missing"` + `missing-path` diagnostics。

## Guarantees

- skill が配布する全 template は実測 `workflow interface` pass と
  一致し、invalid 対処表の error code は実測と一致する。
- catalog 手順は manifest schema/bound(#201 契約)と `workflow list`
  の status/diagnostics 分類に従う。
- skill は `workflow run`・grant/policy/Route 変更を承認なしに
  提案・実行しない。

## Non-goals

- `workflow run` の実行結果・engine 挙動・resource inference の
  詳細は別 spec(#62/#65)が担う。skill 使用時の親 Agent の
  token/コスト効果は検証しない。
- #203 discover skill・#205 E2E の手順は対象外。

## Risk

これらの保証が欠けると、skill が現行 compiler と drift した定義を
生成し、invalid definition が「完成品」として保存・登録される。
承認境界を欠くと、定義保存と実行の分離(#204 の核心)が崩れ、
副作用のない skill が暗黙に権限・実行へ踏み込む。
