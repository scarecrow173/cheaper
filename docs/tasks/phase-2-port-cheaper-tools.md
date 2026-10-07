---
id: "TASK-034BgKiZhENYXeNMRvxyFA"
type: "task"
status: "done"
title: "Phase 2: port cheaper-tools"
created: "2026-10-06"
updated: "2026-10-07"
owners: []
relations:
  source: []
  changes:
    added:
      - "scripts/dump-tool-fixtures.mjs"
      - "crates/cheaper-tools/"
    modified:
      - "crates/cheaper-core/src/errors.rs"
      - "crates/cheaper-core/src/token/count.rs"
      - "docs/impl/ir/phase-2-port-cheaper-tools.md"
      - "Cargo.toml"
    deleted: []
    renamed: []
    moved: []
    generated:
      - "crates/cheaper-tools/tests/fixtures/"
  implements:
    - "docs/plans/phase-2-port-cheaper-tools.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by:
    - "docs/impl/ir/phase-2-port-cheaper-tools.md"
  depends-on:
    - "docs/plans/phase-2-port-cheaper-tools.md"
    - "docs/tasks/phase-1-port-cheaper-core.md"
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
    - "docs/test-specs/rust-migration-run-semantics.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 2: port cheaper-tools

## Work

plan `docs/plans/phase-2-port-cheaper-tools.md`(PLAN-034BepYuOf4htZRfdkZTYc)
の実行単位。`packages/tool-runtime`(src ≈4.5k 行)と `runtime-pi/src/tools`
の boundary operations / Pi helper 接着部を、`cheaper-core` のみに依存し
tokio 依存を許可した `crates/cheaper-tools` crate へ移植する。canonical
tool 8 種(read/write/edit/ls/grep/find/count_tokens/process)・tool
scheduler・budget/accounting・mutation journal・artifact port を TS 版と
意味論同一で成立させる。

plan の Task Breakdown T2-1〜T2-8 が本 task の作業単位:

- T2-1: crate skeleton + 基盤小物(`limits.rs` 全定数 verbatim port・
  `line_framer.rs` の `split("\n")` parity・`walker.rs` の deny 定数/sniff/
  walk・`artifact.rs` の reader/publisher trait)
- T2-2: scheduling core(`execution_metrics.rs`・`operation.rs` 六状態機械・
  `resources.rs` FIFO 単一仲裁・`scheduler.rs` global/exclusive/pool/
  readiness/queued cancel)
- T2-3: `access.rs`・`accounting.rs`(計数 4 分類: 実行到達=toolCalls・
  boundary/limit 拒否=rejectedToolCalls)・`journal.rs`(MutationJournal、
  diff は `similar` 試行→不一致なら jsdiff 直接 port)
- T2-4: `definition.rs`/`catalog.rs`(8 tool schema JSON・resources 宣言・
  `validate_tool_input`)+ fs tools(`boundary.rs`・`read.rs`・`ls.rs`・
  `write.rs`・`edit.rs`・`find.rs`・`count_tokens.rs`。Pi helper 由来の
  description 文言・出力 format・`prepareEditArguments` を verbatim parity)
- T2-5: `tools/grep_eval.rs`(eval subprocess protocol + supervisor +
  `helper_main`、`regress` crate)→ `tools/grep.rs`(streaming batch・budget・
  deadline・`B >= 3*L` invariant・有界 footer)
- T2-6: `tools/process_executable.rs`(`node` は PATH 解決せず明示 error)
  → `process_tree.rs`(POSIX/Windows 両分岐・PID reuse 照合)→
  `process.rs`(固定ルール完全 port)
- T2-7: `registry.rs`(`create_tool_runtime`・execute pipeline)・
  `invocation.rs`・`lib.rs` 公開面確定
- T2-8: `scripts/dump-tool-fixtures.mjs` + `tests/fixtures/` +
  `tests/differential.rs` の差分基盤構築、Windows 実機検証、差分解消

`AbortSignal`/`AsyncLocalStorage` 相当は CancellationToken +
`OnceCell<reason>` / `tokio::task_local!` で再現する(plan File Map の
設計決定)。Rig trait 接着・artifact store 実体・CLI dispatch は後続 phase
の責務であり本 task の範囲外。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-2-port-cheaper-tools.md`(本 task が implement する
  plan。File Map・Implementation Sequence・Verification の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan・共通ルール)
- `docs/plans/phase-1-port-cheaper-core.md`(前提: core 型/API 供給元)
- `docs/designs/rust-rig-runtime-migration-architecture.md`(crate 表・
  tool 移植の固定ルール・計数写像)
- `docs/designs/canonical-tool-runtime.md`・
  `docs/designs/bounded-tool-scheduler.md`・
  `docs/designs/managed-process-canonical-tool.md`・
  `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`
  (各領域の元設計)
- `docs/test-specs/rust-migration-run-semantics.md`・
  `docs/test-specs/rust-migration-differential-golden.md`(本 task を検証
  する spec)

## Dependencies

- `docs/tasks/phase-1-port-cheaper-core.md` — front matter の
  `depends-on` は「Phase 1 の `cheaper-core` が提供する API
  (`resolveWorkspacePath`/`PathDenylist`/`normalizeResourceManifest`/
  `resolveConcurrencyPolicy`/`resolveProcessPolicy`/`compileSchema`/
  `countFileTokens`/`parseDuration`/`CheaperError` 等)が確定済みである
  ことが本 task の着手条件」であることを意味する
- `docs/plans/phase-2-port-cheaper-tools.md` — 承認済み plan への従属
- 依存 crate 追加(tokio・tokio-util・regress・unicode-normalization・
  similar・globset・rand/fastrand・libc/rustix)は `Cargo.toml` 確定時に
  ユーザー承認が必要(AGENTS.md「Ask first」)

## Affected Files

- `crates/cheaper-tools/`(新設 lib crate。`src/{lib,limits,line_framer,
  walker,artifact,operation,execution_metrics,resources,scheduler,access,
  accounting,journal,definition,catalog,registry,invocation}.rs`・
  `src/tools/{boundary,read,write,edit,ls,find,grep,grep_eval,
  count_tokens,process,process_executable,process_tree}.rs`・`tests/`)
- `scripts/dump-tool-fixtures.mjs`(新規、TS registry → JSONL dump driver)
- `Cargo.toml`(workspace member 追加)

## Done When

- [x] T2-1: `crates/cheaper-tools` skeleton + `limits.rs`/`line_framer.rs`/
      `walker.rs`/`artifact.rs` が unit test 付きで移植される
- [x] T2-2: `execution_metrics.rs`/`operation.rs`/`resources.rs`/
      `scheduler.rs` が fake clock・deferred handler による決定的 unit test
      付きで移植される
- [x] T2-3: `access.rs`/`accounting.rs`/`journal.rs` が移植され、計数
      4 分類(toolCalls/rejectedToolCalls 写像)が unit test で固定される
- [x] T2-4: `definition.rs`/`catalog.rs` + fs tool 6 種が移植される
- [x] T2-5: `grep_eval.rs` + `grep.rs` が移植され、eval subprocess の
      timeout kill・output byte cap が unit test で固定される
- [x] T2-6: `process_executable.rs`(`node` 明示 error 化を含む)・
      `process_tree.rs`・`process.rs` が移植される
- [x] T2-7: `registry.rs`/`invocation.rs`/`lib.rs` が移植され、8 種
      native 構築の execute pipeline が成立する
- [x] T2-8: TS fixture dump → Rust differential replay 基盤が構築され、
      fixture matrix(read/grep/find/ls/write/edit/count_tokens/process/
      scheduler/journal)の差分が解消される。Windows 実機で全 test パス +
      process/grep smoke を完了する

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools` が全パス(TS test file の
      意味を写した unit/integration test を含む)
- [x] `mise exec -- cargo clippy -p cheaper-tools --all-targets --
      -D warnings` と `mise exec -- cargo fmt --check` が green
- [x] `node scripts/dump-tool-fixtures.mjs` で TS registry の `execute`
      結果を JSONL dump し、`mise exec -- cargo test -p cheaper-tools
      --test differential` が同 fixture を replay して
      CanonicalToolResult(content/details/error 分類)を比較して一致
      する(normalize: 時刻・tmp 名乱数・pid を除外)
- [x] 計数写像の fixture pin: budget 内実行到達=`toolCalls`、boundary
      拒否・limit 超過=`rejectedToolCalls`、runtime 未到達 invalid call=
      不計上、の 4 分類が TS dump と一致する
- [x] Windows 実機で `cargo test -p cheaper-tools` 全パス + process
      taskkill tree kill・PID reuse 非誤爆・minimal env・grep eval
      timeout kill の smoke を実施する
- [ ] `mise run check` が現行通り全 green(TS 側無破壊)
