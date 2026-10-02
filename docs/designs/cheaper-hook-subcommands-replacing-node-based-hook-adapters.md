---
id: "DESIGN-034z1EhuztlLSb2x2XXxgD"
type: "design"
status: "approved"
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
cheaper hook pre-tool-use --target pretooluse   # claude / codex 共用
cheaper hook before-read-file --target cursor
cheaper hook before-shell-execution --target cursor
```

cursor は現行 adapter が payload key（`file_path` / `command`）で両 event
を区別するため、同一 adapter 実装を 2 event で共有してよい。
claude・codex は hooks.json の `PreToolUse` nested entry を 1 command で
共有する（command を target 別に分けられない）ため、両者を `pretooluse`
という単一 target label に束ねる。

## Non-Functional Constraints

- fail-open / fail-closed 規則は target ごとに現行と同一に保つ:
  infra error・malformed payload は allow 方向、measured deny と
  `FILE_TOO_LARGE` のみ fail-closed
- copilot adapter は非 0 exit を絶対に返さない。cursor は常に
  schema-valid permission JSON を返す
- `count-tokens` は子 process spawn ではなく CLI 内部の同一 runtime
  経路で評価し、`CHEAPER_CLI` / `node` 経由を前提にしない。旧 30s
  subprocess timeout は `AbortSignal.timeout(30_000)` で measured path を
  囲んで等価に復元する（FIFO / device file / 死んだ network FS の
  unbounded I/O hang を防ぐため）
- `cheaper hook` 経路は runCli の envelope / `recordUsage` / 汎用 catch
  から完全に分離する。最深部 catch-all で全 error（args parse 失敗・
  policy 破損・cwd 不存在等を含む）を target 適合の allow + exit 0 に
  変換し、`exit 2` / 非 0 が host 側で deny 意味を持つ経路への漏洩を
  防ぐ。hook 経路は `usage.jsonl` に記録しない（意図的決定）
- `CHEAPER_READ_TOKEN_THRESHOLD`（default 8192）などの環境変数契約は
  維持する
- hooks.json の command は platform launcher / executable 経由とし、
  adapter JS への `node` 参照を残さない。POSIX `command` は `./cheaper`
  launcher を使うが、`powershell` key は ExecutionPolicy（Restricted）
  を避けるため launcher を介さず `cli/bin/win32-x64/cheaper.exe` を直接
  起動する。fail-closed target（copilot: 非 0 全 deny、cursor: 非 0 /
  schema-invalid は block 扱いの可能性）の entry には shell 側 fail-open
  （copilot `|| exit 0`、cursor `|| echo '{"permission":"allow"}'`）を
  command に付与し、launcher/exe 不在・LFS pointer 化・deploy 欠損時の
  deny-all を防ぐ。windsurf entry は exit 2 の deny 意味を潰さないよう
  suffix を付けない

## Risks and Trade-offs

- routing policy を CLI 内に移すと `cheaper count-tokens` の子 process
  呼び出し（30s timeout）が in-process 評価に変わる。`maxBytes` の read
  cap だけでは FIFO / device file の `readFile` hang や死んだ network FS
  の `stat` hang を防げないため、measured path 全体を
  `AbortSignal.timeout(30_000)` で囲んで旧 cap を復元する
- `countFileTokens` には `isFile()` 判定がなく、containment → `isFile` →
  byte fast path の順序を落とすと workspace 内 FIFO で無期限 block する。
  移植時にこの順序を verbatim で維持する
- `@cheaper/core` の tokenizer は module scope eager init のため、
  `cheaper hook` の static import 経路に乗せると tool call 毎の hook
  起動に decode コストが載る。count 系 import は byte fast path 通過後の
  dynamic import に限定する
- target 別 payload drift（copilot の `toolArgs` が object/string 両対応
  等）を contract test で固定しないと deploy 後に検出しにくい
- symlinked cwd 環境では現行実装が常に fail-open 化する quirk がある
  （raw cwd で resolve した path が canonical root と不一致 →
  `WorkspaceEscapeError` → allow）。in-process 化でもこの quirk を再現し
  て挙動不変を保つ
- macOS 等の未同梱 platform では launcher が `exit 1` で fail し、
  copilot の fail-closed 意味論（非 0 全 deny）で deny-all になる。
  hooks.json の command 側 fail-open で吸収する
- remote install で LFS smudge が無い consumer に executable が pointer
  text のまま届く可能性がある。launcher で pointer 検出して明確な error
  を出す
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
- [ ] `hooks.json` の command が launcher / executable 経由の呼び出しに
      なり、`node` 参照を含まない
- [ ] 旧 `pre-read-code*.js` / `pre-run-command.js` / `routing-policy.js`
      が移行後に削除される
- [ ] `CHEAPER_READ_TOKEN_THRESHOLD` と `FILE_TOO_LARGE` fail-closed が
      executable 経路でも同じ意味を持つ
- [ ] hook 処理中の例外（args parse 失敗・policy 破損・cwd 不存在等）が
      runCli の汎用 catch まで到達せず、target 適合の allow + exit 0 に
      変換される
- [ ] measured path が `AbortSignal.timeout(30_000)` で旧 subprocess
      timeout を復元している
