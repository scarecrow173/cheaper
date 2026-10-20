---
id: "IMPL-034JhZPcBMjCslZpw77s0m"
type: "impl"
status: "completed"
title: "Add route declaration schema and three-layer loader"
created: "2026-10-21"
updated: "2026-10-21"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/src/route/mod.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-core/src/errors.rs"
      - type: "file"
        path: "crates/cheaper-core/src/yaml.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-route-declaration-loader.md"
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

# Add route declaration schema and three-layer loader

## Summary

`route::routes` を新設し、routes.yaml の schema 検証・
builtin/user/project 3 層 load・`match.command` key 化 merge・
`trusted` shadow・suppressed provenance・同一層 dup の
`invalid` 隔離・`target.path` 層起点 traversal 拒否を実装。

## Context

Task `add-route-declaration-loader`(plan Phase B1、design §
routes.yaml schema と 3 層解決、spec R1-R7)。hook eval・
`route resolve`・`route run` の 3 caller が共有する route 宣言
解決の入口。

## Implementation

### Changes

- `route/mod.rs`・`route/routes.rs`(新規): `Route`・
  `EffectiveRoutes{routes, suppressed, invalid}`・
  `RouteLoadOptions{builtin_dir, user_config_dir, project_dir}`・
  `load()`。dir 解決は caller 責務とし loader は純粋評価。
- schema 検証(fail-loud): `version: 1` const・`routes` array
  必須・route required(`name`/`match`/`target`/`grant`)・
  `match.command` 非空 token 列・`target.kind == "workflow"`・
  全 level の未知 key 拒否・`output.tokenThreshold` number・
  `passthroughEnv` env 名 validation・`trusted` bool。
- merge: user は builtin を無条件 shadow、project は
  `trusted: true` のときのみ下位層 shadow(無い場合は
  `suppressed` 記録)。同一層内 dup は `InvalidRouteGroup` へ
  隔離し resolve 対象から除外。
- `target.path`: 層起点 relative のみ許容
  (builtin→`<builtin_dir>/workflows/`、user→`<userConfigDir>`、
  project→`<project_dir>`)、`Component::ParentDir`/`RootDir`/
  `Prefix`・NUL を拒否。
- `read_routes_file`: `read_yaml_layer` と同一骨格
  (read → lossy utf8 → parse_yaml → object check、absent は
  `Ok(None)`)。error は `InvalidRuntimePolicy` + details
  `{path, reason, errno?}` — load 失敗は caller が
  `invalid-route` viability へ写像する。
- core visibility: `yaml::parse_yaml`・
  `errors::{is_absent_file_error, io_errno_display}` を
  `pub(crate)` → `pub` へ昇格(cli route loader が canonical
  yaml seam・io 規約を共有するため)。

### Decisions During Implementation

- `trusted` field は builtin/user 層でも parse 許容(inert —
  効果を持たないのみで error にしない、design D7 forward
  compatible 方針)。
- 同一層内 dup は file 全体 error ではなく `InvalidRouteGroup`
  bucket 隔離 — malformed file(全部が信頼不能)と dup(衝突
  key のみ信頼不能)で blast radius を分ける。resolve 側が
  `invalid-route` viability を報告できるよう collided command
  + route 名を保持。
- target.path は `Component` 走査で `..`/root/prefix を構造的に
  拒否 — string 正規化ではなく path semantics で判定するため
  `a/../b`・`./x` 等の迂回形も網羅。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli route::routes` — 6 pass(新規:
  `loads_valid_layer_file`・`absent_files_load_as_empty_set`・
  `malformed_and_unknown_fields_are_config_errors`・
  `user_shadows_builtin_and_project_needs_trusted`・
  `same_layer_duplicates_are_quarantined_as_invalid`・
  `target_path_must_be_layer_relative`)。
- `cargo test -p cheaper-cli --lib` — 43 pass。
- `cargo clippy -p cheaper-cli -p cheaper-core --tests` — warning
  なし。

## Risks

- `ErrorCode::InvalidRuntimePolicy` を route file error に流用
  (route 専用 code は未追加) — load Err は全て `invalid-route`
  viability へ写像されるため識別性は損なわれないが、policy
  error との error code 区別は message/details で行う。
- `passthroughEnv` の env 名 validation は
  `[A-Za-z_][A-Za-z0-9_]*` — spec 未規定部分を既存 env 名規約で
  補った。

## Follow-ups

- B2 `add-route-resolver-and-payload` が `EffectiveRoutes` を
  consumer とする resolve/payload 実装。
- D2 `add-builtin-route-resources` が builtin `routes.yaml`・
  `workflows/routed-exec.workflow.json` を配置し embedded
  materialize 経路を接続。
