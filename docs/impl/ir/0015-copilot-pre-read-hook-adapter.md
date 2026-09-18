---
id: "IMPL-0015"
type: "impl"
status: "completed"
title: "Copilot pre-read hook adapter (PLAN-0020)"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added:
      - type: "file"
        path: "packages/apm/.apm/hooks/pre-read-code-copilot.js"
      - type: "file"
        path: "packages/cli/test/e2e/copilot-pre-read-hook.test.ts"
    modified:
      - type: "file"
        path: "packages/apm/.apm/hooks/hooks.json"
      - type: "file"
        path: "packages/cli/test/e2e/apm-install-targets.test.ts"
      - type: "file"
        path: "docs/discovery/0009-cross-agent-hook-distribution-and-contract-matrix.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/0106-copilot-adapter-script-and-hook-tests.md"
    - "docs/tasks/0107-copilot-deploy-e2e-matrix-and-final-verification.md"
  implemented-by: []
  depends-on:
    - "docs/plans/0020-copilot-pre-read-hook-adapter.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/0025-copilot-pre-read-hook-adapter.md"
    - "docs/specs/0020-copilot-pre-read-hook-adapter.md"
    - "docs/adr/0023-copilot-adapter-separate-script-never-nonzero-exit.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/0020-copilot-pre-read-hook-adapter.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Copilot pre-read hook adapter (PLAN-0020)

## Summary

GitHub Copilot の native `preToolUse` event（`toolName == "view"`）で
動作する self-contained adapter `pre-read-code-copilot.js` を hook
bundle に追加した。`toolArgs` の `string | object` 型揺れを
normalize し、fail-closed 意味論（非 0 exit = 全 deny）の下で
**いかなる経路でも非 0 exit しない**契約を実現 — 意図した block のみ
stdout `{"permissionDecision":"deny",...}` + exit 0 で表現する。
`hooks.json` に `preToolUse` を追加宣言し、verbatim deploy で
copilot の generated config に登録される。

## Context

TASK-0106〜0107（PLAN-0020 / SPEC-0020 / ADR-0023 / DESIGN-0025）に
従う。Issue #10。DISC-0009 の contract matrix に基づく最初の
per-target adapter で、claude・cursor への pattern を確立した。
Codex は upstream が read event を emit しないため対象外のまま。

## Implementation

### Changes

- `packages/apm/.apm/hooks/pre-read-code-copilot.js`（新規）:
  `pre-read-code.js` と同じ判定骨格（workspace 内 file 確認・
  `CHEAPER_READ_TOKEN_THRESHOLD` fast path・`count-tokens`・
  `FILE_TOO_LARGE`）を持つ self-contained script。stdin は
  `{toolName, toolArgs}` を解釈し `toolName == "view"` のみ処理。
  `extractFilePath()` が `toolArgs` の string/object を normalize
  し `path`/`file_path`/`filePath` を探索。block は `deny()` が
  stdout に permissionDecision JSON を書いて exit 0。全異常経路は
  `process.exit(0)` — fail-closed platform 上での fail-open 維持
- `packages/apm/.apm/hooks/hooks.json`: `preToolUse` entry を追加
  （`pre_read_code` は無変更。powershell から `; exit $LASTEXITCODE`
  を省く — adapter は常に 0 を返すため不要）
- `packages/cli/test/e2e/copilot-pre-read-hook.test.ts`（新規、
  10 tests）: stdin 注入で block（object/string toolArgs・
  FILE_TOO_LARGE・lowered threshold・bundled CLI argv[2]）・
  allow（small file・fast path marker）・**never-nonzero**（非 JSON
  stdin・non-view toolName・parse 不能 toolArgs・path key 欠落・
  非 string path・workspace 外・不存在 file・directory・CLI 不在
  — 9 経路すべて `status === 0`）を検証
- `packages/cli/test/e2e/apm-install-targets.test.ts`: copilot の
  `preToolUse` 登録・adapter deploy assert を追加。claude case に
  `PreToolUse`（PascalCase 正規化後の live 登録）assert を追加。
  `.json` 欠落 assert は drift signal として維持
- `docs/discovery/0009-...`: copilot 行を「functional（adapter
  経由）。`.json` 欠落は upstream 待ちの既知制限」に更新、
  claude の event 名正規化を deploy facts に追記、fail-closed
  保証の Open Question を解決済みに更新

### Decisions During Implementation

- **`.json` asset gap は upstream 待ち（ADR-0023 option i）**:
  adapter は `count-tokens` のみ使用で `.json` 非依存。`--version`
  と file-schema worker の copilot deploy 破損は expected-failure
  assert（drift signal）+ matrix 明記で維持し、asset 名変更や
  schema 埋め込みの侵襲的回避は行わない
- **claude での `preToolUse` 正規化（実測発見）**: apm 0.28.0 の
  claude adapter は event 名を native casing に正規化するため、
  `preToolUse` は claude config で PascalCase `PreToolUse`
  （matcher `*`）として **live 登録** される。no-op 想定は誤り
  だったが、claude の stdin shape（`tool_name`/`tool_input`）は
  adapter の `toolName`/`toolArgs` contract と一致しないため全
  payload を exit 0 素通し — 実質 no-op で安全。tool call 毎の
  node spawn overhead のみ残る
- **共有 module を作らない**: adapter を self-contained file とし、
  asset 欠落や load 失敗が全 adapter に波及する経路を作らない
  （per-target risk isolation、issue #10 の要求）

## Validation

- `vitest run test/e2e/copilot-pre-read-hook.test.ts`: 10/10 pass
- `vitest run test/e2e/apm-install-targets.test.ts`: 6/6 pass
  （copilot: adapter deploy + `preToolUse` 登録 assert。claude:
  `PreToolUse` 正規化 live 登録 assert。他: verbatim no-op）
- `vitest run test/e2e/pre-read-hook.test.ts`: 12/12 pass
  （windsurf 回帰なし）
- `mise run check` 全 pass: lint（warnings のみ — 既存 hook と同じ
  useOptionalChain pattern + 生成物サイズ）→ version check →
  typecheck → 666 tests（core 270 + runtime-pi 255 + cli 141）→
  bundle 再生成 → freshness clean
- `audit_docs.js --type all`: 198 files / no findings

## Risks

- copilot `view` tool の `toolArgs` key と runtime payload shape は
  実 copilot binary で未検証（`path`/`file_path`/`filePath` の複数
  key 許容で doc 揺れを吸収。実機検証は open item）
- claude で `PreToolUse` live 登録されるため tool call 毎に node
  spawn が走る（exit 0 素通しで動作は正しいが overhead は残る。
  claude 用 adapter 実装時に集約検討の余地）
- `.json` 欠落が upstream で修正されると drift-signal assert が
  fail し matrix 更新が必要（意図した挙動）

## Follow-ups

- claude・cursor 向け adapter（同一 per-target script pattern で
  後続 chain。claude は `PreToolUse` live 登録が既に存在するため
  payload shape のみ対応すればよい）
- `.json` asset 欠落の upstream 修正追跡（drift-signal assert が
  修正を検知する）
- 実 copilot binary での runtime loader 検証（全 target 共通の
  open item）
