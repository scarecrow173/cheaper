---
id: "DESIGN-034z1EhuztlLSb2x2XXxgD"
type: "design"
status: "draft"
title: "cheaper hook subcommands replacing Node-based hook adapters"
created: "2026-10-02"
updated: "2026-10-02"
owners: ["scarecrow173"]
relations:
  source:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
  implemented-by: []
  depends-on:
    - "./sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "./apm-executable-payload-layout-and-platform-launchers.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./apm-skills-and-pre-read-hook.md"
    - "./copilot-pre-read-hook-adapter.md"
    - "./cross-agent-native-routing-adapter-architecture.md"
    - "./shell-read-candidate-extraction.md"
    - "./huge-file-byte-cap-guard-and-fail-closed-hook.md"
    - "./sea-executable-build-pipeline-and-embedded-resource-resolution.md"
    - "./apm-executable-payload-layout-and-platform-launchers.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/standalone-executable-distribution-for-cheaper-cli-including-apm-bundles.md"
    - "../adr/distribute-cheaper-as-node-26-sea-standalone-executables-shared-by-direct-releas.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# cheaper hook subcommands replacing Node-based hook adapters

## Context

SPEC-034z14nGIf6fXRtTskkrk8 / ADR-034z1xMK43jRs3D79Urhpc の実装設計
（issue #71 の Phase 4 に相当）。CLI 本体を executable 化しても現行
`packages/apm/.apm/hooks/` 配下の JS adapter（`pre-read-code.js`、
`pre-read-code-copilot.js`、`pre-read-code-cursor.js`、
`pre-read-code-pretooluse.js`、`pre-run-command.js`）と shared
`routing-policy.js` は Node.js を要求する。これらの payload 正規化と
routing 判定を `cheaper hook <event> --target <agent>` 系の CLI 内部
command に移し、hook 実行全体を Node.js 非依存にする。

## Scope

### In Scope

- `cheaper hook <event> --target <agent>` 系の CLI 内部 command
- 現行 `routing-policy.js` の canonical request 評価（workspace
  containment・byte-size fast path・`count-tokens` 計測・token
  threshold・`FILE_TOO_LARGE` fail-closed・shell read candidate
  extraction）の CLI 内移植
- target 別の stdin payload 正規化と verdict の response contract 変換
- `hooks.json` の command を launcher 経由の executable 呼び出しへ変更
- target 別 hook contract の regression test
- 移行完了後の旧 JS hook runtime file 削除

### Out of Scope

- platform launcher の配置・実装（DESIGN-034z1EgnQwax4VamK2xmdT）
- SEA executable の build 詳細（DESIGN-034z1EfqYaEBfyNvqZyFSD）
- routing 判定ルール自体の変更（大きい read の閾値・policy 文言などは
  現行維持）
- 新しい hook event や target の追加

## Components and Boundaries

- **`packages/cli` 側の hook command 実装**（`cheaper hook ...`）:
  stdin JSON を target 別 adapter が canonical request に正規化し、
  shared policy が判定、verdict を target 別 response contract に写す。
- **canonical routing policy の CLI 内モジュール**: 現行
  `routing-policy.js` と同じ知識（workspace 判定・byte fast path・
  token 計測・閾値・shell read candidate 抽出・fail-open/fail-closed
  規則）を CLI 側の 1 モジュールに集約する。
- **target adapter**: Windsurf / Copilot / Claude Code・Codex /
  Cursor ごとの payload normalize と response render に限定する。

## Data and Control Flow

```text
native hook payload (stdin)
  ↓ target adapter が canonical request に正規化
    { kind: "file-read" | "shell", path|command, cwd, sourceTool }
  ↓ shared routing policy
    workspace containment → byte fast path → count-tokens 計測 → verdict
  ↓ target 別 response contract に変換
stdout / stderr / exit code
```

現行との対応:

| target | event | 現行 file | deny 表現 |
| --- | --- | --- | --- |
| windsurf | `pre_read_code` / `pre_run_command` | `pre-read-code.js` / `pre-run-command.js` | stderr + exit 2 |
| copilot | `preToolUse` | `pre-read-code-copilot.js` | stdout `{"permissionDecision":"deny"}` + exit 0 |
| claude / codex | `PreToolUse` | `pre-read-code-pretooluse.js` | stdout `hookSpecificOutput.permissionDecision:"deny"` + exit 0 |
| cursor | `beforeReadFile` / `beforeShellExecution` | `pre-read-code-cursor.js` | stdout `{"permission":"deny",...}` 常に schema-valid + exit 0 |

command surface の例:

```bash
cheaper hook pre-read-code --target windsurf
cheaper hook pre-run-command --target windsurf
cheaper hook pre-tool-use --target copilot
cheaper hook pre-tool-use --target claude
cheaper hook before-read-file --target cursor
cheaper hook before-shell-execution --target cursor
```

cursor は現行 adapter が payload key（`file_path` / `command`）で両 event
を区別するため、同一 adapter 実装を 2 event で共有してよい。

## Non-Functional Constraints

- fail-open / fail-closed 規則は target ごとに現行と同一に保つ:
  infra error・malformed payload は allow 方向、measured deny と
  `FILE_TOO_LARGE` のみ fail-closed
- copilot adapter は非 0 exit を絶対に返さない。cursor は常に
  schema-valid permission JSON を返す
- `count-tokens` は子 process spawn ではなく CLI 内部の同一 runtime
  経路で評価し、`CHEAPER_CLI` / `node` 経由を前提にしない
- `CHEAPER_READ_TOKEN_THRESHOLD`（default 8192）などの環境変数契約は
  維持する
- hooks.json の command は platform launcher 経由で executable を呼ぶ
  形とし、adapter JS への `node` 参照を残さない

## Risks and Trade-offs

- routing policy を CLI 内に移すと `cheaper count-tokens` の子 process
  呼び出し（30s timeout）が in-process 評価に変わる。timeout 意味の
  変化を受け入れるか、worker 側で明示的な bounded 実行を入れるかは
  実装時に確認する
- target 別 payload drift（copilot の `toolArgs` が object/string 両対応
  等）を contract test で固定しないと deploy 後に検出しにくい
- JS adapter 削除は破壊的変更になり得るため、移行期間中は旧 file を残し
  hooks.json の切替と削除を分ける

## Deferred Design Concerns

- hook 呼び出しの trace / diagnostics 出力 | 現行 adapter は fail-open で
  diagnostics を持たない | hook が期待通り発火しない報告があった時点 |
  障害切り分けが難しい
- `cheaper hook` を standalone CLI 以外（通常 Node 実行）でも使える形に
  するか | まず executable 経路を canonical にする | 非 APM 経路で同じ
  hook を使いたい要求が出た時点 | 二重実装リスク

## Verification Notes

- [ ] 各 target で stdin payload → stdout/stderr/exit code が現行 adapter
      と一致する contract test がある
- [ ] `cheaper hook ...` が Node.js を使わずに executable から実行できる
- [ ] `hooks.json` の command が launcher 経由の executable 呼び出しに
      なり、`node` 参照を含まない
- [ ] 旧 `pre-read-code*.js` / `pre-run-command.js` / `routing-policy.js`
      が移行後に削除される
- [ ] `CHEAPER_READ_TOKEN_THRESHOLD` と `FILE_TOO_LARGE` fail-closed が
      executable 経路でも同じ意味を持つ
