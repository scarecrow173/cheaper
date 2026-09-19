---
id: "TSPEC-034rzVr1S8qFdi1JqWSN9Z"
type: "test-spec"
status: "approved"
title: "model exclude and explicit selection guards"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/model-pricing-catalog-and-curated-exceptions.md"
  derived-by: []
  verifies:
    - "docs/specs/model-pricing-catalog.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# model exclude and explicit selection guards

## Purpose

`exclude` が eligibility として resolver 候補と `--model` 直接指定の
双方に効き、explicit 指定でも exclude/availability/policy を突破しない
ことを固定する。`exclude` が tier 値に混入する設計変更が起きた場合に
検知する。

## Feature

モデル除外と直接指定のガード

## Rules

- `exclude` は exact `provider/model` と `provider/*` wildcard を
  解釈し、tier resolver の candidate set から先に除外する
- `exclude` は user/project 層で union 合成される（project 層が
  user の除外を消さない）
- `--model <provider>/<id>` は tier classification を bypass するが、
  exclude 一致時は `POLICY_VIOLATION`、catalog 不在時は
  `MODEL_TIER_UNAVAILABLE`、effective tier が `maxTier` 超過時は
  `POLICY_VIOLATION` となる
- explicit 選択は `meta.model.selection: "explicit"` を記録し、
  tier 経路は `"tier"` を記録する

## Examples

- `exclude: ["openai/*"]` で `--model openai/gpt-5` →
  POLICY_VIOLATION
- 存在しない `--model foo/bar` → MODEL_TIER_UNAVAILABLE
- user `exclude: ["a/m"]` + project `exclude: ["b/n"]` → 両方除外
- `--model` 指定 model の effective tier が maxTier を超える →
  POLICY_VIOLATION

## Guarantees

- ユーザーが除外した model はいかなる経路でも選択されない
- 直接指定でも policy の上限は維持される

## Non-goals

- provider 単位の auth readiness の詳細判定
- exclude 解除（un-exclude）構文

## Risk

- exclude を bypass できると、コスト/コンプライアンス上禁止された
  model が実行される
