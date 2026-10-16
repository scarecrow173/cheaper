---
id: "TSPEC-034GtADUaoOPo8wHPI3Liu"
type: "test-spec"
status: "approved"
title: "Native tool name normalization and Devin hook contract"
created: "2026-10-15"
updated: "2026-10-15"
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
    - "../plans/implement-native-tool-normalization-and-devin-bootstrap.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/case-insensitive-native-tools-and-devin-registration.md"
    - "docs/designs/devin-native-hooks-and-apm-bootstrap.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Native tool name normalization and Devin hook contract

## Purpose

T1（normalize tools and add Devin hook capability）の契約を固定する
test-spec。承認済み spec/design の要件のみを検証対象とし、実装
task の分割や再検証の度に同じ判定基準を提供する。

## Feature

Pretooluse adapter の `tool_name` ASCII lowercase 正規化と per-tool
routing、`HookTarget::Devin` の native response contract、
`cheaper hook targets --json` capability query、配布
`hooks.json` の case-insensitive exact matcher。

## Rules

- Rule: `tool_name` は `to_ascii_lowercase` 一度 + exact match。
  全 case variant が同一 tool の既存 routing に達する（`read`/`grep`
  → `FileRead`、`bash`/`powershell` → 既存 shell）。
- Rule: `read` は `tool_input.file_path`、`grep` は `tool_input.path`
  を使う。`file_path` 欠落/非 string、`path` 非 string、
  `tool_input` 非 object、root 非 object、`tool_name` 欠落は
  fail-open。`grep` の `path` 欠落は cwd directory 要求。
- Rule: 全文 token policy は不変。`offset`/`limit`/`max_results`/
  `pattern`/`context`/`glob` は判定に使わない。
- Rule: Devin は `hook_event_name == "PreToolUse"`（CLI event
  `pre-tool-use`）のみ受理、それ以外は silent allow。deny は
  exit 0 + stdout `{"decision":"block","reason":...}` + `'\n'`、
  `reason` は共有 `Verdict::Deny.reason` の passthrough。
  allow/error は silent exit 0。
- Rule: `cheaper hook targets --json` は stdin を読まず
  `{"schemaVersion":1,"targets":[...]}` を 1 行で exit 0、usage
  記録しない。`targets` は `HookTarget.ALL`（現行順
  windsurf, copilot, pretooluse, cursor, devin）由来で parse ↔
  `as_str` が roundtrip する。
- Rule: `packages/apm/.apm/hooks/hooks.json` の `PreToolUse`
  matcher は `^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Bb][Aa][Ss][Hh]|[Pp][Oo][Ww][Ee][Rr][Ss][Hh][Ee][Ll][Ll])$`。

## Examples

- `tool_name:"read"` / `"Read"` / `"rEaD"` の threshold 超過 file は
  いずれも同一 deny 判定。`grep` file は全文 guard、directory・
  `path` 欠落は allow。
- `--target devin` の deny は exit 0 + block JSON 1 行、他 event
  名や malformed payload では silent exit 0。
- matcher は `read`/`GREP`/`Bash`/`PowerShell` に一致し、
  `reader`/`greps`/未知 tool・prefix/suffix 付きには一致しない。

## Guarantees

- 既存 target（windsurf / copilot / claude / codex / pretooluse）の
  deny / allow / fail-open 契約が退行しない。
- capability query が stdin / model / usage / policy に触れない。

## Non-goals

- `offset`/`limit` の範囲評価、bounded grep 保証（#107 本体は
  deferred）。
- native exec/shell 登録、POSIX installer。

## Risk

- 本 spec の欠落は lowercase tool 名の silent allow 復帰、または
  Devin deny が解釈不能になる回帰を見逃す。
