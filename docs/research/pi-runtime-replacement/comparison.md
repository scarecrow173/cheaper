# Pi SDK + TypeScript/Node 置き換え候補 比較サマリ

調査日: 2026-10-06
依頼: 配布用実行ファイルサイズ削減のため、Pi SDK (`@earendil-works/pi-ai` / `pi-coding-agent`) および TypeScript/Node.js ランタイム依存を除去する代替の検討。

現状: Node SEA 実行ファイル `cheaper-windows-x64.exe` は **約 113MB**(Node ランタイム全体を内包)。

個別調査レポート:
- `docs/research/rig-agent-runtime/final-report.md`
- `docs/research/molo-agent-runtime/final-report.md`
- `docs/research/goose-sdk/final-report.md`

## 候補比較表

| 観点 | Rig (0xPlaygrounds/rig) | molo (Silwings-git/molo) | Goose GDK (block/goose) |
|---|---|---|---|
| 言語・形態 | Rust ライブラリ (rig-core 等) | Rust ライブラリ (crates.io `molo`) | Rust GDK クレート群 (`goose-agent` / `goose-providers` 等) |
| 成熟度 | v0.43.0、8.8k★、2-4週毎リリース | v0.6.0、公開約2ヶ月、4★/0 fork、単一作者 | 0.1.0-alpha.11、Apache-2.0、Block 本体プロジェクトは活発 |
| ライブラリ埋め込み | Yes (AgentRun が sans-I/O state machine) | Yes (「製品ではなく SDK」設計) | Yes (goose-agent を直接利用; フル `goose` crate は crates.io 未公開) |
| agent loop / turn 計測 | Yes (max_turns, hooks) | Yes (ReActAgent, RunSummary) | Yes (StateMachine+Operation, assistant_turn_count) |
| カスタムツール | Yes (Tool/PortableTool trait) | Yes (Tool trait + harness governed effect) | Yes (SyncTool/AsyncTool/ToolProvider、in-process) |
| provider | 26 内蔵 (Anthropic/OpenAI/Ollama 等) + companion crate | OpenAI 互換のみ内蔵。Anthropic は互換 endpoint 経由で制約大、native は自作要 | native 10種 + declarative JSON + Provider trait |
| usage / streaming / abort | 全て Yes | 全て Yes (cached_tokens 含む) | 全て Yes (Emitter が Usage イベント) |
| structured output | Yes (output_schema + retry/repair) | Yes (jsonschema 検証 + error feedback) | Partial (GDK 直組みでは同等 Operation の自作要) |
| バイナリ見通し | rig-core は tokio/reqwest 非依存、推定 5-15MB 級(実測なし) | 推定 4-8MB(実測なし) | フル CLI は 74-99MB と巨大だが GDK のみ自作なら V8 等を回避でき大幅軽量化の見込み(実測なし) |
| 主要リスク | 0.x の breaking changes 多発(0.40 で 31件)、docs 一部陳腐化、devin provider 相当の自作コスト | 未成熟(採用実績ゼロ)、provider 貧弱、設計 doc が中国語のみ | 全 GDK crate alpha、フル crate は名前衝突で crates.io 未公開、goosed REST API は廃止方向 |
| devin provider 相当 | カスタム provider 実装可だが新設計で docs 古く未検証 | Provider trait は必須2メソッドで境界小、自作前提 | Provider trait で自作可 |

## 評価

- **第一候補は Rig**。要求能力の適合度が最も高く(8項目中ほぼ全て Yes)、成熟度・provider 網羅性・採用実績で優位。最大リスクは 0.x の breaking change 頻度と、TS→Rust 全面 rewrite コスト(後者は全候補共通)。
- **Goose GDK は次点**。機能的には成立し Block の開発勢いは強いが、alpha 版 pin 必須・structured output 自作・フル crate 未公開の制約があり、「ライブラリとしての Goose」はまだ一級の利用形態ではない。
- **molo はコンセプト適合(kernel/harness/effect 分離は cheaper の sandboxed tool モデルと構造的に合う)が未成熟すぎる**。watchlist + スパイク評価止まりが妥当。

## 共通の構造的リスク

- どの候補でも cheaper の TS コードベース(コマンド、count-tokens、pricing、usage.jsonl、APM 統合、workflow 等)の Rust 全面再実装が前提となる。SDK 差異より rewrite コストが支配的。
- provider プラグイン機構(特に bundled `devin` provider)は各候補で自作移植が必要。

## 推奨次アクション

1. Rig + Ollama + 自作 tool で最小 PoC: turn/usage/abort/structured output の動作と `cargo build --release`(lto 有)のバイナリサイズ実測(1-2日規模のスパイク)。
2. 同条件で goose-agent + goose-providers の最小 PoC で実サイズ比較(必要に応じて)。
3. サイズ削減効果と rewrite 範囲を定量したうえで ADR 化を検討。
