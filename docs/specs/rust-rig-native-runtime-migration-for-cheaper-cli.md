---
id: "SPEC-034BbgvL9s41K7NflOddZ6"
type: "spec"
status: "implemented"
title: "Rust + Rig native runtime migration for cheaper CLI"
created: "2026-10-06"
updated: "2026-10-06"
owners: ["scarecrow173"]
relations:
  source:
    - "../research/pi-runtime-replacement/migration-comparison.md"
    - "../research/pi-runtime-replacement/comparison.md"
    - "../research/rig-agent-runtime/migration-plan.md"
    - "../research/rig-agent-runtime/final-report.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./auth-login-command.md"
    - "./devin-local-bundled-provider.md"
    - "./model-pricing-catalog.md"
    - "./ollama-real-provider-worker-e2e.md"
    - "./pi-state-under-cheaper-config-dir.md"
    - "./usage-and-savings-telemetry-aggregation.md"
    - "./version-pinned-launcher-acquisition.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Rust + Rig native runtime migration for cheaper CLI

## What Should Be Built

cheaper CLI の worker runtime を、現行の Pi SDK(`@earendil-works/pi-ai` /
`pi-coding-agent`)+ TypeScript/Node.js SEA 構成から、Rust + Rig
(0xPlaygrounds/rig)ベースのネイティブ実行ファイルへ移行する。

移行は「外部 contract 不変の全面 rewrite」である。親 coding agent から見た
`cheaper` コマンドの入出力・配布形態・設定書式は一切変えず、実装言語と
配布バイナリの中身だけを置き換える。対象は `packages/core`・
`packages/tool-runtime`・`packages/runtime-pi`・`packages/workflow`・
`packages/cli` の全 TypeScript 実装(src ≈16.4k 行)であり、Rig が担うのは
provider 抽象と agent loop のみで、sandboxed tool・envelope・hook・usage 記録・
auth/catalog 層は cheaper 側の責務として Rust で再実装する。

## Why Now

- 配布用実行ファイル(現行 Node SEA `cheaper-windows-x64.exe` ≈113MB)のサイズが
  導入・取得コストの支配要因であり、Rust ネイティブ化で一桁 MB〜十数 MB 級への
  削減が見込める(Rig 調査で transport opt-in・lto+opt-level="s" 構成を確認済み。
  実測は未了のため Phase 0 PoC で最初に潰す)。
- Node.js ランタイム依存の除去により、「binary を取得して exec するだけ」の
  配布モデルが完結し、SEA VFS・esbuild bundle・`cheaper.mjs` fallback という
  Node 前提の経路を廃止できる。
- discovery(`DISC-034B9B0PiBkxUgLGfI70g8`)で Rig / molo / Goose GDK の 3 案
  比較が完了し、Rig が provider 内蔵範囲・成熟度・採用実績の面で第一候補と
  確定した(molo 案・Goose 案とも「不達なら Rig へ」を相互 fallback 指名)。
  次のゲートは spec 確定と ADR であり、本書がその spec を担う。

## Users and Value

- **親 coding agent / 利用者**: 取得・更新が軽い単一ネイティブバイナリ。
  Node.js 不要で、envelope・hook・usage の既存 contract はそのまま使える。
- **cheaper 開発者**: ランタイム依存が Pi SDK + Node から cargo workspace に
  一本化され、provider/auth/catalog を自前 contract として保守できる。
- **consumer(APM 配布先)**: launcher(sh/ps1)・SHA256SUMS 検証・Release asset 名が
  不変のため、移行を意識せずそのまま取得・検証・実行できる。

## Scope

### In Scope

- `packages/core`・`packages/tool-runtime`・`packages/runtime-pi`・
  `packages/workflow`・`packages/cli` の Rust port(全面 rewrite)。
- Rig(0.43 系、`=` pin)への接着を 1 crate(`cheaper-rig-adapter` 仮)に隔離する
  構成。core/tool-runtime/workflow/cli は Rig 型を直接参照しない。
- **provider 範囲**: Anthropic / OpenAI / OpenAI-compatible(Ollama を含む) /
  GitHub Copilot / devin を初期サポート対象とする。認証は API キーに加えて
  サブスクリプション認証(OAuth)も利用可能とする。それ以外の Pi builtin
  provider は初期スコープ外とし、未知・未対応の api 種別は silent fallback
  せず明示 error で返す。
- **auth**: OAuth login flow の移植を含む。対象は github-copilot を含む利用
  実態のある flow(browser auth_url + device code)。`auth login/status/logout`
  の既存 CLI contract を維持し、credential を `<userConfigDir>` 配下の新
  credential store に永続化する。`<userConfigDir>/pi/` の auth.json/
  models.json/models-store.json は**移行しない** — Rust 版では新 store への
  再 login を要求する。
- **model catalog**: models.dev 系 catalog + bundled 静的 snapshot + user
  `models.json` を統合する cheaper 自前 catalog store(G1 相当)。tier/explicit
  解決はこの catalog 上で動く。
- **usage/costUsd 仕様変更**: `meta.usage.costUsd` は provider 応答値ではなく
  cheaper の pricing 層(pricing snapshot × token 数)で算出する仕様に変更する。
  新旧 usage.jsonl record で costUsd 口径が混在することを docs(usage guide)に
  明記する。
- **usage.jsonl 後方互換**: 既存 record(`kind:"run"|"count"`、旧 schema)の
  読み込みを維持し、`cheaper usage` が新旧混在 log を集計できること。
- **transient retry 層**: 429/5xx/network を分類する retry wrapper を自前実装し、
  `meta.transientRetries` に計上する。
- **structured output repair**: tool 無し repair session + json_schema mode +
  schema error フィードバック retry を cheaper 側に再実装し、現行の repair
  意味を維持する。
- **parity 検証基盤**: fake/scripted provider fixture による TS/Rust binary 間の
  differential golden test(後述の正規化規則を先に定義)。
- **配布切替**: cargo release matrix(win-x64 / linux-x64 / linux-arm64)で現行と
  同一の asset 名・SHA256SUMS・`--version` probe を維持し、launcher(sh/ps1)は
  無改修とする。built-in resource(workers/*.md・schemas・policy.yaml・
  version.json)はバイナリ埋込に置き換える。

### Out of Scope

- 機能追加・worker 追加・envelope schema の拡張(新機能は移行後の別 spec)。
- 配布 contract(GitHub Release + SHA256SUMS + launcher acquisition)の再設計。
- APM package(`packages/apm/`)の構造変更 — `.apm/hooks/`・skills・launcher は
  無改変。
- molo / Goose GDK への切替実装(Rig PoC 不達時のピボット判断は ADR/plan の
  責務であり、本 spec は Rig 採用を前提とする)。
- Pi builtin provider 全種の parity — 上記の初期 provider 範囲以外は移植しない。
- darwin-* target の Release 追加(現行未提供。別途判断、Deferred Design
  Concerns 参照)。
- `<userConfigDir>/pi/` 既存 credential の読み取り・import(再 login 方針のため)。

## Requirements

### 不変 contract(移行前後で同一を要求)

- stdin/stdout JSON envelope: `schemaVersion: 1`、success/utility/error/workflow
  の 4 形態、error は `code`/`message`/`exitCode`/`retryable`/`details?`/`cause?`、
  stdout は envelope 1 JSON + 末尾改行 1 つのみ。
- exit code: CheaperError code 表(INVALID_REQUEST=2 … INTERNAL_ERROR=70)を
  そのまま維持。
- `cheaper hook <event> --target <agent>` の 4 target adapter 出力と fail-open
  契約(envelope 外の別 contract)。
- usage.jsonl: `<userConfigDir>/usage.jsonl`、1 行 JSONL、best-effort 記録、
  `kind:"run"|"count"` の record 形を維持(新 record は旧 reader が読める範囲で
  拡張し、旧 record の欠落 field は許容)。
- worker `.md`(YAML frontmatter + schema ref)・`policy.yaml`・`models.yaml`・
  `pricing.json` の書式と探索順序(`CHEAPER_CONFIG_DIR` →
  `XDG_CONFIG_HOME/cheaper` → `~/.config/cheaper`、project `.cheaper/`)。
- Release asset 名(`cheaper-linux-x64`/`cheaper-linux-arm64`/
  `cheaper-windows-x64.exe`)・SHA256SUMS 生成/検証フロー・`--version` probe・
  launcher(sh/ps1)の無改修。
- built-in worker alias(`bulk-read`/`code-write`/`doc-write`)と全 CLI コマンド群。

### メタデータ互換

- `WorkerRunMeta` の全 field(`model`/`usage`/`turns`/`toolCalls`/
  `rejectedToolCalls`/`repairs`/`transientRetries`/`changes`/`execution`/
  `savings`)を envelope `meta` に維持する。`savings.directlyComparable` の
  意味論も維持する。
- `meta.usage.costUsd` は cheaper pricing 層(pricing snapshot × token 数)
  算出に変更する。provider 応答の cost 値には依存しない。
- abort 分類(timeout/signal/maxTurns/maxToolCalls)・PARTIAL_MUTATION 包装・
  prompt grace settle の意味を維持する。

### サイズ・配布目標

- release binary(strip 後、lto+opt-level="s" 相当)≤ **20MB** を受け入れ閾値、
  **5-15MB** を目標とする。
- Windows は `x86_64-pc-windows-msvc`、Linux は musl static を検討対象とする。

### parity 検証規則

- fake/scripted provider fixture(TS 側 `CHEAPER_TEST_FAKE_STREAM` の応答配列を
  JSON 化し Rust 側 seam が同 fixture を replay)で、代表 worker × 正常系/
  invalid output→repair/abort(timeout・maxTurns・maxToolCalls・SIGINT)/
  PARTIAL_MUTATION/error envelope の 20-40 case を golden 化する。
- 正規化規則: `meta.durationMs`・`meta.execution` の時刻系 field
  (`operationMs`/`activeMs`/`overlapMs`/`queueMs`、wall-clock 由来で非決定。
  `execution` の決定的 field `operations`/`byState`/`byTool`/
  `maxConcurrency` は比較対象)・`usage.costUsd`(TS=provider 応答値 vs
  Rust=pricing 層算出値で系統的不一致)・tokenizer identity(`result.
  tokenizer.*`・`details.tokenizer.*`)・`error.message` 本文(Node
  `err.message` と Rust `io::Error` Display の系統的差異を吸収。
  `error.code`/`exitCode` は厳密比較)・stderr 診断文言・JSON キー順は比較
  除外。schema 検証 error は `instancePath` のみを契約とし、`message` 本文・
  件数・code は非契約(Ajv ≠ jsonschema の文言差・列挙差を吸収)。
  「meta 全 field の比較」はこの正規化規則適用後の意味とする。
- usage token field(`input`/`output`/cache 系)・`turns`/`toolCalls`/`repairs`/
  `changes`/`savings`/`result` は厳密比較対象。

## Acceptance Criteria

- [ ] `cheaper run` 系コマンド(worker alias・`count-tokens`・`usage`・`auth`・
      `workflow`・`hook` 含む全コマンド)が Rust binary で動作し、run 系が
      `schemaVersion: 1` の envelope を stdout に 1 JSON + 末尾改行 1 つで返す。
- [ ] release binary(strip 後)が `windows-x64`・`linux-x64` の両 target で
      ≤20MB(目標 5-15MB)であることを CI 計測で確認できる。
- [ ] scripted fake provider fixture(20-40 case)に対し TS binary と Rust binary
      の stdout envelope・exit code が、正規化規則適用後に全件一致する。
      `meta` の全 field(model/usage/turns/toolCalls/rejectedToolCalls/repairs/
      transientRetries/changes/execution/savings)が比較対象に含まれる。
- [ ] built-in worker 3 種(bulk-reader/code-writer/doc-writer)が Ollama 実
      provider コンテナで成功 envelope を返す既存 e2e 相当が Rust binary で通る。
- [ ] `cheaper auth login github-copilot`(OAuth、device または browser flow)から
      run 実行までが成功し、credential が `<userConfigDir>` 配下の新 credential
      store に永続化される。`<userConfigDir>/pi/` の auth.json/models.json は
      参照・読み込みしない。
- [ ] API キー認証(Anthropic/OpenAI/OpenAI-compatible)でも login → run が成立
      し、secret が stdout に出ない。
- [ ] 未知・初期スコープ外の provider api 種別を指定した場合、明示 error
      envelope(code・exitCode・retryable 付き)で失敗し、silent fallback しない。
- [ ] `meta.usage.costUsd` が provider 応答値ではなく cheaper pricing 層
      (pricing snapshot × token 数)で算出され、新旧 record の口径混在が
      usage guide に記載されている。
- [ ] `cheaper usage` が旧 schema record(`savings`/`turns`/`toolCalls` 欠落を
      含む)と新 record の混在 usage.jsonl を読み込み集計できる。
- [ ] `cheaper hook <event> --target <agent>` の 4 target × allow/deny/malformed
      入力で出力が TS 版と byte 一致し、fail-open 契約が維持される。
- [ ] 現行と同一の asset 名・SHA256SUMS・`--version` probe で Release が組め、
      無改修の launcher(sh/ps1)が Rust binary を取得・検証・exec できる。
- [ ] `<userConfigDir>/pi/` の credential 移行を行わず、Rust 版は新 credential
      store への再 login をユーザーへ案内する(auth status / error で判別可能)。

## Deferred Design Concerns

- darwin-* target の Release 追加 | 現行未提供で移行の完了条件ではない |
  Rust 化後に launcher の darwin acquisition 分岐解放とセットで再検討 |
  無視すると macOS 利用者が sibling executable 前提のまま残る。
- workflow engine 移植のフェーズ位置(CLI 移植と同居か独立か) | Rig 非依存で
  後回し可能なため | plan 策定時にタスク分割として決定 | 遅延すると
  `workflow run/interface` が TS 版に残る期間が延びる。
- APM bundle 同梱 `cheaper.mjs`(JS fallback)の廃止判断 | Node 前提経路の完全
  撤去は offline preseed UX と `pnpm bundle` 検証経路の再設計を伴う |
  Phase 6(撤去)時に判断 | 放置すると Node 前提経路が残り移行目的が半減する。
- version canonical source(package.json → Cargo.toml 移行の是非) |
  `check-version.mjs`/`set-version.mjs`/`version.json`/`apm.yml` の整合経路に
  波及 | 配布切替(Phase 5)までに決定 | 未決のままだと version 宣言不一致で
  `mise run check` が通らない。
- `TOKENIZER_IMPLEMENTATION` 表記(tiktoken-rs への実装名変更を出すか互換表示か) |
  `details.tokenizer` の consumer 影響が不明 | 正規化規則で version は除外済み、
  残りは CLI 移植時に決定 | 親 agent が identity 文字列に依存している場合に
  破壊的変更となる。
- models.dev catalog の online refresh 範囲(bundled 静的 + user `models.json`
  のみに縮小するか、`PI_OFFLINE` 相当 policy まで移植するか) | 実装量と
  offline 契約のトレードオフ | runtime 移植フェーズの設計時 | catalog 鮮度の
  運用ルールが曖昧なまま残る。
- Rig PoC 不達時の Goose GDK ピボット | 本 spec は Rig 前提 | Phase 0 go/no-go
  ゲートの結果 | ピボット時は本 spec の Rig 固有記述の見直しが必要。

## Sources and Relations

- 上流 discovery: `docs/discovery/rust-rig-native-runtime-migration-for-cheaper-cli.md`
  (`DISC-034B9B0PiBkxUgLGfI70g8`)— 3 案比較の確定、Phase 0 PoC ゲート、
  本 spec への promotion 要求を記録(`relations.derives-from`)。
- `docs/research/pi-runtime-replacement/migration-comparison.md` — 移行計画
  3 案の比較と「外部 contract 不変・自前化領域・differential golden 戦略」の
  共通結論。
- `docs/research/rig-agent-runtime/migration-plan.md` — 採用案(Rig)の機能
  マッピング・ギャップ G1-G8・フェーズ計画・不変 contract 早見。
- `docs/research/rig-agent-runtime/final-report.md` — Rig 0.43 の機能適合証跡
  (provider 26 種・usage 粒度・streaming/cancel・structured output・
  breaking change リスク)。
- ユーザー確定事項(provider 範囲・OAuth 移植・costUsd 仕様変更・Pi state
  非移行)は本 spec の Scope/Requirements に直接反映済み。
- 並走で作成予定の ADR(Rust+Rig 採用決定・adapter 隔離・並走/撤去戦略)とは
  `relations.related` で相互リンクする(ADR 作成時に更新)。
