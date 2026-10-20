---
id: "IMPL-034Jm123U9YXuFo4LBNbtz"
type: "impl"
status: "completed"
title: "Add route CLI wiring"
created: "2026-10-20"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
        note: "RouteAction::Run + --payload parse + strict flags"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
        note: "execute signature +cwd/+quiet(record_route へ伝播)"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
        note: "RouteOutput enum + Run dispatch"
      - type: "file"
        path: "crates/cheaper-cli/src/cli.rs"
        note: "Route dispatch Utility/Run 分岐 + log_root 'route' + HELP"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
        note: "Run parse/reject tests + message pin 更新"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
        note: "run_cli_raw helper + 4 CLI tests"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/cli.rs"
        note: "route --help smoke test"
      - type: "file"
        path: "crates/cheaper-cli/tests/observability.rs"
        note: "route --cwd log root anchor test"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-cli-wiring.md"
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

# Add route CLI wiring

## Summary

D4 — `route run --payload` CLI arm を dispatch 層に配線した。
utility envelope を bypass して `route::run::execute` の raw text
を stdout へ直接 emit(exit code mirror)。`route --help` は全 3
arm と新 option を表示する。

## Context

- task: `docs/tasks/add-route-cli-wiring.md`
- upstream: `docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md` Phase D4
- 依存: D1 routed 実行(route::run::execute)、D3 logs arm

## Implementation

### Changes

- `args.rs`: `RouteAction::Run { payload, cwd }`。`--payload` を
  STRING_OPTIONS/Values に追加し `route run`(2 positional)で必須
  nonempty を enforce、per-arm allowlist は
  `[payload,cwd,format,quiet,log,log-format]`。route-only field
  guard に payload を追加し非 route command でも reject。message
  を `route requires resolve | run | logs <id>` に更新。
- `route/run.rs`: `execute(token, cwd, quiet, io, hooks, started_at)`
  — `cwd.unwrap_or(&io.cwd)` で prepare(workspace canonical 比較は
  変わらない crossproject 拒否)、`resolve_workspace_layout(cwd)`、
  `quiet` を `FinishArgs` 経由で `record_route` へ伝播(従来
  hardcode false)。
- `route_cmd.rs`: `pub enum RouteOutput { Utility(CommandOut),
  Run(RouteRunOutput) }` — Resolve/Logs は Utility、Run は
  `route::run::execute(payload, cwd.as_deref(), args.quiet,..)`。
- `cli.rs`: `ParsedCli::Route` で Utility → write_output(format)、
  Run → `io.stdout.write_str(&out.text)` + `out.exit_code`(raw —
  `--format` は成功時に影響しない。infra error は従来の catch で
  generic envelope)。`resolve_log_root` の `--cwd` relevant に
  `"route"` を追加。HELP に route 3 arm + `--command/--argv/
  --payload/--offset/--max-bytes` + `--format` の raw-text 注記を追加。

### Decisions During Implementation

- `RouteOutput::Run` を `CommandOut` に載せず独立 enum にした
  (generic CommandOut の全 command への影響を避ける)。
- `["route","run"]` の stale test pin を `["route","bogus"]` へ
  更新(run arm 実装で shape が valid になったため)。

## Related Experiments

### Adopted

### Rejected

## Validation

- `--test integration args::` → 45/45 pass(`tmp/handoff-d4/args.txt` — Run parse shape + 全 foreign flag/payload 流用 reject)。
- `--test integration route_run::` → 18/18 pass(`tmp/handoff-d4/route-run.txt` — raw text emit・exit7・no-envelope・format 不変・invalid payload envelope・quiet suppress・cwd override 4 case)。
- `--test integration route::` → 5/5、`route_logs::` → 8/8(`tmp/handoff-d4/route-mod.txt`/`route-logs.txt`)。
- `--test integration cli::` → 7/7(`tmp/handoff-d4/cli.txt` — `route --help` 3 arm + flag 表示)。
- `--test observability route_cwd` → 1/1(`tmp/handoff-d4/obs.txt` — log root anchor)。
- `cargo fmt --all -- --check` → clean(`tmp/handoff-d4/fmt.txt`)。
- `clippy --all-targets` → warning 0(`tmp/handoff-d4/clippy.txt`)。
- impl-doc audit: No findings。

以下は当時 log 結果の要約であり、本記録は tmp file 無しで
自己完結する(tmp path は履歴 label であり参照前提ではない)。
数字は D4 時点の suite 規模であり現在の test 総数ではない。

| suite | passed / failed |
| --- | --- |
| `args::` | 45 / 0 |
| `route_run::` | 18 / 0 |
| `route::` | 5 / 0 |
| `route_logs::` | 8 / 0 |
| `cli::` | 7 / 0 |
| `observability route_cwd` | 1 / 0 |

## Risks

- なし(execution 本体は D1 で検証済み、CLI 層は dispatch のみ)。

## Follow-ups

- E: worker observable read evidence + 全 workspace gate。
