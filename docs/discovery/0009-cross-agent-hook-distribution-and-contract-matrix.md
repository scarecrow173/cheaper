---
id: "DISC-0009"
type: "discovery"
status: "resolved"
title: "Cross-agent hook distribution and per-agent hook contract matrix"
created: "2026-09-17"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/0013-cross-agent-distribution-verification.md"
    - "../adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md"
    - "../specs/0020-copilot-pre-read-hook-adapter.md"
    - "../adr/0023-copilot-adapter-separate-script-never-nonzero-exit.md"
  verifies:
    - "../designs/0013-apm-skills-and-pre-read-hook.md"
  verified-by: []
  references:
    - "https://github.com/openai/codex/issues/18491"
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by:
    - "./0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
metadata:
  discovery:
    question: "apm install が全 target へ deploy した pre_read_code hook は各 agent で機能するか。機能しない場合、native contract で pre-read blocking は実現可能か"
    scope: "本 repo への apm install 実 deploy 結果 + 各 agent 公式 hook ドキュメントの contract 調査"
    outcome: "non-windsurf では pre_read_code 登録は documented contract 上 no-op（runtime loader 実機未検証。DESIGN-0013 の想定通り）。Claude/Cursor/Copilot は adapter で実現可能、Codex は upstream gap で不可"
---

# DISC-0009: Cross-agent hook distribution and per-agent hook contract matrix

## Exploration Goal

DISC-0005 が「non-windsurf target への同 hook deploy 挙動（event name が
未知の target では no-op の見込み。Phase 4/5 で確認）」として deferred し、
DESIGN-0013 が「未知 event は no-op の想定。害はないが Phase 5 で実機確認」
と仮定していた論点を検証する。

具体的には (1) 本 repo の `apm install` が全 target へ実際に何を deploy
したか、(2) 各 agent の公式 hook contract で `pre_read_code` 相当の
pre-read interception が実現可能か、の 2 点を確定する。

## Confirmed Findings

### repo 内実測: apm install の全 target deploy

- `apm.yml` の targets: `copilot`, `codex`, `claude`, `cursor`, `windsurf`,
  `agent-skills`
- `apm install` は hook bundle を **全 target** へ deploy した:
  - `.windsurf/hooks/cheaper/`
  - `.codex/hooks/cheaper/`
  - `.cursor/hooks/cheaper/`
  - `.github/hooks/scripts/cheaper/`
  - `.claude/hooks/cheaper/`
- 各 bundle は同一の source（`pre-read-code.js` + `cli/`（bundled
  `cheaper.mjs`））を deploy するが、**copilot だけは `.json` asset
  がすべて欠落する**（`cli/version.json`・`cli/package.json`・
  `cli/workers/schemas/*.json`。後述の e2e 観測 facts 参照）
- 生成された per-target hook config（gitignore 対象、`apm install` が生成）:
  - `.windsurf/hooks.json` + `.windsurf/apm-hooks.json`
  - `.codex/hooks.json` + `.codex/apm-hooks.json`
  - `.cursor/hooks.json`（`"version": 1` を含む）+ `.cursor/apm-hooks.json`
  - `.github/hooks/cheaper-hooks.json`（`"version": 1`）
  - `.claude/apm-hooks.json` + `.claude/settings.json`
    （Claude 固有の nested `matcher`/`hooks` shape、
    `${CLAUDE_PROJECT_DIR}` 補間あり）
- **全 config が hook を Windsurf-native event name `pre_read_code` のまま
  登録している**。Claude / Codex / Cursor / Copilot 向け config でも
  event name は変換されず verbatim で書き込まれる
- e2e coverage:
  - windsurf: `packages/cli/test/e2e/apm-install.test.ts` が install +
    hook block の実動作を検証（skills deploy・hooks.json shape・
    bundled CLI `--version`・large file が exit 2 で block）。
    generated config の完全性（`apm-hooks.json` を含む）は
    `apm-install-targets.test.ts` の windsurf case もカバーする
  - 全 hook target + agent-skills:
    `packages/cli/test/e2e/apm-install-targets.test.ts` が
    install・deploy layout・generated config・bundled CLI capability
    を検証
  - non-windsurf の agent runtime loader 実機 E2E は未実施
- deployment drift の観測点: 本 repo 自身の `.windsurf/hooks.json` の
  `powershell` entry には `; exit $LASTEXITCODE` が無いが、package
  template の `.apm/hooks/hooks.json` には含まれる。dogfooded deployment
  は powershell fix 以前に deploy された stale な状態であり、template
  との drift データポイントとして記録する

### 外部 contract 調査（公式ドキュメント実読）

#### Windsurf Cascade

出典: <https://docs.devin.ai/desktop/cascade/hooks>

- event `pre_read_code` を `.windsurf/hooks.json` に登録
- stdin: `{agent_action_name, tool_info: {file_path}}`
- exit 2 で block。stderr は agent へ見せられる。他の exit は fail-open

#### Claude Code

出典: <https://code.claude.com/docs/en/hooks>

- event `PreToolUse` + `matcher: "Read"` で file read を intercept 可能
- config は `.claude/settings.json`（event → matcher → handlers の
  nested shape）
- stdin: `hook_event_name`, `tool_name`, `tool_input.file_path`
- block は exit 2（stderr → model）または stdout JSON
  `hookSpecificOutput.permissionDecision: "deny"`

#### OpenAI Codex CLI

- hooks は存在する（`~/.codex/hooks.json`, `.codex/hooks.json`,
  `config.toml` の `[hooks]`）。Claude-compatible な `PreToolUse` と
  `permissionDecision` stdout contract を持つ
- **しかし** upstream issue によると `PreToolUse` を emit するのは
  `shell`/`unified_exec`（`tool_name: "Bash"` として）、
  `apply_patch`、MCP handler のみ。file-read tool は event を emit
  **しない** — `read_file` / `grep` の gap は `openai/codex#18491`
  （open）、`list_dir` / `view_image` は `openai/codex#20204` が
  直接の evidence
- 現状 upstream で pre-read interception は **不可能**

#### Cursor

出典: <https://cursor.com/docs/hooks>

- 専用 event `beforeReadFile` を `.cursor/hooks.json`（`version: 1`、
  fields は `command`, `matcher`, `failClosed`）に登録
- stdin は `file_path` に加え file `content` も含む
- block は stdout `{permission: "deny"}` または exit 2。
  `failClosed: true` でない限り fail-open
- 汎用 event `preToolUse` も存在
- Claude-format hook を `.claude/settings.json` から auto-load し、
  `PreToolUse` → `preToolUse` へ mapping する

#### GitHub Copilot（CLI + cloud coding agent）

出典: <https://docs.github.com/en/copilot/concepts/agents/hooks> +
hooks-reference

- event `preToolUse` を `.github/hooks/*.json`（`version: 1`）に登録
- stdin: `toolName` / `toolArgs`。**`toolArgs` の型は公式 doc 間で
  揺れる**: hooks reference は camelCase `preToolUse` payload を
  `toolArgs: unknown`（object として渡される）と定義する一方、
  PascalCase `PreToolUse` payload は `tool_input: unknown`
  （JSON 文字列を parse）であり、Copilot CLI hooks tutorial は
  `toolArgs` を JSON 文字列として説明する。adapter は `string |
  object` の両方を normalize しなければならず、string +
  `JSON.parse` を前提にしてはならない
- `toolName == "view"` に match させると file read を intercept できる
- block は stdout `{permissionDecision: "deny", permissionDecisionReason}`
- **CRITICAL**: command `preToolUse` hook は **fail-closed** — 非 0 exit は
  すべて tool call を deny する。timeout は fail-open

#### agent-skills target

- skills-only の primitive であり hook の概念を持たない

## Analysis

### deploy された `pre_read_code` 登録の実効性

Claude / Codex / Cursor / Copilot 向け config に書き込まれた
`pre_read_code` 登録は **no-op** である — これは documented event
contract に基づく推論であり、実 agent の runtime loader での
accept/ignore 挙動は実機 E2E 未検証。各 agent はこの event name を
認識しないため hook は一度も発火しないはず。無害だが機能もしない —
DESIGN-0013 の「未知 event は no-op の想定」と整合する（per-target
config validation の実機確認は open item として残る）。

### target 別 pre-read blocking 実現性 matrix

| target | native pre-read event | 実現性 | 必要な適合 |
|---|---|---|---|
| Windsurf | `pre_read_code` | ✅ | 検証済み・現行のまま動作 |
| Claude Code | `PreToolUse` + `matcher: "Read"` | ✅ | event/matcher 変換 + stdin shape adapter |
| Cursor | `beforeReadFile` | ✅ | event + output contract adapter |
| Copilot | `preToolUse` + `toolName == "view"` | ✅ adapter 実装済み | `pre-read-code-copilot.js`（DESIGN-0025）。stdout deny JSON + 常に exit 0 で fail-closed 意味論を吸収 |
| Codex | （read tool は `PreToolUse` を emit しない） | ❌ | upstream gap。現状 intercept 不可 |
| agent-skills | hook 概念なし | N/A | skills のみの target |

### Final per-target classification（TASK-0087 e2e で確定、TASK-0088 で close）

Evidence: `packages/cli/test/e2e/apm-install-targets.test.ts`（TASK-0087。
apm 0.28.0 に対する consumer install smoke — bundle layout・generated
config・`pre_read_code` verbatim 登録・bundled CLI 実行を assert）と、
windsurf については既存 `packages/cli/test/e2e/apm-install.test.ts`
（hook が large file を exit 2 + delegation guidance で実際に block
する実動作確認済み）。本表が per-target contract の **canonical
matrix** であり、consumer 向け文書からは参照のみを張る。matrix 内容を
README・skill・spec 等へ複写しない（DRY — SPEC-0013 / DESIGN-0018 §4）。

| target | final classification | evidence |
|---|---|---|
| windsurf | **functional** | `apm-install.test.ts` — hook block 実動作確認済み（exit 2 + delegation guidance） |
| claude | **deploys-but-nonfunctional (expected no-op)** | `apm-install-targets.test.ts` — `pre_read_code` を verbatim 登録。native event は `PreToolUse` + matcher `Read` であり `pre_read_code` は発火しない。`preToolUse` 登録は PascalCase `PreToolUse` に正規化され live になるが、copilot adapter は claude payload shape と不一致で全 exit 0 素通し（実質 no-op） |
| cursor | **deploys-but-nonfunctional (expected no-op)** | 同上 — `pre_read_code` verbatim 登録。native event は `beforeReadFile`（`failClosed` 未指定時は fail-open） |
| copilot | **functional（adapter 経由）。配布物は一部破損（`.json` 欠落、upstream 待ち）** | 同上 — `preToolUse` 登録が native に発火し `pre-read-code-copilot.js` が動作（stdin 注入 e2e で block/allow/never-nonzero を assert、generated config の `preToolUse` 登録も assert）。`.json` 全欠落で `--version` は exit 70、file-schema worker（bulk-read / code-write / doc-write）は `OUTPUT_SCHEMA_NOT_FOUND`（exit 3）で不動のまま（既知の制限、ADR-0023 option i で upstream 修正待ち + drift-signal assert 維持）。`count-tokens` は動作するため adapter 自体は `.json` 非依存 |
| codex | **deploys-but-nonfunctional (expected no-op) かつ unsupported** | 同上 — file-read tool が `PreToolUse` を emit しない upstream gap（`read_file`/`grep` は openai/codex#18491、`list_dir`/`view_image` は #20204）。adapter 対象外 |
| agent-skills | **N/A（skills-only）** | 同上 — 4 skill の `SKILL.md` のみ deploy され、hook bundle / hook config の不存在を assert |

注: 「expected no-op」の分類は generated config の静的検証と各
agent の documented event contract からの推論であり、実 agent の
runtime loader が未知 event を accept/ignore する挙動は実機 E2E
未検証（実 binary を実行していない）。観測 metadata: e2e は
**apm 0.28.0** に対する実測、外部 contract 調査は **2026-09-17**
時点の公式 doc に基づく — upstream の contract drift はあり得る。

#### e2e run で新たに観測された deploy facts（apm 0.28.0）

- **copilot**: `apm install --target copilot` は
  `.github/hooks/scripts/<dep>/` への deploy からすべての `.json`
  file を除外する。`cli/version.json`、`cli/package.json`、
  `cli/workers/schemas/*.json` が欠落するため、bundled
  `cheaper.mjs --version` は "failed to resolve package version"
  （exit 70）で失敗し、file schema を参照する worker
  （bulk-read / code-write / doc-write）は worker definition load
  時に `OUTPUT_SCHEMA_NOT_FOUND`（exit 3、schema は output →
  input 順に load されるため output 側を先に検出）で失敗する。
  `count-tokens` は動作し、worker の `.md` 本体は deploy される。
  いずれも `apm-install-targets.test.ts` の copilot case で
  assert 済み。copilot adapter（pre-read-code-copilot.js）は
  `count-tokens` のみを使うため `.json` 欠落の影響を受けない。
  `--version` と file-schema worker の破損は upstream 修正待ち
  の既知の制限として残る（ADR-0023）
- **codex / cursor / windsurf**: apm は target の `hooks.json`
  （`hooks` 直下に event を nest する shape）に加え、同じ
  `pre_read_code` 登録の top-level event key shape を持つ
  `apm-hooks.json` を追加生成する（`.codex/apm-hooks.json`、
  `.cursor/apm-hooks.json`、`.windsurf/apm-hooks.json`。いずれも
  `apm-install-targets.test.ts` で assert 済み）
- **claude**: `.claude/apm-hooks.json` と `.claude/settings.json`
  の双方を deploy する（nested `hooks → event → matcher → hooks`
  shape、`${CLAUDE_PROJECT_DIR}` 補間はそのまま保持）。**claude
  adapter は event 名を native casing に正規化する**: hooks.json の
  `preToolUse`（camelCase）は `PreToolUse`（PascalCase、matcher `*`）
  として live 登録される（2026-09-18 実測、copilot adapter 追加時に
  観測）。copilot adapter は claude payload（`tool_name` /
  `tool_input`）と contract が合わないため全 exit 0 素通しで
  実質 no-op

### Copilot の fail-closed 意味論反転

現行 hook の contract は fail-open（測定不能・想定外入力では exit 0 で
素通し）を前提とする。Copilot の command `preToolUse` hook は
**非 0 exit がすべて deny** になる fail-closed 意味論を持つため、
generic adapter は「意図的 block 以外で絶対に非 0 exit しない」
ことを保証しなければならない。hook の fail-open contract が
**反転**する点が最大の設計リスク。

### Alternatives and Comparison

| Option | Pros | Cons | Lean |
|---|---|---|---|
| per-target native hook config + stdin/stdout adapter を APM package が生成 | 各 agent で実際に機能する hook | adapter は event・stdin shape・block contract・fail 意味論の全 axis を吸収する必要あり。Copilot は exit code 意味が反転 | yes（要 spec） |
| 現状維持（windsurf のみ hook、他は skill 任せ） | 変更不要・無害 | non-windsurf では pre-read safety net が存在しない。deploy される config は dead weight | maybe（短期） |
| Codex 向けに独自 patch / upstream 修正待ち | 理論上は全 target 統一 | upstream issue open で read event 自体が無い。側道も現状なし | no |

## Tentative Conclusions

- （confirmed: e2e 実測）`apm install` は hook bundle を全 target に
  機械的に deploy するが、event name の変換は行わない
- （inferred: documented contract ベース、runtime loader 実機未検証）
  `pre_read_code` のままの登録は non-windsurf target では no-op で、
  害はないが pre-read blocking としても機能しないはず
- （confirmed）pre-read interception は Windsurf・Claude Code・Cursor・
  Copilot で native contract により実現可能。Codex は upstream の
  event emit gap により現状不可能。agent-skills は対象外
- （hypothesis）cross-agent 対応するなら、単一 script + per-agent
  stdin/stdout 変換 adapter が必要。event name・stdin shape
  （`tool_info.file_path` / `tool_input.file_path` / `file_path` /
  `toolArgs` — doc 間で `string | object` の揺れがあり normalize
  要）・block 表現（exit 2 / stdout JSON /
  `permissionDecision` / `permission`）・fail 意味論（fail-open /
  fail-closed / `failClosed` flag）の 4 軸が target 毎に異なる

## Open Questions

- 各 agent の hook config で認識されない event name が warning / error を
  出すか（per-agent 実機 validation が必要。no-op は doc + deploy
  証拠で支持されるが runtime loader の accept/ignore 自体は実機
  未検証で、startup での noise も未検証）
- ~~Copilot fail-closed の下で adapter が「意図的 block 以外で非 0 exit
  しない」をどう保証するか~~ → ADR-0023 / DESIGN-0025 で解決: adapter
  は全経路で exit 0、block のみ stdout deny JSON で表現（stdin 注入
  e2e で never-nonzero を assert）
- hook script の per-agent stdin/stdout 変換をどの層に置くか
  （bundle 内 wrapper script / generated config 差し替え /
  APM package 側の manifest 拡張）
- `.windsurf/hooks.json` の deployment drift（powershell の
  `; exit $LASTEXITCODE` 欠落）をどう是正するか。再 install で
  template が再 deploy されるか、generated config の更新挙動の確認が必要

## Promotion Candidates

- [x] spec-doc needed: cross-agent hook adapter の contract
  （event 変換表・stdin/stdout mapping・fail 意味論の扱い・Codex 除外根拠）
  → SPEC-0013（`docs/specs/0013-cross-agent-distribution-verification.md`）
- [x] adr-doc needed: non-windsurf target への hook 提供方針
  （adapter 実装 vs skill のみ vs target 限定 deploy）
  → ADR-0016（`docs/adr/0016-keep-verbatim-pre-read-code-as-noop-on-non-windsurf-targets.md`）

## Sources and Relations

- DISC-0005: 本検証を「Phase 4/5 で確認」として deferred した upstream
- DESIGN-0013: 「未知 event は no-op の想定。害はないが Phase 5 で実機確認」
  の仮定を置いた設計。本 discovery がその検証記録
- Windsurf Cascade hooks: <https://docs.devin.ai/desktop/cascade/hooks>
- Claude Code hooks: <https://code.claude.com/docs/en/hooks>
- Cursor hooks: <https://cursor.com/docs/hooks>
- GitHub Copilot hooks: <https://docs.github.com/en/copilot/concepts/agents/hooks>
- Codex PreToolUse emit gap: GitHub issue `openai/codex#18491`
  （`read_file` / `grep`）、`openai/codex#20204`
  （`list_dir` / `view_image`）
