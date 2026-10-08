---
id: "TASK-034BgK4O6Xu8NEvlYtW6lJ"
type: "task"
status: "done"
title: "Phase 0: Rig PoC go/no-go gate"
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
    - "docs/plans/phase-0-rig-poc-go-no-go-gate.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-0-rig-poc-go-no-go-gate.md"
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
  references: []
  defers: []
  deferred-by: []
---

# Phase 0: Rig PoC go/no-go gate

## Work

plan `docs/plans/phase-0-rig-poc-go-no-go-gate.md`(PLAN-034BepX6xCr0AAqkJ63aOv)
の実行単位。`crates/poc-rig/` に使い捨て spike を構築し、設計書
§Verification Notes の go/no-go ゲート 9 条件(サイズ・hook 計測・repair・
custom provider・invalid tool call・error 分類・cancel・panic・3 OS ビルド)
を実測で判定する。対応範囲は plan の Task Breakdown T1-T7。

- T1: Rust toolchain 導入 + Ollama コンテナ起動 + `crates/poc-rig/` scaffold
  (`=0.43.x` pin・release profile)+ 最小 agent + Ollama 実 streaming run
- T2: hook 逐次集計(`ModelTurnFinished`→usage・`ToolCallDelta`→toolCalls/
  turns)と `snapshot()`、scripted fake provider seam(custom
  `Wire`/`Decoder`/`Transport`)、error 経路 snapshot・unknown tool call の
  `InvalidToolCallAction::Retry{feedback}` loop・HTTP status 分類
- T3: 最終 assistant 抽出 + `jsonschema` 検証 + tool 無し repair session
  (同一 accumulator 合算・attempt 上限)
- T4: devin 相当 custom `Wire`/`Decoder`/`Transport` 骨格試作(Connect
  framing/gzip/JWT/metadata/context map の trait 面表現可否。疎通不要)
- T5: `abort-probe`(streaming `cancel()` + reason 分類 + 部分 snapshot)と
  `panic-probe`(`catch_unwind`/`JoinError` → INTERNAL_ERROR 相当・stdout
  無汚染)
- T6: Windows x64 + Linux x64(musl 優先)release サイズ実測・macOS build
  (環境があれば。無ければ gap 記録)
- T7: 全条件の実測値・手順・判定を `docs/research/rig-agent-runtime/
  poc-results.md` に記録し go/no-go を判定

spike は `crates/poc-rig/` と `poc-results.md` に閉じる。既存 TS 資産・
`mise.toml` の恒久 toolchain・CI contract には触れない。no-go 時は plan の
Rollback Strategy に従い本 plan・親 plan を `blocked` に更新し Goose 第 2
候補評価を起票する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/phase-0-rig-poc-go-no-go-gate.md`(本 task が implement する
  plan。File Map・Implementation Sequence・Verification の正)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、Common Rules)
- `docs/designs/rust-rig-runtime-migration-architecture.md`
  (§Verification Notes の go 条件・§意味論固定・§Non-Functional Constraints)
- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
- `docs/test-specs/rust-migration-run-semantics.md`(本 task を検証する spec)

## Dependencies

- `docs/plans/phase-0-rig-poc-go-no-go-gate.md` — front matter の
  `depends-on` は「本 task がこの plan の承認済み手順に従う」ことを意味する。
  先行 task は無く、migration 系 phase task の先頭である
- Ollama 実 provider 検証は `docker compose up -d` + モデル pull 済みが前提
  (AGENTS.md の検証用 Ollama コンテナ手順)

## Affected Files

- `crates/poc-rig/`(新設 spike crate。`Cargo.toml`・
  `rust-toolchain.toml`・`src/{main,agent,hooks,usage,output,repair,
  errors,fake_provider}.rs`・`src/tools/{mod,read,grep,write}.rs`・
  `src/providers/devin_wire.rs`)
- `docs/research/rig-agent-runtime/poc-results.md`(新規、判定記録)

## Done When

- [x] T1: `crates/poc-rig` が `rig`/`rig-core`/`rig-agent` `=0.43.x` pin +
      release profile(lto/opt-level="s"/strip/codegen-units=1)で build され、
      Ollama 実 run で tool call を含む multi-turn が成立する
- [x] T2: `run` の stdout JSON に hook accumulator 由来の `usage`/`turns`/
      `toolCalls` が出、`run-scripted` で provider error 経路の snapshot・
      unknown tool call の Retry feedback loop・`error-probe` の
      429/5xx/timeout 分類が確認できる
- [x] T3: `repair-run` で初回 schema 違反 → repair retry → schema-valid
      JSON が返り、`repairs >= 1` と repair 分 usage/turns の accumulator
      合算が確認できる
- [x] T4: `devin_wire.rs` 骨格が `cargo check` を通り、Connect wire 要件の
      trait 面表現可否が判定メモに明記される
- [x] T5: `abort-probe` で `cancel()` → reason 分類 + 部分 usage snapshot、
      `panic-probe` で panic → INTERNAL_ERROR 相当 JSON が stdout 1 行で
      無汚染に出力される
- [x] T6: Windows x64・Linux x64(musl)の strip 後 release binary サイズが
      実測され ≤20MB 判定が記録される(macOS は環境あれば実測、無ければ
      gap 明記)
- [x] T7: `poc-results.md` に 9 条件全ての実測値・コマンド・判定
      (go/no-go/条件付き)が記録される

## Verification

- [x] `docker compose up -d` 後に `curl http://localhost:11434/v1/models`
      がモデル一覧 JSON を返す(未起動なら `docker compose exec ollama
      ollama pull qwen3.5:9b` でモデル取得)
- [x] `cargo build --release --manifest-path crates/poc-rig/Cargo.toml` が
      成功し、`Get-Item target/release/poc-rig.exe | Select-Object Length`
      の byte 数が ≤ 20971520(20MiB)である
- [x] `cargo build --release --manifest-path crates/poc-rig/Cargo.toml
      --target x86_64-unknown-linux-musl` が成功し、`stat -c %s` の byte 数
      が ≤ 20MiB である(cross/WSL/docker のいずれかで実施)
- [x] `cargo run --release --manifest-path crates/poc-rig/Cargo.toml -- run`
      の stdout JSON に `usage`・`turns`・`toolCalls` が含まれる
- [x] `cargo run --release --manifest-path crates/poc-rig/Cargo.toml --
      run-scripted --scenario provider-error` で error 終了時にも
      `snapshot()` JSON が出力される
- [x] `cargo run --release --manifest-path crates/poc-rig/Cargo.toml --
      run-scripted --scenario unknown-tool-call` で run が Fail せず error
      feedback → loop 継続 → 正常終了し、invalid call が
      `toolCalls`/`rejectedToolCalls` に計上されない
- [x] `cargo run --release --manifest-path crates/poc-rig/Cargo.toml --
      repair-run` で repair retry が動き `repairs >= 1` が返る
- [x] `cargo run --release --manifest-path crates/poc-rig/Cargo.toml --
      error-probe` / `abort-probe` / `panic-probe` が各条件の期待出力を返す
- [x] `cargo check --manifest-path crates/poc-rig/Cargo.toml` が通り
      `devin_wire.rs` の trait 適合が証明される
- [x] `git status` で変更が `crates/poc-rig/` と `poc-results.md` に限定
      されること(TS 資産・CI contract 無改変)
