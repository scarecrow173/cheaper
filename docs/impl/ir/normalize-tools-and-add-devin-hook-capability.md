---
id: "IMPL-034GtH0PU9VrkGZfPEk4FR"
type: "impl"
status: "completed"
title: "Normalize tools and add Devin hook capability"
created: "2026-10-15"
updated: "2026-10-21"
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
  implements:
    - "docs/tasks/normalize-tools-and-add-devin-hook-capability.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Normalize tools and add Devin hook capability

## Summary

T1: `tool_name` の ASCII case-insensitive 正規化を共有
`tool_use_normalize` へ集約し、`HookTarget::Devin` native adapter
（PreToolUse event gate + `{"decision":"block","reason":...}` render）と
`cheaper hook targets --json` capability query を追加した。
`packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher を
case-insensitive exact な char class へ更新。policy /
`RoutingRequest` / count ・threshold は不変。

## Context

- Task: `docs/tasks/normalize-tools-and-add-devin-hook-capability.md`
  （`TASK-034GtBzfyOaeVS6nlm1ZkC`）
- Plan: `docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md`
  （`PLAN-034GtndRTaEqU3lA6Tnquu`、T1 slice）
- Spec: `docs/specs/case-insensitive-native-tools-and-devin-registration.md`
  （approved）
- Design: `docs/designs/devin-native-hooks-and-apm-bootstrap.md`
  （approved）
- Test-spec: `docs/test-specs/native-tool-name-normalization-and-devin-hook-contract.md`
  （approved）

Graph route: probe-to-briefing → briefing-to-design →
design-to-planning → planning-to-task-graph →
task-graph-to-implementation（本 edge、implementation 中）。
commitBaseline: head `047165bbf02a6ce32699ac363af3c18af7faa094`。
この pending edge 中に保持する記録であり committed checkpoint
ではない — 差分は lead review 後に lead が commit する。採取時点の
dirty entry list（lead 提供の固定リスト、再採取で置換しない）:

- modified: `.gitignore`、`docs/adr/README.md`、
  `docs/adr/normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md`、
  `docs/designs/README.md`、`docs/designs/overview.md`、
  `docs/discovery/README.md`、`docs/plans/README.md`、
  `docs/specs/README.md`、
  `docs/specs/apm-delegation-skills-and-read-hook.md`、
  `docs/tasks/README.md`、`docs/test-specs/README.md`
- staged delete: `.sdp/skill-reference-inferences.json`
- untracked:
  `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`、
  `docs/designs/devin-native-hooks-and-apm-bootstrap.md`、
  `docs/discovery/bounded-hook-read-request-contract.md`、
  `docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md`、
  `docs/specs/case-insensitive-native-tools-and-devin-registration.md`、
  `docs/tasks/normalize-tools-and-add-devin-hook-capability.md`、
  `docs/tasks/register-devin-hooks-from-apm-skill.md`、
  `docs/test-specs/apm-distributed-devin-registration-preservation.md`、
  `docs/test-specs/native-tool-name-normalization-and-devin-hook-contract.md`

上流 doc の承認証拠は各 Review Record と lead の ACCEPT 通知に
記録済み。T1 code/test diff は lead が全文 review して ACCEPT
（confirmed P0/P1 なし、easy fix 適用後）。commit gate はまだ
pending。

## Implementation

### Changes

- `crates/cheaper-cli/src/hook/targets.rs`
  - `HookTarget::Devin` 追加、`pub const ALL: [Self; 5]`（windsurf,
    copilot, pretooluse, cursor, devin）を定義。`parse` は `ALL` の
    `as_str` との比較へ、`as_str` match が文字列 authority。
  - 共有 `tool_use_normalize(input, cwd, source_prefix, shell_tools)`:
    root `tool_name` は string 必須、`tool_input` は object 必須、
    `to_ascii_lowercase` を 1 回適用して exact match。`read` →
    `file_path.as_str()`、`grep` → `path` 欠落は `"."` / 非 string
    は `None`、`shell_tools.contains` のみ `command.as_str()` →
    `Shell`。`source_tool` は `{prefix}:{original}` で元 label 保持。
  - `pretooluse_normalize` は prefix `pretooluse`・
    `shell_tools = &["bash","powershell"]` で共有 fn へ委譲。
  - `devin_normalize`: CLI event `pre-tool-use` かつ payload
    `hook_event_name == "PreToolUse"` のみ受理、`shell_tools` 空
    （native exec 登録外）。`devin_render`: `Deny` →
    `{"decision":"block","reason":<共有 reason>}` + `\n`、exit 0・
    stderr 無し。`Allow`・fallback allow は silent。
  - `targets_json_line()`: `ALL` 由来の
    `{"schemaVersion":1,"targets":[...]}` 1 行を生成。
- `crates/cheaper-cli/src/hook/mod.rs`: `execute_hook` 入口に argv
  完全一致 `["targets","--json"]` 分岐を追加 — stdin 非読取・
  通常 hook flow・usage 記録を経ない。
- `crates/cheaper-cli/tests/hook.rs`: T1 contract test 追加（case
  variant・grep path 3 形態・malformed/unknown fail-open・Devin
  gate/response/reason passthrough・capability query・
  `parse`↔`as_str` roundtrip、panic stdin 経路に devin 追加）。
- `packages/apm/.apm/hooks/hooks.json`: `PreToolUse` matcher を
  `^([Rr][Ee][Aa][Dd]|[Gg][Rr][Ee][Pp]|[Bb][Aa][Ss][Hh]|[Pp][Oo][Ww][Ee][Rr][Ss][Hh][Ee][Ll][Ll])$`
  へ更新。
- `scripts/test/devin-hooks-install.test.mjs`: 新規（T1 は source
  matcher 部分のみ。installer 部分は T2）。

### Decisions During Implementation

- TDD: 新 test は全て旧実装で compile 可能な形にし（`HookTarget::ALL` /
  `Devin` variant を参照せず `parse` 経由の roundtrip と argv 文字列で
  検証）、behavioral red を観測してから実装。
- `parse` の table 更新は `ALL` + `as_str` 比較へ一元化し、
  capability 出力と `--target` 受理値の表現ドリフトを構造的に排除。
- capability 分岐は `parse_hook_argv`・`read_stdin_text` より前の
  `execute_hook` 入口に置き、「stdin を読まない」を構造で保証。
- `devin_render` の `reason` は flatten せず共有
  `Verdict::Deny.reason` を passthrough（spec F7 要件）。

## Related Experiments

### Adopted

なし（experiment log は作成していない）。

### Rejected

なし。

## Validation

- red: `mise exec -- cargo test ... --test hook` → exit 101、新規
  7 test が assert failure（log: `tmp/issue107-t1-cargo-red.log`）。
  `mise exec -- node --test scripts/test/devin-hooks-install.test.mjs`
  → exit 1（log: `tmp/issue107-t1-node-red.log`）。
- green: 同コマンド → exit 0、24 passed / 0 failed
  （`tmp/issue107-t1-cargo-green3.log`）；node → exit 0、2 passed /
  0 failed（`tmp/issue107-t1-node-green3.log`）。
- `rustfmt --edition 2021` を変更 3 file のみに適用。
- `mise exec -- biome check --write scripts/test/devin-hooks-install.test.mjs`
  → import order 修正後 clean（`biome check` exit 0）。
- 最終 gate `mise run check`（`CHEAPER_E2E_OLLAMA=0`、実行 process
  のみ）を実施: `tmp/issue107-final-check.log`（biome 段で stop、
  exit 1）→ 修正後 stage を mise.toml 順に継続した
  `tmp/issue107-final-check2.log`。本差分起因の指摘 3 件を修正:
  `hook.rs` の新規 `format!` literal → `.to_string()`（clippy
  `useless_format`）、`devin-hooks-install.test.mjs` の format +
  fixture 未 cleanup（junction 残留で biome warning、
  `afterEach`+`rmSync` 追加）、`apm-install-targets.test.mjs`
  claude smoke の旧 matcher literal `Read|Bash|PowerShell` → 新
  canonical matcher への期待値更新。最終全 stage exit 0
  （biome/fmt/clippy/version/doc-ids/cargo test workspace/node
  scripts 51 pass/node benchmarks 12 pass/lockfile/apm-payload）。
- lead review: T1 全差分 ACCEPT（confirmed P0/P1 なし、easy fix 適用
  後）。commit は lead が実行（T1 = `cb76fcc`）。pending
  task-graph-to-implementation edge の commit gate は T2 commit
  完了まで未通過。

## Risks

- matcher 拡大で `bash`/`powershell`・任意 case の
  `read`/`grep`/`bash`/`powershell` に Pretooluse hook が発火する
  breaking-change 側面（plan risk #6）。判定 semantics 自体は不変。
- #107 の範囲評価（`offset`/`limit`/`max_results`）は本 task の
  scope 外で未解決のまま。

## Follow-ups

- T2（`docs/tasks/register-devin-hooks-from-apm-skill.md`）は実装・
  ACCEPT 済み（IR: `register-devin-hooks-from-apm-skill.md`）。
- 最終 gate: `mise run check` 初回 invocation は biome 段で失敗
  （exit 1）、修正後に未実行残 stage を mise.toml 順で継続して全
  pass を確認（単一 invocation の exit 0 ではない。log:
  `tmp/issue107-final-check.log` / `tmp/issue107-final-check2.log`）。
- pending task-graph-to-implementation edge の commit gate は lead
  commit 待ち。

## Evidence 保存状態（2026-10-21 追記）

当時の wire/probe raw artifact(`tmp/issue107-*` raw file・one-off
script)は履歴 label であり現在は利用不可・参照前提ではない。
durable な正本は
[discovery `bounded-hook-read-request-contract`](../../discovery/bounded-hook-read-request-contract.md)
の実測 section(2026-10-15 setup・test matrix・sanitized record・
bounds・exit outcome)に保持される。`Get-JsonShape` helper
本体と JSON rejection matrix は
[design `devin-native-hooks-and-apm-bootstrap`](../../designs/devin-native-hooks-and-apm-bootstrap.md)
に保持。現行 test path:
`crates/cheaper-cli/tests/integration/hook.rs`・
`scripts/test/devin-hooks-install.test.mjs`。
