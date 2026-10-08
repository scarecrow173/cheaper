# Rig PoC Results — Phase 0 go/no-go 判定記録

- 実施日: 2026-10-07
- 対象: `crates/poc-rig/`(使い捨て spike)、rig `=0.43.0` pin(rig-core /
  rig-agent / rig-reqwest は facade 経由で `0.43.x` 解決、実バージョンは
  `Cargo.lock` が固定)
- 判定基準: `docs/designs/rust-rig-runtime-migration-architecture.md` §Verification Notes の go/no-go ゲート 9 条件
- Task: `docs/tasks/phase-0-rig-poc-go-no-go-gate.md`(TASK-034BgK4O6Xu8NEvlYtW6lJ)
- Experiment Log: `docs/impl/exp/rig-poc-go-no-go-conditions-measurement.jsonl`
- Implementation Record: `docs/impl/ir/phase-0-rig-poc-go-no-go-spike.md`

## 判定サマリ

| # | 条件 | 実測 | 判定 |
|---|------|------|------|
| 1 | サイズ ≤ 20MB(strip 後 release) | Windows x64: **7,983,104 B (7.61 MiB)** / Linux x64 musl: **7,761,504 B (7.40 MiB)** | **go** |
| 2 | hook 逐次集計(usage/turns/toolCalls) | 実 Ollama run で `turns=2, toolCalls=1, usage=1146/181/1327` を accumulator から取得 | **go** |
| 3 | repair session(同一 accumulator 合算) | `repair-run`: 初回 schema 違反 → retry → valid、`repairs=1, turns=2` 合算 | **go** |
| 4 | custom provider seam(`Wire`/`Decoder`/`Transport`) | `devin_wire.rs` が `cargo check` 通過。Connect 要件(base_url/JWT/gzip/metadata/context map)は trait 面で表現可能 | **go** |
| 5 | invalid tool call → `Retry{feedback}` 復帰 | `run-scripted --scenario unknown-tool-call` で回復成功、invalid call は `toolCalls`/`rejectedToolCalls` に非計上。**rollback された attempt の billed usage も meta に残ることを実測**(inputTokens=250 = rolled-back 100 + accepted 150) | **go**(要 budget、注意事項参照) |
| 6 | error 分類(429/5xx/transport) | `error-probe`: 429→TRANSIENT_RATE_LIMIT、500→TRANSIENT_SERVER、request-error→PERMANENT、transport→TRANSIENT_TRANSPORT | **go** |
| 7 | cancel(abort)+ 部分 snapshot | `abort-probe`: turn 1 settle 後に turn 2 途中で stream drop。`partialMeta.turns=1, usage=10/2/12` — **非ゼロの部分 accounting が残ることを実測** | **go** |
| 8 | panic → INTERNAL_ERROR・stdout 無汚染 | `panic-probe`: JoinError.is_panic + catch_unwind 両経路で INTERNAL_ERROR JSON が stdout 1 行、exit 70 | **go** |
| 9 | 3 OS ビルド | Windows x64 ✓・Linux x64 musl ✓・macOS **未実測(環境なし)** | **条件付き go** |

## 総合判定: **go(条件付き)**

9 条件中 8 条件が実測で go。macOS ビルドのみ実環境がなく未実測だが、
musl glibc 系と MSVC の両方で成功しており、macOS(aarch64/x86_64)は
CI の release matrix で初回実行時に確認すればよい。Phase 1 着手を妨げる
リスクではないと判断する。

## 実行手順(再現用)

```bash
# ビルド(rig =0.43.0 pin、release profile: lto/opt-level=s/strip/codegen-units=1)
cargo build --release --manifest-path crates/poc-rig/Cargo.toml

# scripted probes(provider 不要)
./target/release/poc-rig run-scripted --scenario provider-error   # error でも meta snapshot 出力、exit 75(TRANSIENT_SERVER)
./target/release/poc-rig run-scripted --scenario unknown-tool-call # Retry 復帰、invalid 非計上、rollback usage 保持
./target/release/poc-rig repair-run                                # repairs>=1、accumulator 合算
./target/release/poc-rig error-probe                               # 429/5xx/request/transport 分類
./target/release/poc-rig abort-probe                               # cancel + 非ゼロ部分 snapshot
./target/release/poc-rig panic-probe                               # INTERNAL_ERROR、exit 70

# 実 provider(Ollama コンテナ)
docker compose up -d && docker compose exec ollama ollama pull qwen3.5:4b
OLLAMA_API_BASE_URL=http://localhost:11434 ./target/release/poc-rig run \
  --model qwen3.5:4b --prompt "Use the read tool ..."

# Linux x64(musl)サイズ実測(alpine コンテナ)
docker run --rm -v "$PWD/crates/poc-rig:/src" -w /src rust:1.98-alpine \
  sh -c "apk add --no-cache musl-dev && cargo build --release && stat -c %s target/release/poc-rig"
```

## 計測値

### バイナリサイズ(strip 済み release)

| target | bytes | MiB |
|--------|------|-----|
| x86_64-pc-windows-msvc | 7,983,104 | 7.61 |
| x86_64-unknown-linux-musl | 7,761,504 | 7.40 |
| aarch64-apple-darwin | 未実測(環境なし) | — |

依存セット: `rig`(default features: agent/reqwest/derive/rustls)+ tokio +
serde_json + jsonschema + futures + http。`rig-reqwest`(reqwest 0.13 +
hyper + rustls)を含んでも 20MB を大きく下回る。

### 実 Ollama run(qwen3.5:4b)

```json
{"status":"ok","result":"The `grep` command successfully identified 8 function definitions ...",
 "meta":{"turns":2,"toolCalls":1,"usage":{"inputTokens":1146,"outputTokens":181,"totalTokens":1327,...}}}
```

tool(grep)を含む multi-turn が成立し、hook 集計のみで usage/turns/
toolCalls を再構成できた。

## 重要な実装知見(Phase 1+ への引き継ぎ)

1. **`max_invalid_tool_call_retries` の既定は 0**。
   `InvalidToolCallAction::Retry{feedback}` は run 単位の budget を明示
   (`agent.prompt(..).max_invalid_tool_call_retries(n)`)しないと
   `validate_invalid_tool_call_action` が黙って `Fail` に潰す。
   `Retry`/`Repair`/`Skip`/`Stop` のうち本契約では `Retry` のみ許可。
   **設計・Phase 3 plan に反映必須**。
2. **provider error の到達経路**: run 経由では `PromptError::Report(
   ErrorReport)` として届き、`http_status`/`retryable`/`kind`/
   `provider_response` を保持する。分類は `ProviderError::
   provider_response_status()`+`kind()` または `ErrorReport` 側の同規則で
   一貫して実施できる(transport=ErrorKind::Http は status なし)。
3. **rollback された turn は hook を一切発火しない**(engine.rs の
   `settle_model_turn` 前段で `TurnRetried` → break)。`on_outcome` の
   completion も `on_model_turn_finished` も accepted attempt に限られる
   ため、**巻き戻された attempt の billed usage は `on_run_settled` の
   `SettledOutcome::Response(&PromptResponse).usage` でのみ捕捉可能**。
   PoC の accumulator は per-completion outcome で逐次加算し、settle 時に
   run 総量へ reconcile(max)する二段方式でこれを実測証明した
   (`unknown-tool-call` で inputTokens=250 = rolled-back 100 + accepted
   150)。error 終了時(`SettledOutcome::Error`)は `PromptResponse` を
   持たないため rollback 分 usage は取れない — Phase 3 の設計で許容
   可否を確認する。`turns` は accepted turn のみ計上で cheaper 規則と整合。
4. **`catch_unwind` には `panic = "unwind"` が必須**(profile で明示済み。
   `panic = "abort"` は禁止 — 設計どおり)。
5. **`ModelTurnFinished` は tool-call を含む turn でも発火**し、`usage`
   がある。repair session は同一 `Accumulator` に `Arc` で合算可能だった。
6. **`jsonschema` crate(0.58)** で `instancePath` 相当の検証ができ、
   schema 違反 → corrective feedback prompt → 再 run の repair loop が
   hook 集計にそのまま載る。
7. Devin 用 custom provider は `Wire`(endpoint=plain data)+`Decoder`+
   `Transport` 分離で表現可能。`send` は `Opening::failed` で失敗させる
   だけで trait 適合が証明できる(実装は Phase 3)。

## review-protocol レビュー記録(実装レビュー)

2 名の独立レビュアー(A: 協調、B: 敵対)+ ソース照合検証を実施し、
以下を CONFIRMED して修正済み:

- **P1**: deny/skip された tool が `Ok(ToolResult::skipped)` として
  `toolCalls` に誤計上される経路 → `is_skipped()`/`Err` outcome を
  `rejectedToolCalls` 側へ計上するよう修正
- **P1**: rollback usage 欠落(上記知見 3)→ `on_run_settled` reconcile
  で解消、実測値で証明
- **P1**: abort-probe が 1-turn mock で partialMeta 全ゼロ(証明になって
  いなかった)→ 2-turn script に変更し非ゼロ partial を実測
- **P2**: `"timeout"` と命名された case が実際は `ErrorKind::Request`
  で PERMANENT に落ちる矛盾 → `request-error` に改名、真の transport
  timeout(`ErrorKind::Http`)は合成 error で別途検証
- **P2**: `error.code` の表記揺れ(`{:?}` vs serde)→ serde 側に統一、
  scripted provider-error の exit code を `class.exit_code()` に統一
- **P2**: mutex poison で `snapshot()` が panic し得る → `into_inner`
  に変更、`repairs` 重複キー・dead counter(`transient_retries`)・
  `tool_use_prompt_tokens` 欠落を修正、unknown-tool-call の pass 条件を
  強化(usage 保持・turns・非空 result を検証)

報告のみ(修正対象外、Phase 1+ へ申し送り):

- **rig-agent 0.43.0 の upstream wart**: `engine.rs` の streamed invalid
  tool call 経路に残置された `eprintln!("PARTIAL ...")` が provider の
  partial 出力を stderr に流出させる。stdout 契約には影響しないが、
  stderr への内容漏洩として migration 時に把握しておく。
- **`rig` facade の `=0.43.0` pin は sub-crate(rig-agent/rig-core)を
  固定しない** — 再現性は `Cargo.lock` の commit に依存。production
  crate では workspace pin 方針を Phase 1 で確定する。
- **`test-utils` feature は PoC 限定**。production crate へ持ち込まない
  (dev-dependency 化も含めて Phase 3 で決定)。
- repair loop は Rig built-in の `reprompt_for_output`/`extractor` 経路を
  使わない手動 loop で証明した — cheaper 契約の検証が目的であり、
  built-in 経路の hook 集計互換性は未検証(Phase 3 で選択)。
- `rust-toolchain.toml` は channel `stable`、rig の `rust-version` は
  1.95.0 — toolchain pin 方針は Phase 1 で確定。

## 未実測・残ギャップ

- macOS バイナリサイズ・動作: 環境なし。Phase 5 の release matrix 初回
  実行で確認する。
- `usage.costUsd` の pricing 層: PoC 範囲外(Phase 1/3)。
- Windows path canonicalization・JS regex(regress)・envelope byte 互換:
  Phase 1/2 のスコープであり本 PoC の条件ではない。
- 実 provider での retry/backoff 実測: scripted error-probe のみ実施。
  実 429 の観測は Phase 3 の provider 実装時に確認する。
