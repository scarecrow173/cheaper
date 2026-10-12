---
id: "SPEC-034EDjYmUWLJ96Zqy7XOEw"
type: "spec"
status: "approved"
title: "Post-feature structural refactoring scope"
created: "2026-10-12"
updated: "2026-10-12"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/post-feature-structural-refactoring-inventory.md"
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
    - "../adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md"
    - "../designs/post-feature-structural-refactoring.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/post-feature-structural-refactoring-inventory.md"
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/post-feature-structural-refactoring-parity.md"
  references: []
  defers: []
  deferred-by: []
---

# Post-feature structural refactoring scope

## What Should Be Built

並列調査（DISC-034EDjXUQnxTiHz4batpQu）で確定した構造的負債を、
behavior-preserving で整理する。対象は調査レポートの Phase 0（hygiene・
18 件）と Phase 1（structural・17 件）の計 35 候補。

## Why Now

基本機能が揃った v1.0.0 の節目であり、crate 内の巨大ファイル・移植由来の
小規模重複・責務の同居が後続の変更コストを上げ始めている。crate トポロジは
健全（一方向 DAG・循環なし）と確認済みのため、crate 内再編 + 共有 helper
集約に限定して安全に進められる。

## Users and Value

- 本リポジトリを保守する開発者・agent — 巨大ファイルと重複知識の削減による
  変更局所化と review 容易性。
- consumer には観測可能な差分がない（envelope/error/exit code 不変）。

## Scope

### In Scope

Phase 0 — hygiene（小重複・dead 資産の解消）:

- cli: `--timeout` error helper 共通化（workflow_run の details 欠落を解消）、
  `invalid_request` 共通 helper 化、`wait_os_signal`/SignalRelay 集約、
  `Runtime::create`+devin plugin 登録の単一入口化、tests `common/mod.rs`
  集約（VecSink/PipeStdin/ambient_env/Fixture）、`count.rs` BoxError 一本化
- core: `NAME_PATTERN`/`name_re` 統合、`schema_error_entries` 共用 helper、
  absent-file 判定 helper、`MAX_SAFE_INTEGER` 正源化（js.rs。named 定数は
  現状 5 箇所 — policy/{process,schema,workflow}.rs・tools/ls.rs —
  catalog.rs・engine.rs の inline literal も含めて扱う）
- runtime: `model_from_json` vs `extension_models` の共通 ctor 化 +
  機械的なコメント drift 修正（実装意図の確認は Deferred を参照）、
  `dev::{fixture,fake_provider}` 互換 alias の canonical path 統一
  （workspace 内 caller を `dev::` path へ統一し lib.rs の alias を除去。
  alias は `dev-seam` feature gate 付きで consumer から参照不能のため
  外部公開 API の破壊には当たらない）
- provider: `CliError→CheaperError` を `From`/`helper` へ集約
- adapter: error.rs の外部未使用 pub fn を `pub(crate)` 化・RetryPolicy export
  確認（`tests/adapter.rs` が使用する `classify`/`ErrorClass`/
  `abort_reason_to_cheaper` は公開維持。対象は
  `classify_prompt`/`classify_report`/`provider_error_to_cheaper`/
  `report_to_cheaper`/`prompt_error_to_cheaper`/`streaming_error_to_cheaper`/
  `RetryPolicy` の実測で確定）
- tools: `src/js16.rs` shim 削除（caller を core 直接参照へ）、
  `src/bin/glob_probe.rs` 削除 + differential fixture の frozen corpus 明確化
- 横断: `workspace.dependencies` への `uuid`/`base64`/`tempfile` 集約
  （`uuid` は workflow `=1.27.0` exact pin と provider `1` の version
  requirement が異なる — 統一時は厳格側 `=1.27.0` を保持するか、
  意図的な差異として明文化するかを実装時に決定）、
  `.gitignore` Node boilerplate 整理 + `benchmarks/results/` 保持方針明文化

Phase 1 — structural（crate 内再編）:

- tools: characterization test 追加（access observe/operation・process
  lifecycle・grep stop reason・lease 親子 counter を pin）— 先行必須
- cli: composition root を `compose.rs` へ分離（create_runtime/factory/
  resolve_named_worker）、workspace preamble helper 化（5 箇所）、
  dev-seam plumbing の `fake_stream.rs` 境界への集約
- runtime: `catalog/mod.rs` 分割（types/compose/store）、`run/mod.rs` 分割
  （`run/{session,boundary}.rs`）+ `RunCtx::run` の stage method 分割、
  `src/auth/oauth/openai_codex.rs` in-file test を crate 内 sibling module
  file へ分離（private item に依存するため `tests/` 外部 target ではなく
  `src/` 内の別 file — 例: `openai_codex/` dir 化 + `tests.rs`。
  公開 API・可視性の変更は行わない）、
  `ProviderEntry` の refresh state sub-struct 化 + `compose_provider` auth 層
  抽出、atomic write helper 共通化
- tools: `src/tools/process/mod.rs` 分割（prepare/spawn/monitor/cleanup）、
  `src/tools/grep.rs` orchestration 分割（scan loop/footer 分離）、
  `src/tools/read.rs` 3 mode 分離
- workflow: `src/engine.rs` の Pump 再構築廃止 → RunState struct + `stop!`
  fn 化、二重 validate/compile 解消（worker job の冗長な schema compile
  のみ解消 — `JobValidation.input` は `JobKind::Tool` の input validation
  （`compiler.rs` `validate_job_input`）で使用中のため保持。
  `Arc<CompiledSchema>` 共有・`compile_validated` variant で対応）
- core: `src/worker/schema.rs` 分割（AJV strict 再実装 ~500 行を strict.rs へ）、
  `read_policy_file`/`read_models_config`（policy/model）の共通骨格抽出
  （error reason 文字列 parity に注意）

### Out of Scope

- crate トポロジの再編（統合・分割・依存方向の変更）
- Phase 2 候補 14 件（`args.rs` 宣言的 table、boundary/policy adapter
  単一化、`envelope.rs`→`workflow_context` 層違反解消、`run_worker` の
  `!Send` 解消、`Pin<Box<dyn Future>>`→`impl Future`、`ToolError` enum 化、
  process-global cache の instance 化、`serde_yml`→`noyalib`、
  `gen-glob-tables.cjs` Rust port、phase-task コメント rewrite、
  `Vec`→`HashMap` index、`MEASURE_ENV_KEYS` 宣言化、`nodepath.rs` env 参照の
  seam 化、`hook/routing_policy.rs` 分割）
- workflow `ToolCatalog`/`ToolMetadata` の統一または改名 — `ToolCatalog` は
  外部 implementor を持つ `pub trait` であり、統一/改名はいずれも
  behavior-preserving の範囲を超える設計判断のため Phase 2 相当へ defer
- JS 互換層 compat 名前空間集約 — discovery の hypothesis（未確認項目）であり
  実施判断自体を別途行うため、本 scope の対象にも Phase 2 確定候補にもしない
- `glob/mod.rs`・`line_framer.rs` の内部整理（minimatch/UTF-16 parity の正当な
  複雑さ）
- resources.rs の deferred callback 設計・lock 順序（設計不変条件）
- 観測可能挙動の変更（envelope/error code/message・sort 順・abort 意味論・
  usage.jsonl・worker schema・CLI 引数 surface）

## Requirements

- すべての変更は behavior-preserving（公開 API 名・envelope・error contract
  を変えない）。external（workspace 外）参照がある pub 項目の削減は行わない。
- 唯一の例外: `workflow run --timeout` の `details` 欠落修正は観測可能な
  envelope payload の追補だが、調査で確認した contract drift（`run` 側との
  不整合・バグ）の是正として意図的に行う（TSPEC-…-parity でも例外明記）。
- 分割・移動は「呼出し側署名を変えない移動」を基本とする。
- `mise run check` が全 pass すること。
- 変更は task 単位で意味のある境界に分割してコミットする。

## Acceptance Criteria

- [ ] Phase 0 の 18 候補が実施され、該当箇所の重複が解消されている
- [ ] Phase 1 の 17 候補が実施され、crate 内 module 再編が完了している
- [ ] `mise run check` が pass する
- [ ] envelope/error code/exit code の差分がテストで検出されない
- [ ] characterization test が tools の構造変更に先行して追加されている

## Deferred Design Concerns

- Phase 2 候補 14 件 — 設計判断・影響大・現状では過剰なもの。実施時は別途
  spec/ADR を起こす。
- `extension_models` の実装意図（comment が記述する default 埋め挙動が
  TS parity 的に正しいか）の確認 — 本 scope では機械的なコメント drift
  修正のみ行い、挙動自体の意図確認は保留。
- `usage_report` の canonicalize 省略 — 調査で発見した drift だが
  behavior 確認が必要なため意図確認メモとして記録のみ
  （timeout details 欠落は Phase 0 で修正）。

## Sources and Relations

- discovery: `docs/discovery/post-feature-structural-refactoring-inventory.md`
- ADR: `docs/adr/refactor-within-the-existing-crate-topology-via-intra-crate-module-reorganizatio.md`
- design: `docs/designs/post-feature-structural-refactoring.md`
- 調査レポート: `docs/research/structural-refactoring-inventory/refactoring-plan.html`（canonical 保存版）
