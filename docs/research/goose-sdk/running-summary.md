# Running Summary

## Iteration 1 (構成・crates.io・ヘッドレス)

確定:
- Goose は Block 発祥だが現在 **aaif-goose/goose**(Agentic AI Foundation / Linux Foundation)に移管。Apache-2.0。リリースはほぼ週次(v1.53.0 時点)。
- Workspace: `goose`(core,未公開名)、`goose-cli`、`goose-server`(goosed)、`goose-acp`、`goose-mcp`、`goose-sdk` 等。Rust 1.91.1。`vendor/v8` patch あり。
- **crates.io の `goose` / `goose-cli` は別プロジェクト**(Tag1 負荷試験 / Michael Mendy インストーラ)。Block の CLI は crates.io 未公開。
- **GDK(Goose Development Kit)が crates.io に公開**: `goose-sdk`, `goose-agent`, `goose-providers`, `goose-provider-types`, `goose-context-management`, `goose-sdk-types`, `goose-local-inference`, `goose-download-manager`。すべて 0.1.0-alpha.x。
- `goose run` は非対話対応: `--output-format json|stream-json`, `--no-session`, `--max-turns`, `--provider`, `--model`。

## Iteration 2 (agent loop・provider・サイズ・structured output)

確定:
- **goose-agent = 再利用可能な agent loop crate**。StateMachine + Operation リストを利用者が組み立てる設計。`ToolOperation` が `SyncTool`/`AsyncTool`(rmcp ベース)を直接登録可能 → **自前 Rust ツールのプロセス内登録が可能**。`Emitter` が Usage/MessageUsage イベントを発行 → token usage 取得可。`CancellationToken` で abort。`assistant_turn_count` 等の predicate あり。
- goose-providers: Anthropic/OpenAI/openai_compatible/Google/Databricks(+OAuth)/Azure Foundry/Snowflake/Ollama/local_inference + declarative JSON(~30 定義内蔵、`load_custom_providers` で外部定義追加可)。Provider trait は goose-provider-types で自前実装可能。
- structured output: recipe `response.json_schema` → `final_output` tool、検証失敗で再試行(repair)。非対話自動化向けに設計済み。ただしこれは recipe/フル goose 層の機能。
- サイズ: CLI tarball 圧縮で 74-99MB(v1.53.0)。Issue #3914 で「release binary is huge」(~86Mi text)。V8(v8-goose)同梱が一因。GDK crates の依存には v8 無し → GDK 限定の自作バイナリは大幅に小さくできる見込み。
- usage: `goose run --output-format json` の metadata.total_tokens(累積,#8910)、stream-json Complete イベント、Session(accumulated_*_tokens, SQLite)。
- ライブラリ利用の実例: PR #10600(外部アプリが goose をライブラリ使用)。Issue #1726 での歴史的議論 → GDK で実現方向。Discussion #7697: 単一 ACP バイナリ化計画。

矛盾・注意:
- 公式トップの `cargo install goose-cli` 記載は crates.io 上では別ツールを指す(名前衝突)。実際のインストールは download_cli.sh / GitHub Releases。
- フル `goose` crate(ExtensionManager、bundled skills、recipe、developer toolset 等)は crates.io 未公開 → git 依存が必要。
- lib.rs の日付表示(2026 年)は環境の現在日付に依存する表示。正確なリリース日は GitHub 側で別途確認が望ましい。

## Iteration 3 (監査・ギャップ確認)

残ギャップ(許容範囲と判断):
- tarball 内個別バイナリサイズ(goose vs goosed)の分解 → 未確認だが結論に影響小。
- `--no-session` 時に json 出力の total_tokens が取れるか(session 永続化依存の可能性)→ Partial として記載。
- goose-agent の structured output 相当(final_output_tool 相当の Operation を自前実装する必要) → Partial。
- goose crate を git 依存で使う場合の vendored v8 patch の影響(workspace patch は downstream に伝播しないが v8-goose crate が公開済みなので解決可能と推測)。

カバレッジ: T1-T6 すべて一次ソースで確認済み。T7 へ進める。
