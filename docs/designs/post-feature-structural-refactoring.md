---
id: "DESIGN-034EDkfp76WJ5Sn8aKMnUv"
type: "design"
status: "approved"
title: "Post-feature structural refactoring"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/post-feature-structural-refactoring-scope.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../discovery/post-feature-structural-refactoring-inventory.md"
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/post-feature-structural-refactoring-scope.md"
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Post-feature structural refactoring

## Context

`cheaper` Rust workspace（8 crate・約 95.5k 行）に対する behavior-preserving
な構造整理。spec（SPEC-034EDjYmUWLJ96Zqy7XOEw）で Phase 0/1 の 35 候補が
確定済みで、ADR（ADR-034EDk9uMH1AuZQn3BMpe4）で「crate トポロジ不変 +
crate 内 module 再編」の方針が確定済み。本 design は各候補の「どこを
どう配置し直すか」の配置図。

依存 DAG（不変）:

```text
cheaper-cli ──▶ cheaper-core
            ──▶ cheaper-tools ──▶ cheaper-core
            ──▶ cheaper-runtime ──▶ cheaper-core/tools/rig-adapter
            ──▶ cheaper-rig-adapter ──▶ cheaper-core/tools
            ──▶ cheaper-provider ──▶ cheaper-core/runtime/rig-adapter
            ──▶ cheaper-workflow ──▶ cheaper-core/tools
            ──▶ cheaper-observability（依存なし leaf）
```

## Scope

### In Scope

Phase 0（hygiene）と Phase 1（structural）の全 35 候補（spec の In Scope）。

### Out of Scope

crate トポロジ変更・Phase 2 候補・観測可能挙動の変更（spec の Out of Scope）。

## Components and Boundaries

### cheaper-cli

- `compose.rs`（新規）: composition root を分離。`create_runtime`・
  devin plugin 登録・session factory 配線・`resolve_named_worker`・
  workspace preamble（5 箇所の重複を helper 化）を保持。run.rs は
  command 本体のみにし、models.rs/workflow_run.rs/auth.rs/usage_report.rs
  は `crate::compose` から参照する。
- `errors.rs`（新規または cli.rs 内集約）: `invalid_request`・
  `--timeout` error helper・`BoxError` 定義を 1 箇所へ（`count.rs:24` の
  重複を解消）。workflow_run の timeout details 欠落を run.rs と同じ形に
  揃える（contract drift の是正的例外 — spec Requirements 参照）。
- `signal.rs`（新規）: `wait_os_signal`・`SignalRelay` を run/workflow で
  共有（compose.rs からは参照のみで所有はこちら）。auth の thread 型
  signal は別機構のまま（結合しない）。
- dev-seam plumbing の `fake_stream.rs` 境界への集約。
- `tests/common/mod.rs`（新規）: VecSink/PipeStdin/ambient_env/Fixture を
  集約。10 テストファイルの重複を解消。

### cheaper-core

- helper 集約: `NAME_PATTERN`/`name_re`（resolve/validate 重複）、
  `schema_error_entries`（3 箇所 verbatim 重複）、absent-file 判定、
  `MAX_SAFE_INTEGER`（named 定数 5 箇所 — policy/{process,schema,
  workflow}.rs・tools/ls.rs — と catalog.rs・engine.rs の inline
  literal を js.rs 正源から導出）。
- `worker/schema.rs` → `worker/schema/{mod,strict}.rs`（AJV strict 再実装
  ~500 行を strict.rs へ機械的移動。公開 API 不変）。
- `read_policy_file`/`read_models_config` 共通骨格抽出（policy/model の
  60 行構造重複。error reason 文字列の parity を維持）。

### cheaper-runtime

- `catalog/mod.rs` → `catalog/{types,compose,store}.rs`。型・merge helper
  9 本・compose・store を分離し re-export で後方互換を保持。
- `run/mod.rs` → `run/{mod,session,boundary}.rs` + `RunCtx::run` を private
  stage method に分割（stage 順序・分岐は不変）。
- `src/auth/oauth/openai_codex.rs` の in-file test ~850 行を crate 内
  sibling module file へ分離（`openai_codex/` dir 化して `tests.rs` に
  置く案）。private fn/const に依存するため `tests/` 外部 target への
  移動は行わず、可視性・公開 API は不変（github_copilot との「テスト
  分離済み」対称化は配置レベルで実現する）。
- `ProviderEntry` の refresh state を sub-struct へ + `compose_provider` の
  auth 層抽出。atomic write helper 共通化（store.rs vs persisted.rs の
  windows 差分は option 化）。
- `model_from_json`/`extension_models` の共通 ctor 化（コメント drift の
  機械的修正含む — 挙動の実装意図確認は Deferred）。
- `dev::{fixture,fake_provider}` alias 解消: workspace 内 caller
  （cli run.rs/fake_stream.rs・dev-seam 系 tests）を `dev::` path へ統一し、
  `lib.rs` の compat re-export を除去。`dev-seam` gate 下のみの item なので
  consumer 側の公開 API は不変。

### cheaper-tools

- characterization test 先行追加（access observe/operation・process
  lifecycle・grep stop reason・lease 親子 counter を pin）。
- `process/mod.rs` → `process/{prepare,spawn,monitor,cleanup}.rs`。
- `grep.rs` orchestration 分割（execute_grep から scan loop/footer 分離。
  `GrepEvalWorker` seam は維持）。
- `read.rs` 3 mode 分離（artifact read/publication/workspace read_bounded）。
- `src/js16.rs` shim 削除・`src/bin/glob_probe.rs` 削除 + differential
  fixture の frozen corpus 明確化。
- 不変: `glob/mod.rs`・`line_framer.rs`・scheduler/resources/operation の
  3 者分離・deferred callback・lock 順序。

### cheaper-workflow

- `engine.rs`: `Pump` 再構築 ×3 + `stop!` macro → `struct RunState` 化。
  children/remaining/status/outer_error/timers を所有し、pump/admit/
  execute/on_admission/on_executed/stop を method として実装。
  event-loop 意味論・deterministic topological order・admission/deadline/
  abort/result shape は不変。
- 二重 validate/compile 解消: worker job で schema が二重 compile されて
  いる経路を `Arc<CompiledSchema>` 共有・`compile_validated` variant で
  解消。`JobValidation.input` 自体は `JobKind::Tool` の
  `validate_job_input`（compiler.rs）で使用中のため**保持**する。
  tool job の input validation 意味論は不変。

### cheaper-provider / rig-adapter / observability

- provider: `CliError→CheaperError` を `From`/helper へ集約（plugin.rs・
  models.rs の重複）。
- adapter: error.rs の外部未使用 pub fn を `pub(crate)` 化、RetryPolicy
  export の要否確認。`tests/adapter.rs` が使用する `classify`/
  `ErrorClass`/`abort_reason_to_cheaper` は公開維持（対象は残りの
  `*_to_cheaper`/`classify_*`/`RetryPolicy` を実測して確定）。
- observability: 変更なし。

### 横断

- `workspace.dependencies` へ `uuid`/`base64`/`tempfile` 集約。`uuid` は
  workflow `=1.27.0` exact pin と provider `1` の差異があり、厳格側を
  保持するか差異を明文化するかを実装時に決定する。
- `.gitignore` Node boilerplate 整理 + `benchmarks/results/` 保持方針明文化。

## Data and Control Flow

CLI → compose（runtime 構築）→ core resolve → tools scope/scheduler →
rig-adapter session → provider → model。本 design はこのフローを変えず、
各 crate 内の配置と重複知識の一権威化のみを行う。詳細は調査レポート
`docs/research/structural-refactoring-inventory/refactoring-plan.html` の
§2 sequence diagram・§5 責務配置の変化を参照。

## Non-Functional Constraints

- 観測可能挙動の byte parity（envelope・error code/message・sort 順・
  abort 意味論・usage.jsonl・worker schema・CLI 引数 surface）。
  例外は workflow_run の timeout details 修正のみ（spec Requirements）。
- `mise run check` 全 pass。
- differential fixture・`cheaper-cli/tests/envelope.rs`・regression
  55 case が regression 防壁（「envelope parity test」はこの envelope.rs
  統合テストを指す — cheaper-workflow 配下には同名の専用 harness は
  存在しない）。

## Risks and Trade-offs

- tools の構造変更は表現変換点が多く drift リスクがある →
  characterization test を先行追加して pin してから触る。
- workflow engine の RunState 化は scheduling/serialization に近い →
  `cheaper-cli/tests/envelope.rs` と regression case で防衛し、stage 順序・
  分岐を変えない。
- `read_policy_file`/`read_models_config` 共通化は error reason 文字列の
  parity が contract → 文字列差分が出ないことを差分テストで確認。

## Deferred Design Concerns

- Phase 2 候補 14 件 + workflow `ToolCatalog`/`ToolMetadata` 統一/改名
  （`pub trait` で外部 implementor あり・behavior-preserving を超える設計
  判断）+ JS 互換層 compat 名前空間集約（discovery hypothesis、実施判断
  自体が未決定）— 別 spec/ADR で個別判断。
- `extension_models` の実装意図・`usage_report` の canonicalize 省略の
  drift 2 件 — 意図確認メモとして記録（timeout details 欠落のみ Phase 0
  で修正、コメント drift の機械的修正は Phase 0 scope）。

## Verification Notes

- [ ] `mise run check` 全 pass
- [ ] differential fixture（tools）・`cheaper-cli/tests/envelope.rs` が緑
- [ ] characterization test が tools 構造変更に先行
- [ ] 各 crate の巨大ファイルが責務単位に分割されている
