---
id: "IMPL-034NgekixDuJNQy3znXgws"
type: "impl"
status: "completed"
title: "Verify route check contract and document usage"
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
    - "docs/tasks/verify-route-check-contract-and-document-usage.md"
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

# Verify route check contract and document usage

## Summary

`route check` の contract fixture・CLI HELP・guide 記載を approved plan の最終 slice として検証・補強した。既存 report 実装に対し、subprocess fixture の所有 module を task contract どおり `route_check` へ揃え、route resolve parity・post-merge policy attribution の不足 coverage を追加した。review gate の confirmed findings として、absent layer・sample indeterminate・normalized schema field・success/quiet stderr・spawn tripwire・internal error envelope の coverage も追加した。

## Context

`TASK-034N7DciWkGuUAOgrmnr5g`。approved plan の step 10、`route-check-diagnostics-issue-185` spec、`route-check-*` test specs、および read-only embedded boundary ADR に従う。#162 の実機検証は対象外。

## Implementation

### Changes

- `route_check` integration module へ malformed/reordered invocation の sanitized error・side-effect suppression fixture を移動し、task が要求する module 内 fixture 一覧と一致させた。
- `route_check_cargo_test_no_grant_and_granted_match_route_resolve` を追加し、no-grant/grant-enabled の双方で `route check` sample viability が `route resolve` と一致することを固定した。
- `route_check_policy_post_merge_failure_uses_effective_attribution` を追加し、user/project policy が各々 `loaded` のまま post-merge validation で失敗する場合に `layer: "merged"`・`source: "policy:effective"`・exit 3 となることを固定した。
- `route_check_read_only_probe_does_not_spawn_or_touch_state` を追加し、`CHEAPER_ROUTE_CHECK_CHILD_MODE`/`CHEAPER_ROUTE_CHECK_CHILD_MARKER` を制御する `spawn_probe` fixture test で grant executable（test binary 自身）の spawn を positive tripwire として検出する。marker 変数は grant `env:` block に焼き込み（managed spawn の `env_clear` 後も届く）、ambient env も併用して unmanaged spawn 経路も検出する。read-only 時は marker 未作成・`credentials.json` 不変・`usage.jsonl`/`history`/`route-logs`/`.cheaper` 未作成を固定した。
- absent layer assertion を追加し、user/project route layer の `status: absent`・`routeCount`/`invalidCount` 0・`error: null`・policy `layers.user`/`layers.project` の `absent` を明示した。
- layer failure + sample 指定時の `sample.status`/`matched`/`selected` `indeterminate`・viability 非出力・`route.sample-indeterminate` diagnostic を固定した。
- schema error の normalized `field` 射影（`routes[0].grants`）を integration で固定した。
- ambient logging 下の成功経路 `stderr: ""`、malformed `--quiet` で required stderr `error INVALID_REQUEST: route check failed` 保持、quiet/normal 成功の stderr 空を固定した。
- `internal_error_envelope_maps_to_exit_seventy_without_details_or_cause` unit test を追加し、`route_check_error_envelope` が `ok:false`/`INTERNAL_ERROR`/`route check failed`/exit 70/`reason: internal-error`・cause なし・内部 detail/secret 非漏出を満たすことを固定した。
- post-merge policy fixture を `process.tasks.shared` 衝突へ修正し、`PolicyDocumentLayer::Merged` attribution が実際に発火する経路へ揃えた。
- `route_check` suite は ambient logging・clean-temp embedded production path・absent/malformed layer・merge determinacy・policy/layer failure・missing/invalid workflow/worker/schema・pin unresolved/environment-dependent・sample/no-sample/indeterminate/unknown・sanitized output・quiet・dedicated deterministic text renderer・static capability vs unverified host・positive spawn tripwire を網羅する。

### Decisions During Implementation

- malformed invocation の parse-level detection は `args` module に残し、外部副作用まで観測する subprocess fixture は `route_check` module に配置した。unit parse coverage と end-to-end contract coverage の責務を分ける。
- exit 70 の internal-failure class は subprocess から再現可能な production failure seam を追加せず、`route_check_error_envelope` unit test・`sanitize_internal_error` coverage・canonical workspace test・sanitized process boundaryで検証済みとして扱う。実害経路を作るためだけの fault injection は read-only diagnostics の責務外と判断した（task fixture 文言との deviation として記録）。
- spawn tripwire は既存の self-re-exec child-marker 方式（`route_run` fixture と同型）を採用し、marker 変数を grant `env:` block へ宣言する。managed spawn は ambient env を `env_clear` するため、ambient env だけでは canonical exec 経路の回帰を検出できない（review finding として修正済み）。`route check` では marker 未作成を assert する。
- network/auth 非実行は専用 fault injection seam を追加せず、route check に network/auth 呼出し経路が存在しないこと・credential file 不変 probe で間接検証に留める。安全な注入 seam が無いため positive tripwire は spawn（子プロセス marker）に限定した。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo fmt --all` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::check` — 6 tests pass（`internal_error_envelope_maps_to_exit_seventy_without_details_or_cause` 含む）。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 22 tests pass。
- `mise exec -- cargo clippy -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --all-targets -- -D warnings` — pass。
- `mise run check` — exit 0（追加 fixture 後に再実行）。biome、fmt、workspace clippy、version/doc-id checks、workspace tests、node script tests、lockfile、APM payload checks が通過。
- `node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` — 1054 documents、blocking findings 0。
- `node .claude/skills/impl-doc/scripts/audit_impl_record.js --dir docs/impl/ir --json` — 207 files、findings 0。
- `git diff --check` — pass。

## Risks

- subprocess で exit 70 を発生させる deterministic fault injection は追加していない。internal failure envelope は `route_check_error_envelope` unit test・`sanitize_internal_error`・process panic/startup boundary の coverage に依存する。
- network/auth 非実行は呼出し経路不存在と credential 不変 probe による間接検証であり、専用 tripwire は無い。将来 network seam が導入される場合は同型の marker 方式で補強できる。

## Follow-ups

- なし。#162 の real-host verification は引き続き対象外。
