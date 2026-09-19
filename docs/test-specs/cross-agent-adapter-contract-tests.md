---
id: "TSPEC-034rAaJtM27Ibpy78mq0kP"
type: "test-spec"
status: "approved"
title: "Cross-agent adapter contract tests"
created: "2026-09-19"
updated: "2026-09-19"
owners: ["scarecrow173"]
relations:
  source: []
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
    - "docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md"
    - "docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md"
    - "docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/cross-agent-native-routing-adapters-over-apm-distribution.md"
    - "docs/designs/cross-agent-native-routing-adapter-architecture.md"
    - "docs/tasks/shared-routing-policy-module-and-windsurf-adapter-refactor.md"
    - "docs/tasks/new-adapter-scripts-and-hooks-json-event-registration.md"
    - "docs/tasks/adapter-contract-tests-and-apm-install-e2e-assertions.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  test-spec:
    level: "e2e"
---

# Cross-agent adapter contract tests

## Purpose

issue #43 の完了条件「各 target の native hook event が canonical
routing request へ到達し、deny が target 固有の response 契約で
返る」を pin する。target ごとの fail semantics 差分が adapter に
吸収されていることを回帰として固定し、#42 が shell seam に接続する
前提を守る。

## Feature

Per-target hook adapter が native stdin payload を canonical request
（`file-read` / `shell`）へ normalize し、shared policy verdict を
native response（exit code / stdout JSON）へ変換する。

## Rules

- Rule: adapter は native payload の parse・normalize・response 変換
  のみを持ち、large-read 判定（threshold / count-tokens /
  FILE_TOO_LARGE）を複製しない。
- Rule: infrastructure failure（非 JSON stdin・欠落 field・workspace
  外 path・CLI 不在）は全 adapter で fail-open。
- Rule: intentional deny は target native response で表現する —
  windsurf exit 2 + stderr、claude/codex `hookSpecificOutput` deny
  JSON + exit 0、cursor `{permission:"deny"}` + exit 0、copilot
  `{permissionDecision:"deny"}` + exit 0。
- Rule: copilot adapter は全 code path で exit 0（never-nonzero）。
- Rule: cursor adapter は全 code path で schema 適合
  `{permission:"allow"|"deny"}` JSON + exit 0（空出力禁止）。
- Rule: `apm install --target <t>` の generated config に target の
  native event 登録と adapter command が含まれる。
- Rule: `kind:"shell"` request が policy まで到達する seam が各
  shell-capable adapter に存在する（現状 verdict は allow）。

## Examples

- Given `{agent_action_name:"pre_read_code", tool_info:{file_path}}` and
  a file over threshold, when windsurf adapter runs, then exit 2 and
  stderr contains `bulk-read` + `workspace-relative path`。
- Given `{agent_action_name:"pre_run_command", tool_info:{command_line}}`,
  when windsurf shell adapter runs, then exit 0（allow seam）。
- Given `{tool_name:"Read", tool_input:{file_path}}` over threshold,
  when pretooluse adapter runs, then stdout has
  `hookSpecificOutput.permissionDecision:"deny"` + exit 0。
- Given `{tool_name:"Bash", tool_input:{command}}`, when pretooluse
  adapter runs, then exit 0 silent（shell は allow seam）。
- Given `{file_path}` cursor payload over threshold, then stdout
  `{permission:"deny"}` + exit 0; Given `{command}` cursor payload,
  then `{permission:"allow"}` + exit 0。
- Given `{toolName:"view", toolArgs}` over threshold, then copilot
  deny JSON + exit 0。Given `{toolName:"bash", toolArgs:{command}}`,
  then exit 0 silent。
- Given malformed stdin to any adapter, then its target の fail-open
  response（windsurf exit 0、cursor allow JSON、copilot exit 0）。

## Guarantees

- shared policy を通った判定結果のみが deny になる（adapter 固有の
  deny 経路を持たない）。
- deny guidance に executable shell command が混入しない。
- generated config が target native event（claude `PreToolUse`、
  cursor `beforeReadFile`/`beforeShellExecution`、copilot
  `preToolUse`、codex `PreToolUse`、windsurf `pre_read_code`/
  `pre_run_command`）を含む。

## Non-goals

- 各 agent binary での runtime loader 実機検証。
- #42 の shell read candidate 抽出の判定精度。
- Codex file-read interception（upstream gap で unsupported）。

## Risk

- この test spec を外すと、adapter 間の判定複製や fail semantics の
  破壊（copilot 非 0 exit、cursor 空出力）を検出できず、各 agent で
  tool call が誤 block / 誤 pass する。
