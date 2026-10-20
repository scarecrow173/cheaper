---
id: "IMPL-034JidY17wnSZzkiMrhixl"
type: "impl"
status: "completed"
title: "Add route log store and RouteLogReader"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/log_store.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-log-store.md"
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

# Add route log store and RouteLogReader

## Summary

`<userConfigDir>/route-logs/` への routed 実行 log 保存を実装。
`<runId>.log.tmp` streaming write → `finish()` rename・unix
`0600`・`<unix-ms>-<pid>-<hex>` runId 採番・retention sweep・
`RouteLogReader`(`ArtifactReader` impl)による runId-addressed
bounded read を提供。

## Context

Task `add-route-log-store`(plan Phase B3、design §route log
store、spec R28-R31)。route 実行 log は secret を含みうるため
workspace 外へ置き、`route logs` CLI・analyze worker の
artifact read が同一 runId 検証・bounded read contract を共有する。

## Implementation

### Changes

- `route/log_store.rs`(新規): `RouteLogStore`(dir 注入で test
  可能な純粋 fs 操作)、`RouteLogWriter`(`Write` impl —
  `create_new` の `.tmp` へ streaming、`finish()` で rename)、
  `RouteLogReader`(`ArtifactReader` impl)。
- `is_valid_run_id` — `^[0-9]+-[0-9]+-[0-9a-f]+$` を hand-rolled
  検証(regex dep を持ち込まない)。`route logs`・
  `RouteLogReader`・invalid runId 拒否が同一関数を共有。
- runId 採番: `fs_util::temp_path` と同型の pid+nanos 規約 +
  同 nanos 衝突用の atomic counter XOR。
- `sweep()`: `*.tmp`(実行中 log)除外・TTL 30 日・200 file・
  256 MiB を古い順削除。best-effort(個別 entry 失敗は skip、
  dir 走査自体の失敗のみ error)。上限は `sweep_with` seam で
  注入可能(test 用)。
- bounded read: `{data, offset, nextOffset, eof}` の
  `ArtifactReadResult` contract。offset > len は
  `InvalidRequest`、EOF と同値は空 result で許可。
- `read_routes_file` と同規約の io error shape(`errno` detail)。

### Decisions During Implementation

- tmp の命名は `<runId>.log.tmp` — `*.tmp` suffix で sweep が
  in-flight file を構造的に除外する(rename 失敗で run を
  殺さない)。crash 残り `.tmp` は sweep が消さない(実行中と
  区別不能なため保持側に倒す)。
- `0600` は `OpenOptionsExt::mode` で create 時に付与 — write
  中の一瞬も secret 内容が他 user に読めない。
- `ArtifactReader` impl は `reference.run_id` を validate し
  `read_log` へ委譲 — CLI の `route logs <id>` と同一検証を通る。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli route::` — 11 pass(新規 5:
  `run_id_format_and_validation`・
  `begin_writes_tmp_then_finish_renames`・
  `read_log_is_bounded_and_offset_aware`・
  `artifact_reader_shares_run_id_validation`・
  `sweep_enforces_ttl_count_size_and_skips_tmp`)。
- `cargo clippy -p cheaper-cli --tests` — warning なし。

## Risks

- crash 残りの `.tmp` file は sweep が消さず蓄積する —
  in-flight との区別が不能なため意図的な残留(容量は bounded
  ではないが通常 run では稀)。

## Follow-ups

- D1 `add-route-run-delegated-exec` が `begin()`/sweep/exit
  mirror の組立点、`route logs` CLI(D4)が `read_log` と
  `is_valid_run_id` を使う。
- D2 routed-exec workflow の analyze worker は
  `ArtifactPorts.reader` へ `store.reader()` を注入する。
