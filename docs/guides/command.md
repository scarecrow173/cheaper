# Command tool

`command` は trusted policy の grant が許可する executable に動的な argv を付けて実行する managed process tool。固定 argv の named task を実行する `process`（[managed process ガイド](./managed-process.md)）とは別の canonical tool で、grant が pin した executable に呼び出しごとの引数を許す。model input は `{name, args}` のみで、`name` は grant 名、`args` は argv token 配列。executable・cwd・env は model 入力経路に一切乗らず、全て trusted policy の grant に由来する。Worker と Workflow は同じ `{"kind":"tool","tool":"command","input":{"name":"git-read","args":["log","--oneline"]}}` を使う。

## gate と信頼境界

Worker 経路は三重ゲート: worker 定義の `tools` 宣言 ∩ policy の `tools.allow` ∩ `command.grants`。Workflow `kind: tool` job は `tools.allow` の admission check と nonempty-grants registration gate の二重で、worker の `tools` 宣言 gate は適用されない。

`tools.allow` は exposure toggle であり authorization ではない。`command` の唯一の信頼境界は `command.grants` で、authoring できるのは trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml` + invoker の `--grant` CLI 引数から構成される合成 invocation layer）のみ。project の `.cheaper/policy.yaml` に `command` section（narrowing 用の field を含む任意の key）を含めると schema validation が fail し、policy load 全体が `InvalidRuntimePolicy` で拒否される — project は grant の追加も `command.*` の tightening もできない。builtin policy は `grants: {}`（空）を同梱するため既定の grant は 0 個で、`command` を expose する runtime 構築は `POLICY_VIOLATION` で fail closed する。例外は worker が `cheaper.optionalTools` で `command` を opt-in 宣言した場合のみで、そのときは grants 空でも scope 構築は失敗せず `command` は未登録へ縮退し、run は残りの tool で継続する（[workers ガイド](./workers.md) の `cheaper.optionalTools` 参照）。縮退が緩めるのは登録面のみであり、grant 解決・argv pattern・executable pin 等の call 経路の認可機構は一切変わらない。`command` を expose しない runtime は grant が ambient PATH で解決不能でも dormant として許容される。

この fail-closed gate（content-gate）と `tools.allow` の除外（admission）は別の層である。admission は scope 構築より先に評価され、`tools.allow` が宣言 tool を除外した場合は `optionalTools` opt-in の有無に関わらず run 全体が `POLICY_VIOLATION` で fail する。したがって admin が「`command` の実行を禁止しつつ opt-in worker の縮退実行は許す」には、`tools.allow` から `command` を外すのではなく grant を定義しないことで実現する — `tools.allow` から外すと縮退ではなく run 全体の fail になる。

## grant の宣言

```yaml
# user-scope policy (~/.config/cheaper/policy.yaml)
version: 1
command:
  grants:
    git-read:
      executable: git                    # bare name または absolute path
      argv: ["log", "*"]                 # positional pattern（省略可）
      cwd: .                             # workspace 相対（default "."）
      env: {GIT_CONFIG_NOSYSTEM: "1"}
      timeout: 30s                       # 以下 3 項目は section default の per-grant override
      killGrace: 2s
      maxOutputBytes: 65536
```

grant 名は `^[a-z][a-z0-9_-]{0,63}$`（task 名と同規則）。必須 key は `executable` のみ。`cwd`・`env`・`timeout`・`killGrace`・`maxOutputBytes` は optional で、lifecycle 3 項目は grant override → section default の順で畳み込まれる。`cwd`・`env` は `process` task と同規則（workspace 相対検査・env key pattern・NUL 拒否）で、子 env は sanitized minimal env + grant 宣言分のみ — 親の ambient credential は継承しない。

`command.grants` と `process.tasks` は resolve 時に**単一 `exec` grant namespace** へ畳まれる — `command` grant は `fixed:false`（argv pattern 契約）、`process` task は `fixed:true`（argv literal）の grant として解決される。両 section の名が衝突する（`command.grants` ∩ `process.tasks` 非空）と policy load は `InvalidRuntimePolicy` + `details.{grant,task}` で fail closed する。authoring の section 分離は維持されるが、grant 名空間は共有される点に注意。

trusted policy YAML の grant spec はさらに `fixed`・`inputs`・`outputs`・`scope` を宣言できる（issue #165 の schema 拡張）:

- `fixed: true` — `argv` を pattern ではなく固定 argv literal として扱う（`process` task 相当の宣言を `command` section から直接 authoring する経路。lifecycle default は引き続き `command` section 由来）。
- `inputs` / `outputs` — declared I/O から ResourceManifest へ導出する task と同規約（workspace 相対・末尾 `/**` subtree）。未宣言は保守的な root subtree claim に落ちる。
- `scope` — `"trusted"`（既定）または `"project"`。`"project"` は自己弱体化（route 不可視の project task 相当 grant）としてのみ意味を持つ。

**`--grant`・`policy grant add` の CLI 入口では `fixed:true`・明示 `scope` を含む spec は `InvalidRequest` で拒否される** — これらの入口は route-eligible な `!fixed` command grant のみ供給する契約で、inert/route 不可視な宣言は利用者の意図と矛盾するため（trusted YAML での直接 authoring は合法）。`fixed:false`・`inputs`/`outputs` の宣言は CLI spec でも受理される。

route の `grant:` 参照が解決するのも `scope:"trusted" && !fixed` の grant のみ — `fixed`・`scope:"project"`・未知名を指す route は `Viability::NoGrant`（route ineligible、error ではなく viability 情報）として畳まれる。

executable は bare name または absolute path（workspace 相対 path は拒否）。bare name は scope 構築時点の ambient PATH で解決し、realpath 済み canonical path を pin する。native magic（MZ / ELF / Mach-O）を持つ executable のみ実行可能で、shell・script shim（`.cmd`/`.bat`/`.ps1`/`.sh`）は `process` baseline の deny で拒否される。

### `security.execDenyPatterns` — opt-in executable deny-list

`security.execDenyPatterns`（issue #164）は operator が宣言する deterministic executable deny-list — 各要素は executable 名の regex で、match した executable への grant を拒否する。**既定は `[]`（deny 無し）**で、`security` 節の省略・`execDenyPatterns: null`・`execDenyPatterns: []` は全て同じ「何も拒否しない」を意味する。deny は #163 で mandatory 層から除去されたため、宣言しない限り何も拒否されない。

match 規約は deterministic に固定される:

- 照合対象は executable の **basename stem** — `/`・`\` 区切りの最終要素を取り、小文字化し、末尾の `.exe`/`.com` suffix を 1 回 strip する（`python3`・`/usr/bin/Python3.EXE`・`C:\tools\SSH.EXE` はそれぞれ `python3`・`python3`・`ssh` の stem で照合される）。
- 各 pattern は stem への**完全一致**（内部的に `^(?:pattern)$` 相当で compile）・**case-insensitive** で評価され、複数 pattern は OR で効く。anchored なので `sh` は `bash` や `sshd` に substring match **しない** — substring を意図する場合は `.*` を明示する。
- pattern は trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）のみ authoring できる。project の `.cheaper/policy.yaml` に `security` 節を書くと schema が `InvalidRuntimePolicy` で拒否し、invocation 層（`--grant` 合成 policy）に `security` key を書くと `InvalidRequest` + `details.layer="invocation"` で拒否される。
- layer merge は配列の**丸ごと置換**（union ではない）— builtin `[a]` + user `[b]` なら effective は `[b]`。user が `security` を書かなければ builtin 値を継承する。

deny は 3 箇所に同一 predicate で、**全 exec grant**（`command.grants` 由来 + `process.tasks` 由来の fixed grant）へ適用される:

1. policy resolve — grant/task の **declared** executable spec が match すると `InvalidRuntimePolicy`（`details.grant`・`details.executable` 付き — task 由来の fixed grant には `details.task` も併記）で policy load / `policy grant add` が失敗する。
2. pin 時 — PATH 解決した **resolved** basename が match すると `POLICY_VIOLATION`（`details.grant` 付き）。declared 名と resolved basename が異なる shim・symlink 経路もここで捕捉される。1 grant でも match すればその surface の pin 全体が `Err` となり scope 構築が失敗する（blast radius — unpinnable grant と同規約で、deny が operator-reachable になっただけ）。
3. spawn 前再検証 — pin 済み path の再検証（pin-equality）直後に同じ deny が再適用される。

**これは containment 機構ではない** — known-bad executable への fail-fast deny であり、任意 argv を別 process 起動へ変換できる binary（plugin/extension 機構を持つ tool、将来の新 runtime）は列挙を免れ得る。`security.execDenyPatterns` に何を書いても、**`command` の executable への grant は shell 相当の信頼として扱う**こと。

補足:

- `.*` は schema 上許容される明示的な deny-all — これを宣言すると grant を持つ policy は全て load 失敗になるため、意図的な利用に限る（substring 意図の唯一の表現でもある）。
- stem 正規化は unix でも `\` を区切りとして扱い `.com` suffix も strip する — `my\python.exe` や `foo.com` のような unix 上の奇名が deny に match する false-positive を生じ得る。方向は fail-closed（過剰に拒否するが漏らさない）であり、旧 builtin 由来の構造的制約として意図的に保持される。
- `security.execDenyPatterns` は unified `exec` namespace の全 grant — `command.grants` の executable と `process.tasks` 由来の fixed grant executable の双方に適用される（issue #165 で適用範囲が均一化された）。

#### 旧 mandatory deny-list（#163 で除去）との対応

旧版は builtin に固定の deny regex を持ち、同等の名を既定で拒否していた。#164 以降は opt-in のため、その効果を再現したい operator は以下を user `policy.yaml` の `security.execDenyPatterns` に seed として使える。旧 regex の verbatim（コメント除去・単一行）:

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

**migration 注記** — 上記は旧 mandatory 挙動の近似再現であり、厳密な等価ではない。旧実装はこの regex とは別の predicate で shell・shim 名（`sh`/`bash`/`cmd`/`powershell` 等）も拒否していたが、そちらは #163 で別機構として除去済みで本一覧には含まれない。同等の shell 拒否が必要なら `sh|bash|dash|zsh|fish|cmd|powershell|pwsh` 等の pattern を operator が別途追加する。

## `--grant`（invocation-scoped grant）

`cheaper run`・全 worker alias（`bulk-read` 等）・`cheaper workflow run` は repeatable な `--grant <json>` で、その invocation のみ有効な grant を注入できる。policy file を編集せず 1 回限りの command 実行を認可する経路で、恒久的な grant は `cheaper policy grant add` で user policy へ登録する（[usage ガイド](./usage.md)）。

```bash
echo '{"task": "..."}' | cheaper tool-operate \
  --grant '{"name":"git-read","executable":"git","argv":["log","*"]}'
```

- flag 値は grant name を `name` key で内包する complete JSON object。`name` は `command.grants` の map key として取り出され、残りの key は上記「grant の宣言」と同一の spec shape（`executable` 必須、`argv`・`cwd`・`env`・lifecycle override は optional）。`--grant <v>` と `--grant=<v>` の両形を受理し、繰り返すと宣言順に複数 grant を同時注入する。値の上限は 64KiB（超過は `InvalidRequest`）。`fixed:true`・明示 `scope` を含む spec は `InvalidRequest` + `details.{grant,layer:"invocation"}` で拒否される（CLI 入口規約 — 上記「grant の宣言」節参照）。
- `run`・`workflow run` 以外の command（`usage`・`count-tokens`・`models`・`auth`・`policy`・`workflow interface`）へ渡すと `InvalidRequest` で reject される。`workflow run` でも `run` と同一の意味論で注入され、`kind: tool` job の `command` がその grant を使える。例外として `hook` command は `parse_cli_args` に到達しない独立 parse path を持ち、`--grant` を含む未知 option を silent に無視する — run 系 command ではないため reject 契約の対象外の spec 適合 known gap（[test-spec](../test-specs/invocation-scoped-command-grants.md) 参照）。
- 注入された grant はその run の effective grant 集合に畳み込まれ、runtime 上は user policy の grant と区別なく振る舞う。`security.execDenyPatterns`・argv pattern・executable pin 等の検査は policy 由来 grant と同一経路で走る — invocation 経路は deny-list を迂回しない。deny 該当の spec は merged grant 集合の再 resolve で `InvalidRuntimePolicy`（`details.grant`・`details.executable` 付き）となる。このとき `details.layer` は付かない（accepted gap — deny 元が builtin/user・grant 元が invocation と別 layer のため merged-map 評価では帰属不能）。なお invocation 層の policy 自体に `security` key を書くことは `InvalidRequest` + `details.layer="invocation"` で拒否される（authoring 権限の問題として別経路で検出される）。
- **非永続**: grant は in-memory の `RuntimePolicy` にのみ存在し、policy file・`usage.jsonl`・`.cheaper/` のいずれにも書き込まれない。flag 無しの後続 run では効果を持たない。
- 同名の grant が builtin/user policy に既に存在する場合、および同一 invocation 内で複数の `--grant` が同名を指定した場合は、run 開始前に `InvalidRequest`（`details.grant` に衝突名）で失敗する — silent な上書き・last-wins は行わない。
- `--worker-file` との併用は可。worker が `command` tool を宣言しない場合、grant は露出せず warning 付きで run は継続する（`tools.allow` 除外・executable pin 失敗は従来通り `POLICY_VIOLATION`）。
- audit surface: invocation grant が適用された run は開始時に stderr へ `grant: <names>` の diagnostic 行を出し（`--quiet` で抑制）、log 有効時は `cheaper.run.tool_scope` span の `grants.invocation` field に宣言順の名前列を記録する。`usage.jsonl` の record shape は不変。

### cmdline 露出と secrets

`--grant` の flag 値は呼び出し元 process の cmdline（`ps`・`/proc`・shell history）に露出する — grant の `env` に secret を含めると cmdline 経由で観測されうる。加えて spawn される command は `env_clear` + grant 宣言の `env` のみの最小 env で起動し、親の ambient 環境変数は子プロセスへ一切届かない。したがって secret が必要な grant は、flag 値ではなく user policy（POSIX `0600` の `<userConfigDir>/policy.yaml`）の grant `env` 値か、子プロセス自身が読む credential file に置くこと。`cheaper policy grant add <name> '<json>'` も spec を一時的に cmdline へ載せる同種の caveat を持つ。

invoker の CLI 引数から供給される `--grant` は trusted layer の一つとして扱われるが、grant を authoring できる経路は builtin policy・user `policy.yaml`・`--grant` のみで、model 入力・worker prompt・project policy からは一切供給できない（信頼境界は不変）。

## argv pattern

grant の `argv` は positional token pattern で、要素は 3 種のみ。

- literal: 対応する 1 token と exact match。
- `prefix*`: 末尾に `*` を 1 個だけ持つ要素は対応 token への prefix match（`"--format=*"` は `--format=%H` に match）。
- 最終要素の bare `"*"`: zero-or-more の残余 token を受ける rest marker。`["log", "*"]` は `["log"]` 単体も `["log", "--oneline", "HEAD~3"]` も受理する。

rest marker を持たない pattern は arg 個数の完全一致を要求し（超過 args は拒否）、args が pattern の固定要素より短い場合も拒否される（fail-closed）。最終要素以外の bare `*`・`*` を 2 個以上含む要素・要素中間の `*` は policy resolve 時に `InvalidRuntimePolicy` で拒否される。`argv` 省略時は args 個数無制約。全 args token（pattern 適合・rest 残余を問わず）には match 前の独立 check として shell metachar 拒否（`` ` ``・`|`・`&`・`;`・`<`・`>`・`$(`・CR/LF/NUL）が適用され、token 長 ≤4096・args 個数 ≤256。pattern 不一致・metachar 含有は `POLICY_VIOLATION`。

## 実行・制御

spawn・timeout・killGrace・abort・process-tree cleanup・bounded output・heartbeat・concurrency cap は `process` と同じ managed-subprocess 基盤を共有する（挙動の詳細は [managed process ガイド](./managed-process.md)）。`command` は独自 pool `"command"`（`command.maxConcurrent`、default 1）を持ち `process` pool と独立。子の副作用が未知のため、resource scheduling では workspace root subtree に対する保守的な Exclusive claim を宣言する。結果 details は `{grant, argv, state, exitCode, signal, durationMs, stdout, stderr}` で、stdout/stderr は `process` と同じ bounded stream shape。`timeout`・`killGrace`・`maxOutputBytes` は grant ごとに override 可で、`heartbeat.interval` は section-level のみ。

`meta.changes` は実測した write/edit のみを報告し、child の未観測副作用を audited change として主張しない。`command` を使う run の `savings.directlyComparable` は常に `false`（estimated savings 母数に参加しない）。

## trust / sandbox 限界（必読）

`command` の子 process は **OS sandbox 無し**で動作する。以下は全て grant 作者が引き受ける信頼であり、cheaper は抑止しない。

- `filesystem.denyPaths`・workspace boundary・worker の `writeScope` は `command` の子 process の副作用を bound **しない**。cwd の検査（denylist + workspace boundary）は宣言された cwd のみに適用され、**argv 内の path operand には filesystem 検証を適用しない** — argv に対する唯一の filesystem 面の制約は grant の argv pattern である。
- bare-name executable は **scope 構築時点の ambient PATH** で解決して canonical path へ pin する（per-scope pin — runtime 生存期間中 immutable、spawn 前に同じ検証を再適用）。pin 時点の PATH が attacker-controlled なら別 executable が pin され得るため、ambient PATH への信頼は grant 作者の責務。pin と spawn の間の PATH 変化は pin 済み path に影響しないが、pin 自体が古い PATH で行われた場合の残余 TOCTOU は accepted limitation。
- argv pattern が bound するのは argv の *shape* であり executable の capability ではない。workspace 管理下の config で挙動が変わる executable — git の `core.hooksPath`・external diff・fsmonitor・filter driver、cargo / make / npm の build script、`git -c alias.x=!cmd` のような inline config override — は pattern 適合の argv で **repo 管理下の code を実行し得る**。そのような executable への grant は repo content を信頼することと等価。真に固定した invocation が必要なら動的 argv を持たない `process.tasks` を使う。`GIT_CONFIG_NOSYSTEM=1` のような env hardening は有効だが repo の `.git/config` は無効化しない。
- `security.execDenyPatterns`（opt-in、既定 `[]` = deny 無し）は containment 機構ではなく known-bad executable への fail-fast。任意 argv を別 process 起動へ変換できる binary（plugin/extension 機構を持つ tool、将来の新 runtime）は列挙を免れ得るため、deny-list の有無に関わらず **`command` の executable への grant は shell 相当の信頼として扱う**こと。
- grant の `env` 値は子 env にそのまま載る。`LD_PRELOAD`・`NODE_OPTIONS` 等の injection 変数を含め得るため、env 値も trusted として扱う。
- writeScope を宣言した worker で `command` を expose すると scope 生成が `POLICY_VIOLATION` で fail closed する（非両立）— child の副作用を resource 層が観測不能で writeScope coverage が vacuous になるため、silent hole を許さず拒否する。resource manifest を narrow した scope では registration ではなく op admission（`covers()`）で拒否される — fail する層は異なるが両者 fail-closed。
- `process.tasks` は project policy から extend できるが `command.grants` はできない（意図的な divergence）— project は固定 argv の named task を事前認可できるが、動的 argv の executable 認可は trusted layer のみに限定される。

長寿命 service・interactive TTY・unrestricted shell・OS sandbox containment はこの tool の対象外。
