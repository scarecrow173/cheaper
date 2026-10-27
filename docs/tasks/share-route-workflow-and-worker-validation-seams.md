---
id: "TASK-034N7DaqWj9vCOHvtN2joH"
type: "task"
status: "todo"
title: "Share route workflow and worker validation seams"
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
    - "docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md"
    - "docs/tasks/add-embedded-route-check-source-supply.md"
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
    - "docs/test-specs/route-check-layer-and-policy-diagnostics.md"
    - "docs/test-specs/route-check-validation-readiness.md"
  references: []
  defers: []
  deferred-by: []
---

# Share route workflow and worker validation seams

## Work

approved plan の step 4 と step 6 に対応する implementation slice。route layer の read/validate/merge provenance、document 入力の shared workflow validation、worker/schema の source-aware resolver seam を実装する。shared domain predicate・loader は seam 注入のみで再利用し、diagnostics 専用の複製を作らない。

- `route/routes.rs` の `read_routes_file`/`validate_routes_doc` と merge fold を、filesystem `load` と diagnostics の両方が使う shared step へ整理する。merge result は `shadowed`/`suppressed`/`invalid`/`indeterminate` の provenance を返す: 同一 layer duplicate は他 layer の状態に関わらず常に `invalid`（name/command/layer/trust/`invalidReason` のみの shallow row）、effective builtin/user route と同じ `match.command` を持つ project route は `suppressed`、user-over-builtin replacement は `shadowed` とする。`trust` は宣言 key ではなく layer provenance（builtin/user = `trusted`、project = `untrusted`）から導く。diagnostics entry point は builtin→user→project の各 layer を独立評価し、absent `routes.yaml` は正常な空 layer、failed layer は自身の error entry のみを生成して valid sibling row を保持する。merge determinacy は design 規約どおり（user layer failed なら builtin row は `indeterminate`、invalid bucket の route は merge 対象外）に従う。
- `cheaper-workflow/src/loader.rs` と `workflow_context.rs` を file-read wrapper と document 入力の shared validation に分離し、`workflow_run.rs` の `validate_routed_convention` を `pub(crate)` にする。workflow validation は route の宣言 layer から document を取得し、policy-guarded fs read と compiler catalog は policy がある場合にのみ実行する。
- `cheaper-core` の `worker/resolve.rs`・`worker/validate.rs`・`worker/schema/**` に `WorkerContentSource`/`SchemaContentSource` 相当の seam を追加し、`resolve_worker`/`resolve_worker_index`/`WorkerCatalog` が project → user → builtin precedence を同じ source-aware resolver で維持するようにする。`source_path` は display 用の logical path とし、schema base-dir resolution は `\\` → `/` normalize と `/builtin/` prefix strip を経て relative key で catalog lookup する。`WorkerNotFound` は compile port の error ではなく route diagnostic として扱えるようにする。
- worker 定義本体と `schema: {file: ...}` の validity は policy 非依存として評価可能な構造にする。既存 caller は全て filesystem default のまま維持し、`route resolve`/`route run`/hook の挙動を変えない。本 slice は strictly read-only とする。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-route-check-read-only-diagnostics.md`
- `docs/specs/route-check-diagnostics-issue-185.md`
- `docs/designs/route-check-read-only-diagnostics-boundary.md`

## Dependencies

- `docs/tasks/add-route-check-invocation-detection-and-side-effect-gates.md`
- `docs/tasks/add-embedded-route-check-source-supply.md`（`RouteLayerSource`/`EmbeddedResources` 規約を worker/schema seam が共有するため）

## Affected Files

- `crates/cheaper-cli/src/route/routes.rs` — shared per-layer read/validate step、merge provenance、layer 独立評価 entry point
- `crates/cheaper-cli/src/workflow_context.rs` — file-read wrapper と document-based validation の分離
- `crates/cheaper-cli/src/workflow_run.rs` — `validate_routed_convention` の `pub(crate)` 化
- `crates/cheaper-workflow/src/loader.rs` — document 入力 shared validation
- `crates/cheaper-core/src/worker/resolve.rs`, `crates/cheaper-core/src/worker/validate.rs`, `crates/cheaper-core/src/worker/schema/` 配下 — `WorkerContentSource`/`SchemaContentSource` seam

## Done When

- [ ] absent user/project `routes.yaml` が正常な空 layer、malformed layer が per-layer diagnostic、valid sibling row が保持される。
- [ ] merge fold が `shadowed`/`suppressed`/`invalid`/`indeterminate` を design の determinacy 規約どおりに報告し、`invalid` が name/command/layer/trust/`invalidReason` のみの shallow row になる。
- [ ] filesystem `load` と diagnostics が同一の shared step を使い、diagnostics 専用の merge/validate 複製が存在しない。
- [ ] `builtin_dir: None` production path で `WorkerCatalog`/`resolve_worker_index` が embedded builtin worker/schema を同一 validation pipeline で解決し、false `missing-target`/worker/schema failure が無い。
- [ ] `WorkerNotFound` が route diagnostic として扱え、policy 不在でも worker 定義/schema validity が評価できる。
- [ ] 既存 caller が filesystem default を維持し、`route resolve`/`route run`/hook の parity が保たれる。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam route::routes` — merge provenance・layer 独立評価・shallow `invalid` row の unit coverage が pass する。
- [ ] `mise exec -- cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam worker::` — `WorkerContentSource`/`SchemaContentSource` seam と catalog lookup 正規化が pass する。
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration route_check::` — `builtin_dir: None` production-path・absent/malformed layer・workflow/worker validation fixture が design 規約どおりに pass する。
