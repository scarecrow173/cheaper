---
id: "TSPEC-034N7wKi3nujK4sqlLRk9n"
type: "test-spec"
status: "proposed"
title: "Route check output and exit contract"
created: "2026-10-27"
updated: "2026-10-27"
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
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  derived-by: []
  verifies:
    - "docs/specs/route-check-diagnostics-issue-185.md"
    - "docs/designs/route-check-read-only-diagnostics-boundary.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Route check output and exit contract

## Purpose

`route check` の report 形状・stable `route.*` code・allowlist sanitization・text renderer・envelope/exit class 写像・unverified host 表現が、利用者向け契約どおりに維持されることを保証する。output/exit 契約が変更された場合にのみ廃止する。

## Feature

Feature: `route check` は allowlist 済みの deterministic report を `ok:true`/`ok:false` envelope と分類済み exit class で返し、host 実行状態を `unverified` として明示する。

## Rules

- `RouteCheckReport` は `schemaVersion`/`sampleProvided`/`merge`/`layers`/`policy`/`routes`/`summary` を持ち、canonicalized `cwd` は内部評価のみに使い serialize しない。
- diagnostic code は stable `route.*` namespace に限定し、`RouteLayerReport.error`・policy error は stable code・bounded reason enum・normalized schema field label（例: `routes[0].match.command`）・logical layer-source のみを持つ allowlist object とする。
- JSON/text いずれの出力にも raw `argv`/`args`/`execArgs`/`command`/env value、cwd、host absolute/home/temp path、`searched` candidate path、OS error Display、errno、`cause` chain、source snippet、schema-violation の free-form detail、secret-bearing error detail を含めない。`source` は `<layer>:<relative>` の logical path（例: `builtin:workflows/routed-exec.workflow.json`）に限定する。
- `--format text` は generic `utility_text` fallback ではなく `text_format.rs` から dispatch される route-check 専用 deterministic renderer（layer → route → diagnostic の確定的行）を使う。`--quiet` は auxiliary stderr diagnostics のみを抑制し、stdout report・exit code・必須の stderr error diagnostic は変えない。
- exit class は 0 = info のみ、1 = warning あり・error なし、2 = CLI input-stage error（`--command`/`--argv` 同時指定・malformed `--argv`・unknown option・cwd canonicalize 失敗）のみ、3 = error severity の diagnostic が 1 件以上、70 = 上記に分類できない internal failure とする。warning と error が同居すれば 3。
- exit 0/1/3 は `ok:true` の utility envelope `result` に report を載せ、2/70 は `ok:false` の sanitized error envelope とする。diagnosable な layer/policy/workflow/worker/schema failure は report diagnostic へ fold し `Err` へ伝播しない。input error 以外の escape は detail/cause を引き継がない `InternalError` wrap とする。
- renderer capability は静的に報告し、host registration/approval/execution は常に `unverified` 固定とする。CLI `HELP`・`docs/guides/parent-agent-integration.md`・`docs/guides/configuration.md` が usage・exit class・read-only 制限を記載する。

## Examples

- Given info severity のみの診断結果、when envelope と exit を射影すると、`ok:true` report envelope で exit 0 になる（host `unverified`・`requires-sample-input` があっても同じ）。
- Given no-grant のような warning 診断のみ、when envelope と exit を射影すると、`ok:true` で exit 1 になる。
- Given layer parse failure、when diagnostic を生成すると、report の error severity entry に fold され `ok:true` で exit 3 になる。
- Given `--command` と `--argv` の同時指定、when input stage が error を作ると、`ok:false` sanitized envelope で exit 2 になる。
- Given diagnostic folding を漏れた内部 error、when boundary が wrap すると、detail/cause を持たない `ok:false` envelope で exit 70 になる。
- Given `--quiet` と warning 診断、when 出力すると、auxiliary stderr のみが抑制され stdout report と exit 1・必須 error diagnostic は維持される。

## Guarantees

- report 形状と `route.*` stable code、error object の allowlist shape を固定する。
- 出力が raw argv/command/env・cwd・host path・OS error・errno・cause・searched path・source snippet・schema-violation free-form detail・secret を含まないことを固定する。
- exit 0/1/2/3/70 と `ok:true`/`ok:false` envelope の写像、diagnosable failure が `Err` へ漏れないことを固定する。
- dedicated text renderer、`--quiet` の意味、HELP/両 guide の coverage、host `unverified` 表現を固定する。

## Non-goals

- sanitization 対象となる diagnostic の生成条件・layer/sample 意味は他の route-check spec が担う。
- Err path での Usage/History bypass や ambient logging probe は invocation-boundary spec が担う。
- exit code の実行成否としての解釈（exit code は実行結果ではなく診断分類）や #162 の実 host 検証は対象外とする。

## Risk

これらの保証が欠けると、sample 由来の secret や host path が診断出力へ漏出し、diagnosable error が `ok:false` envelope に混同して exit class 契約が壊れ、静的 capability と未検証の host 状態が利用者に区別できなくなる。
