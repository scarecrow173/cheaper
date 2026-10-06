---
id: "TASK-034BgKiqu2ZlaprN931tnY"
type: "task"
status: "todo"
title: "Phase 1: port cheaper-core"
created: "2026-10-06"
updated: "2026-10-06"
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
    - "docs/plans/phase-1-port-cheaper-core.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-1-port-cheaper-core.md"
    - "docs/tasks/phase-0-rig-poc-go-no-go-gate.md"
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
    - "docs/test-specs/rust-migration-differential-golden.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 1: port cheaper-core

## Work

plan `docs/plans/phase-1-port-cheaper-core.md`(PLAN-034BepXMKsB4VzqIP9XV7l)
の実行単位。`packages/core`(src 約 2.65k 行)の全責務を、Rig 非依存・
tokio 非依存の純ロジック Rust crate `crates/cheaper-core` へ移植し、TS
実装から dump した golden fixture との差分一致で挙動互換を検証可能にする。

plan の Task Breakdown(Implementation Sequence step 1-18)に従い、
Done 条件を以下の作業単位に分解する:

- step 1-3(workspace 初期化): `mise.toml` に Rust toolchain 追加 + repo
  root virtual manifest `Cargo.toml`(`members=["crates/*"]`)+
  `.gitignore` `/target` + `crates/cheaper-core` skeleton
- step 4(leaf 移植): `errors.rs`(`ERROR_TABLE` 写し)・`duration.rs`・
  `tools/vocabulary.rs`・`config/dirs.rs`
- step 5-6(workspace): `workspace/paths.rs`(lexical resolve・symlink
  policy・最深存在祖先 + tail・`dunce` 相当 verbatim 除去)・
  `workspace/deny_paths.rs`(glob→regex 変換・win32 `i` flag)
- step 7(resources): `resources/` 3 モジュール(型・predicates・
  normalization)
- step 8(worker): `worker/` 5 モジュール(parse/schema/validate/resolve/
  write_scope。`jsonschema` draft 2020-12 + `{instancePath,message}` 写像)
- step 9(policy): `policy/` 7 モジュール(schema/merge/load/restrict/
  capability/process/workflow。§23 ロード順と restrict-concurrency を写す)
- step 10(model): `model/` 8 モジュール(tier/model_ref/config/classify/
  exceptions/pricing/resolve・pricing_snapshot。`preserve_order` で
  snapshot pattern 探索順=挿入順を保証)
- step 11(token): `token/count.rs`(`tiktoken-rs` `o200k_base`・byte cap
  16MiB・utf8 lossy・`FileTooLargeError` 相当)
- step 12(number): `json/number.rs` JS 互換 canonical number formatter
  (整値は小数点なし・`1e21`/`1e-6` 境界の JS 指数表記・`-0`→`0`・
  非有限→`null`)
- step 13-15(fixture): `scripts/dump-core-golden.mjs` で
  `crates/cheaper-core/tests/golden/fixtures/` に JSON fixture を生成
  commit し、`tests/golden/` replay ハーネスで File Map 対応表どおり
  全件消化
- step 16-18(検証): 境界ルール(`cargo tree` に rig/tokio 無し)・
  Windows 実機 fixture 一致・`mise run check` 無破壊

共通事項: cargo 操作は `mise exec -- cargo ...` 経由。依存 crate 追加は
`Cargo.toml` 確定時にユーザー承認(AGENTS.md「Ask first」)。TS 側
`packages/core` の実装は変更しない(fixture dump 機構の追加のみ)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-1-port-cheaper-core.md`(本 task が implement する
  plan。File Map・対応表・Verification の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、共通規則)
- `docs/plans/phase-0-rig-poc-go-no-go-gate.md`(go/no-go ゲート)
- `docs/designs/rust-rig-runtime-migration-architecture.md`(crate 表・
  Windows path canonicalize・envelope 数値フォーマット規則)
- `docs/test-specs/rust-migration-differential-golden.md`(本 task を検証
  する spec)

## Dependencies

- `docs/tasks/phase-0-rig-poc-go-no-go-gate.md` — front matter の
  `depends-on` は「Phase 0 の go 判定が本 task の着手条件」であることを
  意味する。no-go 時は本 task も着手しない
- `docs/plans/phase-1-port-cheaper-core.md` — 承認済み plan への従属
  (移植対象の責務・File Map・fixture 対応表は plan が正)
- 依存 crate 一覧(serde/serde_json preserve_order・serde_yml or
  serde_norway・jsonschema・tiktoken-rs・dunce・regex・tempfile)の確定時に
  ユーザー承認が必要

## Affected Files

- `Cargo.toml`(repo root virtual manifest・新規)
- `.gitignore`(`/target` 追加)
- `mise.toml` / `mise.lock`(Rust toolchain)
- `crates/cheaper-core/`(lib crate 全モジュール + `tests/golden/` replay
  harness + `tests/golden/fixtures/`)
- `scripts/dump-core-golden.mjs`(新規、TS → JSON fixture generator)
- `packages/core/`(実装は変更しない。fixture 対応確認の参照元)

## Done When

- [ ] workspace 初期化: `mise exec -- cargo build -p cheaper-core` が
      skeleton で通る(root `Cargo.toml`・`/target` ignore・mise Rust
      toolchain)
- [ ] leaf 移植完了: errors/duration/vocabulary/config dirs が unit test
      付きで移植される
- [ ] `workspace/paths.rs`・`workspace/deny_paths.rs` が plan File Map の
      規則どおり移植される
- [ ] `resources/`・`worker/`・`policy/`・`model/`・`token/`・
      `json/number.rs` の全モジュールが移植される
- [ ] `scripts/dump-core-golden.mjs` が fixture を生成し commit され、
      `crates/cheaper-core/tests/golden/` replay が全 fixture と一致する
      (schema error は `instancePath` のみ比較。message 文言・errno 診断値・
      tokenizer identity は normalize 除外)
- [ ] `cargo tree -p cheaper-core` に `rig*`・`tokio` が含まれない
- [ ] Windows 環境で `cfg(windows)` 分岐(paths/deny/env 大小文字)が
      fixture 一致する(CI `windows-latest` job 追加またはローカル実測)
- [ ] `mise run check` が現行通り全 green(TS 側無破壊)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core` が全 green(unit + golden
      replay)
- [ ] `mise exec -- cargo clippy -p cheaper-core --all-targets --
      -D warnings` と `mise exec -- cargo fmt --check` が green
- [ ] `node scripts/dump-core-golden.mjs` 再実行後に
      `git diff --exit-code -- crates/cheaper-core/tests/golden/fixtures`
      が clean(fixture 鮮度)
- [ ] `pnpm --filter @cheaper/core test` が現行通り green で、dump 対象
      API の TS test が fixture 入力集合と 1:1 対応する(File Map 対応表と
      突合)
- [ ] `mise exec -- cargo tree -p cheaper-core` の出力に `rig`・`tokio`
      が無い
- [ ] Windows: `cargo test -p cheaper-core` で case-insensitive path・
      deny `i` flag・env 大小文字の `cfg(windows)` 分岐が fixture 一致する
- [ ] `mise run check` が exit 0
