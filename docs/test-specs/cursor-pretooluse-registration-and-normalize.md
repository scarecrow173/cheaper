---
id: "TSPEC-034KsKHl3xhvb61D3S2SL6"
type: "test-spec"
status: "draft"
title: "Cursor preToolUse registration and normalize extension (issue #150)"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
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
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
    - "docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  derived-by: []
  verifies:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
  verified-by:
    - "docs/tasks/register-cursor-pretooluse-hook.md"
  references:
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "packages/apm-hook-cursor/.apm/hooks/hooks.json"
  defers: []
  deferred-by: []
---

# Cursor preToolUse registration and normalize extension (issue #150)

## Purpose

cursor target の rewrite 有効化に必要な受け口 — `preToolUse` event
登録と `tool_input` nested payload の normalize — が、
SPEC-034KrX7RnDs0gLVcMPf7vy R9-R13 と
ADR-034KrYGRHZuvOIx1d0S0nE D3/D4 の contract を満たし続けることを
pin する。

保つべき意図は 3 つ。(1) **未検証 emit を発行しない**: cursor・
copilot の rewrite emit contract(`updated_input`/`updatedInput`/
`modifiedArgs` の実効 field・`permission`/`permissionDecision` 併記の
approval 意味)は実機検証で確定するまで、いかなる flag 状態でも
emit しない — issue の文言規則「実機検証で確認できない挙動は
仕様上の保証として扱わない」。(2) **fail-open**: normalize 失敗・
malformed stdin・panic は schema 適合の `{"permission":"allow"}` に
倒し、tool call を block しない。(3) **受け口は両 shape
hypothesis を吸収**: `tool_name` domain が未検証の間、normalize は
name matching ではなく field の有無で payload を判別する。

cursor の emit contract が実機検証で確定して rewrite emit arm が
追加された場合、あるいは `preToolUse` 登録が確定 payload shape で
再構成された場合に退役する(emit 側の保証はその時点で
`codex-hook-rewrite-emit` と同型の spec へ移る)。

## Feature

`packages/apm-hook-cursor/.apm/hooks/hooks.json` への `preToolUse`
entry 追加(旧 `beforeReadFile`/`beforeShellExecution` と併存)と、
`cursor_normalize` の `tool_input.command`/`tool_input.file_path`
nested payload 対応 — `RoutingRequest` への正規化と deny/allow/
fallback の schema-valid JSON 応答。

## Rules

- Rule: hooks.json に `preToolUse` flat entry を追加し
  `cheaper hook pre-tool-use --target cursor` を呼ぶ — sh・
  powershell 両 entry で、失敗時は `{"permission":"allow"}` を
  echo する(現行 `beforeReadFile`/`beforeShellExecution` entry と
  同型)。`matcher` field は payload/tool 名 domain が未検証のため
  付けず全 tool firing を受ける。
- Rule: 旧 `beforeReadFile`/`beforeShellExecution` 登録は併存維持
  (D4) — 撤去可否は `preToolUse` が file-read/shell 両方を emit
  するかの実機検証に依存する。同一 firing の二重評価は
  allow/deny verdict で dedup が吸収する(Rewrite verdict は非
  persist で再 eval される nuance あり)。
- Rule: `cursor_normalize` は top-level `file_path`/`command`
  (beforeXxx payload)に加え、nested `tool_input.file_path` /
  `tool_input.command` も field 駆動で読む — `tool_name` の
  domain(`"Shell"`/`"Read"` 等)は未検証のため name matching
  ではなく field の有無で判別する。`tool_input.command` →
  `RoutingRequest::Shell`、`tool_input.file_path` →
  `RoutingRequest::FileRead` で、source_tool は
  `cursor:preToolUse`。
- Rule: deny/allow の render は現行 `cursor_respond` の flat
  `{"permission":…}` schema を再利用する(preToolUse 応答 schema
  も `permission` 系 — docs 報告ベース、実機要確認)。
- Rule: Rewrite arm は `cursor_render(*fallback)` のまま不変 —
  `permission`/`updated_input`(or `updatedInput`)併記の emit
  contract が確定するまで、flag の有無にかかわらず fallback
  verdict を render する(未検証 shape は発行しない)。
- Rule: copilot の登録・normalize・render は変更しない —
  `modifiedArgs`/`updatedInput` の実効 field と
  `permissionDecision` 併記値(`"ask"` が approval を温存するか)は
  実機検証で確定後に flag gated emit を追加する。非 0 exit =
  deny 規約は不変で、rewrite emit を含む全応答は exit 0。
- Rule: cursor の schema-valid JSON 必須規約(`cursor_respond`
  :660・`fallback_allow` :790)は維持 — normalize 失敗・malformed
  stdin でも schema 適合の allow を返す。

## Examples

- `preToolUse` firing が nested payload
  `{"tool_input": {"command": "routed ..."}, "cwd": "..."}` を
  渡す → `RoutingRequest::Shell{command, cwd, source_tool:
  "cursor:preToolUse"}` に normalize される。
- nested `{"tool_input": {"file_path": "/abs/big.rs"}}` →
  `RoutingRequest::FileRead{path, cwd, source_tool:
  "cursor:preToolUse", range: None}` に normalize される。
- 従来の flat `{"command": "..."}` / `{"file_path": "..."}`
  (beforeXxx payload) → 従来どおり `cursor:beforeShellExecution` /
  `cursor:beforeReadFile` の source_tool で normalize される
  (両 payload shape hypothesis を吸収)。
- route match する shell firing(flag 有無にかかわらず)→
  Rewrite arm は fallback verdict のみ render し、
  `updated_input`/`updatedInput` を emit しない。
- malformed stdin・field 欠落 payload → `{"permission":"allow"}`
  の schema-valid JSON を返し exit 0。

## Guarantees

- cursor hook が `preToolUse` event で発火する登録が宣言され、
  `cheaper hook pre-tool-use --target cursor` 経路が payload を
  `RoutingRequest` へ正規化できる。
- cursor/copilot の rewrite emit は、emit contract が実機検証で
  確定するまでいかなる flag 状態でも発行されない。
- いかなる入力(unknown tool・malformed・normalize 失敗)でも
  schema 適合の allow 応答を返し、tool call を block しない
  (fail-open)。
- hooks.json 変更が slim APM payload check
  (`scripts/check-apm-payload.mjs`)に整合する。

## Non-goals

- cursor/copilot の Rewrite emit 本体(field 名・`permission`/
  `permissionDecision` 併記値)は実機検証 task の成果物 — 本 spec
  は「発行しない」ことと受け口の形状のみを固定する。
- `preToolUse` entry の `matcher` field・旧 event 撤去 —
  payload/tool 名 domain と併せて実機検証で確定。
- cursor `preToolUse` 応答 schema の実機正しさ(docs 報告ベース
  の再利用であり spec 保証ではない — 検証 task 側の責務)。
- `permissionDecision:"allow"` bypass flag の schema/trust 境界は
  `route-rewrite-approval-bypass-policy-flag` test spec が担う。
- `copilot_handle` の foreign-payload dual-schema allow 経路 —
  既存振る舞いの維持であり新規保証ではない。

## Risk

- emit gate の保証を落とすと、未検証 shape の emit が
  silent no-op・approval 誤 bypass・schema 不適合を引き起こす —
  「rewrite したつもりで原 command が走る」または tool call 全体の
  block が起き得る。
- field 駆動判別は over-match し得る — `tool_name` domain が
  確定するまで、read 以外の tool が `file_path` input を持つ場合に
  FileRead として評価され(巨大 target で deny になり得る)誤判定
  面が残る。matcher で tool 種別を絞れないことも同じリスクを
  残す(design §Risks)。
- `preToolUse` 応答 schema が実機で拒否された場合、matcher 無しの
  全 tool firing 登録は deny/allow 応答も含めて tool call を block
  し得る — 登録・応答面固有の risk で、emit gate とは別軸。
