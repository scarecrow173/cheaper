---
id: "DESIGN-034r05iHy2gdnoUHGunDtE"
type: "design"
status: "approved"
title: "Copilot pre-read hook adapter"
created: "2026-09-18"
updated: "2026-09-18"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/copilot-pre-read-hook-adapter.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/copilot-pre-read-hook-adapter.md"
  implemented-by:
    - "../plans/copilot-pre-read-hook-adapter.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./apm-skills-and-pre-read-hook.md"
    - "./cross-agent-distribution-verification.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
  refines: []
  refined-by: []
  derives-from:
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Copilot pre-read hook adapter

## Context

SPEC-034r05iHK8bA7gsvijtkou / ADR-034r05iHrZUE6WizQ4LgPa の実装設計。copilot `preToolUse`（`toolName ==
"view"`）で動作する self-contained adapter
`pre-read-code-copilot.js` を hook bundle に追加し、`hooks.json` に
event を宣言する。block は stdout deny JSON + exit 0、adapter は
一切の非 0 exit を行わない。

## Scope

### In Scope

- `packages/apm/.apm/hooks/pre-read-code-copilot.js`（新規）
- `packages/apm/.apm/hooks/hooks.json` への `preToolUse` 登録
- `packages/cli/test/e2e/pre-read-hook.test.ts` または新規 test file
  への adapter 検証（stdin payload 注入）
- `packages/cli/test/e2e/apm-install-targets.test.ts` の copilot
  case への deploy/config assert 追加
- DISC-034r05iHy2gdnpdai9KGpW matrix の copilot 行更新 + consumer 向け既知制限の明記

### Out of Scope

- claude・cursor adapter（後続 chain）
- `.json` stripping の回避実装（upstream 待ち、ADR-034r05iHrZUE6WizQ4LgPa）
- 実 copilot binary での runtime 検証

## Design

### `pre-read-code-copilot.js`

`pre-read-code.js` と同じ判定骨格を持つ self-contained script
（共有 module なし）。差分は stdin/stdout contract のみ:

```
stdin:  {toolName, toolArgs}      toolArgs: string | object
stdout: {"permissionDecision":"deny","permissionDecisionReason": "..."}
exit:   常に 0（非 0 exit = copilot が全 deny するため禁止）
```

処理フロー（各 `process.exit(0)` は早期素通し）:

1. stdin を JSON.parse — 失敗 → exit 0。
2. `input.toolName !== "view"` → exit 0。
3. `toolArgs` normalize: string なら `JSON.parse`（失敗 → exit 0）、
   object ならそのまま。`path` / `file_path` / `filePath` の順で
   string の file path を探索 — 見つからない → exit 0。
4. `path.resolve` → `path.relative(process.cwd(), ...)` で
   workspace 内を検査（windsurf hook と同じ `..` 判定、`..` 始まりの
   file 名は内部扱い）— outside → exit 0。
5. `fs.statSync` で file 確認 — 失敗・非 file → exit 0。
6. fast path: `st.size <= threshold`（`CHEAPER_READ_TOKEN_THRESHOLD`、
   default 8192）→ exit 0（CLI spawn しない）。
7. `cheaperCommand()`（argv[2] → `CHEAPER_CLI` → PATH の `cheaper`、
   `.m?js` は node 経由）で `count-tokens <resolved> --cwd <cwd>` を
   spawn（30s timeout）— error → exit 0。
8. envelope parse 可能かつ `error.code === "FILE_TOO_LARGE"` →
   **deny**（bytes・cap 情報を reason に含める）。
9. `run.status !== 0` または envelope 不可・形状不正 → exit 0。
10. `tokens <= limit` → exit 0。超過 → **deny**。

deny の表現:

```js
process.stdout.write(JSON.stringify({
  permissionDecision: "deny",
  permissionDecisionReason:
    `cheaper: '${relPosix}' has ${tokens} tokens (threshold ${limit}). ` +
    "Do not read it directly. Delegate this file through the " +
    "cheaper-bulk-read skill. workspace-relative path: " + relPosix,
}) + "\n");
process.exit(0);
```

guidance は windsurf hook と同一文言（stderr → reason field に写像）。
実行可能 shell command は埋め込まない（non-executable guidance）。

### `hooks.json`

```json
{
  "hooks": {
    "pre_read_code": [
      {
        "command": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\"",
        "powershell": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\"; exit $LASTEXITCODE",
        "show_output": false
      }
    ],
    "preToolUse": [
      {
        "command": "node ./pre-read-code-copilot.js \"./cli/bin/cheaper.mjs\"",
        "powershell": "node ./pre-read-code-copilot.js \"./cli/bin/cheaper.mjs\"",
        "show_output": false
      }
    ]
  }
}
```

APM は event 名をほぼ verbatim で全 target config に書き込む。
copilot では `preToolUse` が native に発火し `pre_read_code` は
no-op、windsurf では逆になる。**例外（実測）**: apm 0.28.0 の
claude adapter は event 名を native casing に正規化するため、
`preToolUse` は claude config では PascalCase `PreToolUse`
（matcher `*`）として live 登録される。ただし claude の stdin
shape（`tool_name` / `tool_input`）は本 adapter の
`toolName` / `toolArgs` と一致しないため、adapter は全 payload を
exit 0 で素通しし実質 no-op となる。powershell の
`; exit $LASTEXITCODE` は windsurf 側の exit 伝播に必要なため
pre_read_code には残すが、copilot adapter は常に 0 を返すため
adapter 側では不要。

### `.json` asset gap の扱い（ADR-034r05iHrZUE6WizQ4LgPa option i）

adapter が使う `count-tokens` は `.json` 不要で動作確認済みのため、
`.json` gap は本件の blocker ではない。upstream 修正待ち + 既存
expected-failure assert（`version.json`・schemas の不存在 assert）
を drift signal として維持する。file-schema worker が copilot
deploy で不動である点は DISC-034r05iHy2gdnpdai9KGpW matrix の copilot 行に明記する。

## Test Design

### adapter の unit 的検証（stdin 注入、pre-read-hook.test.ts 流儀）

新規 `packages/cli/test/e2e/copilot-pre-read-hook.test.ts`:

- **block**: `{toolName:"view", toolArgs:{path}}`（object）と
  `{toolArgs: "<json string>"}`（string）の両方で、threshold 超過
  file に対し stdout が `permissionDecision: "deny"` の JSON で
  reason に `bulk-read` と workspace-relative path を含み exit 0。
- **FILE_TOO_LARGE**: 小さな byte cap + threshold で envelope
  FILE_TOO_LARGE 経路も deny JSON + exit 0。
- **never-nonzero assert**: 非 JSON stdin・`toolName` 違い・
  `toolArgs` が parse 不能 string・path key 欠落・workspace 外・
  不存在 file・directory・CLI 不在 — 全ケースで `status === 0` と
  stdout が空または deny JSON でないことを確認。
- **small file / threshold 内**: exit 0・stdout なし。
- **fast path**: bytes ≤ threshold の file で CLI spawn されない
  （fake CLI marker 方式、windsurf test と同じ）。

### deploy / config e2e（apm-install-targets.test.ts copilot case）

- `pre-read-code-copilot.js` が deploy される（.js は strip されない）。
- generated copilot config（`.github/hooks/cheaper-hooks.json`）に
  `hooks.preToolUse` entry が存在し command が
  `pre-read-code-copilot.js` を含む。
- 既存の `pre_read_code` verbatim assert・`.json` 欠落 assert を
  維持（drift signal）。
- 他 target の config assert は無変更（`preToolUse` 登録も入るが
  no-op — 必要なら各 target での存在を注記 assert。必須ではない）。

## Verification

- `pnpm --filter cheaper test`（cli suite — adapter test + apm-install
  targets を含む）全件。
- `pnpm typecheck`・`pnpm lint`。
- bundle 再生成は不要（adapter は `.apm/hooks/` 直下の file であり
  `bundle-cli.mjs` の生成物ではない — ただし `git diff` で
  freshness check 対象 path に不要な差分が出ないことを確認）。
- `audit_docs.js --type all`。
