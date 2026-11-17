---
id: "TSPEC-034ZiF8TCafeWV4OHI9beT"
type: "test-spec"
status: "proposed"
title: "Workflow catalog manifest and list contract"
created: "2026-11-17"
updated: "2026-11-17"
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
    - "docs/specs/workflow-catalog-issue-201.md"
    - "docs/designs/workflow-catalog-manifest-and-read-only-list.md"
  derived-by: []
  verifies:
    - "docs/specs/workflow-catalog-issue-201.md"
    - "docs/designs/workflow-catalog-manifest-and-read-only-list.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow catalog manifest and list contract

## Purpose

`workflow list` の envelope/exit 契約、catalog manifest の schema/bound
規則、entry status/diagnostics の分類、fail-closed と一覧診断の二分が、
利用者・親 Agent 向け契約どおりに維持されることを保証する。catalog
の位置づけ(発見用 index)や schemaVersion が変わった場合にのみ改訂する。

## Feature

Feature: `workflow list` は project catalog manifest を bounded に読み、
`{catalog, workflows, diagnostics, truncated}` を `ok:true` の utility
envelope で返す。manifest 破損は `ok:false` error envelope、entry 単位の
異常は per-entry status + diagnostics で報告し、compiler・private
DAG/prompt・worker/tool/model を一切実行しない。

## Rules

- catalog path は `<workspace root>/.cheaper/workflows/catalog.json`
  固定で、manifest 欠如は `present:false`・空 `workflows`・空
  `diagnostics`・exit 0 の正常な空一覧とする。
- manifest は `{"schemaVersion":1, "workflows":[{id,path,summary,tags?}]}`
  で、64 KiB・256 entries・field 長 cap(id<=64・path<=512・
  summary<=500・tags<=16×64)を超えた場合・JSON parse 失敗・
  `schemaVersion` 非対応・`workflows` 非 array は `ok:false` の
  error envelope + 非 0 exit とする。
- entry `path` は `resolve_workspace_path` + `policy.filesystem` の
  deny/symlink 入力で検証し、結果は `status`:
  `ok`/`missing`/`invalid`/`duplicate` のいずれかとする。
- diagnostics `kind` は `malformed-entry`/`duplicate-id`/
  `missing-path`/`path-out-of-workspace`/`denied-path`/
  `not-regular-file` のいずれかで、`index`/`id`/`path`/`message` を
  持ち、raw file 内容・private definition field を含めない。
- duplicate `id` は該当全 entry を `status:"duplicate"` として両方
  diagnostics で報告し、黙った last-wins 上書きをしない。
- `workflows[]` の出力 field は `id`/`path`/`summary`/`tags`/`status`
  のみとし、private jobs/prompt/binding/worker/model/resource/
  definition 本文を含めない。
- `--limit` は entries を打ち切り `truncated:true` を立てる。
  `workflow list` の受理 option は `--cwd`/`--format`/`--quiet`/
  `--log`/`--log-format`/`--limit` のみ。
- `workflow list` は `worker:null` の utility envelope で usage
  record を残さず、workflow file の read・compile・worker/tool/model
  起動・grant/policy 変更・file 書込みを一切行わない。

## Examples

- Given catalog なしの workspace、when `workflow list` を実行すると、
  `ok:true`・`present:false`・空一覧で exit 0 になる。
- Given `{"schemaVersion":1,"workflows":[{"id":"a","path":"wf.json",
  "summary":"s"}]}` と存在する `wf.json`、when list すると、
  `workflows[0].status:"ok"` で `jobs`/`prompt` 等の key を含まない。
- Given `{ not json` の catalog.json、when list すると、`ok:false`
  error envelope で非 0 exit になる(空一覧で誤魔化さない)。
- Given 同一 `id` を持つ 2 entry、when list すると、両 entry が
  `status:"duplicate"` で `duplicate-id` diagnostic が出る。
- Given `path:"../outside.json"` の entry、when list すると、
  `status:"invalid"` + `path-out-of-workspace` diagnostic になる。
- Given `path:"deleted.json"` の entry、when list すると、
  `status:"missing"` + `missing-path` diagnostic になる。
- Given 3 件の valid entry と `--limit 2`、when list すると、
  `workflows` が 2 件で `truncated:true` になる。
- Given `workflow list --input x`、when parse すると、
  `--input is not valid for workflow list` の INVALID_REQUEST になる。

## Guarantees

- manifest path・schema・bound・fail-closed 境界を固定する。
- entry status と diagnostic `kind` の分類・duplicate の非 last-wins
  を固定する。
- 出力が private workflow data を含まず read-only であることを固定
  する(副作用・usage 記録なし)。
- option 受理集合と `--limit` の truncated 契約を固定する。

## Non-goals

- `workflow interface`/`workflow run` の既存 contract は別 spec が担う。
- catalog 書込み・登録 CLI・#204 create skill の手順は対象外。
- list による workflow 内容の正当性(compiler 通過)は検証しない。

## Risk

これらの保証が欠けると、壊れた manifest が空カタログと誤認されて
discover が候補を見落とし、stale/duplicate entry が黙って別 Workflow
へ置き換わり、private DAG 内容が親 Agent context へ漏出して省トークン
目的が損なわれる。
