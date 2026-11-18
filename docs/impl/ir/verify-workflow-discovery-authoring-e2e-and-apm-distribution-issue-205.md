---
id: "IMPL-034ZwWwZ3zfEltfI2xS2F7"
type: "impl"
status: "completed"
title: "Verify workflow discovery+authoring E2E and APM distribution (issue 205)"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
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
    - "docs/tasks/verify-workflow-e2e-issue-205.md"
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
  verified-by:
    - "docs/test-specs/workflow-e2e-verification-contract.md"
  references:
    - "docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted:
      - "docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl"
    rejected: []
---

# Verify workflow discovery+authoring E2E and APM distribution (issue 205)

## Summary

issue #205 の横断検証を実行し、測定証拠を記録した。epic #200 の
assembled 導線（catalog 発見 → L1 比較 → 承認 → L2
authoring/登録 → 明示 path 再利用）を `target/debug/cheaper.exe` と
`path:` dep の scratch APM consumer で実際に通し、issue matrix の
positive/negative/権限/invalid case を fixture で再現した。code
変更は無し — 成果物は doc chain と experiment log のみ。

## Context

`docs/tasks/verify-workflow-e2e-issue-205.md`（plan:
`docs/plans/verify-workflow-e2e-issue-205.md`、spec:
`docs/specs/workflow-e2e-verification-issue-205.md`、design:
`docs/designs/workflow-e2e-verification.md`）に従う。

## Implementation

### Changes

- scratch workspace（`$TEMP/cheaper-e2e-205/ws`）+ isolated
  `CHEAPER_CONFIG_DIR` で全 CLI 検証を実行 — repo `.cheaper/` と
  実 user config は無接触。
- tool-only fixture workflow `file-inspect.json`
  （`count_tokens`+`grep`、builtin `tools.allow` 内・grant 不要）で
  `interface` → catalog 登録 → `workflow list` → `workflow run`
  （`--input <file>` / `--input -`）を完走。`result` は
  `{tokens,bytes,matches}` の公開 outputs のみ、`jobs[]` は
  `{id,kind,status,meta}` で private input/result を含まない。
- negative battery: malformed JSON（`INVALID_INPUT`）、v1 shape
  （`invalid Workflow JSON v1` + `details.errors[].instancePath`）、
  unknown tool/worker（`unknown tool:`/`unknown worker:`）、
  未宣言 `$input`（`unknown input reference`）、binding wrapper
  違反（`invalid binding wrapper` + `details.keys`）、
  `$from`+`path` 欠落（outputs 位置では schema の oneOf 違反で
  `invalid Workflow JSON v1`、job input 位置では `invalid binding
  wrapper`）、impossible pointer（`primitive output has no
  child`）、cycle（`workflow dependency cycle` +
  `details.jobs`）— 全て exit 2 で、L2 skill の error 対処表と
  一致。
- catalog battery: duplicate（両 entry `status:"duplicate"`）、
  missing、path-out-of-workspace、malformed-entry は per-entry
  diagnostics で `ok:true` 維持。manifest 破損（bad JSON・
  `schemaVersion`≠1・非 array・`summary` 500 超）は `INVALID_INPUT`
  + exit 2 で fail-closed — `catalog.rs` の two-tier 規約と一致。
  `--limit` は 1..256 で `truncated:true` が立つ。
- 権限非退行: `command` job は `interface` 通過後も run 時に
  `POLICY_VIOLATION`（envelope `TOOL_EXECUTION_FAILURE`、exit 6）—
  catalog は実行権限を与えない。`workflow run <id>` は named lookup
  でなく path 解釈で `cannot read workflow file`。
- read-only 確認: `workflow list`/`interface`/`count-tokens`/
  `route check` の前後で history record 数不変。
  `workflow run` は `<userConfigDir>/history/` に
  `kind:"workflow"`/`kind:"job"`（失敗 run も `errorCode` 付きで
  記録）— workspace `.cheaper/` には書かない。
- APM 配布: `apm install`（0.33.0、consumer `path:` dep on
  `packages/apm`）で 3 target を実測 — `agent-skills` →
  `.agents/skills/` に 7 SKILL.md のみ、`windsurf` →
  `.agents/skills/` + `.windsurf/hooks/apm-hook-windsurf/` +
  hooks.json、`claude` → `.claude/skills/` +
  `.claude/hooks/apm-hook-claude/` + settings.json（
  `${CLAUDE_PROJECT_DIR}` placeholder・`PreToolUse` matcher）。
  非 active target の `apm-hook-*` は per-dep `targets:` 非一致で
  skip されることを install log で確認。7 SKILL.md 全ての
  `name`/`description` front matter が parse 可能。
- docs 照合: guide の主張（skills 7 件・deploy root matrix・
  catalog 契約・`--input` 規約・exit code・`--limit` 上限・
  `route check` exit class 1=warning）を shipped skill 本文・
  `catalog.rs` 定数・`--help` と照合し全て一致。#201-#204 chain の
  README index 行は main の `d80ad03`/`069b741` で既に front
  matter と同期済み — `doc_status lint` に
  `index-metadata-mismatch` は無く、修復不要を確認。
- 静的 text 検証: L0 の抑制規約（候補判定のための catalog/
  history/usage/file scan 禁止・session 1 回・hook 非依存）・
  L1 の no-write/evidence label/提案≠作成≠実行・L2 の承認境界と
  5 template が、shipped SKILL.md 本文に存在することを確認。
  5 template は全て `workflow interface` 相当の valid v1 形（
  実機 spot-check で template 2 の `required:false` default 処理を
  `file-inspect.json` の `searchDir`/`pattern` で確認）。

### Decisions During Implementation

- `workflow run` の実証には `command`/`process` job を避け tool-only
  fixture を選んだ — grant 供給は permission surface を変えてしまい
  「catalog が権限を与えない」検証と矛盾するため。
- `{"$from":"x"}`（path 欠落）は position によって error message が
  2 系統に分かれることを実測で確認 — outputs `.value` では v1 JSON
  schema の oneOf 違反、job `input` 内では `invalid binding
  wrapper`。skill 対処表の message 自体はどちらも実在するため drift
  ではない。
- exp log の event は各検証 step の envelope/exit/deploy tree の
  実測値のみを記録し、timing は debug build 由来と明示（性能
  claim はしない）。

## Related Experiments

### Adopted

- `docs/impl/exp/workflow-e2e-and-apm-distribution-verification-issue-205.jsonl`
  — issue matrix を fixture で再現した全測定値。採用理由: 各 event
  が実 binary/実 `apm install` の observed output であり、検証の
  replayable な正本となるため。

### Rejected

- なし。

## Validation

- `workflow list`/`interface`/`run`/`count-tokens`/`route check`/
  `route resolve` を scratch workspace で実実行（exit code・
  envelope・diagnostics を exp log に記録）。
- `apm install` を 3 consumer で実実行（deploy tree・skip log・
  front matter parse を記録）。
- `doc_status audit --type all --blocking --json` → blocking 0。
- `git diff --check` → clean。
- `mise run check` → 記録の通り（env flake が出た場合は filter 単位
  で再実行して確認）。

## Risks

- per-target agent runtime の skill auto-firing は未検証（documented
  limitation・manual fallback あり — issue 自身の caveat と同一）。
- Windows のみの実測 — POSIX launcher は内容照合のみ。
- debug binary 由来の timing は性能 claim に使わない。

## Follow-ups

- 実 consumer runtime での auto-firing 観測が必要になったら、
  `agent-skills` dogfood 環境（Devin CLI）で L0 判定を明示依頼する
  manual 経路が既に documented 済み — その限界を超える保証は
  runtime 側の対応待ち。
