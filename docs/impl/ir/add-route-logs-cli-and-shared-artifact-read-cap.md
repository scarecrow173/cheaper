---
id: "IMPL-034JlKZTnImZRXS5r7TSsf"
type: "impl"
status: "completed"
title: "Add route logs CLI and shared artifact read cap"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/logs.rs"
        note: "`route logs <id>` bounded read arm(policy cap + route-refetch record)"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_logs.rs"
        note: "route logs contract tests(8 cases)"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
        note: "reader(max_read_bytes) + validate_read_range 共有 + cap/range tests"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
        note: "RouteExecution.max_read_bytes — projected policy から注入"
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
        note: "mod logs 宣言"
      - type: "file"
        path: "crates/cheaper-cli/src/args.rs"
        note: "RouteAction::Logs + --offset/--max-bytes parse + strict numeric"
      - type: "file"
        path: "crates/cheaper-cli/src/route_cmd.rs"
        note: "execute_resolve 抽出 + Logs dispatch"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
        note: "maxBytes 1048576 → 32768(contract 修正)"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/args.rs"
        note: "RouteAction::Logs 対応の irrefutable→refutable + message 更新"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/main.rs"
        note: "mod route_logs 宣言"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-logs-cli.md"
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

# Add route logs CLI and shared artifact read cap

## Summary

D3 — `route logs <id> [--offset --max-bytes --cwd]` CLI arm と、
`RouteLogReader` への `RuntimePolicy.workflow.artifacts.max_read_bytes`
強制(worker が read cap を bypass できた gap の是正 — source owner
側の shared invariant として修正)。

## Context

- task: `docs/tasks/add-route-logs-cli.md`
- upstream: `docs/plans/implement-pretooluse-route-resolver-and-delegated-workflow.md` Phase D3
- test-spec: `docs/test-specs/route-logs.md`

## Implementation

### Changes

- `log_store.rs`: `reader(&self, max_read_bytes)` が cap を保持する
  `RouteLogReader` を返す。`ArtifactReader::read` は
  `validate_read_range`(offset/max_bytes ≤ JS safe・max_bytes>0、
  InvalidInput 固定 message)→ cap 超過で `PolicyViolation`
  "artifact read limit exceeded"(RunArtifactStore と同一 semantics、
  silent clamp 無し)→ `read_bounded`。range 検証は `read_bounded`
  先頭にも配置し `read_log` 直経路も同一規約を強制(単一 helper
  共有)。runId 検証は path 構築前のまま。
- `run.rs`: `RouteExecution` に `max_read_bytes` field(7 引数
  constructor)→ `compose_reader` が `store.reader(cap)`。execute
  は projected policy の `workflow.artifacts.max_read_bytes` を渡す。
- `args.rs`: `RouteAction::Logs { id, offset, max_bytes, cwd }`。
  `--offset`/`--max-bytes` を STRING_OPTIONS + Values +
  `parse_js_safe_u64`(ASCII digits のみ・JS safe 上限、
  negative/plus/float/scientific/empty/overflow は reject、
  offset default 0、max-bytes present なら >0)に。route arm は
  resolve(2 positional)/logs(3 positional)を shape 分岐し
  per-arm allowlist を維持。route-only field guard に
  offset/max_bytes を追加し非 route command で reject。
- `route_cmd.rs`: `execute_resolve` 抽出 + `Logs` →
  `route::logs::execute` dispatch(comment は移動のみ)。
- `logs.rs`: `resolve_workspace_layout` → `load_policy`
  (builtin/user/project + invocation None、supply::load は
  使わない — malformed routes.yaml の下でも旧 run の log が
  読めること)→ cap 解決 → `store.reader(cap).read`(
  `{artifact:id,runId:id,producer:{exec,command}}` ref)→
  `{data,offset,nextOffset,eof}` utility envelope。成功 read 毎に
  `record_route_refetch`(empty EOF 含む、失敗 read は記録しない)。

### Decisions During Implementation

- `read_log()` 直接経路にも同一 range 検証を置いた(central
  helper — lead 指針 "or central helper called from both" の
  実装形)。`read_bounded` 内 runId → range → open の順。
- `route requires resolve` message を `route requires resolve |
  logs <id>` に更新 — 既存 test の message pin を追随させた。
- D1 integration の `maxBytes: 1048576` は 65536 cap 超過の
  contract 誤りだったため 32768 へ修正(2 箇所、policy 側を
  緩めない)。

## Related Experiments

### Adopted

### Rejected

## Validation

- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib route::` → 32/32 pass(`tmp/handoff-d3/lib-route.txt` — `reader_enforces_policy_read_cap` で cap 内 paging・cap 超過・0/JS-safe 超過・offset 超過・read_log 共有検証を pin)。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_logs::` → 8/8 pass(`tmp/handoff-d3/route-logs.txt` — paging/eof・invalid id 列・policy cap 既定・cap 超過・numeric flag 列・他 command reject・usage append failure+quiet・malformed routes・byte semantics)。
- `--test integration route_run::` → 14/14 pass(`tmp/handoff-d3/route-run.txt` — reader API 変更の回帰確認)。
- `--test integration args::` → 43/43 pass(`tmp/handoff-d3/args.txt`)。
- `--test integration route::` → 5/5 pass(`tmp/handoff-d3/route-mod.txt`)。
- `mise exec -- cargo fmt --all -- --check` → clean(`tmp/handoff-d3/fmt.txt`)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` → warning 0(`tmp/handoff-d3/clippy.txt`)。
- impl-doc audit: `audit_impl_record.js --dir docs/impl/ir` → No findings。

## Risks

- `route logs` の help/dispatch 上の surface は D4 で surface 化
  (lib.rs/HELP 変更は未実施)。

## Follow-ups

- D4: `route run --payload` CLI arm dispatch + help。
- E: worker observable read evidence。
