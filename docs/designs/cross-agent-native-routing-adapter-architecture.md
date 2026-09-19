---
id: "DESIGN-034rA79Uog7JOa7evw6rOL"
type: "design"
status: "approved"
title: "Cross-agent native routing adapter architecture"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./apm-skills-and-pre-read-hook.md"
    - "./copilot-pre-read-hook-adapter.md"
    - "../specs/shell-read-large-file-blocking.md"
    - "../adr/conservative-shell-read-candidate-extraction-inside-the-shared-routing-policy.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  design:
    kind: "detail"
---

# Cross-agent native routing adapter architecture

## Context

APM package の `.apm/hooks/` bundle は全 target に verbatim deploy される。
各 Coding Agent は独自の hook event 名・stdin payload・block 表現・fail
semantics を持つため、bundle 側で native contract の差分を adapter が
吸収し、large-read 判定は target 非依存の shared module に集約する。

対象 event と deny 表現（DISC-034rA43JiB611pUjLPA4yR で再検証）:

| target | file-read event / payload | shell event / payload | deny 表現 | fail semantics |
| --- | --- | --- | --- | --- |
| windsurf | `pre_read_code` / `tool_info.file_path` | `pre_run_command` / `tool_info.command_line` | stderr guidance + exit 2 | infra failure は fail-open（exit 0） |
| claude | `PreToolUse` / `tool_name:"Read"`, `tool_input.file_path` | 同左 / `Bash`・`PowerShell`, `tool_input.command` | stdout `hookSpecificOutput.permissionDecision:"deny"` + exit 0 | malformed は fail-open |
| cursor | `beforeReadFile` / `file_path` | `beforeShellExecution` / `command`,`cwd` | stdout `{permission:"deny",...}` + exit 0 | exit 0 は常に schema 適合応答 |
| copilot | `preToolUse` / `toolName:"view"`, `toolArgs.{path,file_path,filePath}` | 同左 / `bash`・`powershell`・`shell`, `toolArgs.command` | stdout `{permissionDecision:"deny",...}` + exit 0 | never-nonzero（ADR-034r05iHrZUE6WizQ4LgPa） |
| codex | unsupported（upstream gap: file-read tool が PreToolUse を emit しない） | `PreToolUse` / `tool_name:"Bash"`, `tool_input.command` | stdout `hookSpecificOutput.permissionDecision:"deny"` + exit 0 | malformed は fail-open |

## Scope

### In Scope

- `packages/apm/.apm/hooks/routing-policy.js` — canonical request と
  shared policy（large-read 判定 + shell seam）を export する CJS module。
- `packages/apm/.apm/hooks/` 配下の adapter script:
  - `pre-read-code.js`（windsurf file-read、既存の判定を policy 呼出しへ移譲）
  - `pre-run-command.js`（windsurf shell、新規）
  - `pre-read-code-pretooluse.js`（claude + codex 共用、新規）
  - `pre-read-code-cursor.js`（cursor file-read + shell、新規）
  - `pre-read-code-copilot.js`（copilot、shell tool 対応を追加して統合）
- `hooks.json` の event 登録更新。
- adapter contract test + `apm-install-targets.test.ts` の per-target
  assertion 拡張。
- contract matrix / parent-agent guide の更新。

### Out of Scope

- #42 の shell read candidate 抽出 parser 本体。
- 各 agent binary での runtime loader 実機検証。
- Copilot `.json` asset 欠落の upstream 修正。
- Codex file-read interception（upstream gap が解消されるまで不可）。

## Components and Boundaries

### Canonical routing request

```js
// routing-policy.js が解釈する唯一の request 型
{ kind: "file-read", path: string, cwd: string, sourceTool?: string }
{ kind: "shell", command: string, cwd: string, sourceTool?: string }
```

`sourceTool` は診断用 metadata であり、policy の分岐には使わない。

### `routing-policy.js`（shared policy module）

責務: canonical request → verdict。export は以下:

- `evaluate(request, opts)` → `{decision:"allow"}` |
  `{decision:"deny", reason}`。`opts.cli`（adapter の argv[2]）と
  `opts.env` を受け、CLI 解決は argv[2] > `CHEAPER_CLI` > PATH の既存
  順序を維持する。
  - `file-read`: path resolve → workspace 内判定（`rel === ""` /
    `..` / `../` prefix / absolute rel は outside → allow）→ stat（非
    file / 不在 → allow）→ fast path（`st.size <= threshold` → allow、
    CLI spawn しない）→ `count-tokens <resolved> --cwd <cwd>` →
    `FILE_TOO_LARGE` envelope → deny（fail-closed）、`tokens > limit` →
    deny、その他の envelope/infra failure → allow。
  - `shell`: 現時点では常に `{decision:"allow"}` を返す seam。#42 が
    ここに command からの read candidate 抽出を接続する。
- `formatDenyReason(detail)` → deny 理由文字列を単一箇所で生成する
  （`tokens` / `bytes` / `measurement cap` / `workspace-relative path` /
  `bulk-read` 委譲 guidance の文言は既存 test が assert する文字列を
  維持する）。
- `threshold(env)`、`cheaperCommand(cli)` の既存 helper をここへ集約。

### Adapter（責務は parse / normalize / response 変換のみ）

1. `pre-read-code.js`（windsurf file-read）: `agent_action_name ===
   "pre_read_code"` → `tool_info.file_path` → file-read evaluate。
   deny → `reason` を stderr + exit 2。それ以外は exit 0。
2. `pre-run-command.js`（windsurf shell）: `agent_action_name ===
   "pre_run_command"` → `tool_info.command_line` → shell evaluate。
   deny → stderr + exit 2、それ以外は exit 0（現状 policy は shell を
   常に allow するため実質常に exit 0。構造のみ提供）。
3. `pre-read-code-pretooluse.js`（claude + codex）: `{tool_name,
   tool_input}` → `Read` は file-read、`Bash`/`PowerShell` は
   `tool_input.command` で shell。deny → stdout に
   `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":<reason>}}`
   + exit 0。malformed / unrelated tool / 欠落 field は exit 0 silent。
   claude と codex は同一 payload shape のため共用（codex では matcher
   `Read|Bash|PowerShell` のうち `Bash` のみ発火する）。
4. `pre-read-code-cursor.js`: payload key で event を識別 —
   `file_path` があれば file-read、`command` があれば shell。
   deny → stdout `{"permission":"deny","user_message":<reason>,
   "agent_message":<reason>}` + exit 0。allow / malformed を含む全
   経路で schema 適合応答 `{"permission":"allow"}` + exit 0
   （cursor の permission hook は exit 0 + schema 不適合を block 扱い
   するため空出力は禁止）。
5. `pre-read-code-copilot.js`: `toolName === "view"` → file-read、
   `toolName ∈ {bash, powershell, shell}` → `toolArgs.command` で
   shell（`toolArgs` の object/string 揺れは既存 normalize を再利用）。
   deny → stdout `{"permissionDecision":"deny",...}` + exit 0。
   never-nonzero invariant は維持。parsed payload が foreign shape
   （`toolName` 欠落で `tool_name`/`command`/`file_path` 等を持つ）の
   場合のみ `{"permission":"allow","permissionDecision":"allow"}` を
   返して cross-fire（cursor `preToolUse` で誤発火した場合等）を
   schema 適合 allow で吸収する。copilot payload の非 deny 経路は
   従来どおり silent exit 0。

### `hooks.json` event 登録

```json
{
  "hooks": {
    "pre_read_code":   [{ "command": "node ./pre-read-code.js ...", "powershell": "...", "show_output": false }],
    "pre_run_command": [{ "command": "node ./pre-run-command.js ...", "powershell": "...", "show_output": false }],
    "preToolUse":      [{ "command": "node ./pre-read-code-copilot.js ...", "powershell": "...", "show_output": false,
                          "matcher": "view|bash|powershell|shell" }],
    "PreToolUse":      [{ "matcher": "Read|Bash|PowerShell",
                          "hooks": [{ "type": "command",
                                      "command": "node ./pre-read-code-pretooluse.js ..." }] }],
    "beforeReadFile":      [{ "command": "node ./pre-read-code-cursor.js ...", "powershell": "...", "show_output": false }],
    "beforeShellExecution":[{ "command": "node ./pre-read-code-cursor.js ...", "powershell": "...", "show_output": false }]
  }
}
```

- APM は flat entry の `command`/`powershell`/`matcher`/`show_output` と
  nested group の `{matcher, hooks:[{type,command}]}` を verbatim
  pass-through する（DISC-034rA43JiB611pUjLPA4yR で実測）。
- claude target では flat `preToolUse` が `PreToolUse`+matcher `*` へ
  正規化される。copilot adapter の matcher を lowercase
  `view|bash|powershell|shell` に絞ることで、claude（`Read`/`Bash`
  capitalized tool 名）・cursor（`Shell` 等）での誤発火を防ぐ。
  matcher が case-insensitive でも adapter 側の payload shape 判定が
  no-op に倒す。
- codex / cursor / windsurf では `PreToolUse`（PascalCase）・
  `beforeReadFile` 等の foreign event は未認識 event として no-op。
  windsurf のみ `pre_run_command` が実動し、non-windsurf では no-op
  event として残る（既存 `pre_read_code` の no-op 維持と同じ扱い）。

## Data and Control Flow

```text
native target stdin payload
  → adapter script（event 識別 + field 抽出 + normalize）
  → canonical request {kind, path|command, cwd, sourceTool?}
  → routing-policy.evaluate()
      file-read: path/stat/fast-path → count-tokens → allow|deny
      shell:     （seam、現状 allow）
  → adapter が verdict を native response へ変換
      windsurf: exit 2 + stderr / exit 0
      claude+codex: hookSpecificOutput deny JSON / silent exit 0
      cursor:  {permission:"deny"} / {permission:"allow"}、常に exit 0
      copilot: {permissionDecision:"deny"} / silent、常に exit 0
```

## Non-Functional Constraints

- 新規 dependency 禁止。adapter / policy は `node` builtin のみの CJS。
- deny guidance は executable shell command を含めない（既存契約）。
- `CHEAPER_READ_TOKEN_THRESHOLD` / `CHEAPER_CLI` /
  `CHEAPER_COUNT_MAX_BYTES` の env contract を維持する。
- APM verbatim deploy を維持し、target ごとの config patch / 独自
  installer を作らない。
- Copilot `.json` asset 欠落は upstream 制限として drift assert を維持。

## Risks and Trade-offs

- foreign event の誤発火: `preToolUse` matcher + payload shape 判定で
  吸収。cursor permission hook の空出力禁止には foreign-shape allow
  応答で対応。
- Codex `PreToolUse` deny の解釈は runtime 実機未検証（公式 doc に
  `systemMessage` と unsupported field での hook failure は明記）。
  best-effort deny + fail-open で受け入れ、matrix に記録する。
- `hooks.json` に 6 event 登録されるため、non-native target では
  no-op entry が増える。generated config に残る stale entry は
  unavoidable であり、e2e assert で記録する。

## Deferred Design Concerns

- #42: shell command 内 read candidate 抽出（`cat`/`head`/`tail` 等）。
  canonical shell request が seam として用意済み。再着手トリガは
  issue #42 の着手。
- Codex file-read: upstream（openai/codex#18491・#20204）が close され
  read 系 tool が `PreToolUse` を emit した時点で adapter matcher に
  tool 名を追加するだけで対応可能。
- 実機 E2E: 各 agent binary の runtime loader 検証。従来どおり generated
  config + stdin 注入 contract test で代替する。

## Verification Notes

- [ ] 全 adapter が representative payload fixture で normalize /
      deny / fail-open を検証する contract test を持つ。
- [ ] `apm install --target <t>` の generated config で各 target の
      native event 登録・adapter command・bundled CLI path を assert する。
- [ ] Copilot adapter の全 code path が exit 0 であることを test が
      保証する（never-nonzero invariant）。
- [ ] Windsurf 既存 test（`pre-read-hook.test.ts`）が regression なく
      pass する。
- [ ] `mise run check` が通る。
