---
id: "TASK-034N7DciWkGuUAOgrmnr5g"
type: "task"
status: "todo"
title: "Verify route check contract and document usage"
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
  implements:
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-route-check-read-only-diagnostics.md"
    - "docs/tasks/implement-route-check-diagnostics-report.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-check-read-only-invocation-boundary.md"
    - "docs/test-specs/route-check-layer-and-policy-diagnostics.md"
    - "docs/test-specs/route-check-validation-readiness.md"
    - "docs/test-specs/route-check-sample-resolution-semantics.md"
    - "docs/test-specs/route-check-output-and-exit-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Verify route check contract and document usage

## Work

approved plan の step 10 に対応する implementation slice。`route_check` integration module による end-to-end contract fixture、`cli.rs` HELP・両 guide の document coverage、canonical `mise run check` を実施する。

- `crates/cheaper-cli/tests/integration/route_check.rs`（新規）を作成し `tests/integration/main.rs` に `mod route_check;` を登録する。fixture は以下を網羅する:
  - ambient logging（`CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`/`--log`）有効下での read-only probe（spawn・network・auth・History/Usage/RouteLogs/credential/log file write・embedded temp materialization の否定、cross-platform）。
  - malformed/reordered invocation の detection と fail-safe suppression・Err-path gate。
  - clean-temp `builtin_dir: None` production path での embedded builtin diagnostics。
  - absent/malformed layer、`shadowed`/`suppressed`/`invalid`/`indeterminate` の merge determinacy。
  - policy failure の per-layer `absent|loaded|failed` と pseudo-layer `merged`/`policy:effective` attribution。
  - builtin `cargo-test` の no-grant/grant-enabled parity（`route resolve` と同一判定）。
  - missing/invalid workflow、unresolved/`environment-dependent` executable readiness。
  - sample/no-sample/`indeterminate`/`unknown` の match/viability 意味。
  - output sanitization（raw argv/command/env・cwd・host path・OS error・errno・cause・searched path・source snippet・schema-violation free-form detail・secret を含まない）。
  - exit class 0/1/2/3/70 と `ok:true`/`ok:false` envelope、`--quiet` semantics、renderer capability vs unverified host、dedicated text renderer dispatch。
- `cli.rs` の `HELP` に `route check` usage を追加する。
- `docs/guides/parent-agent-integration.md` と `docs/guides/configuration.md` に usage・exit class・read-only 保証・`unverified`/`requires-sample-input` の意味・制約を記載する。
- named focused command で対象 fixture を実行した後、canonical `mise run check` を 1 度だけ実行する。focused test target が repository の integration layout と異なる場合は module filter を維持したまま実 target に合わせ、target の disable/skip はしない。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- `docs/tasks/implement-route-check-diagnostics-report.md`

## Affected Files

- `crates/cheaper-cli/tests/integration/route_check.rs` （新規） — 上記全 fixture
- `crates/cheaper-cli/tests/integration/main.rs` — `mod route_check;` 登録
- `crates/cheaper-cli/src/cli.rs` — `HELP` への `route check` 追加
- `docs/guides/parent-agent-integration.md`, `docs/guides/configuration.md` — usage・exit class・read-only 制限の記載

## Done When

- [ ] `route_check` integration module が `main.rs` に登録され、named fixture（ambient logging probe、malformed/reordered detection、clean-temp embedded production path、absent/malformed layer、merge determinacy、policy failure、cargo-test no-grant/grant parity、sample/no-sample/indeterminate/unknown、output sanitize、exit class、read-only probe、renderer capability vs unverified host）を全て含む。
- [ ] CLI `HELP` と `route` usage が `check` を含み、両 guide に usage・exit class・read-only 保証・`unverified`/`requires-sample-input` の意味・制約が記載される。
- [ ] blocking な document/index/relations finding が無い。
- [ ] `mise run check` が exit code 0 を返す。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 全 `route_check` fixture が pass する。
- [ ] `mise exec -- node .agents/skills/doc-status/scripts/doc_status.js audit --type all --blocking --json` — blocking document/index/relations finding が 0 件になる。
- [ ] `mise run check` — canonical verification が exit code 0 を返す。
