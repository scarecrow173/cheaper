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

- **skills**（5 件）:
  - `cheaper-usage`: CLI contract（command・envelope・exit code・
    provider/model 設定）を agent へ教える
  - `cheaper-bulk-read` / `cheaper-code-write` /
    `cheaper-doc-write` / `cheaper-tool-operate`: 各 built-in
    worker への委譲方法
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
- **thin launcher**（`cheaper.sh` / `cheaper.ps1`）: PATH 上に
  install 済みの `cheaper` CLI を解決して起動する薄い wrapper。
  CLI 本体は consumer が `cargo install` で用意する（後述）。

## install

consumer（親 agent 環境）は 2 段階で install する:

1. **CLI 本体**: Cargo source install で PATH 上へ配置する
   （ADR-034Dmb3kNAO4Ub6FxuQCtC — Release download / bundled binary は
   廃止。前提: Rust ≥1.95・git・C toolchain・cargo bin の PATH 設定。
   詳細は [配布 guide](./distribution.md)）。

   ```bash
   cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli
   cheaper --version   # preflight
   ```

2. **skills / hooks**: `cheaper` の distributable APM package を依存
   として install する:

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
- **Claude Code**: `PreToolUse`（`read` / `grep` / `bash` /
  `powershell` の ASCII case-insensitive exact matcher）経由で
  `cheaper hook pre-tool-use --target pretooluse` が動作する。
  単一ファイルの read/grep と shell command 内の large-read を
  共有の全文 policy で判定する。deny は
  `hookSpecificOutput.permissionDecision: "deny"` + exit 0。
- **Cursor**: `beforeReadFile` / `beforeShellExecution` event
  経由で `cheaper hook before-read-file` / `before-shell-execution
  --target cursor` が動作する。応答は `{"permission":"deny"}`
  JSON（allow 経路でも `{"permission":"allow"}` を emit して
  schema 適合を保つ）。
- **Copilot**: `preToolUse` event（matcher `view|bash|powershell|
  shell`）経由で `cheaper hook pre-tool-use --target copilot`
  が動作する。Copilot では非 0 exit が deny 扱いになるため、
  hook は常に exit 0 で終了し、threshold 超過または
  `FILE_TOO_LARGE` verdict の呼び出しだけを stdout の
  `{"permissionDecision":"deny"}` で block する
  （ADR-034r05iHrZUE6WizQ4LgPa）。`view` の file-read に加え、
  shell tool（`bash` / `powershell` / `shell`）内の large-read
  も同一 policy で block される。worker 定義・schema・policy は
  binary に embedded されているため、copilot target の `.json`
  deploy 除外は worker の利用に影響しない（issue #85）。
- **Codex**: `PreToolUse`（Claude と同じ ASCII case-insensitive
  exact matcher と nested group 登録）経由で
  `cheaper hook pre-tool-use --target pretooluse` が動作し、
  `Bash` / `PowerShell` の shell command 内の large-read を block
  できる。**file-read interception は unsupported** — Codex の
  read 系 tool は `PreToolUse` を emit しない upstream gap
  （openai/codex#18491・#20204）が残存する。
- **Devin CLI（Windows）**: APM が `cheaper-usage` skill と一緒に配布する
  helper を明示実行し、`.devin/config.json` の native `PreToolUse`
  （read/grep）へ `cheaper hook pre-tool-use --target devin` を登録する。
  deny は `{"decision":"block","reason":...}` + exit 0。native shell
  tool は登録しない。互換 hook との競合は検出して停止し、自動移行は
  行わない。配置・更新・解除の手順は [配布 guide](./distribution.md) を参照。
- **agent-skills**: skills とその resource のみを deploy する target。
  native hook の自動登録はない。
- **その他の target**: hook の配布・有効性は APM と harness の対応に
  依存する。skills の利用と native hook の対応は区別して確認する。

skills（`cheaper-usage` 等）は target 非依存で機能する。

## shell 経由の file read 検出（issue #42）

read tool を介さない shell command 内の file read も、shared
routing policy が保守的 parser で read candidate を抽出し、
file-read と同一の token 計測（`count-tokens` と同じ判定経路）で
評価する。

- **検出対象 command**: `cat` / `head` / `tail` / `less` /
  `more`。option とその値（`head -n 200` 等）は path とみなさ
  ない。複数 operand は全て判定し、1 件でも deny なら command
  全体を block する。`|` / `||` / `&&` / `;` / 改行で区切った
  各 segment を独立して評価する（`cat a | grep x` の前段、
  `cat small.txt ; cat huge.ts` の後段など）。
- **fail-open（allow）**: 対象外 command、operand なし
  （`cat` 単体）、glob・`$`・backtick・`~`・redirect・未閉塞
  quote など安全に path を特定できない構文、`cd` / `pushd` /
  `Set-Location` 等で cwd が変わる segment の後段にある read
  candidate（実際に読まれる file を特定できないため ambiguous）、
  file 不存在・directory・workspace 外 path、CLI 起動失敗、
  `count-tokens` 出力の parse 失敗、malformed hook payload。
- **fail-closed（deny）**: token 数が threshold 超過、または
  parseable な `FILE_TOO_LARGE` envelope。deny の診断には
  block された command・対象 file・token 数または
  `FILE_TOO_LARGE`・閾値・`bulk-read` への誘導が含まれる。
- **既知の制限**:
  - `cd` 等の cwd 遷移は追跡しない。`cd src && cat huge.ts` の
    ように後段の read 対象が実行時 cwd に依存する command は
    ambiguous として fail-open する。実体（`src/huge.ts`）が
    threshold 超過でも見逃すが、誤 block を避ける保守設計として
    受け入れる。`cat src/huge.ts` のように workspace 相対で
    path を特定できる場合は通常どおり判定される。
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
8192 token）を変更できる。hook の判定は workspace 内 file に対する
token 計測結果（`count-tokens` と同一経路）のみに依存する。
workspace 外・不存在・非 file・CLI 起動失敗・計測失敗などの
infrastructure failure は fail-open（素通し）で、`FILE_TOO_LARGE`
（計測 byte cap 超過）は fail-closed で block される。

hook 呼び出し自体は Node.js を必要としない。`hooks.json` の
entry は POSIX では `sh ./cheaper.sh`、Windows（powershell key）では
`./cheaper.ps1` の thin launcher を経由する。launcher は PATH 上に
install 済みの `cheaper` CLI を解決して起動するだけで、download・
cache・version pinning を行わない。CLI 不在時は install command と
PATH 設定の案内を stderr に出して非 0 で終了する。launcher 契約の
詳細は [配布 guide](./distribution.md) を参照する。
hook 経路の計測は `usage.jsonl` に
記録されない（telemetry 対象は明示的な `cheaper run` /
`count-tokens` のみ）。

## 連携の確認

```bash
# PATH 上の CLI（cargo install 済み）を直接確認
cheaper --version
cheaper count-tokens src/huge.ts

# POSIX: deploy 済み launcher 経由でも同じ CLI が解決される
sh <deployed-hook>/cheaper.sh --version

# skills/hooks が install されたかは target の config を確認
# （例: .windsurf/hooks/ 等、target 固有の deploy path）
```

Windows も PowerShell launcher で確認する。

```powershell
& '<deployed-hook>/cheaper.ps1' --version
```

CLI が PATH 上に無い場合は launcher が install command と PATH 設定の
案内を stderr に出す。前提・PATH 設定の詳細は
[配布 guide](./distribution.md) を参照する。

実際の e2e 検証手順は [CONTRIBUTING.md](../../CONTRIBUTING.md) の
「実 provider 検証」を参照。

## PreToolUse 自動 route（実験的）

shell command を `route run` payload へ rewrite し、route に
宣言した workflow（command exec + 任意 worker postprocess）へ
実行を委譲する仕組み。`hook pre-tool-use` が command を判定し、
実行可能（runnable）な route への match 時のみ rewrite を返す
— hook 自体は実行を行わない。runnable でない場合は従来の
Allow/Deny verdict（legacy allow/deny）へフォールバックする。
一方、`route run` 側の payload 再検証は fail-closed で、
失敗時に raw command 実行へ落ちる経路は存在しない。

### route の解決と宣言層

route は 3 層で宣言する: **builtin**（`cheaper` 組み込み、
binary embedded）・**user**（`<userConfigDir>/routes.yaml`）・
**project**（`<workspace>/.cheaper/routes.yaml`）。層間の衝突は
同じ `match.command` token prefix を key に判定され（route 名
ではない）、上位層（builtin → user → project）が勝つ。project
層が下位を shadow するのは `trusted: true` のときのみ —
untrusted project route の match は suppressed provenance（
`suppressed` field）として報告され、下位の route は active
のまま残る。`trusted: true` は
「この workspace の宣言を信頼する」という**明示的な layer
override** であり、ユーザー承認やセキュリティ許可の代替では
ない。

builtin declaration の例（`resources/routes.yaml` に組み込み済み）:

```yaml
version: 1
routes:
  - name: cargo-test
    match: {command: [cargo, test]}
    target: {kind: workflow, path: routed-exec.workflow.json}
    grant: cargo-test
    output: {tokenThreshold: 8192}
    passthroughEnv: [CI]
```

target の `path` は各層を root とする相対パス — builtin は
`builtin/workflows/`、user は `<userConfigDir>/`、project は
`<workspace>/.cheaper/` が起点となる。route を宣言するだけでは
target workflow はその層へ配置されず、実行権限（grant）も
発生しない。

builtin の `cargo-test` route は組み込み済みだが、builtin
policy に対応 grant は含まれない — **no-grant は fail-closed**
で、ユーザーが明示的に grant するまで実行できない
（opt-in）。確認は dry-run で行う:

```bash
cheaper route resolve --command 'cargo test --workspace'
cheaper route resolve --argv '["cargo","test","--workspace"]'
```

`resolve` は実行しない（dry-run）— match・route・target・grant・
viability（`runnable`/`no-grant`/…）・sanitized argv/args を
JSON で返す。`no-grant` のままなら権限が無い。grant は user
policy に追加する（下記は PLACEHOLDER — 実際の cargo path に
置き換えること）:

```bash
cheaper policy grant add cargo-test '{"executable":"<absolute-native-cargo-path>","argv":["test","*"]}'
```

grant は repository のコードや build script を実行する権限を
与える — OS sandbox ではなく、`argv` allowlist で対象 executable
と引数形を限定する仕組み。route の `grant:` 参照が解決できるのは
`scope:"trusted" && !fixed` の grant のみ（unified `exec` grant
namespace — [configuration ガイド](./configuration.md)）で、
`process` task 由来の `fixed` grant・`scope:"project"` grant・
未知名は `no-grant` viability に畳まれる。

### hook rewrite の承認保全と fallback

`hook pre-tool-use` の rewrite 出力は native adapter の
approval 機構を通る — cheaper は承認を bypass しない:

- **Claude / Devin**: `hookSpecificOutput.updatedInput.command`
  で rewrite command を返す — `permissionDecision`/`decision`
  は付けず、承認は native 側の通常経路で行われる
- **Codex / Cursor / Copilot**: rewrite の approval-bypass
  検証が未了のため `updatedInput` を発行しない — 従来評価の
  fallback verdict（deny/allow）をそのまま返す
- **Windsurf**: fallback verdict 毎の native shape —
  fallbackAllow は silent exit 0、fallbackDeny は stderr +
  exit 2(read candidate 評価の deny 規約をそのまま維持)

rewrite command は `<cheaper-exe> route run --payload <token>`。
payload は opaque な base64url — 値は process の
command line に現れ得る（cmdline は秘匿路ではない）。実行時に
現在の routes/policy へ再検証され、route 削除・grant 削除・
target 不在・cwd 不一致等の stale payload は全て fail-closed
で拒否 — raw command fallback は存在しない。

### `route run` の出力

`route run --payload <token>` は実行に成功すれば**raw text**
（stdout+stderr + route log footer）を直接 emit し、
`--format json` でも envelope 化しない。子 process の
exit code をそのまま mirror する — 元 command が非ゼロなら
route run も非ゼロ。一方、payload 検証・policy 違反等の
infrastructure error は従来の error envelope を返す。

### `route logs` と log の取り扱い

`route run` は子出力を `<userConfigDir>/route-logs/<runId>.log`
（Unix では owner 限定の `0600` permission が適用される。
Windows filemode では同規約が異なるため、owner 限定 permission
は Unix `0600` の意図として扱うこと。workspace 外）へ全量
tee する。
`cheaper route logs <id> [--offset <bytes>] [--max-bytes <bytes>]`
で bounded page を読む — offset/byte 単位（UTF-8 境界をまたぐ
場合がある）で `{data,offset,nextOffset,eof}` を返し、
`maxBytes` 未指定は policy の `workflow.artifacts.maxReadBytes`
上限に従う。無効・不在 runId は拒否する。retention は
best-effort sweep（30 日 / 200 件 / 256 MiB 上限）。

### env と計測

hook 実行環境は interactive shell env とは一致しない —
`passthroughEnv` allowlist に列挙した名前だけが hook ambient
env から payload へ capture される。command prefix
（`FOO=bar cmd`）> captured ambient > grant env の順に適用
され、子 env は最小化される。stdin を使う command や
interactive/TTY 前提の実行には対応しない。

route record（`kind:"route"`）と refetch record
（`kind:"route-refetch"`）は別々に usage.jsonl へ記録する —
集計は refetch 数を join して出す（[usage ガイド](./usage.md)
参照）。env 値は payload transport にのみ使われ、usage record
や worker input へ serialise されない — ただし子 process
自身が env 値を stdout/log へ print することはあり得るため、
route log は sensitive として扱うこと。worker postprocess の
skip・失敗（`postprocessFailures`）は command 実行や exit
code に影響しない — リトライもしない。

## 関連

- CLI contract: [usage ガイド](./usage.md)
- provider / auth 設定: [providers ガイド](./providers.md)
- 設計記録: `docs/designs/apm-skills-and-pre-read-hook.md`・
  `docs/adr/distribute-cli-via-cargo-source-install-and-thin-path-launchers.md`
  （現行の配布 ADR — 旧 bundled CLI / Release 取得 ADR は
  superseded）・
  `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
