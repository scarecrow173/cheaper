---
id: "IMPL-034N7PuFShAmcfe3INqWrR"
type: "impl"
status: "completed"
title: "Add route check invocation detection and side-effect gates"
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
    - "docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md"
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

# Add route check invocation detection and side-effect gates

## Summary

`route check` の CLI grammar と pre-parse invocation detection、observability 完全 Off 化、generic Err path の read-only bypass を実装した。現段階では report 本体は未実装のため、valid `route check` は sanitized `INTERNAL_ERROR` envelope で exit 70 を返す。

## Context

TASK-034N7D9cw4atBZEhXxf1qm。承認済み plan の step 1–2、DESIGN-034N3kihuoqVnngaNEt7q2、および `route-check-read-only-invocation-boundary` test spec に従う。

## Implementation

### Changes

- `args.rs` に `RouteAction::Check { cwd, command, argv }` を追加し、sample option token が 2 個以上なら input error にした。
- `route_cmd.rs` の `parse_command`/`parse_argv` を `pub(crate)` seam 化し、`Check` arm を後続 task までの sanitized internal error placeholder とした。
- `args.rs` に `is_route_check_invocation` を追加し、strict parse と同じ option arity table と `--` terminator 規約で positional 先頭を畳むようにした。
- `cli.rs` で detection true 時に log level/format・stderr sink・log-root 解決を全て無効化した。
- `cli.rs` の generic Err arm で route check 専用 sanitized envelope を使い、config dir 解決・Usage・History を bypass した。
- `tests/integration/args.rs` に parse、detection matrix、fresh-process side-effect probe を追加した。

### Decisions During Implementation

- `route check --help`/`--version` は strict parse の Help/Version precedence に委ねるため、detector は invocation として true を返すが dispatch は diagnostics を実行しない。
- `route check` report 本体はこの task の所有範囲外のため、`RouteAction::Check` は後続 task で `route::check` へ差し替える placeholder error とした。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo fmt --all` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam args::` — 51 tests pass。
- `route_check_malformed_invocation_is_sanitized_and_has_no_side_effects` subprocess probe — ambient `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` および `--log`/`--log-format` 下で `.cheaper`・`usage.jsonl`・`history` が生成されず、exit 2 sanitized envelope と `route check --help` precedence を確認。

## Risks

- valid `route check` は後続 task が report orchestration を実装するまで sanitized `INTERNAL_ERROR` / exit 70 を返す。
- generic Err arm の input/internal 分類は CheaperError code に限定し、raw message/details/cause を serialize しない。

## Follow-ups

- `route::check` 本体が実装されたら `RouteAction::Check` dispatch を placeholder から実装へ差し替える。
- verify task で `tests/integration/route_check.rs` へ移行・拡張する。
