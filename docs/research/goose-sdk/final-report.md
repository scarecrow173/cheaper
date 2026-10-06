# Research Report: Goose SDK as Pi SDK replacement

## Scope and Assumptions

- 対象: Block 発祥・現 AAIF(Agentic AI Foundation / Linux Foundation)管理の Goose(`aaif-goose/goose`、旧 `block/goose` は redirect)と、その SDK 群「Goose Development Kit (GDK)」。依頼者の「Goose GDK」表記は正確 — crates.io の `goose-sdk` の正式説明が "The goose Development Kit (GDK)" である。
- 目的: cheaper の `packages/runtime-pi`(Pi SDK + TypeScript/Node.js SEA)を、Goose 系 Rust クレートまたは goose バイナリで代替できるかの評価。
- 調査は Web 一次ソース(GitHub、docs.rs/lib.rs、公式 docs、Releases、Issues)ベース。実ビルド・実測は未実施。

## Executive Summary

**結論: Goose は「ライブラリとして埋め込める」条件を満たす。ただしパスが2つあり、推奨は GDK crates の直接利用。**

1. **GDK crates(`goose-agent` + `goose-providers` 等)を crates.io 依存として使い、cheaper 自身の Rust バイナリに組み込むパスが成立する**。`goose-agent` は StateMachine 型の再利用可能 agent loop で、`SyncTool`/`AsyncTool`/`ToolProvider` により自前 Rust ツールをプロセス内登録できる。`Emitter` が Usage イベントを発行し、`CancellationToken` で abort 可能。`goose-providers` は Anthropic/OpenAI/openai_compatible/Ollama 等 native + declarative JSON で ~40 プロバイダ + trait 自前実装を提供。
2. **フル `goose` クレート(ExtensionManager・recipe・builtin tools 等)は crates.io 未公開**(crates.io の `goose`/`goose-cli` は別プロジェクトによる名前占有)。git 依存 or vendoring が必要で、V8(v8-goose)を含む重量依存を引きずる。
3. **最大リスクは API 安定性**: GDK crates はすべて `0.1.0-alpha.x`。cheaper が現在依存する Pi SDK 0.85.1 と同程度かそれ以上に不安定と見なすべき。
4. 代替として `goose run --output-format stream-json` を subprocess 呼び出しするパスも成立するが、カスタム sandboxed tool を Rust でプロセス内登録できず(MCP server として別プロセスで渡す形)、usage は session 永続化経由の total_tokens に留まる。

## Key Findings

- **リポジトリ**: aaif-goose/goose。workspace = `goose`(core), `goose-cli`, `goose-server`, `goose-acp`, `goose-mcp`, `goose-sdk`, `goose-agent`, `goose-providers` 等。Apache-2.0、Rust 1.91.1、リリースほぼ週次(v1.53.0)。[S1,S2,S3,S8]
- **GDK 公開クレート**(すべて 0.1.0-alpha.11、Apache-2.0): `goose-sdk`(UniFFI binding 層、provider API)、`goose-agent`(agent loop)、`goose-providers`、`goose-provider-types`(Provider trait + 会話型)、`goose-context-management`(compaction)、`goose-sdk-types`、`goose-local-inference`、`goose-download-manager`。[S5,S6,S7,S12]
- **goose-agent の設計**: 固定ループではなく `Operation` の ordered list を StateMachine が回す構成。`Inference` step が provider 呼び出し、`ToolOperation` が `SyncTool`/`AsyncTool`(rmcp tool trait)を in-process 登録、`ToolProvider` で動的ツール供給も可。`Emitter` → `AgentEvent`(Message/Usage/MessageUsage/McpNotification/HistoryReplaced)。`CancellationToken` で協調 cancel。`SessionLoader`/`EffectHandler`/`MachineSession` trait は利用者が実装(永続化を自前化できる)。predicate `assistant_turn_count` 等で turn 計測可。[S6]
- **provider**: native = anthropic, openai, openai_compatible(任意 OpenAI 互換 endpoint), google, databricks(+OAuth), azure_foundry, snowflake, ollama, local_inference(feature)。declarative = JSON 定義を `load_custom_providers(dir)` で外部追加可(内蔵 ~30+)。Provider trait を自前実装すれば任意 provider を追加可能。[S7,S2]
- **ヘッドレス実行**: `goose run -t/-i/stdin`、`--no-session`、`--output-format json|stream-json`(JSONL イベントストリーム)、`--quiet`、`--provider/--model`、`--max-turns`。[S9,S10,S20]
- **usage**: json 出力 metadata.total_tokens(累積、#8910 で修正)、stream-json の Complete イベント、Session の accumulated_*_tokens(SQLite sessions.db)。ライブラリ経由なら Emitter の Usage/MessageUsage イベントで per-call usage 取得可。[S15,S6]
- **structured output**: recipe の `response.json_schema` → `final_output` tool で schema 強制 + 検証失敗時にエラーを返して agent に修正させる repair ループ。これはフル goose/recipe 層の機能。GDK のみで組む場合は同等の Operation を自前実装する必要あり(ツール + Operation は自作可能なので実現可能、ただし工数)。[S11,S6]
- **バイナリサイズ**: 公式 CLI tarball は圧縮 74-99MB(v1.53.0、darwin/linux aarch64)。Issue #3914 でメンテナ自ら「release binary is huge」(~86Mi __text)。要因は dep 重複・lancedb(削除済み方向)・**vendored V8(v8-goose)** 等。一方 GDK crates の依存には v8 が含まれず、goose-agent+goose-providers 限定の自作バイナリは大幅に軽量化できる見込み(lib.rs 概算で goose-sdk  deps ~11-34MB、goose-agent ~26-38MB、goose-providers ~26-60MB)。v1.53.0 で "Lean ACP-only Goose binary" と GDK の wasm32 対応も進行中。[S8,S16,S17]
- **配布形態の将来**: Discussion #7697 で goose/goosed/goose-acp-server を ACP を話す単一バイナリに統合する計画。goosed の REST+SSE API(103 endpoints)は廃止方向 → goosed への依存は避けるべき。[S14]
- **ライブラリ利用の実績**: Issue #1726 で crate 公開要求が長らくあり(当時メンテナは消極的)、現在は GDK として実現中。PR #10600 では外部アプリが "uses goose as a library" と明記しており実例あり。TypeScript からは `@aaif/goose-sdk` が ACP クライアント(goose プロセスを stdio で子プロセス起動、ネイティブバイナリを optional dep 同梱)。[S12,S13,S19]
- **crates.io 名前衝突**: `goose`=Tag1 負荷試験、`goose-cli`=第三者インストーラ。Block/AAIF のフル CLI は crates.io 未公開で GitHub Releases/download_cli.sh 配布。[S4,S6]

## Evidence Matrix

| Claim | Support | Source IDs | Confidence | Notes |
|-------|---------|-----------|------------|-------|
| goose core crate は crates.io 未公開(名前衝突) | crates.io の goose/goose-cli は別物を一次確認 | S4,S6 | High | git 依存なら利用可能(実例あり S13) |
| GDK crates が crates.io に公開されている | lib.rs/docs.rs で 8 crate 確認 | S5,S6,S7,S12 | High | すべて 0.1.0-alpha.x |
| goose-agent で自前 Rust ツールを in-process 登録できる | ToolOperation/SyncTool/AsyncTool/ToolProvider | S6 | High | rmcp ベース |
| usage(turn/token)取得可能 | Emitter Usage/MessageUsage、assistant_turn_count、session total_tokens | S6,S15 | High | turn 数は predicate、tool call 数は conversation から算出 |
| streaming/abort | ProviderStream/next_chunk、CancellationToken、stream-json | S5,S6,S20 | High | — |
| provider 範囲が要件を満たす | native 10 + declarative ~30+ + trait 実装 | S7,S2 | High | Bedrock は native 一覧に無し(declarative/OpenAI 互換経路か custom 実装が必要な可能性 — 要確認) |
| structured output + repair 可能 | recipe response.json_schema + final_output tool 検証 retry | S11 | Medium | GDK のみでは自前実装が必要 |
| 非対話 JSON envelope CLI 化可能 | --output-format json/stream-json、--no-session、または自作 CLI | S9,S10,S20 | High | stdin prompt 対応 |
| 小さいネイティブバイナリ化可能 | GDK のみなら v8 等を回避可。フル goose は ~60-100MB+ | S8,S16,S17 | Medium | 実測ビルド未実施、見込み評価 |
| API 安定性リスク | alpha バージョン、週次リリース、活発な再構成(#7697) | S5,S6,S8,S14 | High | pin 必須 |

## Analysis — cheaper 要件適合表

| 要求能力 | 適合度 | 証拠・備考 |
|----------|--------|-----------|
| ライブラリとして埋め込み可能 | **Yes**(GDK 経由) | goose-agent/goose-providers が crates.io 公開、agent loop を StateMachine として組み立てる設計(S6,S7)。フル goose crate は git 依存が必要(S4,S13) |
| エージェントループ(複数 turn、計測) | **Yes** | StateMachine.run()、assistant_turn_count、Emitter Usage/MessageUsage(S6) |
| カスタムツール定義(read/grep/write 等 sandboxed) | **Yes** | SyncTool/AsyncTool を ToolOperation に直接登録。MCP サーバー化不要で in-process(S6) |
| 複数 provider(Anthropic/OpenAI/OpenAI-compatible/Ollama) | **Yes** | S7。plugin 追加 = Provider trait 実装 or declarative JSON(既存の CheaperProviderPlugin 相当は自前 provider 実装で代替可能) |
| usage(token)取得 | **Yes** | Emitter イベント + session accumulated_*。`--no-session` 時の json total_tokens は未確認(Partial リスク) (S6,S15) |
| streaming/abort | **Yes** | ProviderStream、CancellationToken(S5,S6) |
| structured output(JSON schema + repair) | **Partial** | フル goose の recipe/final_output tool で実現(S11)。GDK 直組みでは同等 Operation の自前実装が必要(実現可能だが工数) |
| 非対話 stdin/stdout JSON envelope CLI | **Yes** | `goose run` がそのまま近似動作(S9,S10,S20)。GDK で自作 CLI を書けば完全に独自 envelope 化も可 |
| 小さいネイティブバイナリ | **Partial→Yes 見込み** | フル goose バイナリは巨大(圧縮 74-99MB、V8 等)(S8,S16,S17)。GDK crates のみで組めば v8/lancedb/UI を回避でき大幅軽量化見込み。実測ビルドでの確認が必要 |
| メンテナンス状況 | **Yes** | Linux Foundation AAIF 移管、55k stars、週次リリース、91+ contributors(S1,S8) |
| API 安定性 | **No(リスク)** | 全 GDK crate が 0.1.0-alpha。breaking change 頻発の見込み。version pin + upgrade 追従コストが必要(S5,S6,S8) |

### 決定的論点への回答

「ライブラリとして埋め込めるか、フルアプリ subprocess か」→ **埋め込める**。公式が GDK として crates.io に agent loop(`goose-agent`)と provider 層(`goose-providers`)を分離公開しており、subprocess に限定されない。ただし「フル goose の機能一式(recipe、ExtensionManager、bundled tools、session DB)をライブラリ利用」は crates.io 未公開の `goose` crate への git 依存が必要で、V8 等の重い依存を引き込む。cheaper は自前ツール実装・自前 session 管理を既に持つため、**GDK crates だけで要件を組むのが最短かつ最軽量のパス**であり、これは Goose 側の意図した利用形態(GDK = "the source of every language binding")とも一致する。

## Gaps / Uncertainties

1. GDK crates だけで組んだ場合の実バイナリサイズ(実測未実施。lib.rs 依存サイズ概算のみ)
2. goose-agent の Inference step が `goose-providers` の Provider trait を直接受けるかの API 詳細(docs.rs API リファレンスの精読が次段階で必要)
3. structured output repair を GDK で組む際の工数(final_output 相当 tool + retry Operation は自作可能と判断したが、フル goose の該当コード流用可否は未確認)
4. `goose run --no-session` 時に total_tokens が出力されるか(session 永続化に依存する実装のため)
5. Bedrock 等 native 一覧外 provider の declarative 経路可否
6. `goose` crate を git 依存した場合の vendor/v8 patch の伝播可否(workspace patch は downstream に効かないため v8-goose 公開 crate で解決されると推測、未検証)
7. goose-sdk(UniFFI)経路は provider API のみで agent loop を含まない点 — TypeScript から移行する場合 `@aaif/goose-sdk` は ACP 子プロセスモデルでありプロセス分離が残る

## Recommendations or Next Actions

1. **採用パス候補A(推奨評価対象)**: Rust 製 worker runtime を新規に書き、`goose-agent` + `goose-providers` + `goose-provider-types`(+必要なら `goose-context-management`)を pin 依存で使う。cheaper の既存 sandboxed tools を `SyncTool`/`AsyncTool` 実装として移植。envelope・usage 集計は自前(現行 TypeScript 実装の移植)。
2. **採用パス候補B(低コスト検証)**: まず `goose run --output-format stream-json` を subprocess 呼び出しする形でプロトタイプし、provider/usage/stream-json の実動作を確認。カスタムツールは MCP server(別プロセス)として接続。これだと Node 依存は消せるがツールの in-process 化と独自 envelope 制御に限界。
3. **PoC タスク**: 最小 Rust バイナリで goose-agent+goose-providers+Ollama を組み、turn/usage/abort/structured-output 相当を確認 + リリースビルドのバイナリサイズ実測(strip+opt-level=z+lto)。
4. **リスク管理**: alpha API のためバージョン pin と upgrade 頻度(週次リリース)への追従方針を決める。goosed REST API には依存しない(#7697 で廃止方向)。
5. Pi SDK 側と機能パリティ比較(特に structured output repair、canonical model 情報、cost 計算)をコードレベルで照合。

## Source List

- S1 https://github.com/aaif-goose/goose
- S2 https://github.com/block/goose/blob/main/AGENTS.md
- S3 https://block-goose.mintlify.app/development/architecture
- S4 https://crates.io/crates/goose
- S5 https://lib.rs/crates/goose-sdk
- S6 https://lib.rs/crates/goose-agent
- S7 https://lib.rs/crates/goose-providers
- S8 https://github.com/aaif-goose/goose/releases
- S9 https://github.com/block/goose/blob/58f3cc9e/documentation/docs/guides/running-tasks.md
- S10 https://github.com/aaif-goose/goose/blob/main/crates/goose-cli/src/cli.rs
- S11 https://github.com/block/goose/blob/58f3cc9e/documentation/docs/guides/recipes/recipe-reference.md
- S12 https://github.com/block/goose/issues/1726
- S13 https://github.com/aaif-goose/goose/pull/10600
- S14 https://github.com/aaif-goose/goose/discussions/7697
- S15 https://github.com/block/goose/blob/44ac5dc1/crates/goose-cli/src/session/mod.rs (+ commit 9ca29a1 / PR #8910)
- S16 https://github.com/block/goose/issues/3914
- S17 https://docs.rs/crate/v8-goose
- S18 https://goose-docs.ai/docs/gdk/sdk/
- S19 https://www.npmjs.com/package/@aaif/goose-sdk
- S20 https://github.com/block/goose/pull/6228
