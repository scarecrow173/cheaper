# Migration guide

cheaper の破壊的変更と移行手順を issue 番号起点で列挙する — version 番号への紐付けは release 時の責務。以下は現在の main に適用される変更群（#163–#168）である。

## 前提: cheaper が強制するもの・しないもの

- cheaper は OS sandbox ではない。
- canonical file tool（write/edit/ls/read/find/grep）の workspace boundary（E1/E2）・`filesystem.denyPaths`・worker `writeScope` は cheaper 所有の tool surface で強制される。spawn した子プロセスの fs 副作用は、OS sandbox 無しでは cheaper が強制・完全観測できない。
- cheaper の spawn は shell を介さず argv を直接渡す（shell metachar は executable の literal argv として渡り、cheaper が解釈・拒否しない）。POSIX で shebang を持たない +X file の実行が ENOEXEC となった際に libc が `/bin/sh` で retry するのは OS/libc 既定動作（glibc `execvpe` 系。musl は retry しない残留差分あり）であり cheaper の保証対象外。

## #163 — 内容審査 7 項目の除去

### 変更内容

exec 系 grant（`command` grant・`process` task）の executable/argv に対する内容審査 7 項目を除去: A8 shell/shim 拒否・B6 script ext 拒否・B3 native magic 検査・B4 workspace-resident executable 拒否・B5 bare `node` 拒否・A10 `argv_shell_re` metachar 拒否（call-time argv + task 宣言 argv の両適用点）・F3 process task の native/shim 検査。executable 解決は spawn viability（realpath・is_file・win32 `.exe`/`.com`・POSIX X_OK）のみを検査し、内容・名は審査しない。

### 利用者影響

従来拒否されていた宣言 executable（`python`/`ssh`/`tar`/`.sh`/workspace-resident binary/bare `node`）と metachar を含む argv が受理される — 既定では内容に基づく拒否は何も効かない挙動変化。spawn viability に適合する限り grant は実行される。

### 修復・移行手順

拒否近似が必要な場合は #164 の `security.execDenyPatterns` を user `policy.yaml` へ宣言する（seed 例は次節）。

## #164 — builtin executable deny list 削除 → opt-in `security.execDenyPatterns`

### 変更内容

builtin の mandatory executable deny list を削除し、opt-in の `security.execDenyPatterns`（deterministic executable deny-list・既定 `[]`）として再導入。宣言された policy がある場合のみ deterministic pattern match で executable を拒否する。

### 利用者影響

旧 mandatory deny は `security.execDenyPatterns` を宣言しない環境では一切効かない。宣言時のみ deny 該当 executable が拒否される（`--grant` 経路・`policy grant add` 経路・policy file load のいずれも同一検査）。

### 修復・移行手順

旧 mandatory 挙動を近似再現するには user `policy.yaml` へ seed を宣言する。旧 regex の verbatim（コメント除去・単一行 — `command` guide の deny-list 節から転記）:

```yaml
security:
  execDenyPatterns:
    - 'pythonw?|python\d+(?:\.\d+)*[a-z]?|pypy\d*(?:\.\d+)*|perl\d*(?:\.\d+)*|ruby\d*(?:\.\d+)*|node|nodejs|deno|bun|php\d*(?:\.\d+)*|lua|luajit|lua[\d.]+|r|rscript|julia|wscript|cscript|mshta|java|javaw|groovy|tclsh|wish|expect|osascript|env|xargs|find|awk|gawk|mawk|nawk|sed|busybox|toybox|nice|nohup|setsid|timeout|watch|stdbuf|flock|taskset|sudo|doas|su|pkexec|setpriv|nsenter|unshare|chroot|docker|podman|systemd-run|ssh|scp|rsync|screen|tmux|ld\.so|ld-linux.*|ld-musl.*|dyld|rundll32|regsvr32|msbuild|dotnet|installutil|regasm|regsvcs|forfiles|pcalua|msiexec|wsl|jshell|vim|ex|nvim|less|man|mysql|psql|sqlite3|tar|nu|elvish|xonsh|oil|ion'
```

可読性のため分類別に分解した同等 seed（OR 評価のため同じ効果）:

```yaml
security:
  execDenyPatterns:
    # interpreters（versioned name まで）
    - 'pythonw?|python\d+(?:\.\d+)*[a-z]?|pypy\d*(?:\.\d+)*|perl\d*(?:\.\d+)*|ruby\d*(?:\.\d+)*|node|nodejs|deno|bun|php\d*(?:\.\d+)*|lua|luajit|lua[\d.]+|r|rscript|julia|wscript|cscript|mshta|java|javaw|groovy|tclsh|wish|expect|osascript'
    # exec wrapper / launcher（grant は shell の grant と等価）
    - 'env|xargs|find|awk|gawk|mawk|nawk|sed|busybox|toybox|nice|nohup|setsid|timeout|watch|stdbuf|flock|taskset|sudo|doas|su|pkexec|setpriv|nsenter|unshare|chroot|docker|podman|systemd-run|ssh|scp|rsync|screen|tmux'
    # dynamic loader
    - 'ld\.so|ld-linux.*|ld-musl.*|dyld'
    # Windows LOLBin + wsl
    - 'rundll32|regsvr32|msbuild|dotnet|installutil|regasm|regsvcs|forfiles|pcalua|msiexec|wsl|jshell'
    # editor / pager / REPL escape
    - 'vim|ex|nvim|less|man|mysql|psql|sqlite3|tar'
    # 追加 shell
    - 'nu|elvish|xonsh|oil|ion'
```

**注記** — 上記は旧 mandatory 挙動の近似再現であり厳密な等価ではない。旧実装はこの regex とは別の predicate で shell・shim 名（`sh`/`bash`/`cmd`/`powershell` 等）も拒否していたが、そちらは #163 で別機構として除去済みで本一覧には含まれない。同等の shell 拒否が必要なら `sh|bash|dash|zsh|fish|cmd|powershell|pwsh` 等の pattern を operator が別途追加する。

## #165 — 統一 exec grant namespace

### 変更内容

`command.grants` と `process.tasks` を単一の `exec` grant namespace へ統合（resolved struct `RuntimePolicy.command` は `RuntimePolicy.exec` へ rename — file-level の `command.grants`・`process.tasks` 宣言 shape・`command`/`process` 節は loader 写像 + alias で存続し即死しない）。名衝突（`command.grants` ∩ `process.tasks` の同名）は `InvalidRuntimePolicy` + `details.{grant,task}` で fail closed。`--grant`/`policy grant add` の CLI spec が `fixed:true`・明示 `scope` を含む場合は `InvalidRequest` で拒否。project task は `scope:"project"` として解決される（route から不可視）。

### 利用者影響

authoring の section 分離（`command.grants` と `process.tasks` の file-level 記述位置）は維持されるが、grant 名空間は共有となる。`command.grants.build` と `process.tasks.build` のように同名を両 section で使っていた policy は load 時に fail closed で拒否される。CLI から `fixed`/`scope` を含む grant spec を渡すと拒否される。

### 修復・移行手順

- 名衝突が発生した場合は一方の grant/task を rename する。
- `--grant`/`policy grant add` に渡す spec から `fixed`・`scope` key を除去する（`fixed:true` の固定 grant を宣言したい場合は trusted layer の policy YAML で直接 `process.tasks` 側を authoring する）。

## #166 — route grant-set 投影と `kind:"route"` record split

### 変更内容

routes.yaml に trusted 層限定で `grants: [name, …]` の set 投影を追加（project 層 `grants` key presence は `InvalidRuntimePolicy` で拒否）。`kind:"route"` record を `{command, executed}` へ split し、`executed` に実行実測ペア（grant・argv）を記録する。

### 利用者影響

- `executed.argv` は平文で記録される — argv に secret を含む command を route 経由で実行すると record に残留する。
- project 層 routes.yaml に `grants` key を書くと route load が全滅する（`InvalidRuntimePolicy`）。
- record schema は additive — 旧 reader は `executed` field を無視して従来どおり集計できる。

### 修復・移行手順

- project routes.yaml の `grants` key を除去する（新 key のため既存 file への影響はその key を宣言した場合のみ）。
- `executed` を含む record を扱う集計系は additive field として読み替える（旧 reader は無変換で動作する）。

## #167 — substitute-exec route mode

### 変更内容

routes.yaml に trusted 層限定で `substitute: {grant, argv}` 宣言を追加（project 層 `substitute` key presence は `grants` と同じく `InvalidRuntimePolicy` で拒否）。route が substitute を宣言する場合、typed command の代わりに substitute argv が実行され、exit は「実行した command」の exit を mirror する。substitute mode では親 agent が substitute の exit を原 command のものとして解釈する — semantic substitution を trusted opt-in で受容する設計。

### 利用者影響

新機構 — 既存 route の挙動への影響なし。project 層 routes.yaml に `substitute` key を書くと `grants` と同じく `InvalidRuntimePolicy` で拒否される。

### 修復・移行手順

なし（新機能）。project 層での使用を意図していた場合は `substitute` key を除去するか、route を user 層 routes.yaml へ移す。

## #168 — project 層 `trusted` key presence 拒否

### 変更内容

project 層 routes.yaml の `trusted` key presence を値不問（`true`/`false`/非 bool いずれも）で `InvalidRuntimePolicy` 拒否（`{label}.trusted: not declarable in the project layer`）。`Route.trusted` field は削除・merge は無条件化 — project 層 route は下位層を shadow せず、同名衝突は常に project 側が負けて `suppressed` provenance へ記録される。builtin/user 層の `trusted:` 宣言は受理されるが inert。

### 利用者影響

- `trusted` を含む project routes.yaml は **load 全滅** — strict 経路（`route resolve`/`route run`）は exit 3、hook 経路は fail-open でその workspace の全層 routing が不活性化する。
- **症状は debug log のみに現れ `route resolve` を実行するまで不可視** — routing が黙って不発する症状としてのみ観測される（hook 経路の fail-open は error を表面化しない）。

### 修復・移行手順

いずれか一方で修復する:

- (a) 当該 `trusted` key を削除する 1 行修正 — shadow を意図していなかったならこれで完了。
- (b) routes.yaml を `<userConfigDir>/` へ vendor して user 層 route 化する — `trusted: true` は user 層で inert 受理されるためゼロ編集で shadowing が復活する。

hook 経路の fail-open 症状（routing が不発）は `route resolve --command '…'` で診断できる。
