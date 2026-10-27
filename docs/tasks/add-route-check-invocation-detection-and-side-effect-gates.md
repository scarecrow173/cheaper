---
id: "TASK-034N7D9cw4atBZEhXxf1qm"
type: "task"
status: "done"
title: "Add route check invocation detection and side-effect gates"
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
    - "docs/test-specs/route-check-output-and-exit-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Add route check invocation detection and side-effect gates

## Work

approved plan の step 1 と step 2 に対応する implementation slice。`RouteAction::Check` の CLI grammar、pre-parse の `is_route_check_invocation` detection、logging 完全 Off 化、dispatch Err path での副作用 bypass を実装する。

- `args.rs` に `RouteAction::Check { cwd, command, argv }` variant を追加する。`--command` と `--argv` は XOR で 0 または 1 個のみ受理し、同時指定・2 個以上の sample は parse error にする。この cardinality check は `resolve` arm の check に巻き込まない。
- `route_cmd.rs` の `parse_command`（shell tokenization + `unsafe_command` flag）と `parse_argv`（JSON string array parse）を `pub(crate)` seam に昇格させ、sample input が shared parse 規約を再利用できるようにする。
- `is_route_check_invocation` を、strict parse と同じ `STRING_OPTIONS`/`BOOL_OPTIONS` arity table と `--` terminator 規約で token を畳む pre-parse helper として実装する。先頭 positional が `["route", "check"]` なら検出する。`--log debug route check`、`route --log debug check`、`route -- check`、`-- route check` のような global option reorder、unknown option・missing value を含む malformed invocation、余分な positional を fail-safe に検出する（reject 自体は strict parse に委ねる）。`route checks`・`route CHECK` は検出せず、global `--help`/`--version`（`route check --help` を含む）の既存 precedence は維持する。
- `cli.rs` の `run_cli` で parse 前に `is_route_check_invocation` を評価し、true なら `flag_level`/`flag_format`/`env_level`/`env_format` を全て `None`・stderr sink 無効・`log_root` に `PathBuf::new()` placeholder を持つ `Activation` を構築し、sink 構築を行わず return させる。ambient `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`/`--log`/`--log-format` が有効でも同一とする。
- 同じ detection flag を dispatch の generic Err arm に渡し、`resolved_user_config_dir`・builtin dir 解決・`record_usage*`・`finalize_worker_history` を bypass させる。Err arm は stable code・generic message・bounded reason のみを持つ sanitized error envelope を返す。
- 本 slice は strictly read-only: process spawn、network/auth、config/credential/History/Usage/RouteLogs/tracing file write を行わない。detection helper と Err-path gate は必ず同一の `is_route_check_invocation` を single source として共有する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/adr/use-a-read-only-embedded-resource-boundary-for-route-check.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- なし（plan の dependency graph 上、本 slice は step 3/5/6/7/8 の stream と並行して進められる）。

## Affected Files

- `crates/cheaper-cli/src/args.rs` — `RouteAction::Check` variant と XOR cardinality check
- `crates/cheaper-cli/src/route_cmd.rs` — `parse_command`/`parse_argv` の `pub(crate)` 化
- `crates/cheaper-cli/src/cli.rs` — `run_cli` の pre-parse detection、Off `Activation`、Err arm bypass
- `crates/cheaper-cli/tests/integration/args.rs` または新規 `route_check` fixture — detection unit/integration coverage（`tests/integration/main.rs` への登録は verify task が担う）

## Done When

- [ ] `RouteAction::Check { cwd, command, argv }` が parse でき、`--command`/`--argv` の同時指定と 2 個以上の sample が parse error になる。
- [ ] `is_route_check_invocation` が strict parse と同じ arity table・`--` 規約で reordered/malformed invocation を fail-safe に検出し、`route checks`/`route CHECK` を検出しない。
- [ ] ambient `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`/`--log`/`--log-format` 有効下で `.cheaper/logs`・tracing subscriber・file sink が一切生成されない。
- [ ] dispatch Err arm が `resolved_user_config_dir`/builtin-dir 解決/`record_usage*`/`finalize_worker_history` へ進まず、exit 2/70 が `ok:false` sanitized envelope を返す。
- [ ] global `--help`/`--version` の既存 precedence が維持され、`route check --help` は diagnostics を実行しない。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam args::` — `RouteAction::Check` parse、XOR cardinality、`is_route_check_invocation` の検出/非検出 matrix が pass する。
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — `CHEAPER_LOG=debug` 下の fresh-process probe で `.cheaper/logs`/file sink が生成されず、malformed invocation でも Err-path gate が有効なまま exit 2/70 が `ok:false` sanitized envelope を返すことを確認する。
