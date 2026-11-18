---
id: "TASK-034ZuedgAEVT4fbOtnEEap"
type: "task"
status: "done"
title: "author cheaper-workflow-create skill"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md"}
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-workflow-create-skill.md"
  implemented-by:
    - "docs/impl/ir/author-cheaper-workflow-create-skill.md"
  depends-on:
    - "docs/plans/implement-workflow-create-skill.md"
  blocks:
    - "docs/tasks/link-workflow-create-skill-from-guides.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
---

# author cheaper-workflow-create skill

## Work

`packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` を新設する。
構成は design の節構成に従い、発動条件(明示的作成依頼 or discover
提案の承認)・承認境界(定義保存と実行の分離・grant/policy/Route
非改変)・authoring 9 手順・Workflow JSON v1 anatomy・検証済み
template 5 形態(最小 worker・tool+worker `when` chain・fan-in・
inputs defaults・skip 耐性)・`workflow interface` 検証手順と error
対処表・catalog 登録手順・出力契約・良い/悪い例を含める。template
は `tmp/wf-fixtures/` で実測 pass した fixture と
`routed-exec.workflow.json`・`saved-workflow.json` の形に揃え、
`--input <file>|-` 規約・実在 worker/tool 名のみを使う。

## Classification

- `normal-plan-task`

## Upstream Documents

- plan: `../plans/implement-workflow-create-skill.md`
- design: `../designs/workflow-create-skill.md`
- spec: `../specs/workflow-create-skill-issue-204.md`
- test-spec: `../test-specs/workflow-create-skill-verification.md`

## Dependencies

- なし(#201 catalog 契約は merge 済み、CLI 検証は debug build で
  実測済み)。

## Affected Files

- `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` (new)

## Done When

- [ ] SKILL.md が design の節構成を持ち、APM 配置・相互 skill 参照
  規約(`cheaper-usage` 参照)に合う。
- [ ] 5 template(最小 worker・tool+worker `when`・fan-in・defaults・
  skip 耐性)が現行 `workflow interface` を pass する形で記載。
- [ ] invalid 対処表の error code が実測(INVALID_REQUEST/
  INVALID_INPUT + exit 2)と一致。
- [ ] catalog 登録手順が `{id,path,summary,tags}` 追記 + `workflow
  list` 確認で、重複 id の扱いを含む。
- [ ] 承認なしの `workflow run`・副作用 smoke・grant/policy/Route
  変更を skill が提案しない。
- [ ] `--input '{...}'` inline JSON・架空 worker/tool 名・
  `run --model`・stdin 定義・YAML を記述しない。

## Verification

- [ ] skill 内 template 相当の fixture が `workflow interface` を
  pass(実測済み fixture の形を使用)。
- [ ] `git diff --check` clean、doc_status audit blocking 0。
