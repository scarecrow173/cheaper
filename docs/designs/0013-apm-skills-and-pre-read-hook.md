---
id: "DESIGN-0013"
type: "design"
status: "approved"
title: "APM skills and pre_read_code delegation hook"
created: "2026-09-16"
updated: "2026-09-16"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/0008-apm-delegation-skills-and-read-hook.md"
  implemented-by: []
  depends-on:
    - "./0012-token-counter-contract.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../discovery/0005-windsurf-cascade-hooks-and-apm-hook-deployment.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# APM skills and pre_read_code delegation hook

## Context

SPEC-0008 の実装設計。parent agent 向け skill 4 本と Windsurf
`pre_read_code` hook を `.apm/` producer layout で提供する。
DISC-0005 で deploy 形式は実測済み。

## Scope

### In Scope

- `.apm/skills/cheaper-usage/SKILL.md`
- `.apm/skills/cheaper-bulk-read/SKILL.md`
- `.apm/skills/cheaper-code-write/SKILL.md`
- `.apm/skills/cheaper-doc-write/SKILL.md`
- `.apm/hooks/hooks.json` + `.apm/hooks/pre-read-code.js`
- hook script の e2e test（`packages/cli/test/e2e/` から node で spawn）

### Out of Scope

- pack 後の CLI 同梱（Phase 4）、他 event、dir 集計、privacy rules

## Components and Boundaries

### skills

各 SKILL.md は `name` + `description` frontmatter + 英語本文。
責務分担:

| skill | 内容 |
|---|---|
| cheaper-usage | CLI 全 command（`run`/alias/`count-tokens`）、stdin JSON contract、envelope（ok/result/meta/error）、exit code、`--cwd`/`--format`/`--quiet`、provider/model 設定の入口 |
| cheaper-bulk-read | WHEN: file/dir を読む前に大きいと判明した・大量 file の要約が必要。input schema の組み立て（task/paths）、result の summary/findings/relevantFiles の消費、メタ（usage/changes=空）の読み方 |
| cheaper-code-write | WHEN: 機械的 code 変更（rename/定型 edit/雛形生成）。input（task/targets/constraints）、meta.changes（journal 実測）を verify する手順 |
| cheaper-doc-write | WHEN: 機械的 doc 作成・編集。input（task/documentType/targets/decisions）、unresolved の扱い |

worker skill は `cheaper-usage` を名前参照し、CLI 細部を重複記述しない。

### hook: `.apm/hooks/hooks.json`

```json
{
  "hooks": {
    "pre_read_code": [
      {
        "command": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\"",
        "powershell": "node ./pre-read-code.js \"./cli/bin/cheaper.mjs\"",
        "show_output": false
      }
    ]
  }
}
```

`./` 相対で script を参照し、APM が `.windsurf/hooks/<pkg>/` へ bundle
deploy する（DISC-0005 実測）。

### hook script: `pre-read-code.js`

zero-dep CommonJS。処理:

1. stdin を全部読み JSON.parse。失敗 → exit 0
2. `agent_action_name !== "pre_read_code"` → exit 0
3. `tool_info.file_path` を `fs.statSync`。非 file / 非存在 → exit 0
4. `cheaper count-tokens <abs path> --cwd <process.cwd()>` を
   `spawnSync` で実行（hook の cwd は workspace root が default）
   - CLI 解決: argv[2]（hooks.json が渡す bundled path、DESIGN-0014）→
     `process.env.CHEAPER_CLI` → `cheaper` on PATH。
     `.js`/`.mjs` で終わる path は `process.execPath`（node）で起動
   - 起動失敗 / 非 0 exit / envelope 非 `ok` → exit 0（fail-open）
5. `result.tokens > THRESHOLD` → stderr に guidance を書き exit 2。
   それ以外 → exit 0

THRESHOLD = `parseInt(process.env.CHEAPER_READ_TOKEN_THRESHOLD)` または
default `8192`（provisional。Phase 6 benchmark で確定値へ更新し、
この default を docs から参照可能な定数として定義する）。

block message（stderr、agent が読む）:

```text
cheaper: '<path>' has <N> tokens (threshold <T>). Do not read it directly.
Delegate with `cheaper bulk-read` (see the cheaper-bulk-read skill):
echo '{"task":"<goal>","paths":["<path>"]}' | cheaper bulk-read
```

## Data and Control Flow

```text
Cascade intends Read(<file>)
  → pre_read_code hook (stdin: {agent_action_name, tool_info.file_path})
    → stat: not a file → exit 0 (read proceeds)
    → cheaper count-tokens → envelope
      → tokens <= T → exit 0 (read proceeds)
      → tokens > T  → exit 2 + stderr guidance (read blocked,
                       agent sees message and delegates to bulk-read)
```

## Non-Functional Constraints

- hook は read の成否を変えてはいけない（fail-open）。block は
  「測定で large と確定した」場合のみ
- hook 自体が遅いと全 read が遅延する。`count-tokens` は tokenizer の
  初期化込みでも数十 ms 程度であり、spawnSync で同期実行する
- script は repo 外 file を書かない・network を呼ばない

## Risks and Trade-offs

- `pre_read_code` の `file_path` が directory の場合は pass-through。
  dir 再帰 read の large 判定は未対応（SPEC-0008 Out of Scope 明記）
- Windsurf 以外の target に deploy された場合、未知 event は no-op の
  想定。害はないが Phase 5 で実機確認
- CLI 不在時は fail-open で素通し。誤 block より誤 pass を選ぶ
  （UX 破壊回避）

## Verification Notes

- `node .apm/hooks/pre-read-code.js` を e2e spawn し、
  large/small/missing/dir/非 JSON stdin の各 exit code を test
- `apm install --target windsurf` での deploy は DISC-0005 の実測で
  形式確認済み。consumer install の再現確認は Phase 4 pack と一体で行う
