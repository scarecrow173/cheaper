---
id: "TASK-034KsKTKF4k3oBwHXkLXR6"
type: "task"
status: "todo"
title: "Register cursor preToolUse hook and extend cursor_normalize"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified:
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/hooks.json"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  blocks:
    - "docs/tasks/document-approval-bypass-policy-flag.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/cursor-pretooluse-registration-and-normalize.md"
  references:
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "packages/apm-hook-cursor/.apm/hooks/hooks.json"
  defers: []
  deferred-by: []
---

# Register cursor preToolUse hook and extend cursor_normalize

## Work

plan `docs/plans/implement-per-target-hook-rewrite-enablement.md`
(PLAN-034KsKo6nYOBOpIpkNoviY)の T3。cursor target の rewrite
有効化に必須の受け口 — `preToolUse` event 登録と
`cursor_normalize` の nested payload 対応 — を先行整備する。
**emit は一切追加しない**(emit contract 未検証 — R10/D3)。
本 task は flag(T1)に非依存で、T1/T2 と並列実行可能。

### 1. `preToolUse` event 登録 — `apm-hook-cursor/.apm/hooks/hooks.json`

- `hooks` に `"preToolUse"` flat entry を追加し
  `cheaper hook pre-tool-use --target cursor` を呼ぶ —
  sh・powershell 両 entry で、失敗時は
  `{"permission":"allow"}` を echo する。shape は現行
  `beforeReadFile`/`beforeShellExecution` entry と同型
  (`show_output: false` 含む):

  ```json
  "preToolUse": [
    {
      "command": "sh ./cheaper.sh hook pre-tool-use --target cursor || echo '{\"permission\":\"allow\"}'",
      "powershell": "powershell -NoProfile -ExecutionPolicy Bypass -File \"./cheaper.ps1\" hook pre-tool-use --target cursor; if ($LASTEXITCODE -ne 0) { '{\"permission\":\"allow\"}' }; exit 0",
      "show_output": false
    }
  ]
  ```

- `matcher` field は **付けない** — payload/tool 名 domain が未検証
  のため全 tool firing を受け、normalize 失敗は schema-valid
  allow の fail-open で吸収する。
- 旧 `beforeReadFile`/`beforeShellExecution` entry は併存維持
  (D4 — 撤去可否は実機検証依存)。同一 firing の二重評価は
  allow/deny verdict で dedup が吸収する(Rewrite は非 persist で
  再 eval — design §Risks の dedup nuance)。
- `hook/mod.rs` の `KNOWN_EVENTS` は `pre-tool-use` を既に含む
  ため変更不要。

### 2. `cursor_normalize` の nested `tool_input` 対応 — `targets.rs:630`

- 現行の top-level `file_path`/`command`(beforeXxx payload)読みに
  加え、nested `tool_input.file_path`/`tool_input.command` を field
  駆動で読む — `tool_name` domain(`"Shell"`/`"Read"` 等)は未検証
  のため **name matching ではなく field の有無で判別**し、両方の
  payload shape hypothesis(claude 型 nested / cursor 固有 flat)を
  吸収する。
- `tool_input.command` → `RoutingRequest::Shell`、
  `tool_input.file_path` → `RoutingRequest::FileRead{range: None}`。
  source_tool は `cursor:preToolUse`。top-level 判定を先に試す
  現行順序を維持し、nested は fallback として読む(beforeXxx
  payload との誤判別を避ける)。
- field 駆動判別は over-match し得る(read 以外の tool が
  `file_path` input を持つ場合に FileRead として評価され、巨大
  target で deny になり得る) — 既知の残 risk で、design §Risks
  に記録済み。`tool_name` domain 確定まではこの範囲を超えて
  絞り込まない。
- deny/allow render は現行 `cursor_respond` の flat
  `{"permission":…}` schema を再利用(preToolUse 応答 schema も
  `permission` 系 — docs 報告ベース、実機要確認)。
- `cursor_render`(:675)の Rewrite arm は `cursor_render(*fallback)`
  のまま**不変** — emit contract 確定まで flag の有無にかかわらず
  fallback(R10/AC)。`handle_hook` の cursor arm も変えない。

### 3. unit test(targets.rs tests)

- `cursor_normalize` が nested `tool_input.command` →
  `RoutingRequest::Shell`・`tool_input.file_path` →
  `RoutingRequest::FileRead` を返す(preToolUse payload
  hypothesis)。source_tool が `cursor:preToolUse`。
- 従来の flat `command`/`file_path` payload が従来 source_tool
  (`cursor:beforeShellExecution`/`cursor:beforeReadFile`)で
  normalize される非回帰。
- malformed・field 欠落 payload で schema-valid
  `{"permission":"allow"}` が返る(`cursor_respond`・
  `fallback_allow` 経路の fail-open)。
- cursor Rewrite render が flag 有無にかかわらず fallback verdict
  のみを返す(emit gate — `updated_input`/`updatedInput` を
  emit しない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-per-target-hook-rewrite-enablement.md` — 本
  task が implement する plan(File Map・Dependency Graph の T3)
- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1)— 登録 entry 形・field 駆動
  判別・emit gate の AUTHORITATIVE 定義
- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy)— R9-R11・R16-R17
- `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
  (ADR-034KrYGRHZuvOIx1d0S0nE)— D3・D4 の根拠

## Dependencies

- なし(flag 系 T1/T2 とは独立 — `blocks` は T5 の最終検証のみ)。

## Affected Files

- `packages/apm-hook-cursor/.apm/hooks/hooks.json`(modify —
  `preToolUse` entry 追加)
- `crates/cheaper-cli/src/hook/targets.rs`(modify —
  `cursor_normalize` のみ。`cursor_render`・`handle_hook` cursor
  arm は不変)

## Done When

- [ ] hooks.json に `preToolUse` の sh/powershell entry が
      `cheaper hook pre-tool-use --target cursor` + allow echo
      fallback で登録される(matcher 無し、旧 event 併存)
- [ ] `cursor_normalize` が nested `tool_input.command`/`file_path`
      を `cursor:preToolUse` source_tool で `RoutingRequest` へ
      正規化する
- [ ] flat beforeXxx payload の normalize が非回帰
- [ ] Rewrite verdict が cursor で `*fallback` render のまま
      (emit 無し)
- [ ] malformed/失敗入力が schema-valid `{"permission":"allow"}`
      で応答される(fail-open)

## Out of Scope

- cursor/copilot の rewrite emit 本体 — emit contract
  (`updated_input` vs `updatedInput`・`permission` 併記要否)は
  実機検証で確定するまで発行しない(別検証 task)。
- `preToolUse` entry の `matcher` field・旧 event 撤去 —
  payload/tool 名 domain と併せて実機検証で確定(D4)。
- copilot の登録・normalize・render — 一切変更しない(R12/R13)。
- flag schema・codex emit(T1/T2 — `cursor_render` は flag を
  読まない)。

## Verification

- [ ] `node scripts/check-apm-payload.mjs` exits 0(cursor hooks.json 変更が slim payload check に整合)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib` exits 0(cursor_normalize unit test 全 pass)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0(既存 hook contract の非回帰)
