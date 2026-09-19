---
id: "DISC-034rA43JiB611pUjLPA4yR"
type: "discovery"
status: "resolved"
title: "Cross-agent native routing adapter contract re-verification"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/43"
    - "https://docs.devin.ai/desktop/cascade/hooks"
    - "https://docs.github.com/en/copilot/reference/hooks-configuration"
    - "https://developers.openai.com/codex/hooks"
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
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
    - "./shell-read-large-file-detection-boundary.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
    - "https://github.com/openai/codex/issues/18491"
    - "https://github.com/openai/codex/issues/20204"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "issue #43 の cross-agent native routing adapter は各 target の現行 native hook contract で実現可能か。APM の hook entry pass-through でどこまで native schema を表現できるか"
    scope: "Windsurf / Claude Code / Cursor / Copilot / Codex の公式 hook contract 再検証 + openai/codex source inspection + APM generated config の実測"
    outcome: "Windsurf・Claude Code・Cursor・Copilot は native contract で file-read + shell を adapter 化可能。Codex は shell（PreToolUse matcher Bash）のみ到達可能で file-read は upstream gap で unsupported。APM は matcher / nested hooks group を verbatim pass-through するため独自 installer 不要"
---

# Cross-agent native routing adapter contract re-verification

## Exploration Goal

issue #43「APM 配布を維持した cross-agent native routing adapter」の実現性を
判断するため、各 target の現行 native hook contract（event 名・payload・
block 表現・fail semantics）を再検証し、APM の generated config が
per-target native schema を表現できるかを実測で確認する。
DISC-034r05iHy2gdnpdai9KGpW（前回 matrix）の Codex「unsupported」分類は
upstream 更新で陳腐化している可能性があり、再検証が必須要件である。

## Key Issues and Assumptions

- 各 target で「file read を intercept する pre-hook」と「shell command を
  intercept する pre-hook」が存在し、canonical request に必要な field
  （path / command / cwd）を payload が持つこと。
- APM は `hooks.json` の entry field（`matcher`、nested `hooks` group、
  `type` 等）を generated config へ verbatim pass-through するか。
- fail-closed target（Copilot）では intentional deny と infrastructure
  failure を分離できる block 表現があること。
- Codex: 2026-09-19 時点で `PreToolUse` の tool coverage が拡大されている
  が、file-read tool が `PreToolUse` を emit するかは別問題。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| canonical request + shared policy + per-target adapter | 判定の単一化、#42 seam、target 追加が薄い | adapter file 数が増える | yes |
| universal adapter が payload shape を sniff | script 1 本 | fail semantics の分岐が集中、脆弱 | no |
| per-target installer / config patch | config 完全制御 | APM 維持の前提と矛盾 | no |

## Tentative Conclusions and Hypotheses

### Confirmed

- **Windsurf**: `pre_read_code`（`tool_info.file_path`）で file-read、
  `pre_run_command`（`tool_info.command_line`）で shell。exit `2` で
  block。Windows は `powershell` field を使う。（confirmed:
  https://docs.devin.ai/desktop/cascade/hooks）
- **Claude Code**: `PreToolUse` は nested matcher group
  （`{matcher, hooks:[{type:"command", command}]}`）で登録。payload は
  `tool_name` / `tool_input`（Read → `tool_input.file_path`、Bash /
  PowerShell → `tool_input.command`）。deny は stdout の
  `hookSpecificOutput.permissionDecision: "deny"` +
  `permissionDecisionReason`。（confirmed: code.claude.com hooks 参考）
- **Cursor**: `beforeReadFile`（payload `file_path`）で file-read、
  `beforeShellExecution`（payload `command` / `cwd`）で shell。応答は
  `{permission: "allow"|"deny"|"ask", user_message?, agent_message?}`。
  permission hook は exit 0 + schema 不適合応答を block 扱いするため、
  allow でも schema 適合 JSON を返す必要がある。（confirmed:
  cursor.com docs hooks）
- **Copilot**: `preToolUse`、payload `{toolName, toolArgs}`（`toolArgs` は
  object / JSON string の揺れあり）。deny は stdout
  `{"permissionDecision":"deny","permissionDecisionReason":...}` + exit 0。
  entry の `matcher`（toolName regex）は公式 doc の example で確認済み。
  （confirmed: https://docs.github.com/en/copilot/reference/hooks-configuration）
- **APM pass-through**: `hooks.json` source entry の `matcher` /
  nested `hooks` group / `failClosed` 等の custom field を generated
  config（`.claude` / `.cursor` / `.codex` / `.github`）へ verbatim で
  伝搬することを temp consumer への `apm install` で実測確認。Claude
  target では flat `preToolUse` entry が `PreToolUse` + matcher `*` の
  nested group へ casing 正規化される。（confirmed: temp dir 実測）
- **Codex shell**: `PreToolUse` matcher `Bash` で shell invocation を
  intercept 可能。payload は `tool_name` / `tool_input.command`。
  `.codex/hooks.json` に nested group を配置でき、APM 経由で deploy 可能。
  （confirmed: https://developers.openai.com/codex/hooks +
  codex-rs/hooks/src/events/pre_tool_use.rs）
- **Codex file-read**: `codex-rs` の read 系 tool handler
  （`file_read.rs` 等）は `PreToolUse` を emit しない（emit するのは
  shell / unified_exec / apply_patch 系）。upstream issue #18491・#20204
  も open のまま。file-read interception は unsupported。（confirmed:
  source inspection）

### Hypothesis

- Copilot `preToolUse` entry の `matcher` を `view|bash|powershell|shell`
  に絞ると、Claude（`Read`/`Bash` 等 capitalized tool 名）と Cursor
  （`Shell` 等）では case-sensitive に不一致となり、cross-fire を防げる。
  matcher が case-insensitive の target では adapter 側の payload shape
  判定で no-op になるため、どちらでも安全側に倒れる。
- Codex adapter は Claude と同一の `tool_name` / `tool_input` shape を
  期待できるため、1 つの pretooluse adapter を共用できる（matcher
  `Read|Bash|PowerShell` で codex は Bash のみ発火）。

## Open Questions

- Codex が `PreToolUse` の `hookSpecificOutput.permissionDecision` を
  解釈して deny するかは公式 example では明示されず、runtime 実機検証が
  ないため「best-effort deny + fail-open」として扱う。
- Copilot の `.json` asset 欠落は引き続き upstream 制限（対処方針は
  従来どおり drift assert で監視）。

## Promotion Candidates

- [x] spec-doc needed: SPEC-034rzTJydXfLvQhTIqiDqW（canonical request +
      per-target adapter の要件）
- [x] adr-doc needed: ADR-034rzTJYDRmre6D5OS8HDW（shared policy module +
      per-target adapter の構造決定）

## Sources and Relations

- Issue #43: 要件の正。
- DISC-034r05iHy2gdnpdai9KGpW: 前回 matrix。本 discovery の確認結果で
  Codex 行（shell 到達可 / file-read unsupported）と APM pass-through
  実測を追記する。
- 公式 docs: Windsurf（docs.devin.ai/desktop/cascade/hooks）、Copilot
  （docs.github.com/en/copilot/reference/hooks-configuration）、Codex
  （developers.openai.com/codex/hooks）。
- Codex upstream gap: openai/codex#18491・#20204。
