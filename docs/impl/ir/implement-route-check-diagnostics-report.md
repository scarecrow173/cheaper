---
id: "IMPL-034NdxQhfMatCG95DXPmH1"
type: "impl"
status: "completed"
title: "Implement route check diagnostics report"
created: "2026-10-27"
updated: "2026-10-27"
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
  implements:
    - "docs/tasks/implement-route-check-diagnostics-report.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement route check diagnostics report

## Summary

`route check` の placeholder を、builtin/user/project route layer を独立評価する read-only diagnostics report へ置き換えた。report は merge provenance、policy/grant readiness、workflow/worker/schema validity、sample match state、static renderer capability、stable `route.*` diagnostic、severity-based exit class を deterministic に返す。

## Context

`TASK-034N7DbHZTQLhRywkbRruo`。approved plan の step 9、`route-check-diagnostics-issue-185` spec、read-only embedded boundary ADR、および `route-check-*` test specs に従う。実行・host integration・#162 実機検証は対象外。

## Implementation

### Changes

- `route/check.rs` を追加し、`schemaVersion`/`sampleProvided`/`merge`/`layers`/`policy`/`routes`/`summary` を持つ report projection、`Diagnostic`/`SeverityCounts`、diagnostic → exit 0/1/3 mapping、input-stage 以外の escape を detail/cause 無し `InternalError` へ wrap する `sanitize_internal_error`、dedicated `text_report` renderer を実装した。
- `route_cmd.rs` の `RouteAction::Check` を placeholder error から `route::check::execute` へ接続し、`route/mod.rs` に module を登録した。
- `route/supply.rs` に source-aware `RouteCheckSources`/`RouteLayerSource`/`RouteWorkflowInput` を実装し、embedded builtin routes/policy/workflow/worker を logical `/builtin/` catalog から read-only に取得する。filesystem layer は従来どおり canonical loader/guarded file boundary を使う。
- `workflow_context.rs` に `load_workflow_from_with`/`load_workflow_document` と `WorkerIndex` preload/failure model を追加し、route-check source から worker definition/schema resolution を注入しても canonical `validate_definition` path を通るようにした。
- policy は `load_policy_documents` を使い、builtin/user/project の `absent|loaded|failed` status と post-merge failure の `merged`/`policy:effective` attribution を report する。layer acquisition failure は routes report を止めず `route.policy-load` diagnostic へ fold する。
- route row は `RouteMergeStatus` を `active|suppressed|shadowed|invalid|indeterminate` へ投影し、invalid duplicate row は shallow projection のみに留めて workflow/grant evaluation を行わない。
- workflow target は missing/invalid/convention/worker missing/worker invalid/worker schema を stable diagnostic code へ写像する。missing worker が workflow schema の `InvalidRequest("unknown worker:…")` として返る経路は内部 message prefix で分類し、raw message は serialize しない。
- grant set は `{route.grant} ∪ grants ∪ {substitute.grant}` を `is_route_eligible` + `pin_exec_grant` で静的評価し、`missing`/`ineligible`/`pin-unresolved`/`environment-dependent` を報告する。process spawn は行わない。
- optional sample は shared `match_command`/`resolve_with_target_exists` を使い、source-aware `missing-target` と route resolve parity の viability を返す。no-sample は `requires-sample-input`、layer failure は `indeterminate`、policy failure は winner viability `unknown` とする。
- renderer target は `TargetCapability::all()` を投影し、registration/approval/execution を常に `unverified` とする。codex approval bypass のみ policy-dependent status を返す。
- selected sample が `missing-target` の場合、same-cause `route.workflow-missing` diagnostic を error severity へ引き上げ、diagnostic code を複製せず exit 3 を満たす。`invalid-route` は per-name `route.invalid` error rows が既に severity を供給する。
- `layers[].invalidCount` は invalid declaration 数（`InvalidRouteGroup.names` の合計）を返し、per-name invalid rows と `merge.invalid` に揃える。`details.field`・schema `violation` 由来 label は同じ normalized field guard を通す。
- untrusted route/worker/grant 名を diagnostic message へ入れる場合は `metadata_label` で JSON escape + 64 chars 制限し、dedicated text renderer は全文字列投影を `text_field` で escape して multiline/ANSI injection を防ぐ。
- `text_format.rs` は report envelope shape を検出して専用 renderer へ dispatch し、generic utility fallback を使わない。`cli.rs` HELP と両 guide に usage/exit/read-only/renderer 限界/#162 除外を記載した。
- `main.rs` の async panic `Err` path と process-level `panic_exit` も `is_route_check_invocation` を共有し、route-check invocation では `route_check_error_envelope` の sanitized shape を使う。
- unit test 5 件と integration fixture 17 件を追加し、production embedded path、ambient logging 下の no-mutation、policy/layer failure、merge determinacy、workflow/worker/schema failure、grant readiness、sample parity、missing-target severity、invalid count、sanitized output、quiet/text/exit contract を固定した。

### Decisions During Implementation

- diagnosable failure は `Err` へ伝播させず report diagnostic へ fold した。`Err` は CLI input-stage と分類不能な internal failure に限定し、後者は stable code/message のみを持つ `InternalError` へ置き換える。
- workflow load error の raw detail を report せず、inner `CheaperError.code`（および missing worker だけを表す既知の `InvalidRequest` prefix）から bounded `route.*` code へ写像した。
- PATH/env に依存する executable pin は実行せず、成功しても `environment-dependent` として表示する。実 spawn による確定 runnable 判定は read-only contract 外行為のため行わない。
- filesystem workflow は file content を `route check` 側で読まず、`file_root + relative path` を `load_workflow_file` へ渡して policy guard を loader 側の単一境界へ残した。embedded builtin のみ document text を注入する。
- `RouteCheckSources.dirs.builtin_dir` は `/builtin/` sentinel を host path として使わないため、destructure でも束縛しない方針を維持し、worker/policy/route resource は `RouteLayerSource` の layer-aware source 経由に限定した。
- review の confirmed finding は同一責務境界で修正し、sample viability severity は diagnostic code を増やさず same-cause diagnostic の severity を変更する形にした。panic boundary は全 command 共通だが、`is_route_check_invocation` gate により route-check のみ sanitized envelope を選ぶ。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `cargo fmt --all` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::check` — 5 tests pass。report projection、diagnostic fold、no-sample state、sanitized internal wrap、normalized schema field、metadata/text escaping、dedicated text renderer を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 17 tests pass。ambient `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`・`--log`/`--log-format` 下でも config/temp/workspace mutation や builtin temp materialization が無いこと、policy/layer failure、sibling preservation、shadow/suppress、invalid workflow、missing/invalid/schema worker、pin unresolved、sample parity、selected missing-target の error severity/exit 3、invalid declaration count、raw secret/path/control-character 非漏洩、exit 1/2/3、quiet、dedicated deterministic text renderer を確認。
- `mise exec -- cargo clippy -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --all-targets -- -D warnings` — pass。
- `git diff --check` — pass。
- `node .claude/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir --json` — 206 files、findings なし。
- `node .claude/skills/doc-status/scripts/doc_status.js audit --type task --blocking --json` — blocking 0。
- `node .claude/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` — blocking 0。
- review-protocol — Reviewer A/B → Validator → Judge を実施し、confirmed `missing-target` severity、`invalidCount`、text/name projection、panic envelope の指摘を修正後に `ACCEPT WITH NON-BLOCKING NOTES`。残 notes の startup early-return sanitization も修正済み。

## Risks

- exit 70 は実災害経路として subprocess で誘発していないが、escape した非入力 error を detail/cause 無し `InternalError` へ wrap する単一境界と unit coverage はある。
- PATH/env 依存の grant pin は静的評価であり、成功時も `environment-dependent` としか表明しない。実 executable spawn による最終 runnable 保証は read-only contract 上行えない。
- ambient network/auth/host integration は呼び出しを持たない構造で否定しているが、host registration/approval/execution は仕様どおり `unverified` のままである。
- guide/HELP の記載は本 task で先行して更新済みだが、canonical `mise run check` と document audit は後続 verification task の正式 evidence として扱う。

## Follow-ups

- `verify-route-check-contract-and-document-usage` task で全 fixture の再確認、document audit、canonical `mise run check` を実行する。
- issue #162 の real-host registration/approval/execution 検証は今回も実施せず、upstream issue で扱う。
