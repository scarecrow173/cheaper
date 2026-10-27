---
id: "IMPL-034N8MVeECVyQHIBorOfVt"
type: "impl"
status: "completed"
title: "Add embedded route check source supply"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added:
      - { type: file, path: crates/cheaper-cli/tests/integration/route_check.rs }
    modified:
      - { type: file, path: crates/cheaper-cli/src/embedded.rs }
      - { type: file, path: crates/cheaper-cli/src/route/supply.rs }
      - { type: file, path: crates/cheaper-cli/src/route_cmd.rs }
      - { type: file, path: crates/cheaper-cli/tests/integration/main.rs }
      - { type: file, path: crates/cheaper-core/src/policy/load.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/add-embedded-route-check-source-supply.md"
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

# Add embedded route check source supply

## Summary

`route check` の read-only 診断境界に必要な embedded builtin resource accessor、layer source model、check 専用 supply builder、policy document 入力 API を実装した。production builtin は temp materialization を経ずに `/builtin/` logical catalog から読め、既存実行経路 `RouteSupply::load` は変更していない。

## Context

TASK-034N7D9MYpblZB6oi3FOuw。承認済み plan の step 3・step 5、ADR の read-only embedded resource boundary、DESIGN-034N3kihuoqVnngaNEt7q2、および `route-check-layer-and-policy-diagnostics` test spec に従う。

## Implementation

### Changes

- `embedded.rs` に `EmbeddedResources` と logical key normalizer を追加した。`/builtin/` root と plain relative key を受理し、`\\`→`/`・`.`/empty segment を正規化し、親参照・host path・root-like key・drive letter・NUL・`:` を拒否する。catalog は `OnceLock<BTreeMap>` で遅延構築し、`read`/`contains`/`read_text` は byte/text を返して filesystem path や temp dir を露出しない。
- `route/supply.rs` に `RouteLayerSource::{Filesystem, Embedded}`、`RouteCheckSources`、check 専用 `check_sources` builder を追加した。`CliHooks.builtin_dir == None` の production builtin は `Embedded`、override 指定時は `Filesystem`、user/project は常に `Filesystem` とする。`worker_base_dir` は resolver の `workers/<name>.md` join 規約を維持する layer base dir。
- `route_cmd.rs` の placeholder `Check` arm で `check_sources` を通し、後続 report task の接続前でも production path が embedded catalog を使い temp cache を作らないことを確認できるようにした。
- `policy/load.rs` に `PolicyLayerStatus`、`PolicyLayerStatuses`、`PolicyDocumentLayer`、`LoadPolicyDocuments`、`PolicyDocumentLoad`、`PolicyDocumentLoadError`、`load_policy_documents` を追加した。file API と同じ builtin required、user `Partial`+`Replace`、invocation grants-only guard、project `ProjectPartial`+`RestrictConcurrency`、typed default resolution、final `Complete`+`normalize_policy` pipeline を共有する。`layers` は各 supplied document を layer 固有 schema mode で独立検証して `absent|loaded|failed` を決め、layer failure と post-merge `Merged` pseudo-layer failure を区別できる。
- `crates/cheaper-cli/tests/integration/route_check.rs` を追加し、`builtin_dir: None`・clean temp・ambient logging 下で `route check` が placeholder `INTERNAL_ERROR` / exit 70 に到達すること、および `cheaper-builtin-*` temp dir・user config state・workspace `.cheaper` を作らないことを fresh process で確認した。

### Decisions During Implementation

- `EmbeddedResources` は catalog key を host filesystem path ではなく `/builtin/` logical relative key として扱う。存在しない key は read error ではなく `None` とし、後続 diagnostics が layer absent/malformed を制御できるようにした。
- `check_sources` は config dir 解決だけ既存 `resolve_config_dirs` を再利用し、`builtin_dir()` は呼ばない。production builtin のみ embedded catalog を渡し、dev/test override は既存 filesystem resource 形を維持した。
- `load_policy_documents` は parse 済み `Map<String, Value>` を受け取る。YAML read/parse は後続 route loading/report task が source-aware に扱うため、この slice では policy merge/validation の責務だけを共有した。
- `PolicyLayerStatuses` は merge 到達順ではなく、各 supplied document の standalone layer validation を独立に評価する。これにより earlier layer failure の後ろにある malformed sibling を `Loaded` と誤報しない。merge 後の failure は `Merged` pseudo-layer として扱う。
- `PolicyDocumentLoadError` は `CheaperError` を `Box` 保持して `Result` の Err variant を小さくした。inner `CheaperError` の code/details は失わない。
- `check_sources` の `resolve_config_dirs` 呼出しでは `ConfigDirs.builtin_dir` を後続へ束縛しない。`/builtin/` sentinel が host filesystem path として誤用される経路を source model の外へ出さない。
- test-only counter は `materialize_builtin_dir()` ではなく `builtin_dir()` 呼出しを数える。process cache 済みでも check 経路が materializing API を呼んだことを検出できる。
- report 本体が未実装のため、`RouteLayerSource` 等の後続 seam には targeted `#[allow(dead_code)]` を付けた。既存実行経路や public API の形は変えていない。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo fmt --all` — pass。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` — pass。
- `mise exec -- cargo test -p cheaper-core policy::load::tests:: --lib` — 22 tests pass。document API の absent/loaded/failed、builtin required、layer schema failure、earlier-failure 後の sibling validity、post-merge `Merged` failure attribution を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam embedded:: --lib` — 6 tests pass。`routes.yaml`、`policy.yaml`、`workflows/**`、`workers/**`、`workers/schemas/**` の read/contains、logical key normalization、invalid root/escape rejection、no `builtin_dir()` call を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::supply:: --lib` — 2 tests pass。production `Embedded` builtin、`builtin_dir` override `Filesystem`、user/project filesystem source、worker base-dir 規約を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 1 test pass。fresh process・clean temp・ambient `CHEAPER_LOG=debug` 下で placeholder `INTERNAL_ERROR` / exit 70 に到達し、`cheaper-builtin-*` temp dir・user config state・workspace `.cheaper` が生成されないことを確認。
- `git diff --check` — pass。

## Risks

- report orchestration は後続 task のため、valid `route check` は依然 sanitized `INTERNAL_ERROR` / exit 70 placeholder を返す。
- `RouteLayerSource` はまだ route loading/workflow/worker validation/report から消費されていない。後続 task が `layers()` と embedded/file source を接続する。
- `PolicyDocumentLoadError` は layer status と failure attribution を保持するが、final report JSON/text への射影は後続 task。

## Follow-ups

- shared route source-aware loading と user-over-builtin shadow provenance を後続 task で実装する。
- workflow/worker validation、sample matching、renderer capability、route-check report output を残 task で接続する。
