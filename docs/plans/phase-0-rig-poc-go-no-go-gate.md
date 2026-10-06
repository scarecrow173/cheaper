---
id: "PLAN-034BepX6xCr0AAqkJ63aOv"
type: "plan"
status: "approved"
title: "Phase 0 Rig PoC go-no-go gate"
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
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-run-semantics.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 0 Rig PoC go-no-go gate Implementation Plan

## Goal

`cheaper-rig-adapter` の縮小版 spike(`crates/poc-rig/`)を構築し、設計書
§Verification Notes の go/no-go ゲート 9 条件(サイズ・hook 計測・repair・
custom provider・invalid tool call・error 分類・cancel・panic・3 OS ビルド)
を実測で判定する。

## Scope

- 1 つの実装ストリーム。**使い捨て spike のみ**を作り、候補中立資産
  (後続 Phase が引き継ぐ production crate)は作らない。no-go 時は全量廃棄、
  go 時も原則として破棄 or `rust/` workspace への部分吸収に留める
  (設計書 §段階計画との対応の Phase 0 行)。
- スコープ内: Rig 最小 agent・hook ベース計測・repair 動線・scripted fake
  provider seam・devin 相当 custom `Wire`/`Decoder`/`Transport` の骨格試作・
  abort/panic probe・release サイズ実測・3 OS ビルド・判定記録。
- スコープ外: envelope serializer・exit code 表・usage.jsonl・worker/policy
  解決・catalog/credential store(G1/G2)・tool-runtime 本実装・CI matrix 改修
  (全て Phase 1-5 の責務)。`mise.toml` への Rust toolchain 恒久追加も
  Phase 5(本 PoC は rustup 等の一時導入で足りる)。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (SPEC-034BbgvL9s41K7NflOddZ6)
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan、Common Rules
  を共用)
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
  (ADR-034BbhP6hJV7DFtv7k7AGs)

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md`(approved)
  - §crate 構成: Rig 型は `cheaper-rig-adapter` (+ `cheaper-provider-devin`)
    のみが触る隔離原則 → PoC でも spike 内の adapter 相当 module に閉じ込める
  - §Rig 適用の意味論固定: `default_max_turns` は大値を渡し maxTurns 判定は
    hook 側・`InvalidToolCallAction::Retry{feedback}` 選択・`tool_concurrency` = 1・
    transient retry は completion call 単位・streaming run のみ expose
  - §Verification Notes: go 条件 5 項目 + 追加 (a)-(e)(本 plan の 9 条件)
  - §Non-Functional Constraints: strip 後 ≤20MB(目標 5-15MB)、
    `rig` 系 `=0.43.x` pin、`rig-reqwest` + rustls
  - §Risks: hook 表現力不足時は `rig-core` Model/streaming + 自前 loop へ退避
- `docs/research/rig-agent-runtime/migration-plan.md` §3 Phase 0・§6.2 ゲート
- `docs/research/rig-agent-runtime/final-report.md`(Rig 0.43 API 適合証跡:
  hook event 名・`PromptResponse.usage`・`cancel()`・`Wire`/`Decoder`/
  `Transport` 分離設計)
- 現行実装の写像源(意味確認のみ、移植自体は Phase 3):
  `packages/runtime-pi/src/usage.ts`(UsageCollector)、`repair.ts`、
  `run.ts` の abort 分類(`AbortReason` = timeout/signal/maxTurns/
  maxToolCalls)

## File Map

- `crates/poc-rig/Cargo.toml` - spike crate manifest。`rig`/`rig-core`/
  `rig-agent` を `=0.43.x` pin(実 patch は取得時の最新 0.43.x)、transport は
  `rig-reqwest`(rustls)。`tokio`・`serde_json`・`jsonschema` を追加。
  `[profile.release]` に `lto = true`・`opt-level = "s"`・`strip = true`・
  `codegen-units = 1` を記述(サイズ計測を本番構成で行うため)
- `crates/poc-rig/rust-toolchain.toml` - stable channel pin(PoC 再現性のみ。
  恒久 toolchain 管理は Phase 5 の `mise.toml` 改修で行う)
- `crates/poc-rig/src/main.rs` - エントリ。subcommand dispatch
  (`run` = Ollama 実 streaming run、`run-scripted` = fake provider 駆動、
  `error-probe` = HTTP status 分類、`abort-probe` = cancel 検証、
  `panic-probe` = catch_unwind 検証、`repair-run` = repair 動線)。判定用に
  hook 集計・error 分類・abort reason を JSON で stdout に出す
- `crates/poc-rig/src/agent.rs` - `AgentBuilder` 構成の唯一箇所。
  system prompt 文字列・`default_max_turns` 大値(判定は hook 側の契約)・
  `tool_concurrency(1)`・`InvalidToolCallAction::Retry{feedback}`・tool 登録
- `crates/poc-rig/src/hooks.rs` - agent hook 実装。`ModelTurnFinished`
  (per-turn usage)の加算・`ToolCallDelta` で tool call 計測・
  `CompletionCallEvent`/`RunSettled` の補助観測を `UsageAccumulator` へ流す
- `crates/poc-rig/src/usage.rs` - `WorkerUsage` 相当 struct + accumulator +
  `snapshot()`。成功・error・abort 全経路で共通の snapshot 源(go 条件 2 の
  証跡。「hook 逐次集計が唯一の正」の縮小版)
- `crates/poc-rig/src/output.rs` - 最終 assistant text 抽出 + `jsonschema`
  crate による検証(縮小版 `extract_output` 相当)
- `crates/poc-rig/src/repair.rs` - tool 無し repair session 試作。
  `output_schema_raw`/`output_mode` + schema error(`instancePath`+message)
  を feedback する retry loop・同一 accumulator への合算・attempt 上限
- `crates/poc-rig/src/errors.rs` - Rig error surface の分類 probe。
  `PromptError` 系 variant / `provider_response_status` 等から HTTP status
  (429/5xx/timeout)を抽出する分類器と、分類結果の JSON 出力
- `crates/poc-rig/src/tools/mod.rs` - fake tool 群の登録
- `crates/poc-rig/src/tools/read.rs` / `grep.rs` / `write.rs` - 簡易
  `Tool` trait 実装 2-3 個(固定応答 or 単純 FS 読みでよい。tool-runtime
  の本実装ではない)
- `crates/poc-rig/src/fake_provider.rs` - scripted stream provider(G7 縮小
  版)。応答 item 列・unknown tool call 注入・stream 途中 error(429/5xx)・
  hang-until-abort を deterministic に再生できる custom `Wire`/`Decoder`/
  `Transport` 実装。go 条件 5/6/7 の再現性と go 条件 4 の練習台を兼ねる
- `crates/poc-rig/src/providers/devin_wire.rs` - devin 相当 custom
  `Wire`/`Decoder`/`Transport` 骨格試作。`packages/runtime-pi/src/
  providers/devin/wire.ts`/`stream.ts` の Connect-protocol 要件(protobuf-lite
  framing・gzip・JWT・metadata・context map)が trait 面で表現可能かを
  確認するための最小実装(疎通は不要、コンパイル + trait 適合の証明が目的)
- `docs/research/rig-agent-runtime/poc-results.md` - 判定記録(新規。
  各 go 条件の実測値・手順・判定を書く spike 成果物)

## Implementation Sequence

環境構築:

- [ ] Rust stable toolchain を導入する(rustup 推奨。`mise.toml` への恒久追加は Phase 5 の責務であり本 plan では行わない)
- [ ] Ollama コンテナを起動しモデルを pull する(`docker compose up -d` → `docker compose exec ollama ollama pull qwen3.5:9b` → `curl http://localhost:11434/v1/models` で稼働確認。モデルは `CHEAPER_E2E_OLLAMA_MODEL` 慣習に合わせて変更可)
- [ ] `crates/poc-rig/` spike crate を作成し、`Cargo.toml` に File Map 記載の依存 pin と release profile を記述する
- [ ] `crates/poc-rig/rust-toolchain.toml` を作成する

最小 agent:

- [ ] `src/agent.rs` に `AgentBuilder` 構成を実装する(system prompt・`default_max_turns` 大値・`tool_concurrency(1)`・`InvalidToolCallAction::Retry{feedback}`)
- [ ] `src/tools/` に簡易 tool 2-3 個(read/grep/write 相当)を `Tool` trait で実装し agent に登録する
- [ ] `src/main.rs` に `run` subcommand(Ollama 向け streaming run)を実装する
- [ ] Ollama 実 run で multi-turn(tool call を含む)が成立することを確認する

hook 計測:

- [ ] `src/hooks.rs`/`src/usage.rs` に hook 逐次集計(`ModelTurnFinished` の per-turn `usage` 加算・`ToolCallDelta` で toolCalls・turn 計数)と `snapshot()` を実装する
- [ ] `run` の末尾に hook 集計の JSON(`usage`/`turns`/`toolCalls`)を出力し、Ollama 実 run で取得を確認する(go 条件 2 成功系)
- [ ] `src/fake_provider.rs` に scripted stream provider(custom `Wire`/`Decoder`/`Transport` 駆動)を実装する
- [ ] scripted provider で provider error を注入し、error 経路でも accumulator `snapshot()` が取得できることを確認する(go 条件 2 error 系)
- [ ] scripted provider で unknown tool call を再生し、`InvalidToolCallAction::Retry{feedback}` で error feedback → loop 継続を確認する(go 条件 5)
- [ ] `src/errors.rs` に HTTP status 分類を実装し、429/5xx/timeout を scripted/local stub で誘発して分類可否を確認する(go 条件 6)

repair:

- [ ] `src/output.rs` に最終 assistant 抽出 + `jsonschema` 検証を実装する
- [ ] `src/repair.rs` に tool 無し repair session(`output_schema_raw`/`output_mode` + schema error feedback retry・同一 accumulator 合算・attempt 上限)を実装する
- [ ] invalid output シナリオで repair retry が動き、usage/turns が main run の accumulator に合算されることを確認する(go 条件 3)

custom provider:

- [ ] `src/providers/devin_wire.rs` に devin 相当 custom `Wire`/`Decoder`/`Transport` 骨格を試作し、Connect wire 要件が trait 面で表現可能か確認する(go 条件 4)

abort/panic:

- [ ] `abort-probe` で streaming run の `cancel()` による in-flight abort と cheaper 側 reason 分類 + snapshot 取得を確認する(go 条件 7)
- [ ] `panic-probe` で tool/hook 内 panic → `catch_unwind`(または tokio `JoinError`)→ INTERNAL_ERROR 相当写像と stdout 無汚染を確認する(go 条件 8)

サイズ計測・3 OS ビルド:

- [ ] Windows x64 で `cargo build --release` し strip 後サイズを実測する(go 条件 1・Windows)
- [ ] Linux x64(`x86_64-unknown-linux-musl` 優先)で release build しサイズを実測する(go 条件 1・Linux。cross or docker or WSL)
- [ ] macOS build を実施する(環境が利用可能な場合のみ。不可なら gap として記録)(go 条件 9)

判定記録:

- [ ] 全 go 条件の実測値・手順・判定を `docs/research/rig-agent-runtime/poc-results.md` に記録する
- [ ] go 判定なら本 plan を完了として親 plan に報告する。no-go 判定なら Rollback Strategy に従う

## Dependencies and Risks

- **Rig API drift**: `ModelTurnFinished`/`ToolCallDelta`/`InvalidToolCallAction`/
  `cancel()`/`provider_response_status` の名称は final-report 調査時点のもの。
  `=0.43.x` の docs.rs API reference + source を正として実装時に確定する
  (設計書 §Risks「docs 陳腐化」に対応)。
- **Ollama コンテナ依存**: 実 provider 検証はコンテナ起動 + モデル pull 済みが
  前提。未起動時は `docker compose up -d` からやり直す(AGENTS.md の手順)。
- **scripted fake provider の二重利用**: go 条件 5/6/7 の deterministic 再現と
  go 条件 4(custom Wire 実装の練習台)を同じ seam で兼ねる。custom
  `Wire`/`Decoder`/`Transport` の実装難度が高いと判明した場合、条件 5/7 は
  「real model + 分岐誘導 prompt」への切替可否を検討するが、deterministic で
  ないため最終手段とする(この場合は判定記録に再現性の弱さを明記)。
- **hook 表現力不足**: `rig-agent` hook で `WorkerRunMeta` 粒度の計測が
  取れないと判明した場合のみ、設計書 §Risks の fallback 通り `rig-core` の
  Model/streaming 面で自前 loop に切替えて再検証する。それでも不達なら
  no-go 判定材料。
- **HTTP status 露出不足**: Rig error surface に構造化 status が無い場合、
  設計書 §意味論固定の許容に従い error 文字列分類への退避を「条件付き go」
  材料として記録する(silent に妥協しない)。
- **macOS 環境不在**: 条件 9 の 3 OS のうち macOS は実施環境次第。不在なら
  「win+linux 実測 + darwin は Phase 5 の CI matrix で担保」を明示 gap として
  判定に付記する。
- **`crates/` 新設の衝突**: 現行 repo に `crates/` は未存在。spike 専用 dir として
  新設し、go 後の workspace 名(`rust/` or `packages-rs/`)とは別管理とする。

## Task Breakdown

- T1 scaffold + 最小 agent + Ollama run(steps: 環境構築〜最小 agent)
- T2 hook 計測 + fake provider seam + error/unknown-tool-call probe(hook 計測節)
- T3 output 抽出 + repair 動線(repair 節、T2 の accumulator 完了が前提)
- T4 devin wire 骨格(custom provider 節、T2 の fake provider で trait 面を
  学んだ後に実施)
- T5 abort/panic probe(T2 完了後並行可)
- T6 サイズ計測 + 3 OS ビルド(T1 以降いつでも可、最終判定前に完了)
- T7 判定記録(全完了後)

## Verification

- [ ] 条件 1(サイズ): `cargo build --release --manifest-path crates/poc-rig/Cargo.toml` で `target/release/poc-rig.exe`(Windows)、
  `cargo build --release --target x86_64-unknown-linux-musl` で Linux binary を
  生成し、Windows は `Get-Item ... | Select-Object Length`、Linux は
  `stat -c %s` で byte 数を記録。判定: strip 後 **≤ 20MB(20×1024×1024 bytes)**、
  目標 5-15MB。超過時は TLS backend/依存 feature 削りで一度だけ再測定し、
  それでも超過なら no-go 材料として記録
- [ ] 条件 2(hook 計測): `poc-rig run` の stdout JSON に `usage`(input/output
  等の token field)・`turns`・`toolCalls` が hook accumulator 由来で出力される。
  さらに `poc-rig run-scripted --scenario provider-error` で error 終了時にも
  `snapshot()` JSON が出力される(error/abort 経路の snapshot 取得)
- [ ] 条件 3(repair): `poc-rig repair-run` で初回 schema 違反 → repair session
  retry → schema-valid JSON が返り、`repairs >= 1` かつ repair 呼出し分の
  usage/turns が accumulator に合算される
- [ ] 条件 4(custom provider): `cargo check` が通る `devin_wire.rs` 骨格 +
  判定メモに「Connect framing/gzip/JWT/metadata/context map が trait 面で
  表現可能」の可否を明記(実 endpoint 疎通は不要)
- [ ] 条件 5(invalid tool call): `run-scripted --scenario unknown-tool-call`
  で run が `Fail` せず error feedback → loop 継続 → 正常終了。runtime 未到達の
  invalid call は `toolCalls`/`rejectedToolCalls` に計上しないこと
- [ ] 条件 6(error 分類): `error-probe` で 429/5xx/timeout を誘発し、Rig error
  surface から HTTP status(または retryable 分類可能な情報)が抽出できる。
  構造化露出が無い場合は文字列分類 fallback の可否を記録
- [ ] 条件 7(cancel): `abort-probe` で streaming run 中に `cancel()` →
  `prompt_cancelled` 相当の終了 + cheaper 側 reason 記録 + 部分 usage
  snapshot 取得ができる
- [ ] 条件 8(panic): `panic-probe` で tool/hook 内 panic が `catch_unwind`/
  `JoinError` で捕捉され、INTERNAL_ERROR(exit 70)相当の JSON が stdout に
  1 行で出、panic 生出力で stdout が汚染されない
- [ ] 条件 9(3 OS): Windows x64・Linux x64(musl)・macOS(環境あれば)で
  `cargo build --release` が成功する。macOS 不可の場合は gap 記録で代替判断
- [ ] 判定記録: `poc-results.md` に全条件の実測値・コマンド・判定(go/no-go/
  条件付き)が記録され、親 plan の Common Rules と矛盾しないことの review

## Open Questions / Gaps

- `rig` facade 経由の feature opt-in と `rig-core`/`rig-agent` 個別 crate pin の
  どちらを採るかは spike 作成時に `=0.43.x` の crates.io/docs.rs で確定する
  (設計書は両者を併記)。依存最小化(サイズ)の観点で個別 crate 優先を想定。
- HTTP status の構造化露出(`provider_response_status` 相当)の有無は未検証で
  あり、条件 6 の実測が初検証。露出なしの場合の文字列分類は retryable 誤判定
  リスクを伴うため判定時に別途審査する。
- macOS build 環境の有無が未確認。不在なら条件 9 を「win+linux 実測 + darwin
  は Phase 5 CI matrix で担保」と読み替えるかを go/no-go 判定時に審査する。
- `Wire`/`Decoder`/`Transport` の trait 境界が Connect protocol(gzip +
  protobuf-lite framing + JWT cache)を表現しきれるかは条件 4 の試作次第。
  不適合なら「devin を OpenAI-compat 扱い」または「devin を v1 scope から
  切り下げ」(設計書 §Deferred)の判断材料として記録する。
- scripted fake provider の応答 DSL(応答 item 列・遅延・stream 途中 error・
  hang-until-abort)の詳細設計は本 plan では固定せず、T2 で最小実装する。
  設計書 §Verification Notes の「fixture DSL 先行定義」は Phase 3 の本番
  seam 向け要件であり、spike では subset でよい。
- Ollama モデルタグは環境依存(`qwen3.5:9b` または `:4b`)。判定に影響しない
  ため実施環境で選択する。

## Rollback Strategy

- spike は `crates/poc-rig/` と `poc-results.md` に閉じ、既存 TS 資産・CI
  contract(`mise run check`)には一切触れない。撤去は dir/file 削除
  (または `git revert`)のみで完結する。
- **go 時**: spike は原則破棄し、再利用可能な知見(hook 利用形・fake seam
  構造・profile 設定)のみ Phase 3 の `cheaper-rig-adapter` 設計へ引き継ぐ。
  `crates/poc-rig` を後続 workspace へ吸収するかは Phase 1/3 plan の責務。
- **no-go 時**: 本 plan の status を `blocked` に更新し、親 plan
  (`migrate-cheaper-runtime-to-rust-rig.md`)も `blocked` にする。
  migration-plan §6.2 / 設計書 §Deferred の既定手順に従い、Goose GDK 第 2
  候補 PoC の評価を起票する。Rig 固有記述のうち adapter 隔離思想のみ
  流用可能と判定記録に明記する。

## Self-Review

- [x] Every spec requirement maps to a task(spec 側の Phase 0 要求は設計書
  §Verification Notes の 9 条件であり、Implementation Sequence と
  Verification に 1 対 1 で対応)。
- [x] No placeholders remain(不明点は Open Questions / Gaps に明記)。
- [x] File paths, relations, status values, and terminology are consistent
  (`crates/poc-rig/`・`=0.43.x` pin・hook 逐次集計・go/no-go 用語を
  設計書・親 plan と統一)。

## Implementation Handoff

- Recommended mode: subagent-capable implementation(T2/T4/T5/T6 は T1/T2
  完了後に分離可能だが、単一 spike のため inline 連続実行でも十分)
- Independent work streams: T4(devin wire)と T5(abort/panic probe)は T2
  完了後に相互独立。T6(サイズ/3 OS)は T1 完了後いつでも実行可
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- レビュー重点: go 条件の判定手順が第三者でも再現できるか(コマンド・判定
  基準の明確さ)、「条件付き go」(HTTP status 文字列分類 fallback・macOS
  欠落)の扱いが曖昧でないか、no-go 時の blocked 化 + Goose ピボット起票
  経路が親 plan と一致しているか。
- 前提: Rig `=0.43.x` が crates.io に存在すること、`docker compose` の Ollama
  コンテナが利用可能であること、Windows ホストで Linux 向け build は
  cross/WSL/docker のいずれかで実施できること。いずれか不成立なら該当条件を
  gap として判定記録に残す。
- 判定権限: 実測結果に基づく go/no-go の最終判定はユーザー承認を要する
  (本 plan は実測手順と記録までを責務とする)。
