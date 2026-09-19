# Parent-agent 連携ガイド

`cheaper` は親 Coding Agent（Claude Code / Codex / Copilot /
Cursor / Windsurf 等）から処理を委譲するための worker runtime。
本 guide は親 agent と cheaper の責務分担・配布 primitive・
target 別挙動・検証方法を説明する。

## 役割分担

- **親 Coding Agent**: 判断・統合・ユーザーとの対話。task の
  定義・worker の選定・結果の解釈・workspace への最終統合を行う。
- **cheaper**: 機械的・大 context の処理の実行。worker 定義・
  tool 制約・model 解決・schema 検証・usage 計測を行い、
  構造化された result と meta を返す。

親 agent は `cheaper` を 1-shot CLI として呼び出し、stdin に
task JSON を渡して stdout の envelope を読む（詳細は
[usage ガイド](./usage.md)）。

## 配布 primitive

cheaper は parent agent へ以下を配布する（APM package 経由）:

- **skills**（4 件）:
  - `cheaper-usage`: CLI contract（command・envelope・exit code・
    provider/model 設定）を agent へ教える
  - `cheaper-bulk-read` / `cheaper-code-write` /
    `cheaper-doc-write`: 各 built-in worker への委譲方法
- **hooks**（`pre_read_code` + shell hook）: read 前に
  `count-tokens` を実行し、threshold（default 8192 token、
  `CHEAPER_READ_TOKEN_THRESHOLD` で上書き）を超える file の
  read を block して `bulk-read` へ誘導する。read tool の直接
  呼び出しに加え、`cat` / `head` / `tail` / `less` / `more`
  による **shell 経由の file read も同一 policy で block**
  される（issue #42、後述の「shell 経由の file read 検出」）。
  malformed payload・file 不存在・非 file・workspace 外・CLI
  起動失敗・出力の parse 失敗といった infrastructure failure
  は fail-open（素通し）なので、設定が壊れていても通常の
  read が止まらない。一方、`count-tokens` が parseable な
  `FILE_TOO_LARGE` envelope を返した場合は「計測不能だが確実に
  巨大」と authoritative に判定して fail-closed で block する
  （Windsurf: exit 2、Copilot: `permissionDecision: "deny"` +
  exit 0 — SPEC-034r05iHK8bA7gjwZTji8t /
  DESIGN-034r05iHy2gdnoLt52txFG の意図的仕様）。
- **bundled CLI**: self-contained の `cheaper` CLI（skills/hooks
  が内部から直接呼べる）

## install

consumer（親 agent 環境）は `cheaper` の distributable APM package
を依存として install する:

```yaml
# consumer の apm.yml
dependencies:
  apm:
  - scarecrow173/cheaper/packages/apm#main   # virtual package 参照
  # または local path: - path: ../cheaper/packages/apm
```

```bash
apm install --target windsurf   # 利用する target を指定
```

## target 別挙動

hook の動作は target の hook event 体系に依存する。

- **Windsurf**: `pre_read_code` event が native で、read 前に
  hook が実行される。threshold 超過 file は block + bulk-read
  への誘導メッセージが返る。`pre_run_command` event は shell
  command を adapter が受け取り、`cat` / `head` / `tail` /
  `less` / `more` による large-read は deny（exit 2 + stderr
  の誘導メッセージ）で block される。
- **Claude Code**: `PreToolUse`（`Read` / `Bash` / `PowerShell`
  matcher）経由で adapter（`pre-read-code-pretooluse.js`）が
  動作する。`Read` の file-read に加え、`Bash` / `PowerShell`
  の shell command 内の large-read も block される。deny は
  stdout の `hookSpecificOutput.permissionDecision: "deny"`
  JSON で返し、adapter は常に exit 0。
- **Cursor**: `beforeReadFile` / `beforeShellExecution` event
  経由で adapter（`pre-read-code-cursor.js`）が動作する。応答は
  `{"permission":"deny"}` JSON（allow 経路でも
  `{"permission":"allow"}` を emit して schema 適合を保つ）。
- **Copilot**: `preToolUse` event（matcher `view|bash|powershell|
  shell`）経由で adapter（`pre-read-code-copilot.js`）が動作
  する。Copilot では非 0 exit が deny 扱いになるため、adapter
  は常に exit 0 で終了し、threshold 超過または
  `FILE_TOO_LARGE` verdict の呼び出しだけを stdout の
  `{"permissionDecision":"deny"}` で block する
  （ADR-034r05iHrZUE6WizQ4LgPa）。`view` の file-read に加え、
  shell tool（`bash` / `powershell` / `shell`）内の large-read
  も同一 policy で block される。既知の制限として、現行の APM
  は copilot target へ `.json` asset を deploy しないため、
  file-schema worker（bulk-read / code-write / doc-write）は
  upstream 修正待ちの間利用できない（`count-tokens` と hook
  adapter は動作する）。
- **Codex**: `PreToolUse`（`Read|Bash|PowerShell` matcher、
  Claude と同一の nested group 登録）経由で adapter
  （`pre-read-code-pretooluse.js`）が動作し、`Bash` /
  `PowerShell` の shell command 内の large-read を block
  できる。**file-read interception は unsupported** — Codex の
  read 系 tool は `PreToolUse` を emit しない upstream gap
  （openai/codex#18491・#20204）が残存する。
- **その他 target**（agent-skills 等）: hook 登録は deploy されるが、
  認識されない event 名（例: `pre_read_code`）の登録は no-op と
  して扱われる（登録は harmless。DISC-034r05iHy2gdnpdai9KGpW）。

skills（`cheaper-usage` 等）は target 非依存で機能する。

## shell 経由の file read 検出（issue #42）

read tool を介さない shell command 内の file read も、shared
routing policy が保守的 parser で read candidate を抽出し、
file-read と同一の `count-tokens` 判定へ流す。

- **検出対象 command**: `cat` / `head` / `tail` / `less` /
  `more`。option とその値（`head -n 200` 等）は path とみなさ
  ない。複数 operand は全て判定し、1 件でも deny なら command
  全体を block する。`|` / `||` / `&&` / `;` / 改行で区切った
  各 segment を独立して評価する（`cat a | grep x` の前段、
  `cd src && cat huge.ts` の後段など）。
- **fail-open（allow）**: 対象外 command、operand なし
  （`cat` 単体）、glob・`$`・backtick・`~`・redirect・未閉塞
  quote など安全に path を特定できない構文、file 不存在・
  directory・workspace 外 path、CLI 起動失敗、`count-tokens`
  出力の parse 失敗、malformed hook payload。
- **fail-closed（deny）**: token 数が threshold 超過、または
  parseable な `FILE_TOO_LARGE` envelope。deny の診断には
  block された command・対象 file・token 数または
  `FILE_TOO_LARGE`・閾値・`bulk-read` への誘導が含まれる。
- **既知の制限**:
  - `cd` / cwd 遷移は追跡しない。`cd src && cat huge.ts` は
    hook 起動時の cwd を基準に `huge.ts` を評価する。
  - shell expansion・command substitution は実行しないため、
    展開を伴う operand は fail-open で素通しになる。
  - `sed` / `awk` / `python` 等の任意コマンド経由の read は
    検出対象外（non-goal）。
  - Codex の file-read interception は unsupported（前述）。

## 自動 vs 明示的な委譲

- **自動**: hook（`pre_read_code`）が threshold 超過 read を
  検出して parent agent へ bulk-read 委譲を促す。
- **明示**: parent agent（またはユーザーが skill を発動）が
  `cheaper bulk-read` 等を直接呼ぶ。

## 閾値の設定

`CHEAPER_READ_TOKEN_THRESHOLD` で large-read threshold（default
8192 token）を変更できる。hook の判定は workspace 内 file に
対する `count-tokens` の結果 envelope のみに依存する。
workspace 外・不存在・非 file・CLI 起動失敗・出力の parse 失敗
などの infrastructure failure は fail-open（素通し）で、
parseable な `FILE_TOO_LARGE` envelope は fail-closed で
block される。

## 連携の確認

```bash
# hook bundle に deploy された CLI の動作確認
<deployed-hook>/cli/bin/cheaper.mjs --version
<deployed-hook>/cli/bin/cheaper.mjs count-tokens <file>

# skills/hooks が install されたかは target の config を確認
# （例: .windsurf/hooks/ 等、target 固有の deploy path）
```

実際の e2e 検証手順は [CONTRIBUTING.md](../../CONTRIBUTING.md) の
「実 provider 検証」を参照。

## 関連

- CLI contract: [usage ガイド](./usage.md)
- provider / auth 設定: [providers ガイド](./providers.md)
- 設計記録: `docs/designs/apm-skills-and-pre-read-hook.md`・
  `docs/adr/distribute-the-cheaper-cli-as-a-committed-esbuild-bundle-inside-the-apm-hook-bun.md`・
  `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
