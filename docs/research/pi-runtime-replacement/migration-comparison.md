# 移行計画 3 案比較 — Pi SDK/Node 置き換え

策定日: 2026-10-06
対象計画書:
- `docs/research/rig-agent-runtime/migration-plan.md`
- `docs/research/molo-agent-runtime/migration-plan.md`
- `docs/research/goose-sdk/migration-plan.md`

## 0. 三案に共通する結論

3 案の独立した計画策定で、以下が一致して確認された。

- **工数の支配項は SDK 差異ではなく TS→Rust 全面 rewrite**(src 約 16.3-16.4k 行 + test 約 23.5k 行)。SDK 選択は rewrite 総量の 15-25% 程度にしか効かない。
- **外部 contract は不変で維持可能**: envelope(schemaVersion:1)・usage.jsonl・hook・config・Release asset 名・SHA256SUMS・launcher(sh/ps1)は全案で無改変を維持できる。変わるのはビルド層(esbuild+SEA → cargo matrix build)と version canonical source のみ。
- **共通の自前化領域**: Pi ModelRuntime が担っていた model catalog(models.dev/models.json)・credential store・OAuth flow はどの候補にも存在せず自作必須。devin provider(Connect wire 656 行)も全案で自作移植。
- **検証戦略も共通収束**: fake/scripted provider fixture による TS/Rust binary 間の differential golden test が parity 確立の中核。
- いずれも **Phase 0 = PoC/スパイクを go/no-go ゲート**とし、バイナリサイズ実測を最初に潰す構成。

## 1. 案別の構造的差異

| 観点 | Rig | molo | Goose GDK |
|---|---|---|---|
| 採用形態 | `rig-agent`/`rig-core` + `cheaper-rig-adapter` 隔離 crate | molo agent loop + Provider/Tool trait。adapter 3 面に隔離 | `goose-agent`+`goose-providers`+`goose-provider-types` のみ(フル `goose`/SDK/MCP 不採用) |
| provider 内蔵 | 26 種(Anthropic/OpenAI/Ollama 含む) | OpenAI 互換のみ | native 10 種(Anthropic/Ollama 含む)+declarative JSON |
| provider 自作量 | devin + catalog + auth のみ | **Anthropic native も自作(L サイズ)** — 自作計 20-35 人日 | devin + auth。Anthropic は内蔵で済む |
| structured output repair | `output_schema`+自前 repair loop(現行設計と同型) | `run_typed` or 自前 loop(envelope 互換優先なら自前) | tools 無し 2 本目 StateMachine で現行設計を直移植可。`response_format` 注入可否は未確認 |
| 工数見積 | フェーズ別のみ(総量は goose 案と同オーダー見込み) | **70-110 人日**(明細表あり) | **約 7-11 週**(1名換算) |
| PoC サイズ閾値 | ≤20MB(目標 5-15MB) | ≤15MB | ≤25MB(圧縮 ≤10MB) |
| 依存成熟度 | v0.43、8.8k★、破壊的変更多発だが移行 doc あり | v0.6、2ヶ月・単一作者・採用ゼロ | 全 crate 0.1.0-alpha.11、Block 本体は活発 |
| 固有の撤退条件 | PoC 不達 → Goose へ | 撤退条件 5 項目を明文化(provider 自作 >35 人日等)→ Rig へ | PoC 不達 → Rig へ |

## 2. 決定的な相違点

1. **Anthropic native provider**: molo だけが内蔵しない(自作 5-10 人日 + 以後の保守)。cheaper の cacheRead/cacheWrite 計測とコスト最適化の目的上、OpenAI 互換 endpoint 迂回は不可と判定済み。**Anthropic を必須 provider とする限り molo は構造的に不利**。
2. **自前実装の総量**: Rig ≈ Goose < molo。Rig/Goose は provider 標準系が内蔵で、自作は devin・auth・catalog に限定。molo は provider 層全体が自作。
3. **harness 構造適合**: molo の kernel/harness/effect 分離のみが cheaper の sandboxed tool モデルに構造的に対応。ただし molo 案自身が「policy 層は harness に写さず tool 内 admission 層に保持」する設計を採っており、実際の恩恵は audit/observation 接続に限定 → 構造適合の価値は限定的と各計画が自己評価。
4. **成熟度リスクの形が違う**: Rig = breaking change 頻度(緩和: pin + adapter 隔離)、Goose = alpha 版(pin + 四半期更新)、molo = bus factor + 採用実績ゼロ(vendoring 可能だが保守転嫁)。

## 3. 未解決事項の共通項(スパイク前に要プロダクト判断)

三案の §7 が共通して挙げる、計画の分岐点となる前提確認:

- **必須 provider の棚卸し**: 現行は Pi builtin 全露出。サポート範囲を絞るか(Anthropic/OpenAI/Ollama + devin のみ等)で自作量が大きく変わる。
- **usage meta の field 粒度**: `costUsd`(現行は Pi 供給)・`cacheRead/Write` 分離・`transientRetries`・error 時 partial usage — 候補 SDK の usage 面で欠落があれば cheaper pricing 層算出への仕様変更が必要(usage.jsonl 新旧 record の口径混在問題)。
- **Pi state 移行方針**: `<userConfigDir>/pi/` の auth.json/models.json を読み互換するか、移行コマンド/再 login を要求するか。
- **OAuth 対象範囲**: openai-codex/anthropic/github-copilot 等の flow を全移植するか api_key+devin CLI 委譲に限定するか。
- darwin-* target の追加可否、version canonical source(package.json → Cargo.toml)。

## 4. 推奨

**Rig を第一候補として Phase 0 PoC 着手**が最も合理的。根拠:

- provider 内蔵範囲が最大(Anthropic 含む)で自作量が最小、成熟度・採用実績とも 3 案中最良。
- 最大リスク(0.x breaking)は `=` pin + adapter 1 crate 隔離で緩和可能と計画が裏付け。
- 各計画が互いの fallback として Rig を指名しており(molo 案・Goose 案とも「不達なら Rig へピボット」)、Rig が実質的な基準線。

**Phase 1(core/tool-runtime 移植)までは候補中立の投資**であり、molo 案も指摘する通り、Rig PoC が不達でも salvage 可能。従って「まず Rust workspace + core/tool-runtime 移植に着手し、runtime 層の SDK 選定は PoC 結果で決める」という進め方も成立する。

Goose GDK は「Rig が理由なく失敗した場合の同格代替」、molo は watchlist(構造適合は興味深いが provider 自作量と未成熟リスクが現時点では割に合わない)。
