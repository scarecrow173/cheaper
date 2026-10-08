---
id: "TASK-034BgKk2iQMEQ0xHA3SThv"
type: "task"
status: "done"
title: "Phase 4: port CLI and workflow"
created: "2026-10-06"
updated: "2026-10-08"
owners: []
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
    - "docs/plans/phase-4-port-cli-and-workflow.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-4-port-cli-and-workflow.md"
    - "docs/tasks/phase-3-port-runtime-adapter-providers.md"
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
    - "docs/test-specs/rust-migration-envelope-parity.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
    - "docs/test-specs/rust-migration-usage-log-hooks.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 4: port CLI and workflow

## Work

plan `docs/plans/phase-4-port-cli-and-workflow.md`
(PLAN-034Beq0dg6IYk2lQCGpWDG)の実行単位。`packages/cli`(≈3.6k 行)と
`packages/workflow`(≈2.1k 行)の全責務を Rust crate `crates/cheaper-cli`・
`crates/cheaper-workflow` へ移植し、envelope(`schemaVersion:1`・1 JSON +
末尾改行)・exit code 表・hook 4 target・usage.jsonl の byte/意味互換を、
TS binary との binary-to-binary differential golden(normalize 規則適用後
厳密比較)で担保する。workflow engine は Phase 4 同居と確定済み。

plan の Task Breakdown T1-T9 が本 task の作業単位:

- T1 `cheaper-workflow` crate port(Sequence A 全区間): `json.rs`・
  `types.rs`・`bindings.rs`(`$input`/`$from`/`$literal`・JSON Pointer)・
  `dag.rs`(deterministic sort・cycle/self/unknown dep error)・
  `schema.rs`・`interface.rs`・`resource_inference.rs`・`artifacts.rs`・
  `compiler.rs`・`engine.rs`(tokio pump/admit/execute・`maxJobs`・lease・
  abortGrace・diagnostic UTF-8 2048B cap)・`loader.rs`。Rig 非依存のため
  Phase 3 と並行可能な独立 stream
- T2 envelope/args/entry 基盤(Sequence B): `main.rs`(panic hook +
  catch_unwind/JoinError → `INTERNAL_ERROR`(70)境界)・`envelope.rs`
  (4 形態・optional 省略・null 意味論・canonical number formatter 適用・
  末尾改行)・`args.rs`(command 別 option 許可表・alias・NAME_PATTERN)・
  `input.rs`・`count.rs`・`embedded.rs`(`include_dir!` builtin resource +
  version 解決、G8)
- T3 run/count-tokens/models(Sequence C 前半): `run.rs`(worker/policy/
  model 解決・SIGINT/SIGTERM → CancellationToken 中継・model line stderr)・
  `models.rs`(executeModels の filter/classify/autoContext 経路)
- T4 usage writer/report: `usage_log.rs`(record schema・additive-only・
  best-effort)・`pricing.rs`・`usage_report.rs`(filter/aggregate・
  `--compare-model` の runtime catalog → user `pricing.json` → bundled
  snapshot 解決・stale 30 日判定)
- T5 auth サブコマンド: `auth.rs`(3 action・`result.providers[]`/
  `credentials[]`・terminal interaction・`--method`・URL opener・非 TTY →
  INVALID_REQUEST)
- T6 hook 経路: `hook/mod.rs`(緩い argv 解析・KNOWN_EVENTS・fail-open
  catch-all)・`targets.rs`(4 target byte 互換 render)・
  `routing_policy.rs`(threshold・byte fast path・30s measure timeout・
  shell tokenizer/`extract_read_candidates`)
- T7 workflow CLI glue: `workflow_context.rs`・`workflow_run.rs`
  (executor/lease port・WorkflowEnvelope・artifact ledger)
- T8 differential harness + fixture: `fake_stream.rs`(fixture JSON → G7
  seam 注入)・`tests/differential/`(harness・normalize 単一実装・
  fixture 20-40 case)・TS e2e/contract test 相当の再現
- T9 互換 test + docs: hook byte 一致 matrix・usage.jsonl 新旧互換・
  `docs/guides/usage.md` に `costUsd` 口径変更と新旧 record 混在を追記

`cli.rs` には **`cheaper __grep-eval` 内部 mode dispatch** を必須で配線する
(argv 先頭一致の早期分岐で `cheaper_tools::grep_eval::helper_main` を起動。
欠落すると production で grep tool が全滅)。adapter session 構築 + devin
plugin 登録の composition root は cli が担う(runtime→devin は循環依存で
不可)。Rig 型参照は禁止。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-4-port-cli-and-workflow.md`(本 task が implement する
  plan。File Map・Sequence A-D・Verification・normalize 規則の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、stdout
  契約・panic → INTERNAL_ERROR・hook fail-open 等の共通ルール)
- `docs/research/rig-agent-runtime/migration-plan.md` §3.3(並走比較戦略)
- `docs/designs/rust-rig-runtime-migration-architecture.md`(crate 表・
  envelope/CLI 互換節・builtin resource 埋込)
- `docs/test-specs/rust-migration-envelope-parity.md`・
  `docs/test-specs/rust-migration-differential-golden.md`・
  `docs/test-specs/rust-migration-usage-log-hooks.md`(本 task を検証する
  spec)

## Dependencies

- `docs/tasks/phase-3-port-runtime-adapter-providers.md` — front matter の
  `depends-on` は「Phase 3 が確定させる runner/catalog/auth trait 面・
  `WorkerRunMeta`/`WorkerRunError`・G7 fake seam(3 層注入)が本 task の
  C/D 区間の着手条件」であることを意味する。trait 面が未確定なら Phase 3
  側で確定させてから着手し、CLI 側で勝手に境界を引かない
- `docs/plans/phase-4-port-cli-and-workflow.md` — 承認済み plan への従属
- Phase 1 の canonical number formatter・ERROR_TABLE・`resolvePricing`、
  Phase 2 の invocation runtime・lease・ToolCatalog も前提
- secret prompt raw-mode crate(crossterm 等)・signal 中継 crate
  (`tokio::signal` vs `ctrlc`)の選定は依存追加のため実装時に確認
  (Ask first)

## Affected Files

- `crates/cheaper-workflow/`(新設 lib crate。`src/{lib,json,types,
  bindings,dag,schema,interface,resource_inference,artifacts,compiler,
  engine,loader}.rs`)
- `crates/cheaper-cli/`(新設 bin crate。`src/{main,envelope,args,cli,input,
  run,count,models,usage_log,usage_report,pricing,auth,text_format,embedded,
  fake_stream,workflow_context,workflow_run}.rs`・`src/hook/{mod,targets,
  routing_policy}.rs`・`tests/differential/`・`tests/{envelope,hook,
  usage_log}.rs`)
- `docs/guides/usage.md`(`costUsd` 口径変更・新旧 record 混在の追記)
- `Cargo.toml`(workspace member 追加)

## Done When

- [x] T1: `crates/cheaper-workflow` が Sequence A 全区間を port し、
      `packages/workflow` の unit/contract test 相当が全パスする
- [x] T2: `cheaper-cli` scaffold + panic 境界 + `envelope.rs`/`args.rs`/
      `input.rs`/`count.rs`/`embedded.rs` が実装される
- [x] T3: `run.rs`/`count.rs`/`models.rs` のコマンド経路が実装される
- [x] T4: `usage_log.rs`/`pricing.rs`/`usage_report.rs` が実装され、
      usage.jsonl writer が additive-only である
- [x] T5: `auth.rs` の 3 action と terminal interaction が実装される
- [x] T6: `hook/` の dispatch + 4 target + routing policy が実装され、
      panic を含む全 error 経路が fail-open に覆われる
- [x] T7: `workflow_context.rs`/`workflow_run.rs` が T1+T3 の成果物を
      接続する
- [x] T8: differential harness + normalize 単一実装 + fixture 20-40 case
      が構築される(実績: `tests/differential/` 43 cases —
      `tests/differential/README.md` に fixture inventory・比較規約・
      pinned divergence を集約)
- [x] T9: hook byte 一致 matrix・usage.jsonl 新旧互換 test・
      `docs/guides/usage.md` 追記が完了する
- [x] `cheaper __grep-eval` 内部 mode dispatch と composition root
      (adapter session + devin plugin 登録)が `cli.rs`/`main.rs` に配線
      される

## Verification

- [x] `mise exec -- cargo test -p cheaper-workflow` が全パス
- [x] `mise exec -- cargo test -p cheaper-cli` が全パス(envelope・args・
      usage・auth・workflow glue の unit/contract test)
- [x] `mise exec -- cargo test -p cheaper-cli --test differential` で TS
      binary と Rust binary の stdout envelope + exit code を normalize
      規則適用後に 20-40 case 厳密比較して全パス(厳密対象: `usage`
      token field・`turns`・`toolCalls`・`rejectedToolCalls`・`repairs`・
      `transientRetries`・`changes`・`savings`・`result`・exit code。
      除外: `meta.durationMs`・`meta.execution` 時刻系 field・
      `usage.costUsd`・tokenizer identity・`error.message` 本文・stderr
      文言・JSON key 順・schema error message 文言/件数/code)。
      実績: 55 cases — pass=52 / pinned divergence=3 / fail=0
      / rust-only=0(follow-up fix 後の再測定。T8 時点 43 cases
      pass=35/divergence=8、T9 で hook matrix 拡張後 55 cases
      pass=47/divergence=8)。
      `usage.costUsd` は spec 上は除外許容だが、adapter が fake model
      cost から `usage.cost.total` を算出して埋めるため厳密比較のまま
      通る(除外しない)。pin 系統は panic 写像・in-flight abort turn
      計上(`run-max-turns`/`run-max-tool-calls`)の 2 系統に縮小
      — `error.details` shape・auth status metadata は follow-up fix
      F4/F5 で binary 一致となり pin 解除済み(README 参照)
- [x] `mise exec -- cargo test -p cheaper-cli --test envelope` で
      `schemaVersion:1`・1 JSON + 末尾改行・optional 省略・null 意味論・
      canonical number formatter(`1.0`/`1e-7`/非有限を網羅)が検証される
- [x] `mise exec -- cargo test -p cheaper-cli --test hook` で 4 target ×
      event × allow/deny/malformed stdin の stdout/stderr/exit code が TS
      版と byte 一致し、fail-open(malformed・未知 event・内部 error・
      panic)が維持される
      (byte parity は `--test differential` の hook 16 case が実 binary
      間で担保 — stdout+stderr+exit code 全一致。fail-open の panic・
      usage 非記録・policy 破損等は `tests/hook.rs` + differential
      `cli-hook-*` fail-open case で網羅)
- [x] `mise exec -- cargo test -p cheaper-cli --test usage_log` で Rust
      writer の新 record を TS `cheaper usage` が集計でき、旧 schema
      record を Rust `cheaper usage` が読める新旧双方向互換が検証される
      (literal shape test + node 経由の実 TS `readUsageRecords`/
      `buildUsageReport`/`buildUsageRecord` cross-check —
      node/dist 不在環境では skip)
- [x] `cheaper usage --compare-model` の pricing 解決順(runtime catalog
      → user `pricing.json` → bundled snapshot)・stale 30 日判定・
      `directlyComparable` 母数が TS 版と一致する
      (`tests/usage_report.rs` が TS `usage-report.test.ts` の全 case を
      port — runtime catalog 優先・pricing.json fallback・stale/境界値・
      comparable subset を検証。bundled snapshot の解決自体は core
      `resolve_pricing` が担保)
- [x] `cheaper auth status` の `result.providers[]`/`credentials[]` 形状が
      TS 版と一致する(ISS-19a)
      — oauth 対応 provider の `methods[]`/`subscription` を provider
      宣言(`auth.oauth`)ベースで列挙するように follow-up fix F5 で解消。
      differential `cli-auth-status` は pin 解除済み・binary 一致を確認
      (oauth login 実行自体は引き続き登録 flow 必須 — github-copilot
      以外の oauth login は ISS-23 の deferred 範囲)
- [x] panic 注入で `INTERNAL_ERROR`(exit 70)error envelope が stdout に
      出て panic trace が stdout に漏れず、hook 経路では panic が
      allow/exit 0 に落ちる
      (envelope 側は differential `run-panic` が Rust INTERNAL_ERROR(70)
      envelope を pin で確認、hook 側は `tests/hook.rs`
      `panic_in_stdin_read_falls_back_to_allow` が 4 target の
      allow/exit 0 を確認)
- [x] Windows runner で contract test(envelope/hook/count-tokens/run 系)
      が全パス
- [ ] `mise run check` が exit 0(TS 側無破壊)
      — **延期**(既存 flake): `packages/cli` の e2e
      `launcher-acquisition.test.ts > downloads only once under
      concurrent first-run` が並列負荷下で失敗する既存 flake(Phase 3
      task doc でも観測済み)。対象ファイル単独再実行では 55/55 pass
      を確認済みで、T9 の変更(crates/ + docs のみ)とは無関係。
      lint/biome 警告(Biome max-size・noNonNullAssertion)も既存分
