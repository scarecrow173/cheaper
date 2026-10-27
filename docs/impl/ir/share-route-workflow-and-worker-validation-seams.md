---
id: "IMPL-034Na1mwvjOE0dsqq8Am1F"
type: "impl"
status: "completed"
title: "Share route workflow and worker validation seams"
created: "2026-10-27"
updated: "2026-10-27"
owners: []
relations:
  source: []
  changes:
    added: []
    modified:
      - { type: file, path: crates/cheaper-cli/src/embedded.rs }
      - { type: file, path: crates/cheaper-cli/src/route/routes.rs }
      - { type: file, path: crates/cheaper-cli/src/route/supply.rs }
      - { type: file, path: crates/cheaper-cli/src/route_cmd.rs }
      - { type: file, path: crates/cheaper-cli/src/workflow_context.rs }
      - { type: file, path: crates/cheaper-cli/src/workflow_run.rs }
      - { type: file, path: crates/cheaper-core/src/worker/resolve.rs }
      - { type: file, path: crates/cheaper-core/src/worker/schema/mod.rs }
      - { type: file, path: crates/cheaper-core/src/worker/validate.rs }
      - { type: file, path: crates/cheaper-workflow/src/compiler.rs }
      - { type: file, path: crates/cheaper-workflow/src/lib.rs }
      - { type: file, path: crates/cheaper-workflow/src/loader.rs }
      - { type: file, path: crates/cheaper-workflow/tests/common/mod.rs }
      - { type: file, path: crates/cheaper-workflow/tests/compile_contracts/interface.rs }
      - { type: file, path: crates/cheaper-workflow/tests/filesystem_contracts/loader.rs }
      - { type: file, path: crates/cheaper-workflow/tests/process.rs }
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/share-route-workflow-and-worker-validation-seams.md"
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

# Share route workflow and worker validation seams

## Summary

route declaration の独立 layer 評価・merge provenance、workflow document 入力 validation、worker/schema の source-aware resolver seam を実装した。`route check` placeholder は report 本体を返さないが、`check_sources` から builtin/user/project layer の独立評価まで到達する。既存実行経路は filesystem default を維持し、embedded builtin resource は materialization せず同一 validation pipeline で扱う。

## Context

TASK-034N7DaqWj9vCOHvtN2joH。承認済み plan の step 4・step 6、DESIGN-034N3kihuoqVnngaNEt7q2、および `route-check-layer-and-policy-diagnostics` / `route-check-validation-readiness` test spec に従う。

## Implementation

### Changes

- `route/routes.rs` に `RouteLayerStatus`、`RouteLayerEvaluation`、`RouteMergeStatus`、`RouteMergeRow`、`RouteMergeOutcome` を追加し、`evaluate_filesystem_layer`・`evaluate_routes_text`・`merge_route_layers` を切り出した。absent `routes.yaml` は正常な空 layer、read/parse/schema failure はその layer のみ `Failed` とし、valid sibling row は merge に残る。
- merge provenance は `Active`・`Suppressed`・`Shadowed`・`Invalid`・`Indeterminate` を返す。user-over-builtin replacement は builtin row を `Shadowed`、project collision は project row を `Suppressed`、same-layer duplicate は常に `Invalid` row とする。upstream sibling failure で shadow 判定不能な row は `Indeterminate`、既知 collision は `Suppressed` を維持する。
- `RouteTrust` は宣言 key ではなく layer provenance から導く。builtin/user は trusted、project は untrusted。invalid row は `name`・`command`・`layer`・`invalidReason` を持つ shallow row で、merge status から `Invalid` を得る。
- `cheaper-workflow/src/loader.rs` の `load_workflow_file` から JSON parse・compiler 作成・definition validation を `load_workflow_document` へ分離した。file loading と document input が同一 compiler/validation 経路を共有する。
- `WorkflowContext::load_workflow_document` を追加し、compiler setup・worker index・tool catalog・resource inference を file loader と同じ条件で document 入力へ適用する。`validate_routed_convention` は diagnostics reuse のため `pub(crate)` にした。
- `WorkflowCompilerPorts.workers` を `Arc<dyn WorkerCatalog>` に変更し、compiler が document ごとに解決した worker index を共有所有するようにした。`WorkflowContext.workers` の `OnceLock` は runtime `worker_index()` 用の最初の index だけを保持し、2 件目以降の document validation が前の index を使い回さない。
- `cheaper-core` に `SchemaContentSource`/`FilesystemContentSource`/`load_schema_with_source`、`validate_worker_definition_with_source`、`WorkerContentSource`/`WorkerSourceCandidate`/`resolve_worker_with_sources` を追加した。既存 `load_schema`・`validate_worker_definition`・`resolve_worker` は filesystem default wrapper として維持する。
- `EmbeddedResources` に `SchemaContentSource`・`WorkerContentSource` を実装した。worker path と schema path は `/builtin/...` logical path で `searched`/`source_path`/error path を表示し、catalog lookup では `\\`→`/` normalize、`/builtin/` strip、relative key 解決を行う。
- `route/supply.rs` の `RouteLayerSource` に `evaluate_routes`・`workflow_input`・`resolve_worker`・`resolve_worker_index` を追加した。filesystem workflow は raw read せず `RouteWorkflowInput::File { source, file_root, path }` を返し、canonical `load_workflow_file` の symlink/denylist/regular-file guard へ接続できる。embedded は `Document`、missing target は `Missing` を返す。
- `route_cmd.rs` の placeholder `Check` arm は `check_sources` に加えて `evaluate_route_layers()` まで実行し、その後に sanitized `INTERNAL_ERROR` を返す。report 本体の実装は後続 task。
- focused tests で shadowed/suppressed/invalid/indeterminate provenance、embedded route text の同一 schema/merge 規約、document-input workflow validation、2 件目 document の独立 worker index、embedded workflow/worker/schema validation、project→user→builtin worker precedence、missing embedded worker の `WORKER_NOT_FOUND`、filesystem workflow input の guarded boundary/logical source を確認した。

### Decisions During Implementation

- runtime `routes::load` は従来どおり最初の layer failure で fail-loud に終了する。diagnostics のために `evaluate_*` と `merge_route_layers` を分けたが、route schema・duplicate quarantine・trust rule・merge precedence の authoritative implementation は共有した。
- `RouteLayerEvaluation.source` と `RouteDocument.source` は report へそのまま出せる `<layer>:<relative>` logical path とし、host filesystem path は保持しない。`RouteWorkflowInput::File.file_root` は loader 接続用の内部値で、serialized output には出さない。
- worker file body と `schema: {file: ...}` validity は policy 非依存の source-aware validation として扱う。filesystem workflow の policy-guarded read は `load_workflow_file`/`load_workflow_from` 側の責務に残し、`workflow_input` は guarded boundary に必要な `file_root + path` だけを返す。
- `WorkerCatalog` port は `Arc` 所有にした。`OnceLock` 内の index を借用して compiler を作る設計だと、同一 `WorkflowContext` で複数 document を検証したとき 2 件目以降が最初の index を参照するため、document ごとの index を compiler へ直接渡せる形にした。
- `resolve_worker_with_sources` は candidate 順を resolver へ注入する設計にし、project→user→builtin precedence と `searched`/origin/error shape を既存 resolver と同一経路で保つ。
- missing worker は compile port error に変換せず、`ResolveWorkerError` 側で `WorkerNotFound` を保持する。`resolve_worker_index` は runtime catalog と同じく missing worker を skip し、direct diagnostics path が `WorkerNotFound` を観測できる。
- report 本体未実装のため、後続 task の entry point に使う型・method には targeted `#[allow(dead_code)]` を付けた。public API や既存 caller の挙動は変えていない。

## Related Experiments

### Adopted

<!-- Record adopted experiment logs and why they were adopted. -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `mise exec -- cargo fmt --all` — pass。
- `mise exec -- cargo check --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — pass。
- `mise exec -- cargo clippy --workspace --all-targets --all-features -- -D warnings` — pass。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::routes` — 21 tests pass。merge provenance、same-layer duplicate `Invalid`、sibling failure の `Indeterminate`、known collision の `Suppressed`、embedded logical route source を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::supply` — 6 tests pass。production embedded source、override filesystem builtin、filesystem workflow input の guarded file boundary/logical source、shared route/workflow/worker source、worker precedence、missing embedded worker diagnostics を確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam workflow_context::` — 2 tests pass。document input で embedded routed workflow と embedded worker の同一 validation pipeline、および同一 context の 2 件目 document が独立 worker index で検証されることを確認。
- `mise exec -- cargo test -p cheaper-core worker::` — 46 tests pass。既存 filesystem default worker/schema resolver の回帰を確認。task の feature 指定付き `cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam worker::` は package 側に該当 feature が無く起動できないため、同等の package-local worker test を実行した。
- `mise exec -- cargo test --workspace --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test filesystem_contracts loader::` — 8 tests pass。file input と document input が同一 compiler/definition-validation path を通ること、および既存 guarded I/O boundary の回帰を確認。
- `mise exec -- cargo test -p cheaper-workflow --tests` — 130 tests pass。`WorkflowCompilerPorts.workers` の `Arc<dyn WorkerCatalog>` 化による compile/interface/filesystem/engine/process test への回帰がないことを確認。
- `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — 1 test pass。`route check` placeholder が `INTERNAL_ERROR` / exit 70 に到達し、`cheaper-builtin-*` temp dir・user config state・workspace `.cheaper` を作らないことを確認。
- `git diff --check` — pass。

## Risks

- `route check` は report orchestration が未実装のため、valid invocation でも layer evaluation 後に sanitized `INTERNAL_ERROR` / exit 70 を返す。
- merge provenance・worker/schema source seam は後続 report task が consumer のため、現時点では一部型・method が `#[allow(dead_code)]` を伴う staged seam である。
- filesystem workflow は `RouteWorkflowInput::File` を canonical `load_workflow_file`/`load_workflow_from` へ接続する前提であり、report task は `file_root` を serialized output へ出さず、policy 不在・失敗時の validation 分岐を設計どおりに扱う必要がある。
- `WorkflowContext::worker_index()` は runtime 実行用に最初の load の index を返す。route check の複数 document validation は `LoadedWorkflow.compiler` 側の per-document index を使い、`worker_index()` を document ごとの診断根拠には使わない。

## Follow-ups

- sample matching と renderer capability seam task で `match.command` の policy 非依存評価、embedded route 向け `target_exists` predicate、renderer capability 判定を接続する。
- report task で `RouteMergeRow`、layer statuses、policy/workflow/worker diagnostics を stable report model へ射影し、exit class と deterministic JSON/text output を実装する。
