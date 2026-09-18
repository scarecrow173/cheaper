---
id: "SPEC-034r05iHK8bA7g59V25ltC"
type: "spec"
status: "implemented"
title: "cheaper V1 Worker Runtime と CLI"
created: "2026-09-15"
updated: "2026-09-17"
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
  related:
    - "../adr/agent-runtime-pi-sdk-earendil-works-pi-coding-agent.md"
    - "../adr/worker-yaml-frontmatter-markdown.md"
    - "../adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "../adr/one-shot-json-first-cli.md"
    - "../adr/model-tier-model-resolver.md"
    - "../adr/wrapper-measured-deterministic-metadata.md"
    - "../adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "../adr/typescript-pnpm-monorepo.md"
  refines: []
  refined-by:
    - "bulk-reader-built-in-worker.md"
    - "cheap-writer-built-in-worker.md"
  derives-from:
    - "../discovery/cheaper-v1-pi-sdk.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper V1 Worker Runtime と CLI

## What Should Be Built

親 Coding Agent（Claude Code / Codex / Copilot / Cursor 等）からの委譲を受け、宣言的に定義された Worker を安価なモデル上で安全かつ決定論的に実行する汎用 Worker Runtime と、その唯一の公開インターフェースである JSON-first one-shot CLI を構築する（引継ぎ §1, §2, §4, §5）。

## Why Now

frontier model のコストを抑えるため、巨大コンテキスト処理や機械的コード生成を安価なモデルへ逃がす必要がある（§1）。設計は引継ぎで確定済みであり、実装着手の根拠となる spec をここで固定する。

## Users and Value

- 親 Coding Agent: CLI contract のみを認識し、安価な委譲先を得る（§2.1）
- 親 Agent を使う開発者: frontier model の消費を削減できる
- Worker 作者: Runtime のソースを変更せず Worker Definition + Output Schema を追加するだけで新 Worker を作れる（§44）

## Scope

### In Scope

- Process model: one-shot process、daemon/server 化しない。request → Worker resolve → validation → Policy resolve → model resolve → session 作成 → 実行 → output validation/repair → filesystem/usage 計測 → structured result → disposal → exit（§4）
- CLI contract: `cheaper run <worker>`、エイリアス `cheaper bulk-read` / `cheaper cheap-write`。task input は JSON（stdin または `--input`）。共通制御: `--input`, `--cwd`, `--timeout`, `--format json|text`, `--quiet`, `--help`, `--version`（§5）
- stdout/stderr 分離: stdout は最終 result envelope のみ（デフォルト JSON、TTY 自動切替なし）。stderr は warnings/diagnostics/errors/progress（§6）
- cwd / workspace: `cwd` = workspace root = filesystem security boundary = project configuration scope。canonicalize 必須、祖先探索しない（§7）
- Worker Definition: YAML frontmatter + Markdown。必須: `name`, `description`, `tools`, `cheaper.model.tier`, `cheaper.output.schema`, Markdown body（全て non-empty）（§8）。name は `^[a-z0-9][a-z0-9-]*$`、ファイル名と一致必須（§9）。portable tool vocabulary `read/grep/find/ls/write/edit` と安全な alias のみ（§10）。`tools` は capability 要求であって permission grant ではない（§11）。`cheaper:` namespace は strict（`cheaper.model`/`cheaper.output` のみ）（§12）。model tier `low/medium/high`（§13）。output schema は `file`/`inline` の exactly one、相対 path は Worker Definition ファイル基準、形式 `.json/.yaml/.yml`、remote schema 対象外（§15）。top-level 未知 field は許容し ignore（§16）
- Worker discovery/resolution: `<cwd>/.cheaper/workers/` > `<user-config>/cheaper/workers/` > built-in の whole-file shadowing（§17）。invalid project Worker は fallback せずエラー（§18）。`--worker-file` で named resolution を完全 bypass（§19）
- Runtime Policy: 全 Worker 共通の単一 policy。built-in（`version: 1` 必須、execution/tools/filesystem/model/output/retry 各セクション）+ user + project の partial override merge + CLI restriction（narrow のみ）（§20–§30）。merge 規則: object deep merge / scalar replace / array replace（§22）。各 source で validation、invalid は silent ignore しない（§23）
- Runtime Invariants: policy から変更不能な制約（workspace 内 access のみ、symlink escape 禁止、shell なし、network tool なし、unknown tool 禁止、output schema validation 必須、CLI/Worker による権限拡張禁止）（§31）
- Error model / exit codes: §32 の error code 分類と §33 の exit code 分類（0/2/3/4/5/6/7/8/9/70）
- Abort: SIGINT/SIGTERM/timeout で abort。rollback せず、mutation 済みなら result/error で明示（§34）
- Usage: provider/model/input/output/cached tokens/duration/turn count/tool count をローカルで取得（§35）

### Out of Scope

- §39 V1 non-goals 全て: daemon, Web UI, REST API, MCP Server, Redis, PostgreSQL, Queue, Kubernetes, distributed workers, central telemetry, vector DB, semantic cache, multi-agent orchestration, Worker inheritance/composition/merge, prompt imports, remote Workers, remote schemas, Worker-specific Runtime Policy, Worker-defined timeout/maxTurns/model/provider/credentials/shell/network, shell tool, network tool, automatic Git rollback, whole-Worker automatic retry
- APM integration（§36–37）: V1 成功条件に含まれないため別 spec に deferred

## Requirements

- Worker Definition の parse/validate/resolve が CLI・Runtime と独立に機能すること
- Runtime Policy が built-in/user/project/CLI restriction の順に merge され、全段階で検証されること
- Runtime Invariants が policy や CLI で緩められないこと
- 決定論的メタデータ（changed files, diff, usage 等）を LLM 自己申告に頼らないこと
- 実装技術（ランタイム基盤・言語）は ADR に委ね、本 spec では挙動のみを規定する

## Acceptance Criteria

V1 成功条件（§38）を検証可能な形で:

- [ ] `cheaper bulk-read` を任意 repo に対して実行したとき、read-only 分析結果が structured result として stdout に JSON で返る
- [ ] 親 Agent へ返るのが短い structured result のみで、ランタイム内部イベントやログが stdout に混入しない
- [ ] Worker を Markdown ファイルとして追加したとき、Runtime ソース変更なしで `cheaper run <worker>` が動く
- [ ] project Worker が存在するとき user/built-in より優先して解決される
- [ ] project Worker が invalid のとき、user/built-in へ fallback せず `INVALID_WORKER_DEFINITION` で exit 3
- [ ] 利用可能なモデル一覧から tier 分類・選択が行われ、user override で分類を変更できる
- [ ] `cheap-write` 実行で workspace が直接変更され、changed files/diff 統計が wrapper 実測で result に含まれる
- [ ] Runtime Policy が built-in/user/project で merge され、invalid な policy が silent ignore されない
- [ ] workspace 外への filesystem access が常に拒否される
- [ ] Worker 宣言 schema に適合しない output が schema validation で検出される
- [ ] 失敗時に non-zero exit code と structured error が返る
- [ ] 親 Coding Agent 固有機能・ランタイム内部詳細が親 Agent へ露出しない

テストカテゴリ（§42）の各 1 行シナリオ（error code / exit code は §32–§33 の分類に従う）:

- [ ] Worker resolution: project Worker が valid のとき user/built-in を shadow する / project Worker が invalid のとき `INVALID_WORKER_DEFINITION` exit 3 で fallback しない / `--worker-file` 指定のとき named resolution を完全 bypass する / `name` とファイル名が不一致のとき `WORKER_NAME_MISMATCH` exit 3 / parent directory は探索されない
- [ ] Worker validation: 必須 field 欠落・空値のとき `INVALID_WORKER_DEFINITION` exit 3 / `cheaper:` 内未知 field のとき `INVALID_WORKER_DEFINITION` exit 3 / 未知 tool のとき `UNKNOWN_TOOL` で fail closed / alias（`Read`→`read` 等）は正規化される / `file`/`inline` schema が両方または無いとき `INVALID_WORKER_DEFINITION` / schema ファイル欠落は `OUTPUT_SCHEMA_NOT_FOUND` / 不正 JSON Schema は `INVALID_OUTPUT_SCHEMA`
- [ ] Runtime Policy: built-in policy が完全 schema を満たす / user・project の partial merge で deep merge・scalar replace・array replace が適用される / 未知 field・不正型は `INVALID_RUNTIME_POLICY` exit 3 / `--timeout` 等の CLI が policy を narrow するとき受理・expand しようとするとき `POLICY_VIOLATION` exit 4 で silent clamp しない
- [ ] Security: `../`・絶対パスによる workspace escape、workspace 外 symlink target、workspace 外への write/read、未知 tool が全て拒否される（`POLICY_VIOLATION` exit 4 または `TOOL_EXECUTION_FAILURE` exit 6）
- [ ] Execution: timeout で `TIMEOUT` exit 7 / abort で `ABORTED` exit 8 / maxTurns・maxToolCalls 超過で打ち切り / 要求 tier が利用不可で `MODEL_TIER_UNAVAILABLE` exit 5 / `tierFallback: higher` で上位 tier のみ探索し maxTier 超過は `POLICY_VIOLATION` / transient failure は `transientAttempts` 回だけ retry / invalid output は `repairAttempts` 回 repair し repair 中は tools 無効

## Deferred Design Concerns

- 正式な JSON Error Contract（error detail fields・retryable semantics・exit code マッピング詳細） | 理由: §40 未確定事項 | 再着手: design-doc | 無視時リスク: error 形式の後方互換
- Usage 記録のローカル形式 | 同上 | design-doc | telemetry 形式の揺れ
- APM integration（Hook contract、各 Skill） | V1 成功条件外 | 別 spec 作成時 | V1 スコープの曖昧化

## Sources and Relations

- 一次ソース: `docs/handoff/2026-09-15-cheaper-v1-implementation-handoff.md`
- 上流: DISC-034r05iHy2gdnp377SlYbj（`docs/discovery/cheaper-v1-pi-sdk.md`）
- 関連 ADR: ADR-034r05iHfSMW9SGV4dpcy2〜0008（`docs/adr/`）— 実装技術（Pi SDK, TypeScript）は各 ADR を参照
- 下流: SPEC-034r05iHK8bA7g6PcCiDBp, SPEC-034r05iHK8bA7g7YRKGyQr が本 spec を refines
