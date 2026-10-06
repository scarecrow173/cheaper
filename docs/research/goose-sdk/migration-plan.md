# Goose GDK 移行計画

作成日: 2026-10-06
対象: `packages/runtime-pi`(Pi SDK / TypeScript)を `goose-agent` + `goose-providers` 等の crates.io 公開 GDK クレート上の Rust 実装へ置き換える移行計画。
前提資料: `docs/research/goose-sdk/final-report.md`、`docs/research/pi-runtime-replacement/comparison.md`。
本計画は現行コードの実調査に基づく。すべての節で「現行責務 → Goose 移行先」を対応付ける。

## 0. 移行の全体像

現行 `cheaper` は「TypeScript ワークスペース 5 パッケージ + Node SEA 単一実行ファイル(約 113MB)」。
移行後は「Rust workspace + per-platform ネイティブ実行ファイル」。配布契約(launcher が
`cli/bin/<platform>/cheaper[.exe]` を参照し、不在時に固定 version の Release + SHA256SUMS 検証で
user cache に取得)は asset を差し替えるだけで維持できる(`scripts/lib/release-artifacts.mjs` の
`releaseTargets` は既に `cheaper-linux-x64` / `cheaper-linux-arm64` / `cheaper-windows-x64.exe` の
素バイナリ名を前提としている)。

コードベース実測: src 配下の TS は約 16.3k 行(test 除く)。移行工数の支配項は GDK 差異ではなく
この全面 rewrite であり、候補比較(comparison.md)と同じ結論となる。

## 1. スコープ整理 — Rust 化対象モジュール

| 現行モジュール | 行数規模 | 責務 | 移行先 crate(案) |
|---|---|---|---|
| `packages/core/src` | ~3.0k | `CheaperError` + error code/exitCode/retryable 表、worker `.md`(YAML frontmatter + JSON Schema)解決・検証、policy.yaml 読込/merge/restrict、model ref/tier 分類/pricing snapshot、workspace path canonicalize/denylist、o200k token 計測(`js-tiktoken`)、resource manifest | `cheaper-core` |
| `packages/tool-runtime/src` | ~4.0k | sandboxed tool 本体: read(596)/grep(634)/find/ls-walker/count_tokens/managed process(spawn・tree kill・timeout・出力 cap)、scheduler(並行度)、access(事前認可)、budget/toolCalls 計上、meter(tool output token 和)、MutationJournal→ChangesSummary、invocation scope/lease、artifact store | `cheaper-tools` |
| `packages/runtime-pi/src/tools` | ~0.9k | canonical tool → Pi ToolDefinition の adapter、write/edit/ls helper 定義 | `cheaper-agent`(SyncTool/AsyncTool 実装として再編) |
| `packages/runtime-pi/src`(本体) | ~1.3k | `runWorker`: abort 分類(timeout/signal/maxTurns/maxToolCalls)、model 解決(tier/explicit)、session 構築、出力抽出 + repair session、UsageCollector、meta 構築、`promptWithAbortGrace` | `cheaper-agent` |
| `packages/runtime-pi/src/providers/devin` | ~1.4k | Devin CLI 委譲 login/catalog、credentials.toml + JWT cache、Connect プロトコル wire(独自 varint/protobuf framing + gzip)stream | `cheaper-provider-devin`(Provider trait 実装) |
| `packages/cli/src` | ~3.4k | argv parse(430)、subcommand dispatch、envelope 契約、auth login/logout/status(対話プロンプト・browser 起動)、count-tokens、usage.jsonl append/集計 report、pricing.json、text format、hook targets + routing-policy(444)、workflow run/context | `cheaper-cli` |
| `packages/workflow/src` | ~2.0k | workflow YAML schema、DAG compile、engine(逐次/並行 job、artifact、bindings)、resource inference | `cheaper-workflow` |

移行対象外(そのまま維持):
- `packages/apm/.apm/hooks/cheaper` / `cheaper.ps1`(launcher)。asset 名が変わらなければ無改変。
- `workers/*.md`、`workers/schemas/*.json`、`policy.yaml`、`skills/*.md` — 全てデータであり Rust 側で同一フォーマットを読む。
- `usage.jsonl` / `pricing.json` / `auth.json` 等のユーザーデータフォーマット — 後方互換で読み書きする(既存ユーザーの計測履歴・credential を壊さない)。
- APM パッケージ構造(`apm.yml`、`.apm/` primitives)。`cli/bin/cheaper.mjs` だけが消える。

## 2. Goose GDK へのマッピング

### 2.1 agent loop と実行制御(`run.ts` → StateMachine + Operation)

| 現行機構 | Pi 実装 | GDK 移行先 |
|---|---|---|
| worker 実行 1 回 | `createAgentSession` + `session.prompt()` | `goose-agent` の `StateMachine` に Operation ordered list を組んで `run()`。構成は `[Inference, ToolOperation, (必要に応じ独自 Operation)]`。会話は in-memory で十分(現行も `SessionManager.inMemory()`)なので `MachineSession`/`SessionLoader` trait を no-persist 実装で埋める |
| sandboxed tools 登録 | `customTools`(ToolDefinition 配列) + tools allowlist | `ToolOperation` に `SyncTool`/`AsyncTool`(rmcp tool trait)実装を登録。`tool-runtime` 側の canonical executor を薄く包む形で現行 `toPiTool` adapter 相当。動的供給が要るなら `ToolProvider` だが現行は静的 allowlist のみなので SyncTool 直接登録で足りる(YAGNI) |
| usage/turn/retry 集計 | `session.subscribe` で `message_end`/`turn_end`/`auto_retry_start` を UsageCollector が加算 | `Emitter` が発行する `AgentEvent::{Message,Usage,MessageUsage}` を collector に流す。**差異**: Pi の `costUsd`・`cacheRead/Write` 粒度と Goose の usage イベント粒度の対応は PoC で確認。`auto_retry_start` 相当イベントが無い可能性があり、その場合 `meta.transientRetries` は provider retry を自作するか field を再定義する必要あり(§5 リスク) |
| maxTurns | `turn_end` カウントで `requestAbort("maxTurns")` | `assistant_turn_count` predicate or Emitter 経由の独自カウント → `CancellationToken::cancel()` |
| maxToolCalls | tool budget wrapper が閾値で `onLimit` → abort | 同一方針: tool 呼び出しを包む側(独自 budget)で count し閾値で cancel。tool 実装は自前なので同等機構を自然に維持できる |
| timeout / 外部 SIGINT | `setTimeout` + `AbortSignal` → runAbort 集約、`abortGraceMs` で settle 待ち | tokio `timeout` + tokio-util `CancellationToken`。grace 期間は `tokio::time::timeout` で `run()` 完了を待つ形に移植(`promptWithAbortGrace` 相当)。abort reason の分類(TIMEOUT/ABORTED/MAX_TURNS/MAX_TOOL_CALLS → PARTIAL_MUTATION 包装)は GDK に無い概念なので `cheaper-agent` がそのまま所有する |
| PARTIAL_MUTATION / meta snapshot 付き error | `WorkerRunError.runMeta` | Rust error type(`cheaper_core::Error` + `run_meta` field)に移植。envelope は serde で現行 schemaVersion:1 を厳密に再現 |
| maxTokens 注入 | `streamFunction` wrap で `min(policy, model.maxTokens)` | Provider 呼び出し request の max_tokens を自前制御(Inference 前段 or Provider wrapper)。Goose の ModelConfig に相当する経路を PoC で確認 |
| transient retry | `SettingsManager.retry` | GDK provider/agent の retry 機構の有無を PoC で確認。無ければ Inference を包む独自 Operation で retry 分類 `transientRetries` を維持 |

### 2.2 structured output + repair(`output.ts` + `repair.ts`)— 自作 Operation が必要な領域

現行要件(実コードから抽出):

1. 最終 assistant message の text 連結 → `stripCodeFence`(``` 1 組除去)→ `JSON.parse` → プレーンオブジェクト必須 → Ajv compiled schema で validate。失敗理由は `no-assistant-message`/`invalid-json`/`schema-mismatch` に分類し `rawOutput` + `errors[{instancePath,message}]` を repair に渡す。
2. repair は **tool 無しの新規 session** を attempt 毎に生成し、`response_format: {type:"json_schema", schema}` を model に付与、専用 system prompt("JSON repair worker")+ 前回 raw output + 検証 error 一覧を user prompt で投入。成功するまで `policy.output.repairAttempts` 回。尽きれば `INVALID_OUTPUT`(errors 先頭 20 件・rawOutput 先頭 2000 文字)。usage は main session と同一 collector に合算、attempt 消費数は `meta.repairs`。abort/timeout は run 全体 signal に従う。
3. system prompt 側の output contract(JSON 単独出力・フェンス禁止・workspace-relative POSIX path)は `buildSystemPrompt` が埋め込む — Rust 側で同一文字列を生成する(worker prompt 契約の温存)。

GDK 移行案(推奨: 現行構造の直移植):

- repair session = **tools 無しの 2 本目 StateMachine**(Operation = Inference のみ)を attempt 毎に立てる。現行「新規 session で会話を引き継がない」設計と構造的に一致し、goose の `final_output` tool 方式(recipe 層、フル crate)に寄せる必要がない。
- `response_format` 付与は GDK の provider request で `response_format`/structured output hint を外部から設定できるかが鍵(Gap)。設定面が無ければ次善として (a) provider wrapper で request JSON を書き換える、(b) `final_output` 相当の SyncTool(入力 = JSON、validate して NG なら error を tool result として返す — Goose recipe と同型)を main loop に追加する自作 `RepairOperation`、の順に検討。(b) は「同じ conversation 内で agent に修正させる」方式で、現行「別 session で修復」方式と usage/turn 計上が変わる点に注意。
- JSON Schema validate は `jsonschema` crate(Ajv 相当、draft 2020-12 対応)で `CompiledSchema` を移植。TypeBox 由来の生成 schema は plain JSON Schema なので互換性上の問題は小さい見込み(PoC で既存 schema 一式を validate して確認)。

### 2.3 provider 層(`model-runtime.ts` + `providers/devin/`)

| 現行 | GDK 移行先 |
|---|---|
| Pi `ModelRuntime`(provider registry、catalog `getAvailable`/`getModel`、auth.json 永続化、refresh) | `goose-providers` の provider factory/registry。**差異**: cheaper の tier 分類は model の `cost{input,output,cacheRead,cacheWrite}`/`maxTokens` を必要とし、Pi は catalog に cost を持つ。Goose 側の canonical model registry(final-report S3)に cost metadata が含まれるか、なければ cheaper 独自 catalog(pricing snapshot + declarative provider の model list)で `ModelInfo` を維持する — **要 PoC 確認項目** |
| native provider: anthropic/openai/openai_compatible/ollama 等 | `goose-providers` native でそのまま。Ollama(E2E で使用中、`CHEAPER_E2E_OLLAMA_MODEL`)も native 対応済み |
| `CheaperProviderPlugin`(bundled = devin のみ) | `goose-provider-types` の **Provider trait を自前実装**。declarative JSON では Connect wire プロトコル(§2.4)を表現できないため declarative 経路は不適。plugin 登録面(registerProvider + oauth login/refresh/getApiKey + refreshModels + streamSimple)は Provider trait + credential 解決層に再配置 |
| auth(`cheaper auth login/logout/status`、Pi OAuth flows、auth.json) | **最大の未確定領域**。Pi は provider 別 OAuth flow(device/code)と credential 永続化を持つ。GDK provider 層の credential/auth lifecycle が同等の login 対話を露出するか未確認。露出しない場合は `cheaper auth` を独自実装(OAuth flow + `<userConfigDir>/` 配下の credential store 自前)とし、Goose 側は既存 credential を注入する形にする — 現行 devin plugin が既に「CLI 委譲 + 独自 credentials.toml」方式なので、この auth 自前化は devin については設計整合的 |
| `resolvePiModel`(tier 候補選択 → catalog.getModel で実体解決) | core の `resolveModel`/`resolveExplicitModel` を Rust 移植し、Goose 側の model handle に解決する adapter。`ModelCatalog` seam はそのまま `trait` 化して test stub を維持 |
| `PI_OFFLINE` / refresh timeout / `<userConfigDir>/pi/` state | Goose provider registry に refresh 経路があれば wrap、無ければ catalog を cheaper 側で管理。Pi state パス(`<dir>/pi/auth.json` 等)は `auth.json` 移行互換を決める必要あり(§7 未解決) |

### 2.4 devin provider 移植の詳細

現行 `providers/devin/` は「Devin CLI 子プロセス委譲 login + `server.codeium.com` への vendored Connect(protobuf-like wire + gzip + JWT)stream」で ~1.4k 行。

- `wire.ts`(118 行): 手書き varint/fixed64/string エンコード + Connect frame。Rust でも同じく手書きで十分(`prost` 依存を引かず bytes 操作のみ)。ただし gzip は `flate2`、HTTP/2 前提の Connect streaming は `reqwest`(stream feature)で代替可能か、HTTP/1.1 chunked 経路かを PoC で検証要。
- `stream.ts`(656 行): context → ChatHistoryItem 写像、SSE/frame パース、`CloudChatEvent` → assistant message event 変換、usage 抽出。Provider trait の `stream`/`complete` に対応させて移植。
- `cli.ts`/`credentials.ts`/`jwt.ts`/`models.ts`(~570 行): `devin` CLI spawn(login/token/catalog)、credentials.toml 読み、JWT decode/cache、`devin models list` → model config。`std::process::Command` + `serde_json`/`toml` + JWT(jsonwebtoken か手書き base64url+JSON decode — 署名検証不要なら自前 decode で軽量)で移植。
- `context-map.ts`(102 行): 会話→wire メッセージ写像。

### 2.5 依存クレート選別 — v8 等の重量依存を引かない構成

採用( crates.io pin ):
- `goose-agent`(StateMachine/Operation/Emitter/CancellationToken) — agent loop のみ
- `goose-providers`(native + declarative provider)
- `goose-provider-types`(Provider trait・会話型。devin 実装の依存先)

不採用:
- `goose`(core): crates.io 未公開 + 名前衝突(Tag1 の負荷試験)+ v8-goose/lancedb を引く。recipe/final_output/ExtensionManager/session DB は使わない判断(§2.2 で自作)。
- `goose-sdk`(UniFFI): provider API のみで agent loop を含まず、Rust 内部利用では不要。
- `goose-mcp` / MCP 経路: ツールは全て in-process SyncTool のため不要(goose-agent が依存として引く分は構わないが、cheaper が直接依存はしない)。
- `goose-server`/goosed: REST API は廃止方向(S14)。
- `goose-context-management`: 現行 compaction 無効(`compaction.enabled=false`)。不要。
- `goose-local-inference`/`goose-download-manager`: 現行に相当機能なし(YAGNI)。

cheaper 側で追加する一般クレート(想定):
- `tokio`(async runtime。goose-agent が tokio 前提のため追従)、`serde`/`serde_json`、`serde_yaml`(policy/worker/workflow YAML)、`jsonschema`(Ajv 代替)、`tiktoken-rs`(o200k_base — `js-tiktoken` との parity を golden test で検証。tokenizer version 表記 `TOKENIZER_IMPLEMENTATION` は実装差異として semver 扱い要検討: usage.jsonl の tokenizer identity が変わる点を §7 で確認)、`similar`(diff — edit tool の差分)、`clap`(argv)、`dirs`(userConfigDir)、`reqwest`/`flate2`(devin Connect)、`thiserror`(error)、`toml`(credentials.toml)、`regex` 相当(grep tool は自前実装のため engine 選定要)。
- `anstream`/`dialoguer` 等は auth 対話の範囲だけに限定して導入可否を判断(secret raw-mode 入力は portable-pty 不要で termios/crossterm 級)。

### 2.6 ツール/ポリシー移植対応

- sandboxed tools(read/write/edit/ls/grep/find/count_tokens/process)= 全て pure Rust で書き直せるロジック(workspace boundary、symlink policy、denylist、write scope、行 framer、artifact 参照、process 管理)。Goose 側の builtin tool は一切使わない(現行 `noTools:"builtin"` と同じ立場)。
- `process` tool(managed spawn + process-tree kill + 出力 cap + resource 宣言)は `tokio::process` + Windows の Job Object / Unix killpg 相当で移植。**process-tree の OS 差分実装は現行 Node 実装の最も platform 固有な部分で、Rust 化でむしろ明示的になる**。
- resource manifest / access 事前認可 / lease / accounting / execution metrics — 全て cheaper 独自概念で GDK に対応物なし。そのまま `cheaper-tools` に移植。

## 3. 段階的移行計画

### Phase 0: 実測 PoC(スパイク、go/no-go 判定)

目的: 未実測の2点(サイズ・API 表面)を潰す。成果物は破棄前提の最小バイナリ。

作業:
1. `goose-agent` + `goose-providers` + `goose-provider-types` を `=0.1.0-alpha.<N>` で pin した最小 bin を作成。
2. Ollama(docker compose、既存 E2E と同じ `qwen3.5` 系)を provider に、1 本の SyncTool(例: workspace `read` 簡易版)を登録し、複数 turn の run を実行。
3. Emitter で Usage/MessageUsage を受けて token 内訳(input/output/cache)を取得できるか確認。
4. `CancellationToken` で abort し、in-flight inference/tool の打ち切りと reason 分類可能性を確認。
5. structured output: provider request に `response_format` 相当を渡せるか、できなければ tool ベースの repair(§2.2)が StateMachine の Operation として組めるか確認。
6. `cargo build --release`(profile: `lto="fat"`, `codegen-units=1`, `opt-level="z"`, `strip=true`, `panic="abort"` 検討)で **host + 少なくとも linux-x64 の静的バイナリサイズ実測**。cargo-bloat/cargo-llvm-lines で依存内訳も記録。
7. devin Connect wire の HTTP 経路(reqwest で streaming が張れるか)を最小確認(実 endpoint 不可なら wire フォーマットの roundtrip テストのみでも可)。

完了条件(go/no-go):
- release binary が閾値以下(目安: **strip 後 ≤25MB**、圧縮 ≤10MB。現行 SEA 113MB に対し 75%+ 削減を狙うライン。閾値は ADR で確定)。
- Emitter usage に cache 内訳を含む token 情報がある、または conversation から同等を算出可能。
- structured output の実現経路(a or b)が確認できた。
- goose-agent の Operation 割り込み(maxTurns/maxToolCalls 用)が API 上可能。

失敗時の fallback: comparison.md の第一候補 Rig へピボット(評価済み)。

### Phase 1: `cheaper-core` 移植

- error code 表・exitCode・retryable、worker/policy/schema 読込、workspace path、token 計測、pricing/model resolve/tier 分類、envelope 型。
- 完了条件: 既存 TS テストの fixture(worker .md、policy.yaml、schema、golden 期待値)を Rust 側 cargo test に移植し、core 関数の入出力が一致(token count は既知 fixture で js-tiktoken vs tiktoken-rs の一致を確認)。
- 比較方法: TS 側に「fixture → JSON」dump ツールを追加し、Rust 側 golden と diff(表現ドリフトを避けるため schemaVersion 付き fixture を共用)。

### Phase 2: `cheaper-tools` 移植

- 全 canonical tool + journal/budget/meter/scheduler/access/invocation。
- 完了条件: fixture workspace 上で TS 実装と Rust 実装の tool 結果(content + details + journal/deny 判定)が golden 一致。process tool は timeout/kill/tree 挙動を OS 別テストで確認。

### Phase 3: `cheaper-agent`(runtime 置換)

- StateMachine 組立、tool 登録、usage collector、abort 分類、repair、meta/envelope。
- test seam: Pi の `streamFn`/testsupport 相当として scripted `Provider` trait 実装(fake provider)を用意 — **これは重要な移植論点**。現行 E2E は fake stream で決定的検証しているため、Goose 移行後も deterministic な契約テスト経路が必要。
- 完了条件: TS/Rust 双方で同一シナリオ(fake provider への scripted 応答列を共用 fixture 化)を流し、envelope の全 field(durationMs と token 実測値を除く)一致 + error code/retryable/PARTIAL_MUTATION 分類一致。さらに Ollama 実 provider で smoke(現行 `real-provider-ollama.test.ts` 相当)を Rust 側でも通す。

### Phase 4: `cheaper-cli` + `cheaper-workflow` + devin provider

- argv 互換(`--input/--worker-file/--cwd/--timeout/--model/--provider/--tier/--worker/--method/--type/--since/--compare-model/--format/--quiet`)、auth 対話、usage.jsonl/report、hook targets(copilot/cursor 等の fail-open contract)、workflow run/interface、devin Provider trait。
- 完了条件: utility 系(count-tokens/usage/models 出力)の envelope が byte 互換。`cheaper hook` の target 別出力/exit 0 fail-open を target fixture で一致。devin は実環境依存のため unit test(wire roundtrip / CLI parse / credentials.toml / JWT cache)+ 手動 smoke。

### Phase 5: 配布切替

- CI matrix build(linux-x64 / linux-arm64 / windows-x64 — 現行 releaseTargets と同集合)で `cheaper-<target>` asset 生成 → SHA256SUMS → `release-artifacts.mjs` / `apm-bundle.mjs` は asset 差し替えのみで再利用可能(esbuild/SEA 経路を cargo artifact copy に置換)。
- `pnpm bundle` 相当で `cli/bin/cheaper.mjs` を廃止し `version.json` + `workers/` + `policy.yaml` を同じ layout で出力。
- mise.toml の toolchain に Rust(stable)追加、`mise run setup/check` 更新、version canonical source は `packages/cli/package.json` → `Cargo.toml`(or version.json)へ移行、`check-version.mjs` 更新。
- 完了条件: 固定 version Release からの launcher 取得(sh/ps1 無改変)が Rust binary で通る、offline preseed pair 手順が同一で動く。

### TS/Rust 並走の方針

- 並走期間は「同一 envelope schemaVersion:1」が唯一の互換境界。`CHEAPER_RUNTIME=ts|rust` の実行時切替は持たず(YAGNI)、リポジトリ内に `packages/`(TS)と `crates/`(Rust)を同居させ、golden fixture を共有して Phase 3 完了=parity 確立をもって TS runtime を削除する。
- 削除単位: `runtime-pi` → `tool-runtime` → `core/workflow` → `cli`。pi-ai/pi-coding-agent 依存は `runtime-pi` 削除時に lockfile から消える。

## 4. 配布変更点(SEA → Rust release binary)

| 項目 | 現行 | 移行後 | 変更 |
|---|---|---|---|
| asset | Node SEA `cheaper-<target>[.exe]` (~113MB) | `cargo build --release` バイナリ | builder のみ変更、asset 名不変 |
| `cli/bin/cheaper.mjs` | esbuild 単一ファイル(Node fallback) | 廃止 | launcher の sibling 優先 + Release 取得に一本化 |
| SHA256SUMS 検証 | `release-artifacts.mjs` の parseManifest/verifyAssets | 同一ロジック維持(manifest 形式・asset 名不変なら無改変) | なし |
| launcher(sh/ps1) | `cli/bin/<platform>/cheaper` 優先→固定 Release 取得 | 同一 | **無改変が目標** |
| builtinDir 解決 | SEA VFS root(`import.meta.dirname`) | 実行ファイル隣接 or embedded(include_str!/include_dir!)で `workers/`、`policy.yaml`、`version.json` を内包するか、launcher が置く sibling 構造を維持 | APM bundle は外部ファイル同梱のまま維持可能(現行 apm-bundle が既に外部配置) |
| cross build | postject/SEA per target | `cargo build --target`(+ GH runner matrix) | linux-arm64 は cross or arm runner、windows-x64 は windows runner |
| dev 起動 | `pnpm dev`/node | `cargo run`/`cargo test` | mise.toml に rust toolchain |

## 5. リスクと緩和策

| リスク | 影響 | 緩和 |
|---|---|---|
| 全 GDK crate が `0.1.0-alpha.x` で breaking change 頻発 | upgrade 都度のコンパイル追従 | `=` 完全一致 pin + upgrade は専用 PR で `mise run check` 相当 + golden parity suite を通してのみ実施。週次リリースに追わず四半期単位の pin 更新を既定化 |
| フル `goose` crate 未公開・crates.io 名前衝突 | recipe/final_output/session DB が使えない | 本計画は最初から GDK のみで構成し final_output/recipe は自作(§2.2)。`cargo add goose` 等の誤追加を防ぐため denied crate list を CI lint に追加 |
| goosed REST API 廃止方向(#7697) | subprocess/REST 統合経路の将来性なし | 依存しない(in-process ライブラリ利用のみ) |
| バイナリサイズ未実測 | 移行目的(サイズ削減)の根拠が未確立 | Phase 0 の完了条件をサイズ閾値に固定。閾値未達なら即 Rig ピボット |
| structured output は自作範囲 | repair 等価性の実装コスト | 現行は「別 session + json_schema」方式で Goose の final_output と独立しており、直移植可能(§2.2)。`response_format` 非対応時の fallback も定義済み |
| usage 粒度差異(cacheRead/Write、costUsd、transientRetries) | `meta.usage` / usage.jsonl schema の非互換 | Emitter usage の実測確認(PoC)。欠落 field は envelope 上 null/0 を許容するか schemaVersion bump を検討。usage.jsonl 旧 record 読み互換は維持 |
| auth(OAuth login/logout/status)の GDK 露出不明 | `cheaper auth` の再実装幅が読めない | PoC で provider credential 注入経路を確認。最悪ケースでも独自 credential store + OAuth flow 実装で代替可能(現行 devin が既に同構造)。openai-codex 等の Pi OAuth flow 一覧の棚卸しが前提 |
| devin Connect プロトコルの Rust 移植 | wire/gzip/stream の正確性 | wire.ts は手書き framing で小さい。roundtrip golden test で TS/Rust byte 一致を確認し HTTP 層のみ差し替え |
| tokenizer parity(js-tiktoken → tiktoken-rs) | count-tokens / meter / savings 値の差分 | o200k_base golden(多種 fixture の token 数一致)を Phase 1 完了条件に含める。identity 表記(`implementation`)の変更は report 側で吸収 |
| `hook` fail-open 契約の regress | consumer 側で tool call ブロック事故 | hook は target fixture の byte 比較テストを Phase 4 完了条件に |
| Windows/Japanese path・process-tree kill 等の OS 差分 | tool-runtime 移植バグ | Phase 2 で OS 別 CI(linux+windows)必須化 |

## 6. 見積もりと判断基準

相対見積(1 名換算の粗いオーダー):

| フェーズ | 規模 | 備考 |
|---|---|---|
| Phase 0 PoC | S(2–4 日) | サイズ実測 + API 表面確認のみ |
| Phase 1 core | M(1–2 週) | ほぼ機械的移植 + tokenizer parity |
| Phase 2 tools | L(2–3 週) | 最大のドメイン複雑さ(process/scheduler/access/journal)。golden fixture 整備含む |
| Phase 3 agent runtime | M(1–2 週) | GDK 組立自体は小さい。repair/abort/fake provider seam が本質 |
| Phase 4 cli + workflow + devin | L(2–3 週) | devin wire 移植と auth 対話が重い。workflow は DAG 移植 |
| Phase 5 配布 | S–M(〜1 週) | asset 名互換のため主に CI matrix |
| 合計 | ~7–11 週 | Pi SDK 差異ではなく rewrite が支配(comparison.md と同見解) |

go/no-go 条件(Phase 0 出口で判定):

1. GDK 構成の release binary が **strip 後 ≤25MB**(圧縮 ≤10MB 目安)。
2. Emitter 経由で run 単位の usage 合算(input/output/total、cache 内訳は尚可)が取れる。
3. structured output + repair の実現経路(response_format or tool-repair Operation)が確認でき、自作量が ~300 行級に収まる見込み。
4. tool 割り込み(maxToolCalls)→cancel、turn 上限→cancel が StateMachine API で成立。
5. Provider trait の必要メソッド面に devin(Connect stream + CLI 委譲 credential)が収まる。

1–4 のいずれか未達なら GDK 採用を見送り Rig へピボット(comparison.md 既評価)。5 のみ未達なら GDK 採用は継続し、devin provider を declarative/別プロセス化する縮小案か devin 切り離しで判断。

## 7. 未解決の前提確認事項

1. `goose-agent` の `Inference` Operation が外部から `max_tokens`/`response_format`/tool 選択を制御できる API 面(docs.rs 精読 + PoC)。
2. Emitter `Usage`/`MessageUsage` イベントの field 粒度(cache 内訳・cost の有無)と、`transientRetries` 相当の retry 可観測性。
3. `goose-providers` の model catalog に cheaper tier 分類が要する cost/maxTokens metadata があるか。無い場合の canonical registry か declarative JSON への model 属性埋め込み可否。
4. GDK provider の credential/auth lifecycle(login 対話・永続化パス・logout)の API 露出。`cheaper auth` の再実装幅。
5. Provider trait 実装時に Session/conversation 型(`goose-provider-types`)が安定しているか(alpha 内での breaking 頻度)。
6. devin Connect endpoint が reqwest の HTTP バージョン/圧縮組合せで張れるか(実 endpoint 検証)。
7. `tiktoken-rs` の o200k_base が js-tiktoken と全 fixture で一致するか。
8. Pi state(`<userConfigDir>/pi/auth.json` 等)の資産を Rust 側 credential store へ移行するか、フォーマット維持で読み替えるか。
9. usage.jsonl/meta の tokenizer `implementation` 表記変更を report/consumer 側が吸収できるか(schemaVersion の扱い)。
10. macOS asset: 現行 releaseTargets は linux-x64/linux-arm64/win32-x64 のみ。Rust 移行を機に darwin を追加するかは別判断(本計画のスコープ外)。
