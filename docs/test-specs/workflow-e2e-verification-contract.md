---
id: "TSPEC-034ZwWOpIL6QGfZGmkj7uq"
type: "test-spec"
status: "approved"
title: "workflow E2E and APM distribution verification contract"
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-e2e-verification-issue-205.md"
    - "docs/designs/workflow-e2e-verification.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-e2e-verification-issue-205.md"
    - "docs/designs/workflow-e2e-verification.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# workflow E2E and APM distribution verification contract

## Purpose

issue #205 が epic #200 の assembled 導線（catalog 発見 → L1 比較 →
承認 → L2 authoring/登録 → 明示 path 再利用）と APM 配布を、実際の
binary/package に対して測定した証拠契約を固定する。catalog schema・
compiler validation・envelope・APM deploy 規約が変わった場合にのみ
改訂する。

## Feature

Feature: scratch workspace + isolated `CHEAPER_CONFIG_DIR` 上で、
tool-only Workflow JSON v1 が `interface` → catalog 登録 →
`workflow run` まで完走し、invalid 定義・catalog 破損・権限不足は
documented error で fail し、read-only 検査 command は history や
workspace `.cheaper/` を一切書かない。APM package は target ごとの
deploy root へ 7 skill と active target の hook bundle だけを配布する。

## Rules

- `workflow list` は absent catalog で `ok:true` +
  `catalog.present:false` + 空 `workflows`/`diagnostics` を返す —
  catalog 不在は fact であり error ではない。
- `workflow interface <path>` は Workflow JSON v1 の静的検証 +
  公開 `{inputs,outputs}` 射影のみを返す — jobs/prompt/binding は
  出力に現れない。invalid 定義は `INVALID_REQUEST`/`INVALID_INPUT`
  + exit 2 で、skill の error 対処表の message と一致する。
- `workflow run <path>` は明示 path のみ — catalog `id` の named
  lookup は存在せず、`id` を渡すと `cannot read workflow file`。
  `--input` は file path または `-` のみで inline JSON は
  `cannot read input file`。
- run envelope は `result` に bound な公開 outputs のみを持ち、
  `jobs[]` は `{id,kind,status,meta}` — private job input/result は
  含まれない。
- catalog manifest の破損（JSON・`schemaVersion`・非 array
  `workflows`・64 KiB/256 entry/field cap 超過）は `INVALID_INPUT`
  + exit 2 で fail-closed。entry shape 違反は per-entry
  `malformed-entry`/`duplicate-id`/`missing-path`/
  `path-out-of-workspace`/`denied-path`/`not-regular-file`
  diagnostics で `ok:true` を維持する。`--limit` は 1..256。
- `command`/`process` job は grant 未供給で run 時に
  `POLICY_VIOLATION` で失敗し、`interface` の静的検証は素通しする
  — catalog 登録は実行権限を与えない。
- `workflow list`/`interface`/`count-tokens`/`route check` は
  history record を書かない。`workflow run` は
  `<userConfigDir>/history/` に `kind:"workflow"`/`kind:"job"`
  record を書く — workspace `.cheaper/` には書かない。
- APM: 全 target で `.agents/skills/` または `.claude/skills/` に
  7 SKILL.md が deploy され `name`/`description` front matter が
  parse できる。hook bundle は active target の
  `<hooks_root>/apm-hook-<target>/` のみに出る。
- L0/L1/L2 の skill text 不変条件（proposal≠creation≠execution、
  no-write-before-approval、evidence label 規約、L0 抑制条件）は
  shipped SKILL.md 本文の存在で検証する — CLI では強制されない。

## Examples

- Given `catalog.json` 不在の workspace、when `workflow list`、
  then `{"ok":true,"result":{"catalog":{"present":false},"workflows":[],"diagnostics":[],"truncated":false}}` + exit 0。
- Given valid `file-inspect.json`（inputs: required `targetFile`・
  optional `searchDir`/`pattern`）、when `workflow interface`、then
  `ok:true` + `result.inputs.targetFile.required:true` /
  `result.inputs.pattern.required:false` + exit 0。
- Given catalog 登録済みの `file-inspect`、when `workflow list`、
  then entry `status:"ok"` で private field が含まれない。
- Given `{"targetFile":"src/main.rs"}` inputs、when `workflow run
  .cheaper/workflows/file-inspect.json --input -`、then `ok:true`
  `result={tokens:9,bytes:33,matches:"src/main.rs:1: fn main()…"}`
  + `jobs[]` が `{id,kind,status,meta}` のみ + exit 0。
- Given `{"$from":"a"}` (path 欠落) が `outputs[].value`、when
  `workflow interface`、then `INVALID_REQUEST: invalid Workflow JSON
  v1` + exit 2。同じ wrapper が job `input` 内なら
  `INVALID_REQUEST: invalid binding wrapper` + exit 2。
- Given `command` job（grant 未供給）、when `workflow run`、then
  job `status:"failed"` `error.code:"POLICY_VIOLATION"` +
  envelope `ok:false` `TOOL_EXECUTION_FAILURE` + exit 6。
- Given `id` 重複 2 entry、when `workflow list`、then 両 entry が
  `status:"duplicate"` + `duplicate-id` diagnostics（後勝ち上書き
  なし）。
- Given `summary` が 500 文字超の entry、when `workflow list`、then
  `INVALID_INPUT: workflow catalog entry <i> 'summary' exceeds 500
  characters` + exit 2（entry diagnostic ではなく manifest fail）。
- Given `path:` dep on `packages/apm` の consumer、when `apm
  install`、then 7 SKILL.md が target の skill root へ deploy され、
  非 active target の `apm-hook-*` は `targets:` 非一致で skip。

## Guarantees

- 検証の全 claim は `docs/impl/exp/` の jsonl に記録された実測
  output に基づく — 推測・未検証の主張を混入しない。
- catalog 経由で特別な権限・暗黙実行・named lookup が生じない。
- 実行結果は公開 outputs のみで、親 Agent は private DAG や大
  中間値を読む必要がない。
- 提案層（L1）と永続化層（L2）の skill text が承認境界を保持する。

## Non-goals

- worker job（model 実行）・billable path・`command`/`process` の
  成功 run は検証しない — engine test と `CHEAPER_E2E_OLLAMA`
  gate が担う。
- 実 consumer runtime の skill auto-firing・hook UI 経路は対象外
  （documented な fallback あり）。
- 親 agent token/cost 削減量は測定しない（`parentUsage`
  unavailable の境界は #44/#67/#183 と同じ）。

## Risk

この契約が欠けると、4 層の結合部（catalog ↔ skills ↔ CLI ↔ APM
payload）の drift が検出されず、壊れた定義・権限逸脱・配布漏れが
consumer へ届く。skill text 不変条件を軽視すると、提案層が
永続化・実行へ踏み込む承認境界の退行を見落とす。
