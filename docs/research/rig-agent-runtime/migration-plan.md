# Rig (0xPlaygrounds/rig) 移行計画

策定日: 2026-10
対象: `packages/runtime-pi`（`@earendil-works/pi-ai` / `pi-coding-agent` 0.85.1 依存）+ TypeScript/Node.js SEA 配布の全面 Rust 化。
前提調査: `docs/research/rig-agent-runtime/final-report.md`、`docs/research/pi-runtime-replacement/comparison.md`。

## 0. 目的と非目的

- **目的**: Node SEA 実行ファイル（現行 `cheaper-windows-x64.exe` ≈113MB）を Rust ネイティブバイナリ（推定 5-15MB 級、実測未了）へ置き換える。外部 contract（envelope・usage.jsonl・hook・config file・worker/policy/models.yaml 書式）は**不変**とする。
- **非目的**: 機能追加、配布 contract（GitHub Release + SHA256SUMS + launcher acquisition）の再設計、APM package 構造の変更。これらは既存 contract を維持する。

## 1. スコープ整理

現行 src 合計 ≈16.4k 行 TypeScript（test は別途 ≈113 file / ≈1.1k case）。Rust 化対象と残す部分を以下に整理する。

### 1.1 Rust 化が必要なモジュール

| 現行モジュール | 規模 | 責務要約 |
|---|---|---|
| `packages/core/src` | ≈2.7k 行 | Pi/Node 非依存の共有ロジック。worker `.md` frontmatter+schema 解決・検証（`worker/`）、policy.yaml schema 検証・merge・capability・restrict（`policy/`）、models.yaml・tier 分類・resolveModel・pricing snapshot・auto 分類（`model/`）、JSON Schema 2020-12 compile/validate（Ajv。worker schema・policy schema 共用）、o200k_base canonical token 計測（js-tiktoken、`token/`）、workspace path canonicalize・symlink policy・denylist（`workspace/`）、CheaperError code→exitCode/retryable 表（`errors.ts`）、duration パース、resource manifest 型・正規化（`resources/`）、tool 名 vocabulary。 |
| `packages/tool-runtime/src` | ≈4.4k 行 | canonical tool 層。`read`/`grep`/`find`/`ls`/`write`/`edit`/`count_tokens`/`process` の 8 tool 実装 + catalog（input/output JSON schema、resource declaration）+ path boundary・denylist・write-scope・resource claim（`access`/`resources`）+ mutation journal（`journal`）+ tool call budget・token meter（`limits`/`accounting`/`meter` 相当）+ 並行 scheduler（`scheduler`/`operation`/`registry`）+ invocation/lease scope（`invocation`）+ artifact store（`artifact`）+ `process` tool（minimal env・output cap・process-tree kill、`process*.ts`）+ line-framer・walker。 |
| `packages/runtime-pi/src`（Pi 依存部を除く） | ≈2.0k 行 | run オーケストレーション（`run.ts`：capability 適用→model 解決→tool scope→session→prompt→output 抽出→repair→meta 構築、timeout/signal/maxTurns/maxToolCalls の abort 分類、PARTIAL_MUTATION 包装）、`session.ts`（session 構築・maxTokens 注入・builtin tool 抑制）、`usage.ts`（UsageCollector）、`output.ts`（最終 assistant 抽出・code fence 除去・schema 検証）、`repair.ts`（tool 無し repair session + `response_format: json_schema` + Ajv error feedback）、`prompt-grace.ts`（abort 後 settle 猶予）、`auth.ts`（provider auth metadata 投影）、`provider-plugin.ts`（bundled plugin contract）、`model-runtime.ts`（Pi state 配置 + ModelInfo 変換 + tier/explicit 解決 adapter）。 |
| `packages/runtime-pi/src/providers/devin` | ≈1.5k 行 | bundled `devin` provider。Devin CLI 子プロセス委譲 login/catalog（`cli.ts`/`credentials.ts`/`models.ts`）、隔離 data dir（`<userConfigDir>/providers/devin/data`）、vendored Connect-protocol streaming transport（`stream.ts` 656 行、`wire.ts` protobuf-lite framing、`jwt.ts`/`metadata.ts`/`context-map.ts`）。 |
| `packages/workflow/src` | ≈2.1k 行 | workflow YAML schema・compile・DAG engine（job 並行実行・lease・executor port）・artifact store・bindings・resource inference・interface 投影。 |
| `packages/cli/src` | ≈3.6k 行 | 引数パース（`args.ts`、コマンド 11 種）、envelope 生成・stdout 契約（`envelope.ts`）、command dispatch・recordUsage・SIGINT 中継（`cli.ts`）、`auth` command + terminal interaction（`auth.ts`）、`count-tokens`（`count.ts`）、`hook <event> --target`（`hook/`: fail-open dispatch + 4 target adapter + 共有 routing policy）、`usage.jsonl` 記録（`usage-log.ts`）、`cheaper usage` 集計（`usage-report.ts`）、`pricing.json` 読込（`pricing.ts`）、stdin input 読取（`input.ts`）、`--format text`（`text-format.ts`）、version 解決（`version.ts`）、workflow command（`workflow-*.ts`）、`CHEAPER_TEST_FAKE_STREAM` seam、`main.ts`/`sea-main.ts` 相当のエントリ。 |

### 1.2 Rust 化不要・残す部分

| 資産 | 理由 |
|---|---|
| `packages/apm/.apm/hooks/cheaper` / `cheaper.ps1`（launcher） | shell/PowerShell 製の Release acquisition + cache + checksum 検証。対象 binary が SEA→Rust に変わっても contract（asset 名・SHA256SUMS・`--version` probe）は不変。**変更なし**。 |
| `packages/apm/.apm/hooks/hooks.json` + `.apm/skills/` | agent 側 hook 登録と skill 文書。`cheaper` CLI のコマンド contract が不変なら無変更。 |
| `cli/workers/*.md` + `schemas/*.json` + `policy.yaml` | built-in resource。Rust 版では `include_dir!`/`include_str!` 等でバイナリ埋込に変換（SEA VFS 相当）。内容は不変。 |
| Ollama 検証コンテナ（`docker-compose.yml`） | real-provider smoke の検証基盤。Rust e2e からもそのまま使う。 |
| usage.jsonl 既存 record | 読み取り後方互換必須（旧 schema record は `savings`/`turns`/`toolCalls` 欠落を許容）。 |

### 1.3 廃止・置き換えとなる部分

- `@earendil-works/pi-ai` / `pi-coding-agent` 依存一式（agent session・ModelRuntime・builtin tool・OAuth flow 実装）。
- esbuild bundle（`cheaper.mjs`、12.9MB）+ Node SEA ビルド（`scripts/build-executable.mjs`）。Rust toolchain（cargo）へ置換。
- `<userConfigDir>/pi/` 配下の Pi 永続 file 3 点（`auth.json`/`models.json`/`models-store.json`）→ cheaper 自前 credential store + model catalog に置換（移行ポリシーは §7 の未解決事項）。

## 2. Rig へのマッピング

### 2.1 機能マッピング表

| 現行機能 | 現行実装 | Rig 側の受け皿 | ギャップ判定 |
|---|---|---|---|
| マルチターン agent loop | `session.prompt()` + `turn_end` 監視、`maxTurns`/`maxToolCalls` abort | `rig-agent` `AgentBuilder` + `.tool()` 登録 + `default_max_turns`。turn 計測は hook（`ModelTurnFinished`/`ToolCallDelta`）+ `PromptResponse.completion_calls` | ほぼ直写。**注意**: `maxToolCalls` は cheaper 側 budget 強制を維持（tool 層に既存）。`maxTurns` の abort 意味（toolUse 継続中のみ abort）は hook で再現 |
| sandboxed tools 8 種 | tool-runtime canonical tool + Pi `ToolDefinition` adapter | `Tool`/`PortableTool` trait or `#[rig_tool]`。`ToolContext` で invocation scope 注入可 | tool 実装本体は全量自前 port。adapter 部分のみ Rig trait 実装に置換 |
| provider 解決・transport | Pi ModelRuntime + `streamSimple` | rig-core provider 実装（anthropic/openai/openai-compat dialect/ollama 他 26 種）、custom は `Wire`+`Decoder`+`Transport`（0.43 設計） | 標準 provider は写る。**devin** は custom provider 自作（§2.2-G4） |
| model catalog / availability | Pi `models.json`/`models-store.json`（models.dev 由来）+ `getAvailable` | **なし**。Rig は model 名定数のみで catalog/price/auth 管理を持たない | **ギャップ G1**: 自前 catalog store（models.dev client or bundled snapshot + user `models.json` 互換読込）を新設計 |
| credential / OAuth login | Pi `auth.json` + provider 別 OAuth flow（openai-codex, anthropic, github-copilot, openrouter, kimi-coding, radius）+ api_key | **なし**。Rig は env/api key 指定のみ | **ギャップ G2**: credential store + OAuth flow（browser auth_url + device code）+ api_key prompt を自前実装。scope 縮小判断が必要（§5-R3） |
| usage 収集 | `UsageCollector`（message_end usage 加算、turn_end/auto_retry_start 計数） | `PromptResponse.usage`（run 集計）+ `completion_calls[].usage`（per-call、cache token 含む `Option<u64>` 群） | usage は Rig 側で充足。**transientRetries** は Pi の auto-retry イベント由来で Rig に対応物なし → 自前 retry 層（下記）で計数 |
| transient retry | Pi SettingsManager `retry`（`transientAttempts`） | 未確認（provider 内蔵 retry は API に面していない） | **ギャップ G3**: 429/5xx/network を分類する薄い retry wrapper を cheaper 側に実装（回数を `meta.transientRetries` に計上） |
| structured output repair | tool 無し repair session + `response_format: json_schema` + Ajv error 反馈、`repairAttempts` | `output_schema(_raw)`/`output_mode` + `ExtractorBuilder::retries`、stream 中 invalid tool-call repair（`StreamedResolution::Repaired`） | main loop は `output_schema` 不使用（tool call 抑制を避ける現行設計を維持）→ **repair session は「tool 無し completion + json_schema mode + schema error 反馈 retry」を cheaper 側に再実装**。Extractor ではなく自前 loop が現行意味に近い |
| abort / timeout / grace | runAbort AbortController + `promptWithAbortGrace` + PARTIAL_MUTATION | streaming `cancel()`/drop、`PromptError::prompt_cancelled` | tokio `CancellationToken` + `tokio::time::timeout` で run 全体を包み、reason 分類（timeout/signal/maxTurns/maxToolCalls）は cheaper 側で保持。grace settle は `timeout(grace)` で再現 |
| structured output schema 検証 | Ajv 2020-12（worker input/output・policy・tool schema） | —（Rig 範囲外） | Rust `jsonschema` crate（draft 2020-12 対応）で port。error は `instance_path`+message → `SchemaValidationError` に写像。**message 文言は Ajv と非互換**（golden test で揺れ吸収が必要、§3.3） |
| token 計測 | js-tiktoken o200k_base | — | `tiktoken-rs`（o200k_base 対応）へ置換。identity 文字列（`TOKENIZER_IMPLEMENTATION`/version）の表記更新は envelope `details.tokenizer` に影響 → 互換方針を決定（§7） |
| usage.jsonl | `usage-log.ts` append + `usage-report.ts` 集計 | — | serde_json で直 port。best-effort 記録・read-only 集計の責務は不変 |
| devin provider | bundled plugin（CLI 委譲 auth + Connect stream + live catalog） | custom `Wire`+`Decoder`+`Transport` or OpenAI-compat 扱い | **ギャップ G4**: Connect-protocol vendored transport（656 行）の Rust port + Rig custom provider 接着。最大の技術的不確定点 |
| workflow engine | `@cheaper/workflow` DAG engine + executor port | —（Rig 範囲外） | 純粋ロジック port（tokio task + join 群）。Rig 非依存 |

### 2.2 自前実装が必要なギャップ一覧

| ID | ギャップ | 内容 | 規模感 |
|---|---|---|---|
| G1 | model catalog store | `getAvailable` 相当：bundled 静的 catalog + models.dev fetch cache + user `models.json`（OpenAI-compat custom provider、`samplingParams` merge、`reasoning`/`maxTokens` metadata）を統合した catalog。tier resolver はこの catalog 上で動く | 中。protocol は models.dev JSON + Pi models.json 互換読取 |
| G2 | credential store + auth flows | `<userConfigDir>` 下の credential 永続化（auth.json 相当）、`auth status/login/logout`、provider 別 OAuth（browser + device code）、api_key login、env var 解決。secret は stdout に出さない制約 | **大**。OAuth flow は provider 毎に仕様確認・実装が必要 |
| G3 | transient retry 層 | provider error 分類（rate limit / 5xx / network）→ `transientAttempts` 回 retry、`meta.transientRetries` 計数 | 小〜中 |
| G4 | devin provider | Devin CLI 子プロセス（`auth login`/`auth status`/`auth logout`/`models list`、隔離 `XDG_DATA_HOME` env）+ Connect-protocol streaming（protobuf-lite framing + gzip + JWT cache + context map）の Rust port + Rig Wire/Decoder 接着 | **大**。wire compat の検証手段が限られる（実 Devin CLI + endpoint 依存） |
| G5 | `process` tool | minimal env 子プロセス・Windows env 大小文字・output byte cap・process-tree kill・task 定義 policy | 中。OS 差分の再検証コスト |
| G6 | resource/concurrency scheduler | resource manifest・shared/exclusive claim・serialized/per-resource mutation・lease の Rust 版 | 中。tool-runtime の中核複雑性 |
| G7 | test seam | `CHEAPER_TEST_FAKE_STREAM` 相当（scripted provider で deterministic e2e） | 小。fake `Model`/Wire 実装を dev-only で注入 |
| G8 | builtin resource 埋込 | workers/*.md・schemas・policy.yaml・version.json をバイナリ同梱（SEA VFS 代替） | 小（`include_dir` 等） |

### 2.3 Rig 使用面の設計指針

- **`rig-agent`（classic runtime）を第一選択**とし、PoC で hook 表現力（turn 数・tool call 計測・abort 制御）が不足と判明した場合のみ `rig-core` の Model/streaming のみ採用して自前 loop に退避する（VT Code 先例あり）。
- **Rig 依存を 1 crate に隔離**: `cheaper-rig-adapter`（仮）に provider/session/stream の接着を閉じ込め、core/tool-runtime/workflow/cli は Rig 型を直接参照しない。0.x breaking change の影響半径を限定する（現行 `createPiToolHandlers`→`toPiTool` と同じ adapter 思想の継続）。
- **version pin**: `rig = "=0.43.x"` 相当で固定。upgrade は MIGRATING.md 手順の四半期周期運用。

## 3. 段階的移行計画

前提: Rust 版は新 cargo workspace（例 `rust/` or `packages-rs/`）に構築し、並走期間中は TS 版も CI で完全に生かす。フェーズ番号は依存関係を表し、フェーズ内タスクは並行可。

### Phase 0: PoC / スパイク（go/no-go）

| 項目 | 内容 |
|---|---|
| 成果物 | `rig`（+ `rig-reqwest`）で Ollama(qwen3.5) と Anthropic or OpenAI を叩く最小 CLI。sandboxed tool 2-3 個（read/grep/write 相当の簡易版）を `Tool` trait で登録し multi-turn run → `PromptResponse.usage`/`completion_calls` を envelope 風 JSON で stdout。abort（cancel）・`output_schema` repair 動線・hook での turn 計測を検証 |
| 完了条件 | (a) release build（`lto+opt-level="s"`+strip、可能なら musl/static）の**バイナリサイズ実測 ≤ 20MB**（目標 5-15MB）、(b) agent loop が turn/tool call/usage/cancel を envelope 化できる粒度で露出、(c) repair session 相当（tool 無し + json_schema + retry）が組める、(d) devin 相当の custom `Wire`/`Decoder` がコンパイル・疎通できる設計見通し（仕組みの確認まで。実装完了は不要） |
| 検証方法 | `cargo build --release` 実測、Ollama コンテナでの実 run、 scripted fake provider での deterministic run |
| 判断 | 不達時は goose GDK の第二候補 PoC に切替（比較レポートの推奨次アクションと同構造） |

### Phase 1: core 移植（Rust `cheaper-core`）

| 項目 | 内容 |
|---|---|
| 成果物 | worker 解析（frontmatter YAML + schema ref 解決 + validate）、policy schema/normalize/merge/restrict/capability、models.yaml・tier 分類・resolveModel/resolveExplicitModel・pricing snapshot・auto 分類、duration、errors（code→exitCode/retryable 表をそのまま）、workspace path canonicalize・symlink・denylist、o200k_base token 計測（tiktoken-rs）、resource manifest 型、tool vocabulary |
| 主要 crate | `serde`/`serde_json`、`jsonschema`（2020-12）、`serde_yml` or `serde_norway`（worker/policy/models.yaml）、`tiktoken-rs` |
| 完了条件 | TS 版 core test 群と同等の case が Rust で再現・パス。worker/policy/model resolve の golden fixture（TS 側で dump した入力→期待出力）が一致 |
| 検証 | cargo test + TS との差分 fixture test（§3.3） |

### Phase 2: tool-runtime 移植（Rust `cheaper-tool-runtime`）

| 項目 | 内容 |
|---|---|
| 成果物 | canonical tool 8 種（read/grep/find/ls/write/edit/count_tokens/process）、boundary+denylist+write scope+resource claim、mutation journal、budget/meter、scheduler（shared/exclusive・serialized/per-resource）、invocation/lease、artifact store、line-framer/walker、`process` tool（minimal env・output cap・process-tree kill） |
| 完了条件 | tool 単体・境界・denylist・journal・budget の TS 同等 test がパス。`process` tool は Windows/Linux で spawn・kill・env 最小化が検証済み |
| 検証 | cargo test + ツール入出力 golden（同 input→同 `CanonicalToolResult` JSON） |

### Phase 3: runtime 移植（Rust `cheaper-runtime` + `cheaper-rig-adapter`）

| 項目 | 内容 |
|---|---|
| 成果物 | (a) model catalog store（bundled 静的 + models.dev fetch + user `models.json`）＝ G1、(b) credential store + auth command 裏側（api_key/env/OAuth flow）＝ G2、(c) transient retry 層＝G3、(d) runWorker 相当の run オーケストレーション（capability→model 解決→tool scope→Rig agent run→output 抽出→repair→meta 構築、abort 分類・PARTIAL_MUTATION・grace）、(e) devin provider＝G4、(f) `WorkerRunMeta` 完全互換（model/usage/turns/toolCalls/rejectedToolCalls/repairs/transientRetries/changes/execution/savings） |
| 完了条件 | fake stream seam（G7）で scripted 応答 → meta/envelope が TS 版と一致（durationMs 等の非決定値を除く）。Ollama real-provider で 3 built-in worker が成功 envelope を返す |
| 検証 | differential golden（§3.3）+ Ollama e2e。devin は実 Devin CLI 環境で auth/catalog/stream の smoke（CI 外の手動検証として切り分け） |

### Phase 4: CLI 移植（Rust `cheaper-cli` bin）

| 項目 | 内容 |
|---|---|
| 成果物 | 全 command（run 系 alias 4・count-tokens・models・usage・auth 3・workflow interface/run・hook・help/version）、envelope serializer（`schemaVersion:1`・null 意味論・末尾改行 1 つを厳密再現）、`--format text`、usage.jsonl 記録・集計、hook 4 target adapter + routing policy、SIGINT/SIGTERM 中継、stdin input、`CHEAPER_TEST_FAKE_STREAM` 相当 seam、builtin resource 埋込（G8）、workflow engine 移植（本フェーズに含めるか別フェーズ化は §7 の未解決事項） |
| 完了条件 | TS CLI の e2e/contract test 群を Rust binary 向けに再現して全パス。hook の target 別出力が byte 一致（fail-open 含む）。usage.jsonl schema 互換（旧 record 読込 + 新 record 同一 shape） |
| 検証 | **binary-to-binary differential**：同一入力・同一 fake provider に対し TS binary（現行 SEA or `node cheaper.mjs`）と Rust binary の stdout/exit code を比較（normalize 後、§3.3） |

### Phase 5: 配布切替

| 項目 | 内容 |
|---|---|
| 成果物 | cargo release matrix（win-x64/linux-x64/linux-arm64 必須、darwin は §7 未解決）、`scripts/build-executable.mjs` 相当を cargo build + artifact rename に置換、SHA256SUMS 生成・検証フロー維持（`scripts/lib/release-artifacts.mjs` の `releaseTargets`/`hashExecutable`/`generateManifest` は asset 名が同一なら無改修で流用可能）、APM bundle から `cheaper.mjs` を外す判断（§7）、`mise.toml` に Rust toolchain 追加・`mise run check` の検証経路更新 |
| 完了条件 | 現行 release workflow と同一の asset 名・SHA256SUMS・`--version` probe で Release が組める。launcher acquisition が Rust binary をそのまま取得・検証・exec できる（launcher 無改修が目標） |
| 検証 | `release-acquisition` workflow の smoke + local preseed + launcher e2e（既存 e2e の再利用） |

### Phase 6: TS 撤去・仕上げ

- `runtime-pi`/`cli`/`core`/`tool-runtime`/`workflow` の TS 実装と Pi 依存の撤去（または archive 化）、`pnpm` 系 task を cargo に再配置、`check-version.mjs` の canonical source 更新、docs（AGENTS.md・release guide・providers guide）刷新。

### 3.3 並走期間の挙動比較戦略（golden / differential）

1. **Normalize 規則を先に定義**: envelope 比較時に除外するフィールド = `meta.durationMs`、`usage.costUsd`（provider 計算差の可能性）、`details.tokenizer.version`（実装名が tiktoken-rs に変わるため）、stderr 診断の文言差、JSON キー順。**schema 検証エラーの `message` 文言も Ajv ≠ jsonschema で揺れる** — `instancePath` と error 件数・code のみ契約とし、message 本文は非契約と明文化する。
2. **fake provider を共通 script に**: TS 版 `CHEAPER_TEST_FAKE_STREAM` の応答配列 fixture を JSON 化し、Rust 側 seam（G7）が同じ fixture を ingest して同じ turn/tool call 列を再生 → `usage`/`turns`/`toolCalls`/`repairs`/`changes`/`savings`/`result` を厳密比較。
3. **fixture 収録**: 代表 worker（bulk-reader/code-writer/doc-writer）× 正常系/invalid output→repair/abort(timeout,maxTurns,maxToolCalls,SIGINT)/PARTIAL_MUTATION/error envelope の 20-40 case を golden として `test/fixtures/` に固定。TS 側でまず収録 → Rust 側で replay 一致。
4. **tool 層 golden**: canonical tool の input→`CanonicalToolResult`（content text + details）を fixture 化（deny/boundary/truncation/continuation を含む）。
5. **hook contract**: 4 target × 各 event × allow/deny/malformed stdin を byte 一致で固定（既存 hook-contract test の出力を流用）。
6. **rig-cassette**（record/replay）の採用は optional — fake provider fixture で足りるなら見送り（YAGNI）。

## 4. 配布変更

### 4.1 現行 → Rust 後の対応

| 層 | 現行 | Rust 後 |
|---|---|---|
| 実行ファイル | Node SEA（`node --build-sea`、VFS asset 埋込） | `cargo build --release`（+strip）+ builtin resource は `include_dir!` 埋込（SEA VFS 代替。`builtinDir` 概念は「埋込 resource root」に置換） |
| build matrix | `.github/workflows/build-executables.yml`（win/linux x64+arm64、`node scripts/build-executable.mjs`） | 同 matrix で `cargo build --release --target <triple>`。Windows は `x86_64-pc-windows-msvc`、Linux は musl static 検討 |
| artifact 名 | `cheaper-linux-x64`/`cheaper-linux-arm64`/`cheaper-windows-x64.exe` | **不変**（launcher/SHA256SUMS contract を維持するため） |
| SHA256SUMS | `scripts/lib/release-artifacts.mjs` の generate/verify | **そのまま再利用可**（asset 名不変なら） |
| launcher | shell/ps1 は binary を取得して exec + `--version` probe | **無改修**を目標（Rust binary が `--version` で canonical version を返せば互換） |
| APM bundle | `cli/bin/cheaper.mjs`（esbuild）+ workers/policy/version.json | `cheaper.mjs` は Node 実行が前提のため廃止 or 別手段に置換（§7 未解決）。workers/policy/version.json の同梱は継続（builtin resource は user/project override の探索起点として引き続き参照） |
| version canonical source | `packages/cli/package.json` | Cargo workspace 化で canonical を `Cargo.toml`（workspace version）に移すか `package.json` を残すか選択（`check-version.mjs`・`set-version.mjs`・`version.json` 生成の更新が必要） |
| APM hook / skills | `cheaper` CLI contract 経由 | CLI contract 不変なら無変更 |

### 4.2 GitHub Release + SHA256SUMS 検証フロー

launcher acquisition の contract は「pinned version の Release から `{asset}` と `SHA256SUMS` を取得 → hash 検証 → `--version` probe → cache に publish → exec」。これは payload の中身（SEA/Rust）を問わないため、**asset 名と `--version` 出力契約を守れば完全に維持できる**。追加で darwin asset を出す場合は launcher の darwin acquisition 分岐（現状は sibling executable 必須で fail）を解放する変更が別途必要。

## 5. リスクと緩和策

| # | リスク | 影響 | 緩和策 |
|---|---|---|---|
| R1 | Rig 0.x breaking change 多発（0.40 で 31 件）・2-4 週毎 release | upgrade 追従コスト・意図しない API drift | `=0.43.x` pin、upgrade は四半期計画。Rig 型を `cheaper-rig-adapter` 1 crate に隔離し、変更面を adapter に限定。MIGRATING.md を upgrade 時の canonical 手順にする |
| R2 | docs 陳腐化（custom provider ガイド等が旧 API） | G4（devin）実装の手探り化 | docs.rs API reference + source を正とする。PoC で custom Wire/Decoder を実際に書いて見積確度を上げる（Phase 0 の go 条件 d） |
| R3 | auth/OAuth ポート（G2）の見えないコスト | provider 別 flow（browser/device）の実装・検証が大きい。Pi の flow 実装が正確な spec 源 | **段階切り分け**: Phase 3 では api_key + env var + devin CLI 委譲を先に成立させ、OAuth（openai-codex/anthropic/github-copilot/openrouter/kimi-coding/radius）は flow ごとに独立 sub-task 化。pi-ai の flow 実装（MIT）を spec として参照。対象 provider を絞る判断（§7）も選択肢 |
| R4 | devin provider 移植（G4） | Connect-protocol vendored transport（656 行）の wire 互換が未検証。実 endpoint でしか確かめにくい | PoC 期に wire framing の設計検証（go 条件 d）。移植後も TS 版 devin provider を残して differential 比較。最悪「devin は初期リリースで非対応（api_key/OAuth provider のみ）」に切り下げ可能か事前合意（§7） |
| R5 | バイナリサイズ未実測 | 目標 5-15MB が外れる可能性 | Phase 0 の go/no-go で実測（lto+opt-level=s+strip）。超過時は `rig-reqwest` の TLS backend 選定・dependency feature 削りで再測定 |
| R6 | rewrite コスト支配（≈16.4k 行 src + test 群） | 全候補共通の最大コスト。契約面積が大きい | フェーズ 1→4 で「port 済み層」から積み上げ、differential golden で早期に回帰を検出。`workflow`/`hook`/`usage`/`auth` を独立フェーズ化して並行化 |
| R7 | schema 検証文言差（Ajv → jsonschema） | repair prompt の error 反馈文言・envelope `details.errors[].message` が変わる | `instancePath`+件数を契約とし message は非契約化（§3.3-1）。repair 成功は schema validity で判定するため機能影響は限定的 |
| R8 | 挙動差の silent regression（abort 分類・PARTIAL_MUTATION・transientRetries・tool 境界） | 親 agent から見えない挙動変更 | §3.3 の differential golden を必須 gate に。`meta` の全 field を比較対象に含める。特に `transientRetries`（G3）・`rejectedToolCalls`・`savings.directlyComparable` |
| R9 | Windows process tool / env / path 差分 | `process` tool（minimal env・tree kill）・path canonicalize の OS 差分 | Phase 2 で Windows runner 上の実検証を完了条件に含める。TS 実装の Windows 分岐（uppercase env 等）を仕様化してから port |
| R10 | TS 版との並走期間中の二重保守 | 長引くと両側に fix が必要 | Phase 0 通過後は新機能を TS 側に追加しない凍結宣言。修正は差分のある層のみ |

## 6. 見積もりと判断基準

### 6.1 相対工数感（人日スケールの粗さ・チーム 1-2 名想定）

| Phase | 内容 | 相対規模 |
|---|---|---|
| 0 | PoC スパイク | 小（1-3 日） |
| 1 | core 移植 | 中（≈2.7k 行の純ロジック。schema/resolve/pricing は移植密度が高い） |
| 2 | tool-runtime 移植 | **大**（≈4.4k 行 + scheduler/resource/process/artifact の設計込み） |
| 3 | runtime 移植（G1/G2/G3/d+devin G4） | **最大**（Rig 接着 + auth/catalog + devin + run orchestration ≈3.5k 行分） |
| 4 | CLI 移植 + workflow | 大（≈5.7k 行分。envelope 厳密互換・hook・usage・workflow engine） |
| 5 | 配布切替 | 中（matrix・bundling・version 経路） |
| 6 | 撤去・doc | 小〜中 |

支配項は Phase 3（G1/G2/G4）と Phase 2。`devin` を初期 scope から外すと Phase 3 が明確に縮小する。

### 6.2 go/no-go 判断条件

| Gate | 条件 |
|---|---|
| **PoC → 本移行 go** | (a) release binary ≤ **20MB**（目標 5-15MB）、(b) Rig agent loop で turn/usage/tool call/cancel を `WorkerRunMeta` に写せる、(c) repair 動線が構成できる、(d) custom `Wire`/`Decoder` で独自 provider が書ける見通し（devin 設計の机上検証まで）、(e) tokio+rustls 構成で 3 OS ビルドが成立 |
| **Phase 1-2 done** | TS 同等 unit/golden が全パス。差分は §3.3 の normalize 規則内のみ |
| **Phase 3 done** | fake provider differential golden 全パス + Ollama real-provider で 3 built-in worker 成功 + `meta` 全 field 一致（normalize 規則内） |
| **Phase 4 done** | CLI e2e/contract（hook byte 一致・envelope schema・usage.jsonl 互換）全パス + binary-to-binary differential 全パス |
| **Phase 5 done** | 同一 asset 名・SHA256SUMS・`--version` probe で Release が組め、launcher acquisition が無改修で Rust binary を取得・検証・exec できる |

### 6.3 採否の総合判断

調査レポートの結論（「Rig は技術要件を満たす有力候補」）を踏まえ、本移行の採否は **「TS→Rust 全面 rewrite コスト（Phase 1-6） vs 配布サイズ 113MB→一桁 MB の便益」** の判断に帰着する。Rig 自体の機能ギャップは G4（devin）の実装コストのみで、それ以外は「移植」ではなく「新設計が必要な領域」（G1 catalog / G2 auth）として計画に組み込んだ。

## 7. 未解決の前提確認事項

1. **対象 platform**: 現行 Release は `win-x64`/`linux-x64`/`linux-arm64` の 3 つで darwin 未提供（launcher は darwin を検知するが acquisition は sibling 必須で fail）。Rust 化で darwin を追加するか。追加なら launcher の darwin acquisition 分岐解放が必要。
2. **workflow orchestration の移植タイミング**: `workflow run/interface` を Phase 4（CLI 移植）に含めるか、独立フェーズ（Phase 4.5）に分けるか。workflow engine は Rig 非依存のため後回しも可能。
3. **`<userConfigDir>/pi/` の去就**: credential/catalog を cheaper 自前 store に移す際、(a) `pi/` 配下を読んで移行するか、(b) 新 dir に切り替えて再 login を要求するか。`auth.json` の互換 import は Pi 依存を残さずに実装可能か。
4. **OAuth provider scope**: 全 OAuth provider（openai-codex/anthropic/github-copilot/openrouter/kimi-coding/radius）を同等に移植するか、初期リリースで対象を絞るか（絞る場合の provider 選定基準）。
5. **devin の初期 scope**: `devin` provider を v1 同梱するか。切り下げる場合の UX（`auth status` で unsupported 表示等）。
6. **`cheaper.mjs` の去就**: APM bundle 同梱の JS fallback を廃止するか。廃止すれば Node 前提経路が消え「binary 取得か sibling executable 必須」に統一されるが、offline preseed の UX と `pnpm bundle` 検証経路（`mise run check` の bundle freshness）の再設計が必要。
7. **`TOKENIZER_IMPLEMENTATION` 表記**: `count-tokens`/`count_tokens` tool の `details.tokenizer` に入る identity 文字列。`tiktoken-rs` への変更をそのまま出すか、互換表示にするか。
8. **version canonical source**: Rust workspace 化後の canonical を `Cargo.toml`（workspace）にするか `packages/cli/package.json` を残すか。`check-version.mjs`/`set-version.mjs`/`version.json`/`apm.yml` の整合経路。
9. **error message 契約の緩和範囲**: Ajv → jsonschema で message 文言が変わる。`details.errors[].message` を非契約と宣言してよいか（親 agent が文言に依存していないか）。
10. **models.dev catalog の取得経路**: Pi が行っている models.dev ベースの catalog fetch/refresh をそのまま再実装するか、bundled 静的 catalog + user `models.json` のみに縮小するか。online refresh の policy（`PI_OFFLINE` 相当 env）の移植範囲。

## 付録 A: 現行 contract 早見（Rust 版で不変とする面）

- stdout: envelope 1 JSON + 末尾改行のみ。`schemaVersion: 1`。success/utility/error/workflow の 4 形態。error は `code`/`message`/`exitCode`/`retryable`/`details?`/`cause?` + `meta` は取得不能 field を `null`。
- exit code: CheaperError code 表（INVALID_REQUEST=2 … INTERNAL_ERROR=70）。
- `cheaper hook <event> --target <agent>`: envelope 外の別 contract。全失敗は fail-open（copilot は非 0 を返さない）。
- usage.jsonl: `<userConfigDir>/usage.jsonl`、1 行 JSONL、`kind:"run"|"count"`、best-effort。
- config: `CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper`。project `.cheaper/`。worker `.md`（YAML frontmatter + schema ref）、`policy.yaml`、`models.yaml`、`pricing.json`。
- built-in worker alias: `bulk-read`/`code-write`/`doc-write`。
- test seam: `CHEAPER_TEST_FAKE_STREAM`（Rust では dev-only fake provider 注入に相当）。
