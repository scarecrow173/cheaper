# 移行計画: cheaper runtime の molo (Silwings-git/molo) ベース Rust 化

調査日: 2026-10-07
前提レポート: `final-report.md`(機能適合評価)、`../pi-runtime-replacement/comparison.md`(3候補比較)
対象コード: `packages/{core,tool-runtime,runtime-pi,cli,workflow}` + `packages/apm` + `scripts/` 配布パイプライン

> **要旨**: molo の kernel/harness/effect 分離は cheaper の sandboxed tool モデルと構造的に
> 整合するが、**移行コストの主体は molo 採用ではなく TS→Rust 全面 rewrite と、Pi
> ModelRuntime が担っていた provider/catalog/auth 層の自前再実装**にある。molo が供給
> するのは agent loop・Provider/Tool trait・harness 基盤のみで、provider 群(Anthropic
> native・devin・models.json catalog・OAuth)は全て自作になる。単一作者・採用ゼロ・
> pre-1.0 の crate にランタイムを委ねるリスクを考慮し、採用は「スパイクで定量検証 →
> 隔離層越しの段階導入 → 撤退条件の明文化」を条件とする。

## 1. スコープ整理 — Rust 化が必要なモジュール

現行 src 実装は合計約 16.4K 行(テスト約 23.5K 行は別)。責務と規模の内訳:

| 現行モジュール | 規模(行) | 責務 | Rust 化の性質 |
|---|---:|---|---|
| `core/worker/*` | ~560 | worker `.md`(YAML frontmatter + 本文)parse・validate・name 解決(project > user > builtin) | 移植(YAML/JSON Schema 依存) |
| `core/policy/*` | ~900 | `policy.yaml` の schema 検証・層 merge・restrict・process/workflow policy | 移植 |
| `core/model/*` | ~900 | tier 分類(explicit-override/curated-exception/auto percentile)・resolveModel/resolveExplicitModel・pricing snapshot 解決・model ref/pattern | 移植(純粋ロジック) |
| `core/token/count.ts` | 102 | o200k_base(js-tiktoken)による canonical token 計測 | 移植(tiktoken-rs 系に要置換) |
| `core/workspace/*` | ~230 | workspace root canonicalize・symlink policy・denyPaths | 移植 |
| `core/resources/*` | ~520 | resource manifest 正規化・claim 包含・predicate | 移植 |
| `core/errors.ts`/`duration.ts`/`config/dirs.ts` | ~250 | CheaperError(code/exitCode/retryable)・duration parse・`CHEAPER_CONFIG_DIR`→XDG→`~/.config` | 移植 |
| `tool-runtime/*` | ~4,500 | sandboxed tool 実行基盤: ResourceAccess(denylist/writeScope/manifest/observation)・ToolScheduler(shared/exclusive claim 直列化・concurrency)・ResourceManager(lease・parent scope)・OperationManager(記録/metrics/abort grace)・MutationJournal・tool 実装(read/grep/find/count_tokens/process)・write/edit/ls の注入 handler seam | 移植(最大の独自資産。molo-coding primitives は下請けとして利用可) |
| `runtime-pi/run.ts`+`session.ts`+`prompt-grace.ts` | ~750 | runWorker: capability 絞込→model 解決→tool scope→Pi session→abort 分類(timeout/signal/maxTurns/maxToolCalls)→repair→meta 構築→PARTIAL_MUTATION 包み | **molo ReActAgent で置換**(outer control は cheaper 側に残る) |
| `runtime-pi/output.ts`+`repair.ts` | ~260 | 最終 assistant text 抽出・fence 除去・JSON parse・schema 検証・別 session repair(error feedback・attempt 計上・INVALID_OUTPUT details) | 移植(jsonschema crate + 自前 repair loop)or molo run_typed へ写す(§3.4) |
| `runtime-pi/usage.ts` | 105 | session event から usage/turns/transientRetries 集計 | molo RunSummary + event stream で置換 |
| `runtime-pi/model-runtime.ts` | 236 | Pi ModelRuntime factory(`<userConfigDir>/pi/` auth/models/models-store)・plugin 登録・refresh・tier→Pi Model 実体解決 | **自前再実装**(molo に catalog/auth 概念なし) |
| `runtime-pi/auth.ts` | 113 | `auth status/login/logout` の runtime bridge(OAuth flow は pi-ai 内蔵) | **自前再実装**(OAuth flow 含む) |
| `runtime-pi/provider-plugin.ts`+`providers/devin/*` | ~1,400 | CheaperProviderPlugin contract・devin provider(Devin CLI 委譲 auth・`devin models list` catalog・vendored Connect transport・credentials.toml・JWT cache) | **自前 Provider 実装へ移植**(molo Provider trait 実装) |
| `runtime-pi/tools/*` | ~900 | canonical tool → Pi ToolDefinition adapter・Pi helper(write/edit/ls)の operations 注入 | molo Tool trait adapter へ書換 |
| `cli/envelope.ts`+`args.ts`+`input.ts`+`text-format.ts` | ~830 | stdin/stdout JSON envelope(schemaVersion 1)・arg parse・text format | 移植(**byte 互換必須**) |
| `cli/cli.ts`+`count.ts`+`version.ts` | ~800 | command dispatch・run orchestration・count-tokens | 移植 |
| `cli/auth.ts` | 299 | terminal interaction bridge(secret raw mode・select prompt・browser open) | 移植 |
| `cli/usage-log.ts`+`usage-report.ts`+`pricing.ts` | ~460 | usage.jsonl 記録・`cheaper usage` 集計・pricing.json snapshot | 移植(record 形式互換必須) |
| `cli/hook/*` | ~860 | 4 target(windsurf/copilot/pretooluse/cursor)の stdin normalize→routing policy→native render・fail-open | 移植(純粋ロジック) |
| `cli/workflow-*.ts`+`workflow/*` | ~2,400 | workflow 定義 compile・DAG engine・resource inference・artifact store・job 並列実行(shared invocation + parent lease) | 移植(tokio 並列) |
| `packages/apm` + `scripts/` | — | APM hooks(launcher sh/ps1)・SEA build・release bundle/SHA256SUMS | **配布層のみ差替**(§5) |

**molo が担わない領域の明示**: model catalog(models.json 互換)、credential store
(auth.json 互換)、OAuth flow、Anthropic/devin provider、cheaper policy 層
(claim/lease/scheduler)、usage.jsonl、envelope、APM 統合。これらは全て cheaper 側
の Rust コードとして新規/移植実装になる。

## 2. molo へのマッピング

### 2.1 実行コア(runWorker → ReActAgent)

| 現行(runtime-pi) | molo 対応物 | 差異・注意 |
|---|---|---|
| `createAgentSession` + `session.prompt()` | `ReActAgent::run_request` → `RunOutput`+`RunSummary` / `run_stream`(`MessageChunk`) | 1:1 相当。streaming は cheaper では usage 集計以外未利用 |
| `turn_end` 計測 + `maxTurns`(stopReason=toolUse で継続する場合のみ abort) | `RunSummary.rounds` + ReActAgent tool-round limit | cheaper の maxTurns 判定は「toolUse で終わった turn」のみ。molo の round limit と意味が一致するかスパイクで検証(不一致なら cheaper 側で turn を数えて CancellationToken で止める) |
| `maxToolCalls`(tool 呼出数で abort) | `RunSummary.tool_calls` / tool 実行 hook | molo に tool-call 数 limit があるか不明。なければ Tool wrapper 側でカウントし超過で cancel(現行 `onLimit` と同じ構造) |
| timeout/外部 SIGINT→abort reason 分類 | `CancellationToken` + `MessageChunk::Cancelled` | reason 分類(TIMEOUT/ABORTED/MAX_*)は cheaper 側が管理する現行構造を維持可 |
| abort 時 journal 非空 → PARTIAL_MUTATION 包み | molo 非対象 | MutationJournal(自前移植)と envelope/meta 層の責務。loop の外側なので阻害なし |
| `min(policy.output.maxTokens, model.maxTokens)` の per-request 注入 | Provider request 面に max_tokens 相当が通せるか**要検証**(Provider trait は 2 メソッドだが request 型の表現力次第) | §7 未解決事項 |
| transientRetries(Pi auto-retry、`transientRetries` 計上) | `RetryProvider`(decorator)で包み回数を計測、または自前 retry wrapper | meta.transientRetries の観測手段は wrapper で確保可 |
| `noTools:"builtin"` + tools allowlist + customTools | `ToolRegistry` に cheaper tool だけ登録 | builtin tool の存在自体が molo にはないため allowlist 設計は単純化される |
| system prompt + output contract 注入 | `ReActAgent` builder の system prompt 面 | 現行 `buildSystemPrompt` の文字列生成はそのまま移植 |

`WorkerRunMeta`(`model{tier,tierSource,selection,auto...}`, `usage{input,output,cacheRead,cacheWrite,costUsd}`,
`turns`, `toolCalls`, `rejectedToolCalls`, `repairs`, `changes`, `execution`, `savings`)は
envelope 契約の一部なので、molo 側の `RunSummary` から取れない項目(tier 系・rejected・
repairs・changes・execution・savings)は cheaper 側ロジックで引き続き生成する。

### 2.2 provider 層 — Provider trait で自前実装する範囲とコスト

molo の `Provider` trait は required メソッド `chat_with_context` /
`stream_chat_with_context` の 2 つのみで境界は小さい(S7)。ただし cheaper が現在
Pi に委譲している範囲は「transport 実装」よりずっと広い:

| 機能 | 現行の担い手 | molo 移行後の担い手 | 推定コスト |
|---|---|---|---|
| models.json(provider endpoint/api 種別/model 定義: cost・contextWindow・maxTokens・samplingParams) | Pi ModelRuntime | **自前 catalog loader**(models.json 互換 format を維持するか新形式化は要決定) | M(3-5 人日) |
| builtin provider 定義(Pi 内蔵の anthropic/openai/google/github-copilot 等) | Pi | **自前 builtin 定義** — サポート範囲の選別が前提課題 | S-M |
| auth.json credential store + refresh | Pi ModelRuntime | 自前 credential store(auth.json 読み互換 or 移行) | M(3-5 人日) |
| OAuth flow(openai-codex の browser/device flow、bun-oauth loader 経由) | pi-ai | **自前実装**(auth.ts の interaction contract は移植可) | M-L(4-8 人日、flow 数次第) |
| `openai-completions` api type | pi-ai | **molo-openai が流用可**(base_url/api_key 可変、Ollama 明記)。samplingParams 任意 merge・Responses API 非対応の差異あり | S(1-2 人日) |
| `anthropic` native(Messages API + SSE + tool_use + usage + prompt cache `cache_control`) | pi-ai 内蔵 | **自前 Provider 実装**。cheaper の cacheRead/cacheWrite 計測とコスト最適化動機上、OpenAI 互換 endpoint 迂回(prompt caching 非対応・response_format 無視)は不可 | **L(5-10 人日)。go/no-go 閾値の主対象** |
| `devin` provider(Devin CLI 委譲 login・credentials.toml 監視・`devin models list` catalog・Connect RPC vendored transport ~656 行・JWT cache) | CheaperProviderPlugin | **自前 Provider 実装として移植**。stream.ts の protobuf/Connect 写像は言語非依存のため port は直線的だが量がある | M-L(6-10 人日) |
| その他 Pi api types(openai-responses, google, bedrock, github-copilot 等) | pi-ai | **初期スコープ外推奨**。models.json で未知 api 種別は明示 error | — |
| model catalog refresh(network で models list 取得・models-store.json cache) | Pi | 自前 refresh(provider 別の list 実装 + cache file) | M(3-5 人日) |

**provider 自作コストの総量が最大の変動要因**。Anthropic native + devin + auth/catalog
で 20-35 人日級の見込み。Anthropic を必須 provider とするならこの領域が molo 採否の
事実上の決定因になる(rig は Anthropic を内蔵するが catalog/auth/devin は同じく自作)。

### 2.3 sandboxed tool モデル → harness / governed effect の構造適合

cheaper の tool 実行経路: `Tool.execute` → input schema 検証 → claim 宣言
(shared/exclusive)→ scheduler(concurrency・mutation 直列化)→ ResourceAccess
(denylist/symlink/writeScope/manifest 包含)→ tool 本体 → journal 記録 →
CanonicalToolResult。

molo の対応構造: 副作用 tool は `EffectRequest` を返し、harness chain
(`policy → approval → sandbox → executor → redact → audit → observation`)を経て実行。

**具体的な利用方針**:

1. **molo-coding primitives を下請けに使う**: `LocalWorkspace`/`WorkspacePath`
   (traversal 拒否)は `core/workspace/paths.ts` の canonicalize+boundary 検査と同じ
   関心。`CommandExecutor` は `tool-runtime/process.ts` の spawn 層、`RipgrepSearcher` は
   `grep.ts` の実行エンジンの置換候補。**ただし採用可否は trait/option 面の適合を
   スパイクで確認**(cheaper の symlink policy・byte/line budget・line-framer 等の独自
   semantics を primitives が受け入れない場合は自前実装を優先 — 無理に適合させると
   挙動差異が出る)。
2. **policy 層は cheaper 側に移植**: denylist・writeScope・resource manifest・
   claim 包含・parent lease は cheaper 固有の不変条件であり、molo harness の policy
   stage に写すのではなく **tool 実装内のアドミッション層として保持**する。harness
   の policy/approval stage をこの粒度(resource claim 包含・per-resource
   serialization)まで表現できるかは不明で、仮に可能でも二重管理になる。approval は
   cheaper に存在しない概念(policy が静的に決める)なので harness の approval stage は
   auto-approve で素通し相当。
3. **harness の利用価値は audit/observation 接続**: MutationJournal・access
   observation・operation record を harness の audit/observation hook に接続できれば
   「governed effect として実行された mutation のみ journal 化」が構造で保証される。
   これが乗らない場合(tool 内で自前記録でも等価)molo harness 採用の差分価値は小さく、
   Tool trait 直実装でも成立する — **その場合 molo の独自価値は減り、rig との再比較
   材料になる**(§6/§7)。
4. **scheduler(claim ベース並行制御)は molo に存在しない領域**: same-turn batching
   契約(system prompt で独立性を要求済み)と workflow job 並列の双方で必要。
   `ToolScheduler`+`ResourceManager`+`OperationManager` はそのまま Rust 移植し、
   tool `call` 内で admission→schedule→execute を回す現在の構造を維持する。

### 2.4 tier resolver・usage.jsonl・structured output repair の移植先

- **tier resolver**(`core/model/{resolve,classify,pricing,tier,config,ref}`): molo に
  対応物なし。`ModelCatalog.getAvailable()` に相当する自前 ModelRegistry の上に純粋
  ロジックとしてそのまま移植。`resolvePiModel`→`catalog.getModel` の二段解決も
  Rust 側 registry で同型に再現。
- **usage.jsonl**: writer は CLI 層に残る。`RunSummary.usage{prompt,completion,total,
  cached}` を `WorkerUsage{input,output,cacheRead,cacheWrite,totalTokens,costUsd}` へ
  写像する際の差異を潰す必要がある:
  - `cached` が cacheRead/cacheWrite を分離するか(分離しないなら Anthropic provider
    実装側で cache_creation/cache_read を別途返す拡張 usage が必要)
  - `costUsd` — 現行は Pi が `message.usage.cost.total` を供給。molo RunSummary に
    cost 面があるか不明。**なければ cheaper の pricing 層×token 数で算出する仕様へ
    変更**(挙動差異。provider 課金口径との乖離リスクを docs に明記)
  - **usage-bearing failure**: `meta.usage` を持つ error を記録する契約(ADR-0025)
    のため、error/abort 経路でも RunSummary 相当の partial usage を取得できるか
    **スパイク必須検証**
- **structured output repair**: 2 経路を比較して決定:
  - (a) `run_typed`(jsonschema 検証 + error feedback retry 内蔵)— 少コードだが
    attempt 回数・最終 errors・途中 abort の観測可否が不明
  - (b) 自前 repair loop(現行 `repair.ts` の移植)— 別 run で repair prompt(raw
    output + 検証 error 一覧)を投げ、`repairs` 計上・INVALID_OUTPUT details
    (errors 先頭 20・rawOutput 先頭 2000)を忠実再現。**envelope/meta 互換を優先する
    なら (b) が安全**。jsonschema crate(draft 2020-12 対応)で Ajv 相当検証を自前で
    行うため error 形式の差異(instancePath/message)は吸収可能
  - 判断基準: (a) が attempt 観測・abort 伝播・failure detail を提供するかをスパイクで確認

## 3. 段階的移行計画

### Phase 0 — スパイク検証(3-5 人日、go/no-go ゲート)

成果物は使い捨て PoC(`docs/research/molo-agent-runtime/spike/` または別 branch)。
本実装に持ち込まない。

1. molo + `FakeProvider` + 自前 `Tool`(read/grep 相当の 2 個)で最小 agent run を作り、
   envelope 相当 JSON を生成できることを確認
2. `molo-openai` + Ollama コンテナ(`docker compose up -d`、`qwen3.5` 系)で実 provider
   run: tool call 往復・usage 取得・`run_stream` 確認
3. `cargo build --release`(lto="fat"、strip、opt-level="z" 候補)でバイナリサイズ実測
   → 目標 ≤15MB(現行 SEA ~113MB からの大幅削減が前提条件)
4. abort/timeout 中の partial usage 取得可否、`run_typed` の retry 観測可否、
   Provider request 面の max_tokens/sampling 注入可否を検証(§7 未解決事項を潰す)
5. Anthropic Messages API の `Provider` 実装の外形だけ試作(送信 shape・SSE parse・
   usage/cache 写像の見通し)して工数見積もりを精緻化

**ゲート**: 全項目クリアかつ provider 自作コスト概算が閾値(§7)以内なら Phase 1 へ。

### Phase 1 — core/tool-runtime の Rust 移植(15-25 人日)

`cheaper-rs` workspace(crate: `cheaper-core`, `cheaper-tools`)を新設。molo 非依存層
なので撤退しても salvage 可能。

- worker/policy/schema(yaml + JSON Schema 2020-12)→ serde + jsonschema crate
- workspace paths/symlink/denyPaths、resource manifest/claim
- token count: o200k_base を tiktoken-rs 系で。**js-tiktoken との rank table 一致を
  fixture test で検証**(tokenizer identity version を記録しているので不一致は契約破壊)
- tool-runtime 本体: access/scheduler/resources/operation/journal + tool 実装
  (read/grep/find/count_tokens/write/edit/ls/process)。`AsyncLocalStorage` 依存の
  active operation 伝播は tokio task-local or 明示 ctx 受け渡しに置換
- 完了条件: TS 側の property/golden test に相当する Rust test が green、tool 出力
  フォーマット(trailer・marker 類)の byte 一致

### Phase 2 — runtime 置換(10-18 人日)

- `cheaper-runtime` crate: runWorker 相当の orchestration(ReActAgent + 自前 abort
  分類 + journal + meta 構築 + repair loop)
- ModelRegistry(models.json 互換 loader + builtin provider 定義 + refresh/cache)
- credential store + auth.json 読み互換(または移行コマンド)
- provider: openai-completions(molo-openai 経由)を先に接続し Ollama e2e を通す。
  Anthropic native・devin は Phase 3 で別途
- 完了条件: `bulk-read` worker が Ollama で TS 版と同形の envelope を返す

### Phase 3 — provider 拡充(12-25 人日、条件付き)

優先度順: ①anthropic native ②devin ③OAuth flow(openai-codex 等、利用実態がある
もののみ)④残 api types。**Phase 2 完了時点で必要 provider が揃わない場合は TS 版を
維持する判断を再評価**(途中撤退ポイント)。

### Phase 4 — CLI 置換(10-18 人日)

- `cheaper-cli` crate: args/envelope/各 command(run・aliases・count-tokens・models・
  usage・auth・workflow・hook)
- hook target adapter + routing policy の移植(fail-open 契約の維持が必須)
- usage.jsonl writer/reader(record 形式は schemaVersion 1 のまま互換)
- workflow engine(DAG・job 並列・artifact store)の移植
- 完了条件: 全 command が envelope byte 互換

### Phase 5 — 配布切替(5-10 人日)

§4 を参照。完了条件: release asset + SHA256SUMS + launcher が現行契約のまま Rust
binary を供給し、consumer の install 経路が不変。

### TS/Rust 並走時の挙動比較方法

- **golden envelope fixtures**: TS 版の `CHEAPER_TEST_FAKE_STREAM` seam(scripted
  応答列)と同等の FakeProvider scripted 応答を Rust 側に実装し、同一 worker/input に
  対する stdout envelope を byte 比較(非決定 field: durationMs・usage 絶対値・ts は
  正規化して比較)
- **e2e 差分**: Ollama コンテナに対し同一 worker×同一 input を両 binary で実行し、
  envelope の構造・exit code・usage.jsonl record 形を比較
- **切替手段**: consumer から見て cheaper は「stdin JSON → stdout envelope +
  exit code + stderr」のプロセス契約のみなので、sibling binary(`cli/bin/<platform>/`)
  の差替だけで切替可能。配布面では release asset 名を維持すれば launcher 無変更
- **後方互換の固定点**: envelope schemaVersion 1、usage.jsonl record 形、
  models.json/auth.json/config dir 配置、exit code 表(CheaperError.exitCode)、
  `--version` 出力、stderr diagnostic の存在有無

## 4. 配布変更 — SEA → per-platform Rust release binary

| 層 | 現行 | 移行後 | 変更有無 |
|---|---|---|---|
| 消費者 interface | `cheaper[.exe]` を exec する launcher(sh/ps1) | 同じ(バイナリ中身は不問) | **無変更** |
| Release asset 名 | `cheaper-windows-x64.exe` / `cheaper-linux-x64` / `cheaper-linux-arm64` | 同名踏襲 + **darwin-* 追加が容易になる**(SEA の host-only 制約が消える) | ほぼ無変更 |
| 検証 | Release の `SHA256SUMS` + `--version` 照合 + atomic rename | **同契約を維持可能**(生成スクリプトの対象が cargo artifact に変わるのみ) | 内部実装のみ |
| ビルド | esbuild bundle → `node --build-sea`(host のみ) | GitHub Actions matrix + `cargo build --release --target`(cross or native runner) | 置換 |
| 埋込 asset | SEA VFS(workers/*.md・policy.yaml・version.json) | `include_str!`/`include_dir!` or env! 埋込で等価 | 置換 |
| version 管理 | `packages/cli/package.json` が canonical、`check-version.mjs` | `Cargo.toml` version が canonical、check スクリプトを書換 | 置換 |
| TLS | Node 内蔵 | **rustls 推奨**(musl 静的化・cross 簡易化のため)。molo-openai の reqwest が native-tls default の場合は feature flag 化を upstream 依頼 or vendored patch | 新規論点 |
| `pnpm bundle`/`apm` payload | `cli/bin/` の SEA を ignore、CI で bundle 鮮度 check | cargo artifact の ignore/check に置換。APM hook(launcher・worker md・policy.yaml・version.json)は不変 | 一部変更 |

APM 側の deploy 対象(hooks/skill/version.json/workers/policy.yaml)と launcher の
契約(asset 名・cache key・SHA256SUMS 形式・lock 手順)は**完全に維持可能**で、
consumer 側の移行手順は不要。

## 5. リスクと緩和策

### 5.1 molo 未成熟リスク(定量的扱い)

観測値: 公開 ~2 ヶ月・単一作者・4★/0 fork/月間 ~40 DL・外部 dependent ゼロ・
0.x で 4 回の破壊的変更(移行パス付与の方針は明言)。

| リスク | 緩和策 |
|---|---|
| pre-1.0 breaking change 追従 | **molo API 面を adapter 層(`agent-loop` module)に隔離**: cheaper 側は「Provider trait 実装を供給する registry」「Tool trait 実装を供給する tool runtime」「run → normalized RunOutcome 変換」の 3 面だけを molo に見せる。upgrade は adapter 単独の修正で閉じる設計を Phase 2 の実装要件にする。追従コストが「1 upgrade あたり >2 人日」を超えたら撤退条件発動 |
| bus factor(作者停止) | MIT/Apache-2.0 のため **vendoring/fork が常時可能**。vendor 化すると molo 側の責務(内部で ~15K SLoC だが利用面は core/agent/openai の subset)が自前保守に変わる。agent loop 本体は代替手段として自作 ReAct loop または rig への差替経路を adapter 層で確保 |
| 採用ゼロ=未知の障害 | Phase 0 スパイクで abort/usage/streaming/structured output の死角を潰す。Phase 2 までは upstream 変更を pin(`=0.x.y`)で受け、update は意図的な作業に限定 |
| 設計 doc が中国語のみ | rustdoc/README は英語。設計判断の不明点は source 直読み + adapter 層テストで挙動を固定する運用で許容 |

**差替コストの構造的評価**: molo が実際に担うのは「agent loop・Provider/Tool
trait 境界・openai transport・(採用なら)harness chain」のみで、cheaper の資産
(policy/sandbox tool/envelope/tier/usage/workflow)は molo 非依存。adapter 層設計が
守られれば molo→rig または自前 loop への差替は runtime crate 内に限定される見込み
(相対工数で全体の 15-25%)。

### 5.2 早期撤退条件(明文化)

以下のいずれか成立時は molo 採用を中止し、adapter 層の salvage 分を評価して
rig/自前 loop へ pivot:

1. Phase 0 で Provider request 面に max_tokens・任意 samplingParams・per-request
   option を通せないことが判明(cheaper の `min(maxOutputTokens, model.maxTokens)`
   注入・`samplingParams` merge が不変条件)
2. abort/error 経路で partial usage(RunSummary 相当)を取得できず、usage-bearing
   failure の記録契約を維持できない
3. Anthropic native Provider 実装が見積閾値(例: 10 人日)を超過
4. harness が claim/lease モデルを受け入れず、かつ Tool trait 直実装に留まる場合に
   「molo 固有の価値が agent loop だけ」と判明 → rig(成熟・Anthropic 内蔵)との
   再比較で劣位なら撤退
5. upstream が 6 ヶ月更新停止、または 1 upgrade の追従が 2 人日超を 2 連続

### 5.3 その他の技術リスク

- **挙動差異の混入**: envelope byte 互換・tool 出力 trailer・error code 表を golden
  test で固定しないと親 agent 側の既存 skill/hook 前提を壊す。Phase 1/4 の完了条件に
  fixture 一致を含める
- **costUsd 口径の変更**: provider 応答 cost → cheaper pricing 層算出への変更は
  usage.jsonl の新旧 record で口径が混在する問題。record schemaVersion 更新または
  meta に算出元を記録するか要決定(§7)
- **Rust rewrite の規模**: 16.4K 行 src + 23.5K 行 test の移植は 70-100 人日級。
  SDK 差異より rewrite コストが支配的(comparison.md 共通リスクと同じ)であり、
  molo 採用の是非と Rust 化の是非は分離して判断すべき

## 6. 見積もりと判断基準

| 領域 | 粗い工数(人日) | 備考 |
|---|---:|---|
| Phase 0 スパイク | 3-5 | 使い捨て |
| core 移植 | 8-12 | 純粋ロジック中心 |
| tool-runtime 移植 | 12-18 | 最大の独自資産。test 移植含む |
| runtime(agent loop + registry + auth store) | 8-12 | molo adapter 層設計込み |
| providers(Anthropic+devin+OAuth+catalog refresh) | 20-35 | **変動最大**。閾値超なら不採用 |
| CLI + hook + usage + workflow | 14-22 | |
| 配布パイプライン + CI | 5-10 | |
| **合計** | **70-110** | test 資産(~23.5K 行)の移植方式次第で上下 |

### go/no-go 条件

- **go(Phase 0→1)**: スパイクで ①binary ≤15MB ②Ollama e2e 成立 ③abort/usage/
  repair の観測可能性確認 ④Anthropic+devin+auth の自作コスト概算 ≤ 閾値
  (推奨 35 人日)
- **provider コスト閾値の解釈**: Anthropic+devin+OAuth+catalog 合計が ~35 人日を
  超える見込みなら、molo 採用の動機(構造適合)より provider 欠落の欠点が支配的
  になり、rig または「TS 維持+配布のみ改善」に pivot するのが合理的
- **全体 go**: 「113MB→~10MB の配布改善 + Node 依存除去」に対し 70-110 人日と
  未成熟依存リスクを許容できるかはプロダクト判断。runtime 以外(core/tool/cli)は
  molo 不採用でも Rust 化する場合そのまま使えるため、Phase 1 までは候補中立の
  投資とみなせる

## 7. 未解決の前提確認事項

スパイク(Phase 0)または設計着手前に潰すべき項目:

1. **molo Provider request 型**: per-request `max_tokens`・`samplingParams` 任意
   merge・`response_format`/`run_typed` のスキーマ受け渡し経路。`Model` 型に
   cost/contextWindow 等の metadata 面があるか(tier resolver の ModelInfo 供給源)
2. **RunSummary の error/abort 時取得**: run が error/cancel で終わった場合に
   partial usage(rounds/tool_calls/usage)を返す API 面があるか
3. **`usage.cached` の分解能**: cacheRead/cacheWrite の分離が取れるか(取れなければ
   provider 実装側で拡張 usage を運ぶ設計が必要)
4. **`run_typed` の観測可能性**: retry 回数・各 attempt の validation error・最終
   failure の error 内容を外部から取れるか(取れなければ自前 repair loop 一択)
5. **molo `Model`/message 型の表現力**: system prompt・tool result 写像・
   thinking off(reasoning_effort 等)・finish reason("error" 文字列 + message)の
   表現が cheaper の lastAssistantStop 相当分類に足りるか
6. **molo-coding primitives の option 面**: symlink policy・byte/line budget・
   denylist 相当を LocalWorkspace/WorkspacePath/CommandExecutor が表現できるか
7. **models.json 互換の維持可否**: 現行ユーザーが Pi 形式の models.json/auth.json を
   持つ前提で、Rust 版が読み互換を持つか・新形式+移行にするか(後方互換方針の決定)
8. **必須 provider の棚卸し**: 現行利用実態(Pi builtin provider 全体を露出して
   いる)のうち移行でサポート必須の api type 集合。openai-completions +
   anthropic + devin に絞る場合、他 provider 利用者への移行ガイドが必要
9. **OAuth flow の実装範囲**: openai-codex 等の device/browser flow + token
   refresh + subscription endpoint(backend api)の再実装仕様を pi-ai から抽出する
   必要がある
10. **costUsd 口径**: provider 応答 cost の供給が途切れる場合の usage.jsonl 後方
    互換(schemaVersion 更新 or 算出元記録)
11. **tiktoken-rs 系の o200k_base 一致**: js-tiktoken と同一 rank/merge 規則かを
    fixture で検証(tokenizer identity を meta/usage に記録しているため不変条件)
12. **TLS backend**: rustls 統一のため molo-openai reqwest feature の扱い
    (upstream 依頼/patch/vendor)
13. **process tool・workflow artifact store**の Rust 側セマンティクス(managed
    process policy・process-tree kill の Windows 実装)
14. **musl/static 化の要否**: linux-x64 を glibc dynamic にするか musl static にするか
    (リリース対象 distro 範囲の決定)
