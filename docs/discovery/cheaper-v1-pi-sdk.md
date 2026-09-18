---
id: "DISC-034r05iHy2gdnp377SlYbj"
type: "discovery"
status: "resolved"
title: "cheaper V1 実装引継ぎの整理と Pi SDK 現状確認"
created: "2026-09-15"
updated: "2026-09-15"
owners: ["scarecrow173"]
relations:
  source:
    - "../handoff/2026-09-15-cheaper-v1-implementation-handoff.md"
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/cheaper-v1-worker-runtime-cli.md"
    - "../specs/bulk-reader-built-in-worker.md"
    - "../specs/cheap-writer-built-in-worker.md"
    - "../adr/agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "../adr/worker-yaml-frontmatter-markdown.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/one-shot-json-first-cli.md"
    - "../adr/model-tier-model-resolver.md"
    - "../adr/wrapper-measured-deterministic-metadata.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/typescript-pnpm-monorepo.md"
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper V1 実装引継ぎの整理と Pi SDK 現状確認

## Exploration Goal

設計確定済みの実装引継ぎ文書 `docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md`（以下「引継ぎ」、§番号は同文書の見出し）を、doc-driven-dev ワークフローの spec / ADR へ昇格させるための整理を行う。あわせて、引継ぎが前提とする Pi SDK（Pi coding agent SDK）の現状を一次情報で確認し、採用判断の根拠を固める。

制約:

- 引継ぎ §0 の「既存の設計判断を独断で置き換えない」および §43 の実装原則（YAGNI / fail closed / no silent fallback / no silent privilege reduction / no silent privilege expansion / single source of truth）に従う。
- 引継ぎ §39 の V1 non-goals を範囲外として保持する。

## Key Issues and Assumptions

- 引継ぎは既に設計確定済みであり、本探索で新たな最終決定は行わない。記録するのは「検討経路」と「昇格先文書の切り分け」。
- Pi SDK のパッケージ名・API 形状が引継ぎ記述と一致するかは外部依存のため一次情報での確認が必要だった。

## Alternatives and Comparison

引継ぎで却下済みの代替案を検討経路として記録する。最終決定は各昇格先 ADR を参照。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| Agent runtime: 独自 agent loop（pi-ai 上） | 制御自由度が高い | §2.2「独自 Agent Loop を実装しない」に反し、保守コスト大 | no |
| Agent runtime: 複数 runtime adapter 抽象 | 将来の runtime 差し替えが可能 | §2.2「複数 Runtime Adapter を切り替える構成にしない」、YAGNI 違反 | no |
| Agent runtime: Pi SDK `createAgentSession` in-process | provider abstraction・認証・agent loop・tool calling・usage・abort を再利用できる | Pi の内部詳細への依存、Node プロセス内実行 | yes |
| Agent runtime: `pi --mode rpc` 別プロセス JSON-RPC | プロセス分離・型安全性を要さない | プロセス管理・起動コスト・エラーハンドリングが複雑化 | maybe |
| Worker 定義: TypeScript コード | 型安全・実行力が高い | §2.3「Worker をコードへハードコードしない」に反し、宣言的配布と相性が悪い | no |
| Worker 定義: JSON | 機械処理しやすい | 長い instruction 文を書きにくい、他 Coding Agent のスキル形式と異なる | no |
| Worker 定義: YAML frontmatter + Markdown | 宣言的・他 Agent スキルと親和・built-in/user 同形式 | パーサ・バリデーションを自前で持つ必要 | yes |
| Process model: daemon / MCP server / REST | セッション再利用・常駐 | §4「one-shot process」、§39 non-goals に列挙済み | no |
| Process model: one-shot JSON-first CLI | 親 Agent 非依存・単純・テスト容易 | 起動ごとのコスト | yes |
| Model 指定: Worker が具体モデル名を持つ | 単純 | §13 に反し環境差・権限昇格リスク | no |
| Model 指定: tier 抽象 + Model Resolver | Worker を環境非依存にできる | 分類ロジックが必要 | yes |
| 計測: LLM 自己申告 | 実装が楽 | §2.6「deterministic な情報は LLM に自己申告させない」に反する | no |
| 計測: wrapper が実測 | 信頼できる決定論的メタデータ | snapshot/diff 計測の実装コスト | yes |
| Filesystem boundary: prompt 制約のみ | 実装不要 | §0「security boundary を prompt だけに依存させない」に反する | no |
| Filesystem boundary: OS sandbox（Docker/Gondolin） | 強い隔離 | インフラ追加・ポータビリティ低下 | maybe |
| Filesystem boundary: cheaper 側 sandboxed tool 実装 | in-process・依存追加なし・portable | path canonicalize・symlink・TOCTOU の実装責任 | yes |

## Tentative Conclusions and Hypotheses

### Pi SDK 現状確認（confirmed、出典付き）

- **パッケージ rename**: 現行スコープは `@earendil-works/*`。`@mariozechner/pi-coding-agent` は 0.73.1（2026-05-07）で凍結し、同日 `@earendil-works/pi-coding-agent` が 0.74.0 で新規公開され継続更新中。最新は **0.85.1**（2026-09-05 更新）。`pi-ai` / `pi-agent-core` も同スコープで lockstep。無スコープ `pi-agent-core` 0.0.1 は別物のスタブ。リポジトリは `badlogic/pi-mono` → `earendil-works/pi` へ移転。出典: `npm view <pkg> time --json` 実測、https://github.com/earendil-works/pi
- **SDK エントリ**: `createAgentSession()` が `cwd` / `agentDir` / `model` / `thinkingLevel` / `scopedModels` / `modelRuntime` / `tools`（allowlist）/ `excludeTools` / `noTools` / `customTools` / `resourceLoader` / `sessionManager` / `settingsManager` を受ける。出典: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md
- **組み込みツール**: `read`, `bash`, `powershell`, `edit`, `write`, `grep`, `find`, `ls`。デフォルト有効は `read`/`bash`/`edit`/`write`。`tools` allowlist・`excludeTools`・`noTools: "all"|"builtin"` で制御可能。
- **sandbox なし（confirmed）**: `docs/security.md` に「Pi does not include a built-in sandbox」と明記。組み込みツールはプロセス権限で cwd 外アクセス可能。出典: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md
- **maxTurns なし（confirmed）**: SDK 側にターン数・ツール呼び出し数の制限オプションは存在しない。呼び出し側が `turn_end` / `tool_execution_start` イベントを監視して `session.abort()` する必要がある。出典: 同 sdk.md のイベント一覧と `AgentSession` API
- **usage**: `turn_end` の assistant `message.usage` が `{input, output, cacheRead, cacheWrite, reasoning?, totalTokens, cost:{...}}`（pi-ai `Usage` 型、`@earendil-works/pi-ai` 0.85.1 `dist/types.d.ts`）。`SessionStats` に tokens + cost の集計あり。
- **model 一覧**: `ModelRuntime.getAvailable()` で認証済みモデル取得。`Model` 型フィールド: `id, name, api, provider, baseUrl, reasoning, input, cost, contextWindow, maxTokens` 等。
- **認証**: `~/.pi/agent/auth.json`（`PI_CODING_AGENT_DIR` で変更可）。OAuth/サブスク対応 provider: ChatGPT Plus/Pro (Codex), Claude Pro/Max, GitHub Copilot, xAI, OpenRouter, Radius。API key は env var（`ANTHROPIC_API_KEY` 等）も可。出典: https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md
- **abort**: `session.abort()`。provider 層では `StreamOptions.signal` の AbortSignal も可。
- **custom tool**: `defineTool({name, label, description, parameters: Type.Object({...}), execute})` + `customTools: [...]`（TypeBox スキーマ）。
- **DefaultResourceLoader の自動読込**: `agentDir`（既定 `~/.pi/agent`）の extensions/skills/prompts/AGENTS.md/settings.json/models.json/auth.json と `cwd` の `.pi/*`・`.agents/skills`・祖先 `AGENTS.md` を自動 discovery する。無効化は `ResourceLoader` インターフェースの自前実装（examples/sdk/12-full-control.ts が実例）または override hook。
- **依存の重さ**: `engines: node >=22.19.0`、`"type": "module"`（ESM）。依存に `@earendil-works/pi-tui`（TUI）を含み、SDK 利用でも TUI 依存が install される。

### 仮説

- hypothesis: Pi 組み込みツールを cwd 制約付きで再利用できる可能性（`operations` 注入等）はあるが、公式に sandbox 境界を保証する API ではないため、cheaper 側 sandboxed tool 実装が本線。
- hypothesis: Node 26 での動作は `engines >=22.19.0` 上限なしで動く可能性が高いが、実機確認が必要。

## Open Questions

引継ぎ §40 の未確定設計項目を列挙する。**いずれも manageable downstream であり、design-doc フェーズで解決する。pre-implementation blocker は存在しない。**

- bulk-reader contract: input/output JSON Schema、path semantics、directory/glob handling、relevant symbol 表現、suggested reads 表現、source excerpt policy → manageable downstream
- cheap-writer contract: input/output JSON Schema、reference files、target/multiple targets、partial writes、changed files 算出、diff 統計 → manageable downstream
- Pi SDK integration: 正確な session 作成 API、model registry 連携、AuthStorage、tool factory、event handling、usage 抽出、abort controller、repair model call → manageable downstream
- Model Resolver: built-in model family 分類、metadata heuristic、user override config schema、same-tier preference config → manageable downstream
- Filesystem security: path normalization アルゴリズム、symlink 実装、TOCTOU、write atomicity、secret/environment 露出 → manageable downstream
- Error Contract: stdout error envelope、error detail fields、retryable semantics、partial mutation 表現、exit code マッピング → manageable downstream
- Usage telemetry: ローカル記録形式、stderr/stdout 露出 → manageable downstream
- APM integration: Hook contract、bulk-read Skill、cheap-write Skill → manageable downstream（V1 スコープ外、別 spec へ deferred）

**design 前に実機確認が必要な検証項目:**

- Node 26 上での Pi SDK 動作（`engines >=22.19.0`、上限未記載）
- Pi 組み込みツールを cwd 制約付きで再利用できるか、それとも cheaper 自前実装が必須か（ADR-034r05iHlWosDpO699OwYF の裏付け検証）

## Promotion Candidates

昇格済み（`relations.derived-by` 参照、本 DISC は `resolved`）:

- [x] spec-doc: SPEC-034r05iHK8bA7g59V25ltC（Worker Runtime と CLI 全体）、SPEC-034r05iHK8bA7g6PcCiDBp（bulk-reader）、SPEC-034r05iHK8bA7g7YRKGyQr（cheap-writer）
- [x] adr-doc: ADR-034r05iHfSMW9SGV4dpcy2〜0008（runtime 選定 / Worker 定義形式 / 三層 capability / process model / model tier / 計測 / filesystem boundary / リポジトリ構成）

**Deferred**: APM integration（引継ぎ §36–37）は V1 成功条件（§38）に含まれないため別 spec へ据え置き。本フェーズでは作成しない。

## Sources and Relations

- 一次ソース: `docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md`（原文引継ぎ）
- Pi SDK 一次情報:
  - https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/sdk.md
  - https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/providers.md
  - https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/security.md
  - https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/environment-variables.md
  - https://github.com/earendil-works/pi/tree/main/packages/coding-agent/examples/sdk
  - npm registry: `@earendil-works/pi-coding-agent` 0.85.1 / `@earendil-works/pi-ai` 0.85.1 / `@earendil-works/pi-agent-core` 0.85.1 / `@mariozechner/pi-coding-agent` 0.73.1（凍結）
