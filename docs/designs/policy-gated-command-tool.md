---
id: "DESIGN-034EMWNJbQpMB0N3ZTXM5A"
type: "design"
status: "approved"
title: "policy-gated command tool の grant 境界と managed-subprocess 共有基盤の設計"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/101"]
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./managed-process-canonical-tool.md"
    - "./managed-process-lifecycle-hardening.md"
    - "./tool-operator-built-in-worker.md"
    - "./invocation-scoped-command-grants.md"
    - "./pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "./documentation-honesty-sweep-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/policy-gated-command-tool.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../discovery/policy-gated-command-tool.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../guides/managed-process.md"]
  defers: []
  deferred-by: []
---

# policy-gated command tool の grant 境界と managed-subprocess 共有基盤の設計

## Context

canonical tool vocabulary に 9 個目の tool `command` を追加する。`command` は trusted layer（builtin policy + user-scope `<userConfigDir>/policy.yaml`）で宣言された grant が許可する executable に対し、動的な argv を付けて実行する managed process tool である。spec `../specs/policy-gated-command-tool.md` が要件・acceptance criteria を、ADR `../adr/command-grant-trust-boundary-and-managed-process-reuse.md` が pure Option A の信頼境界（project partial schema が `command` 以下の grant authoring key を全て除外し、含む project policy は load 時に `InvalidRuntimePolicy` で拒否）と managed-process 基盤の再利用方針を確定済みであり、本 design は両者が defer した残りの設計判断を確定する。

既存 `process` tool（`crates/cheaper-tools/src/tools/process/`）は named task の固定 argv のみを実行し、その contract と挙動は不変とする（AC1）。`command` は Worker（tools 宣言 ∩ tools.allow ∩ command grant の三重ゲート）と Workflow の `kind: tool` job（tools.allow admission + nonempty-grants registration gate の二重ゲート）の両経路から同一の canonical 実装を通じて利用する。

## Scope

### In Scope

- `command` の canonical tool 化: vocabulary・worker validate・catalog・registry・invocation・workflow tool job・runtime plumbing。
- `command` policy section（grant shape・section defaults）と trusted-layer-only の schema 境界。
- argv pattern 言語、executable 解決 + pin、interpreter deny、writeScope 拒否規則、resource/scheduling、error code 写像。
- managed-subprocess lifecycle（spawn / monitor / cleanup / tree-kill / bounded output / heartbeat）の共有化 refactor と file-level change map。

### Out of Scope

- unrestricted shell、interactive TTY、daemon / 常駐 process。
- OS sandbox による containment / read-only 保証。
- tool-operator worker 本体（issue #102）、artifact-store への実行 log 永続化。
- `process` tool の contract / behavior 変更、project policy からの `process.tasks` extend 権限の変更。
- project 側 narrowing field（Option A の範囲で一切提供しない。将来の明示的 workspace trust 設計に委ねる）。

## Components and Boundaries

### (a) policy schema: `command` section と grant shape

`policy.yaml` に新しい top-level section `command` を追加する。YAML shape:

```yaml
command:
  maxConcurrent: 1          # optional, default 1
  timeout: 2m               # optional, default "2m"
  killGrace: 2s             # optional, default "2s"
  heartbeat: {interval: 30s}# optional, default "30s"
  maxOutputBytes: 65536     # optional, default 65536, cap 16MiB
  grants:
    git-read:                 # grant name: ^[a-z][a-z0-9_-]{0,63}$（task 名と同規則）
      executable: git         # required: bare name または absolute path
      argv: ["log", "*"]      # optional: positional token pattern（末尾 "*" は残余 args の rest marker、後述）
      cwd: .                  # optional: workspace-relative、default "."
      env: {GIT_CONFIG_NOSYSTEM: "1"}  # optional: task.env と同規則
      timeout: 30s            # optional per-grant override
      killGrace: 2s           # optional per-grant override
      maxOutputBytes: 65536   # optional per-grant override
```

- section-level key は `process` と同じ lifecycle default 群 + `maxConcurrent` + `grants`。`ResolvedCommandPolicy` は `ResolvedProcessPolicy` と同じ serialize 規約（`{maxConcurrent, grants, timeout, maxOutputBytes, killGrace, heartbeat:{interval}}`）で、resolved grant には effective な `timeout` / `killGrace` / `maxOutputBytes`（grant override ?? section default）が畳み込まれる。
- `ResolvedCommandPolicy` / resolved grant は serde-native（`Serialize` + `PartialEq`）を維持する — invocation parity check が `to_value` → 再 resolve → `PartialEq` 比較（`options.process` の既存 check、invocation.rs:210-216 と同型）で動くため。resolved grant の `argv` は **pattern の source string** を保持し、`ArgPattern` は match 時に compile する（または compiled form を保持しつつ `Serialize` は source string を emit）。pin 済み canonical path は resolved policy に載せない — declared-shape の equality を parity check の比較対象とするため、pin map は `ToolCtx` 側の別 field に保持する（(g) 参照）。
- grant の `env`・`cwd` は `process` task と同一規則（`relative()` による workspace-relative 検査、env key pattern / NUL 拒否 / Windows case 重複拒否）。`inputs`/`outputs` は持たない（後述 (c)）。
- schema は `command_policy_properties()`（新規 `crates/cheaper-core/src/policy/command.rs`）が生成し、complete schema・user partial schema の双方に optional section として載せる。complete schema 側の section required key は `["maxConcurrent", "grants"]`（`process` の `["maxConcurrent", "tasks"]` を mirror、schema.rs:243）。

**PolicySchemaMode 分割（Option A の実現）**: 現行 `Complete | Partial` を `Complete | Partial | ProjectPartial` に拡張する。`Partial` は user layer（trusted）用のまま残し、新しい `ProjectPartial` は `command` section 全体を property から除外する（`additionalProperties: false` により `command` key を含む project policy は load 時に schema validation で拒否）。`command` section 全体の除外を選ぶ理由: grant authoring key（`grants`・各 grant の `executable`/`argv`/`cwd`/`env`/`timeout`/`killGrace`/`maxOutputBytes`）だけを列挙して除外する方式は、将来 grant field が増えたときに除外漏れがそのまま信頼境界の穴になる。section 丸ごと除外なら新 field の追加が自動的に project から閉じられる（KISS + fail-closed）。その代償として project は `command.maxConcurrent` の tightening すら行えないが、現要件に project 側 tightening の要求は無く（YAGNI）、`RestrictConcurrency` の allowlist も触らずに済む（`merge.rs` は無変更）。

`load.rs` の project call site を `PolicySchemaMode::ProjectPartial` に切り替え、user call site は `Partial` のまま。`PolicySchemaMode::as_str` は `"complete" | "partial" | "project-partial"` とし、error message `invalid {mode} runtime policy` が layer を特定できるようにする。

**argv pattern 言語（確定）**: grant の `argv` は positional token template。

- `argv` 省略時: args は個数無制約（各 token は metachar 拒否と長さ cap のみ適用）。
- `argv` 指定時: `args` は pattern と positional に全体 match する。`*` 以外の各 pattern 要素は args の 1 token と対応し、**args が pattern（trailing `*` を除く）より短い場合は reject**。trailing `*` を持たない pattern は arg 個数の完全一致を要求する — 超過 args は既定で拒否（fail-closed）。
- pattern 要素の種別は 3 種のみ: (1) **最終要素の bare `"*"` は zero-or-more の残余 token を受ける rest marker** — `["log", "*"]` は `["log"]` 単体も `["log", "--oneline", "HEAD~3"]` も受理する。最終要素以外の位置の bare `"*"` は意味を持たないため resolve 時に拒否する。(2) 末尾に `*` を 1 個だけ持ち prefix が非空の要素は、対応する 1 token に対する prefix match（`"--format=*"` は `--format=%H` に match）。(3) それ以外の要素は literal exact match。
- `*` が末尾以外・2 個以上出現する要素は grant resolve 時に `InvalidRuntimePolicy` で拒否（`"a*b"`、`"**"` 等は書けない。glob/regex は採用しない — 表現力と安全性の最小共通集合）。

選定理由: 「per-token literal / prefix match + 末尾 rest marker `*`」は glob 包含判定を必要とせず、match 実装が `starts_with` と `==` に閉じる。open-ended な real use（`git log <任意の revision/flag>`）は trailing `*` で明示的に表現し、rest marker を持たない pattern は arg 個数まで静的に pin できる — 「超過 args を既定で許す leading-prefix pin」案より、意図しない引数の追加を structural に拒否するこちらの方が trusted grant の意図を超過授権しないため採用する。

**argv pattern の capability 限界（正直な注記）**: argv pattern が bound するのは argv の *shape* であり executable の capability ではない。workspace 管理下の config で挙動が変わる executable — git の external diff / `core.hooksPath` / fsmonitor / filter driver、cargo / make / npm の build script、`git -c alias.x=!cmd` のような inline config override — は、pattern 適合の argv で project content を実行し得る。真に固定した invocation が必要な場合は動的 argv を持たない `process.tasks` を使うことを推奨し、per-grant `env` による hardening（例: `GIT_CONFIG_NOSYSTEM=1` — ただし repo の `.git/config` は無効化**しない**点に注意）を docs で案内する。

**token 検証**: `args` の全 token（pattern match 済み・残余を問わず）に match 前の独立 check として `argv_shell_re`（`[\0\r\n|&;<>`]`・`$(`）を適用し、含む token は `POLICY_VIOLATION` で reject。token 長 ≤4096、args 個数 ≤256。pattern 文字列自体も resolve 時に同じ metachar 検査を通す（match 不能な pattern を静的に排除）。

### (b) executable 解決: bare name + pin（採用）

grant の `executable` は **bare name または absolute path** を許可する（workspace-relative path は拒否 — grant は user-scope で全 workspace に効く global 宣言であり、workspace 相対の意味を持たない）。bare name は ambient PATH に対して解決し、realpath 済みの canonical path を pin する。

pin の境界（設計上の「load 時」）: `ResolvedCommandPolicy`（core、fs 非依存）は declared executable を保持し、pin は **`create_tool_runtime` 内で `options.names` に `Command` が含まれる場合のみ**行う。`create_invocation_tool_runtime` には `names` field が無い（names は scope ごとの `WorkerScopeOptions.runtime.names` / `JobLeaseOptions.names` に属する）ためそこで pin することはできず、invocation 構築時は declared-shape の `resolve_command_policy` のみが走る。`command` を expose しない runtime は、user grant が ambient PATH で解決不能でも pin を試行せず dormant として許容する。invocation 経路では `create_worker_scope` → `create_tool_runtime`（invocation.rs:257）が scope 構築ごとに pin を実行する。ここは `options.process` が `resolve_process_policy` で resolve されるのと同じ「policy を runtime へ load する」境界であり、tool 登録・expose より必ず前に置かれる。core の `resolve_command_policy` 内で fs 解決しない理由: PATH search + native magic 判定機構は `crates/cheaper-tools`（`tools/process/executable.rs`、shared `proc` 基盤へ移設後も tools 側）にあり、core への移設は `resolve_process_executable` の workspace-relative 解決（workspace root が必要で core policy resolve は root を知らない）を巻き込む大きすぎる refactor になる。tools 境界での pin は「runtime の生存期間中 immutable・expose 前・ambient PATH は pin 時点のもの」という spec の要求を満たしつつ、ADR の「`resolve_process_executable` を再利用せよ」をそのまま守れる。ambient PATH への信頼（pin 時点の PATH が attacker-controlled だと別 executable を pin し得る）は user-facing docs に明記する。

pin の semantic（確定）: pin は **per-scope** であり、scope 構築のたびにその時点の ambient PATH で再解決される（invocation 生存期間をまたぐ cache はしない）。`create_worker_scope` の parity check が比較するのは pin 済み path ではなく **declared-shape の resolved command policy**（`options.process` の既存 check と同型）。pin と spawn の間に ambient PATH が変化しても pin 済み path は影響を受けないが、pin 自体が古い PATH で行われた場合の残余 PATH-change TOCTOU は accepted limitation として文書化する。なお spec は pin を「policy load 時に一度だけ」と記述するが、本設計が採る per-scope pin（`create_tool_runtime` 境界で scope ごとに ambient PATH を再解決し、spawn 時に再検証する）は spec の字面に対する意図的な deviation であり、spec 側でも pin が tool-runtime 構築境界（per scope）+ spawn-time re-verification である旨を明記して両者を整合させる。

pin 手順（grant ごと、tool runtime 構築時）: まず **workspace-resident executable を二段で拒否**する — (a) declared executable が `!is_absolute && ('/' または '\' を含む)` なら `resolve_process_executable` へ委譲する**前に** `POLICY_VIOLATION` で拒否（executable.rs:125-136 は separator 付き relative path を `resolve_workspace_path` で workspace 内へ解決するため、そのまま通すと project 管理下の binary が trusted grant で実行される）、(b) `native()` の realpath 後、resolved canonical path が workspace root 内にあれば同じく拒否（absolute path や PATH 上の symlink が workspace 内へ着地する迂回を塞ぐ）。その上で shared `proc` 基盤の `native()` 相当 candidate 検証（realpath → `is_shell_process_executable` → is_file → win32 `.exe`/`.com` → X_OK → native magic MZ/ELF/Mach-O）に `(d)` の interpreter deny を resolved basename へ適用する。解決不能・deny 該当・workspace-resident の grant は tool runtime 構築自体を `POLICY_VIOLATION` で fail させる。absolute path は同じ検証を直接適用。

`resolve_process_executable` 再利用時の注意: bare `node` 拒否の error message は `"process task \"node\" is unsupported: ..."`（executable.rs:144-149）と process task 文脈に固定されている。`command` grant が同じ error を受けると文脈を誤誘導するため、message の subject を parameterize するか tools/command 側で error を wrap して grant 文脈の message に差し替える（error code `POLICY_VIOLATION` は不変）。

実行時の再検証: pin 後に file が差し替えられた場合の fail-closed として、spawn 直前に pin 済み path へ同じ candidate 検証を再適用する（4-byte read のみで安価）。

absolute-path-only 案を却下した理由: `git` や `cargo` のような一般的 grant は absolute path を要求すると user policy が環境依存の長いパスで埋まり、portability・可読性を損なう。load 時 pin + ambient PATH trust の文書化でリスクは管理可能と判断した。

### (c) writeScope × command: fail-closed reject（採用）

`names` に `Command` を含み、かつ `options.write_scope` が `Some` の worker scope 生成を、`create_tool_runtime` で `POLICY_VIOLATION`（"command is incompatible with worker writeScope"）で拒否する。

理由: writeScope の意味論は「scope が宣言した exact path 以外を write しない」という resource 層の保証だが、`command` の子 process は OS sandbox 無しで動作し副作用を観測不能のため、grant shape に inputs/outputs を持たず coverage check は vacuous になる。vacuous な coverage を許容すると writeScope が「存在するが実質無効」の状態を静かに作り、worker 定義と resource accounting の双方を誤誘導する。documented non-applicability は silent hole になるため却下し、grant gate と同じ registration 境界で fail-closed する。

**resource_manifest との非対称（accepted）**: writeScope は上記のとおり registration（scope 生成）時に fail-closed するが、`resource_manifest` を narrow した scope で `command` を使う場合は registration 時には拒否されず、op admission 時に `covers()`（resources.rs:427）/ `DeniedError::scope()` で落ちる。両者とも fail-closed だが fail する層が異なる。registration gate を manifest 側へ拡張して対称に揃えることも可能だが、manifest の適合判定は resource 層の既存責務であり、command 側での事前検査はその重複になるため、**層の非対称を accepted として文書化**する方針を採る。

### (d) interpreter denial: deny set を `command` 専用に拡張

現行の deny 機構（`crates/cheaper-core/src/policy/process.rs:114-143` と `crates/cheaper-tools/src/tools/process/executable.rs`）が実際に防ぐもの: `shell_executable_re` は `sh|bash|dash|zsh|fish|ksh|csh|tcsh|ash|cmd|powershell|pwsh|busybox`（`.exe` 付き含む）、`script_ext_re` は `.cmd|.bat|.ps1|.sh` 拡張子、`native()` は native magic（MZ/ELF/Mach-O）で shebang script を拒否、`resolve_process_executable` は bare `node`（完全一致 literal のみ）を明示拒否する。**`python.exe`・`perl.exe`・`ruby.exe`・`node.exe`・`deno`・`bun` 等の interpreter binary は全て native binary のため現行の deny set をそのまま通過する**（bare `node` literal だけが例外）。したがって issue の「任意の interpreter 起動による迂回は既定で禁止」を満たすには deny set の拡張が必須である。

拡張の置き場所（確定）: **`command` 専用の deny set** として `crates/cheaper-core/src/policy/command.rs` に `command_denied_executable_re()` / `is_denied_command_executable(name)` を新設する。`process` の `is_shell_process_executable` には手を入れない（AC1: process の contract 不変。process task は固定 argv で、interpreter binary の認可は task 作者の明示判断として現行維持）。core policy 層に置く理由: 適用点が grant resolve（load/validation 時の fail-fast）と tools 側 pin/exec 検証の 2 層に跨るため、両層から到達できる core が所有権を持つ。新たな別実装は作らず、`is_shell_process_executable` と並ぶ policy-level deny wrapper とする。

deny list は basename を小文字化し、**`.exe` / `.com` suffix を剥がしてから** match する（`node.exe` / `node.com` を `node` として捕捉）。interpreter 名だけでは不十分で、grant された binary 自体が任意の子 process を spawn できるクラス（exec wrapper・loader・LOLBin・editor/pager escape・追加 shell）も deny 対象に含める。

- interpreter（versioned name まで）: `python`/`pythonw`/`python\d+(\.\d+)*[a-z]?`（`python3.13t` のような versioned/free-threaded build tag まで）、`pypy\d*(\.\d+)*`（bare `pypy`/`pypy3.10`）、`perl\d*(\.\d+)*`（`perl5.38`）、`ruby\d*(\.\d+)*`（bare `ruby`/`ruby3.2`）、`node`/`nodejs`、`deno`、`bun`、`php\d*(\.\d+)*`（`php8.3`）、`lua`/`luajit`/`lua[\d.]+`、`r`/`rscript`、`julia`、`wscript`、`cscript`、`mshta`、`java`/`javaw`、`groovy`、`tclsh`、`wish`、`expect`、`osascript`。
- exec wrapper / launcher — **これらへの grant は shell の grant と等価**: `env`、`xargs`、`find`、`awk`/`gawk`/`mawk`/`nawk`、`sed`、`busybox`、`toybox`、`nice`、`nohup`、`setsid`、`timeout`、`watch`、`stdbuf`、`flock`、`taskset`、`sudo`、`doas`、`su`、`pkexec`、`setpriv`、`nsenter`、`unshare`、`chroot`、`docker`、`podman`、`systemd-run`、`ssh`、`scp`、`rsync`、`screen`、`tmux`。
- dynamic loader: `ld.so`、`ld-linux*`、`ld-musl*`、`dyld`。
- Windows LOLBin + wsl: `rundll32`、`regsvr32`、`msbuild`、`dotnet`、`installutil`、`regasm`、`regsvcs`、`forfiles`、`pcalua`、`msiexec`、`wsl`、`jshell`。
- editor / pager / REPL escape（`!cmd`・`:!`・`-e` 等で shell 脱出可能）: `vim`、`ex`、`nvim`、`less`、`man`、`mysql`、`psql`、`sqlite3`、`tar`。
- 追加 shell: `nu`、`elvish`、`xonsh`、`oil`、`ion`。

これは deny-by-default の静的 list で、本 iteration では opt-out knob を設けない（後述の Deferred Design Concern）。

**deny list の限界（正直な注記）**: deny list は exec-capable binary を完全には bound できない — 任意 argv を別 process 起動へ変換できる binary（上記 launcher 群の亜種、plugin/extension 機構を持つ tool、将来の新 runtime）は列挙を免れ得る。deny list は known-bad への fail-fast であり containment 機構ではないため、**`command` の executable への grant は shell 相当の信頼として扱い、grant 作者は対象 binary を完全に信頼されたものとみなさなければならない**、と docs に明記する。

適用点: (1) `resolve_command_policy` で declared executable の basename を検査 → `InvalidRuntimePolicy` で fail-fast。(2) pin 時に resolved basename を再検査 → `POLICY_VIOLATION`（`git.exe` が symlink で `python3.exe` に解決するような alias 迂回を塞ぐ — `native()` は realpath 後の名前を見る）。(3) exec 時の pin path 再検証でも適用。

### (e) module 構造: shared `tools/proc/` の抽出

現行 `tools/process/` の内訳: `executable.rs`・`tree.rs` は `pub`、一方 `spawn.rs`・`monitor.rs`・`cleanup.rs`・`prepare.rs` は private `mod` + `pub(super)` で閉じ、`prepare()` は `ProcessTask`・`process_task_resources` に、`drive()`/`execute()` は `options.process.{timeout,kill_grace,heartbeat,max_output_bytes}` に直接結合している。

採用する refactor: **共有 module `crates/cheaper-tools/src/tools/proc/` を新設し、lifecycle 機構をそこへ移す**。`command` から `tools::process` の private 内部へ `pub(crate)` で依存する代替案もあるが、それは `command` を `process` の module layout に結合させ、「managed subprocess lifecycle」という本来独立の責務の所有境界をぼかす。`proc` への抽出は責務境界を正す最小の coherent refactor である。

- `tools/proc/` へ移動・汎化するもの:
  - `executable.rs`（そのまま pub。`resolve_process_executable` + `ProcessExecutableOptions`）。
  - `tree.rs`（そのまま pub）。
  - `spawn.rs`: `build_command` を `(executable, argv: &[String], cwd, env)` 引数に汎化（`&ProcessTask` 依存を除去）。`spawn_code`/`spawn_failure_is_async`/`status_parts`/`display` はそのまま。
  - `monitor.rs`: `drive` の options 依存を `ProcLimits {timeout_ms, kill_grace_ms, max_output_bytes, heartbeat_interval_ms}`（呼出し側で `parse_duration` 済み）+ `Option<&OnTimeout>` + identity（下記）へ置き換え。`max_output_bytes` は現行 `mod.rs:305-309` が `options.process.max_output_bytes` から `Captures` を構築している部分を、`heartbeat_interval_ms` は `mod.rs:379` の heartbeat spawn が `options.process.heartbeat.interval` を読む部分を、それぞれ `proc::run` 内へ移す — `command` の per-grant `maxOutputBytes` override と section-level `heartbeat.interval` を `ProcLimits` 経由で流せるようにするため。`StreamCapture`/`Captures`/`Diagnostics`/`read_stream` はそのまま。
  - `cleanup.rs`: そのまま。
  - `prepare.rs` の `minimal_env` と cwd 解決部分（`resolve_workspace_path` + denylist + `is_dir` stat）を `proc::prepare` として共有化。task の manifest / writeScope coverage 検査は process 側に残す。
  - `mod.rs` の execute orchestration（timeout controller、captures、heartbeat spawn、`command.spawn()`、`drive`）を `proc::run(prepared: &ProcPrepared, limits: &ProcLimits, identity: &ProcIdentity, outer: AbortSignal, on_timeout: Option<&OnTimeout>)` として汎化。`ProcPrepared {executable, argv, cwd, env}`、`ProcIdentity` は details へ差し込む identity field（`process` は `{task: name}`、`command` は `{grant: name, argv: args}`）と heartbeat/diagnostic の label を持つ。`find_cheaper_error`・`aborted_err`・`wait_aborted`・`combined_reason`・`BoxedCause`・`OnTimeout` も proc 側へ移し、`tools::process` は後方互換のため `pub use` re-export する（`registry.rs`・`workflow_run.rs`・既存 test が `tools::process::find_cheaper_error` を参照しているため）。
- `tools/process/` に残るもの: `process_schema`/`process_details_schema`/`process_operation_declaration`/`ProcessTool`/`process_tool_definition` と、manifest + writeScope coverage を含む task 固有 `prepare`。`ProcessToolOptions` は維持し、`ProcLimits` への変換を call site で行う。
- `tools/command/`（新規）: `command_schema()`（`{name: string(pattern "^[a-z][a-z0-9_-]{0,63}$"), args: array of string(maxItems 256・items 各 ≤4096 文字)}`、`required: ["name", "args"]`、`additionalProperties: false` — input は `{name, args}` のみで、`executable`/`cwd`/`env` 等の余剰 key は schema 層で `INVALID_INPUT` に落ちる）、`command_details_schema()`（`{grant, argv, state, exitCode, signal, durationMs, stdout, stderr}` — `durationMs` は spec が要求する duration 報告で、process の details shape を**意図的に超過する spec-mandated な差分**）、`command_operation_declaration`、`CommandTool`/`command_tool_definition`（description に authorized grant 名一覧を埋め込み、model が grant 名を discovery できるようにする — `create_process_tool` が task 名一覧を埋め込む先例、mod.rs:213-222）、grant 解決 + argv match + metachar check + cwd prepare + pin 済み executable 再検証を行う execute glue。

`command` 側の `prepare` は `proc::prepare` の cwd 部分 + pin 済み executable のみで、manifest / writeScope 検査を持たない（(c) で writeScope 自体を拒否するため coverage check の対象が存在しない）。

### (f) resource / scheduling: Exclusive claim + 独自 pool `"command"`

- operation declaration: `command_operation_declaration(policy, grant)` は `OperationDeclaration {access: ClaimMode::Exclusive, pool: Some(OperationPool{name: "command", max_concurrent: policy.max_concurrent}), resources: 保守的 manifest}` を返す。直列化の mechanism を正確に言うと: `declaration.resources` が存在するため prepare は manifest normalize 済み claim（`writes` bucket は Exclusive に normalize される）を使い、`mutations: per-resource` かつ resources ありの条件では op-level access が scheduler.rs:227-233 で Exclusive→Shared に downgrade されて `state.exclusive` の global 直列化経路は通らないが、**claim-level の Exclusive root-subtree write claim が resource claim 仲裁で他 op の重複 claim と直列化する**。`mutations: serialized` または resources 不在なら op-level Exclusive が `state.exclusive`（can_start / start、scheduler.rs:334・:367-368）経由の global 直列化も効かせる。どちらの経路でも command op は直列化される（fail-closed）。
- **pool は専用名 `"command"` を採用**（共有 `"process"` pool 案を却下）。理由: `command` は任意 executable の実行で `process` とはリスクプロファイルと cap の意味が異なり、pool 共有は `command.maxConcurrent` の設定値を process task の cap と混ぜて accounting を曖昧にする。独自 pool なら `command.maxConcurrent`（section key、default 1、trusted layer のみ設定可）が command 実行だけを cap し、scheduler の pool 機構は無変更で済む。
- `resources` manifest（`OperationDeclaration.resources`）と catalog `resources()` arm は同一の保守的宣言を返す: `reads`/`writes` に `[{path: ".", scope: "subtree"}]`（workspace root subtree）、**`produces` は宣言しない**。root-subtree の produces claim は `resource_inference.rs:540-564` の duplicate-producer 検出（二 job の produces が overlap すると `InvalidRequest "duplicate resource producer"`）に衝突し、command job と `outputs` を持つ process job の混在 workflow を load 不能にする。直列化は `writes` の Exclusive root-subtree claim が既に担っており、`produces` の ownership semantics（producer→consumer の dependency edge 対象）は command が充足できないため reads+writes のみとする（`command_grant_resources()` が `produces: None` を返す）。子 process が workspace 外へ副作用を持ち得ることは declaration では表現不能なため、containment を主張せず docs で trust 限界として明記する。
- workflow `resource_inference`: `command` の `kind: tool` job は既存の generic tool 分岐（`resource_inference.rs:188`）を通り、`tools.get("command").resources(input)` が上記保守的 manifest を返す。process 専用分岐（`:123-187`、`{task}` 入力検証・`item.process` 宣言）は触らない。job が resource を明示宣言して既知 access（reads/writes bucket）を underclaim した場合は既存の `declared {bucket} underclaims known access` error（resource_inference.rs:429-440）で fail-closed するため、`command` job の manifest narrowing は構造的に不可能となる — 変更不要、test で挙動を固定する。

### (g) registry plumbing

- `ToolRuntimeOptions.command: Option<Value>`（`Partial<CommandPolicy>` raw JSON）を追加。`create_tool_runtime` で `resolve_command_policy` → **`names` に `Command` を含む場合のみ** pin + gate を適用: `grants` が空なら `POLICY_VIOLATION "command exposure requires nonempty authorized grants"`（registry.rs:1020 の process gate を mirror）、pin 不能・workspace-resident・deny 該当の grant があれば同じく `POLICY_VIOLATION`。`names` に `Command` が無い runtime は grant が ambient PATH で解決不能でも dormant として許容する。(c) の writeScope 拒否も同所で行う。
- `InvocationToolRuntimeOptions.command` も追加（`create_invocation_tool_runtime` は `names` を持たないため pin せず、declared-shape の `resolve_command_policy` のみ実行して `inner.command` に保持）。`create_worker_scope` が `options.runtime.command` を `resolve_command_policy` で再 resolve して `inner.command` と比較する parity check を `options.process` の既存 check（invocation.rs:210-216、`to_value` → re-resolve → `PartialEq`）と同型で追加 — scope 生成後により広い grant set を密輸できないようにする。比較対象は pin 前の declared-shape resolved policy であり、pin map は各 scope の `create_tool_runtime` が構築する。scope には serialization 済み resolved policy を渡す。
- `ToolCtx` に `command: ResolvedCommandPolicy` + pin 済み `BTreeMap<String,String>`（grant → canonical path）を保持し、`registered_definition` に `ToolName::Command` arm（`command_tool_definition` を返す）を追加。
- managed tracking: `execute_inner` の `is_process` flag（registry.rs:845）は `matches!(name, Process | Command)` に一般化し、`managed.track()` が `command` の spawn 済み子 process も数えて `drain()`/`wait_idle` が待つ（flag 名は `is_managed` 相当へ rename）。
- `execute_inner` の declaration 分岐に `ToolName::Command` arm（`command_operation_declaration`）を追加。

### (h) `is_mutating_tool(Command) = true`

`crates/cheaper-core/src/worker/validate.rs` の `is_mutating_tool` に `ToolName::Command` を追加する。効果: registry の generic declaration fallback が `Exclusive` を取り、`is_directly_comparable_worker` が `command` 使用 worker で `false` となり `savings.directlyComparable` が自動的に false（child mutation 未観測のため estimated-savings 母数に参加させない spec 要件）、`access.rs` の scope manifest が root Exclusive write claim を保守的に持つ。`meta.changes` は `MutationJournal` 実測のみで `command` は journal しないため、未観測の child side effect を audited change として報告しない構造が維持される。

### (i) builtin policy.yaml: `command` を `tools.allow` に含める（採用）

`crates/cheaper-cli/resources/policy.yaml` に `command: {maxConcurrent: 1, grants: {}}` を追加し、`tools.allow` に `command` を加える。理由: `command` の実効的な権限境界は nonempty-grants registration gate の一箇所に集約するのが最も単純で、builtin allow から外すと user は grant 宣言に加えて `tools.allow` への追加も必要になり二重の knobs になる。grants が空の既定では実行権限が実質ゼロであり、「tools.allow に入っている = 自動的に危険」ではない。なお `command` は新 vocabulary なので、`tools.allow` に `command` を含まない既存 policy では引き続き完全に拒否される（後方互換）。

**`tools.allow` は trusted gate ではない**: `merge.rs` は array field を wholesale replace するため、project policy は `tools.allow` に `command` を再追加できる（`command` を外す narrowing も含め、project は exposure を自由に toggle できる）。信頼境界を構成するのは **`command.grants` の非空性が trusted layer（builtin + user-scope）でのみ authoring 可能**という一点のみであり、project は `command` の exposure（tool が scope に見えるか）を制御できるが executable を authorize することは決してできない — exposure と authorization の分離として docs に明記する。

### (j) error code 写像（既存 code の再利用、新規 code なし）

| 状況 | code |
| --- | --- |
| registration gate（grants 空 / pin 失敗 / writeScope 併存） | `POLICY_VIOLATION` |
| 未知 grant・argv pattern 不一致・metachar token・interpreter/shell/shim/exec-wrapper/loader executable・workspace-resident executable・declared cwd の denylist/boundary 違反 | `POLICY_VIOLATION` |
| input schema 違反（`{name,args}` 以外の key、非 string、超長） | `INVALID_INPUT`（`validate_tool_input` 経由） |
| policy file の構造違反・grant shape 違反・project policy の `command` 混入 | `INVALID_RUNTIME_POLICY`（load/validate 時） |
| cwd が存在しない・dir でない | `INVALID_REQUEST`（process と同じ写像） |
| spawn 失敗・nonzero exit・cleanup 失敗 | `TOOL_EXECUTION_FAILURE` |
| timeout | `TIMEOUT`（`on_timeout` callback で active operation を cancel、process と同機構） |
| outer/inner abort・queued abort | `ABORTED`（原 reason 伝播） |

### (k) file-level change map

**`crates/cheaper-core`**
- `src/tools/vocabulary.rs`: `TOOL_NAMES` 末尾に `"command"`、`ToolName::Command` variant、`as_str`/`from_canonical`。
- `src/policy/command.rs`（新規）: `command_policy_properties()`、`resolve_command_policy`、`ResolvedCommandPolicy`/`CommandGrant`/`resolve_command_grant`（serde-native: `Serialize` + `PartialEq`、argv は pattern source string を保持）、`command_grant_resources()`（reads+writes の保守的 manifest の単一所有者 — `produces` は返さない、(f) 参照）、`is_denied_command_executable`/`command_denied_executable_re`（共に `pub` — cheaper-tools の pin/exec 検証が cross-crate で使う）、argv pattern の compile(`ArgPattern`)/match。
- `src/policy/process.rs`（可視性のみ、新規実装なし）: `argv_shell_re` を `pub` 化 — cheaper-tools の execute 時 metachar check が cross-crate で参照するため `pub(crate)` では不足。`relative`・`str_vec` は `pub(crate)` 化（core 内 `policy::command` からの参照用）。`bounded_duration`（process.rs:175-184）は section label を引数化して `pub(crate)` で共有し、MAX_TIMER_MS rule の重複実装を避ける（`invalid`（process.rs:107-112）は "invalid process policy" 固定 message のため process 側に残し、`policy::command` は `command_invalid` 相当を別途定義 — message prefix が異なるため共通化は誤り）。`is_shell_process_executable`（process.rs:136）は既に `pub` のため維持する（縮小しない）。
- `src/policy/schema.rs`: `command` section を complete（required `["maxConcurrent", "grants"]`）+ user partial 双方に追加、`PolicySchemaMode::ProjectPartial` + project partial schema（`command` 除外）、`policy_validators()`（schema.rs:288-299）の tuple を 3 validator（complete/partial/project-partial）へ拡張、`RuntimePolicy.command` field、`normalize_policy` が `resolve_command_policy` を呼ぶ、`validate_policy_schema` が `command` section 有無で `resolve_command_policy` を副作用検証として呼ぶ。
- `src/policy/load.rs`: project call site を `ProjectPartial` へ。
- `src/worker/validate.rs`: `is_mutating_tool` に `Command`。
- `src/policy/merge.rs`: 無変更（`command` は project 不可のため RESTRICT allowlist 対象外。`tools.allow` の wholesale replace semantics はそのまま — (i) の trusted gate 注記参照）。
- `RuntimePolicy` literal 構築 site（`policy/capability.rs`・`policy/restrict.rs` の test fixture 等全構築点）に `command` field を追加。cheaper-core に `src/dev` は存在しない — dev fixture は `crates/cheaper-runtime/src/dev/fixture.rs` で runtime 側に列挙済み。

**`crates/cheaper-tools`**
- `src/tools/proc/`（新規）: `executable.rs`・`tree.rs`・`spawn.rs`・`monitor.rs`・`cleanup.rs` を `tools/process/` から移動・汎化、`mod.rs` に `ProcLimits`（`timeout_ms`/`kill_grace_ms`/`max_output_bytes`/`heartbeat_interval_ms`）/`ProcPrepared`/`ProcIdentity`/`run`（captures 構築と heartbeat spawn 込み — 旧 `process/mod.rs:305-309`・`:379` の `options.process` 直接参照を `ProcLimits` 経由へ移す）/`minimal_env`/cwd prepare/abort helpers を集約。
- `src/tools/process/mod.rs`・`prepare.rs`・`tests.rs`: task 固有 glue に縮小し、移動した item は `pub use` re-export で公開 path を維持。
- `src/tools/command/`（新規 `mod.rs`）: input/output schema、`command_tool_definition`、grant 解決・argv match・pin 済み executable の spawn 時再検証・prepare・`command_operation_declaration`（pin 自体は registry の `create_tool_runtime` が行う — (b)/(g) 参照）。
- `src/tools/mod.rs`: `pub mod command; pub mod proc;`。
- `src/catalog.rs`: `NAMES` に `Command`、`input_schema`/`output_schema`/`resources` の各 arm。
- `src/registry.rs`: `ToolRuntimeOptions.command`、nonempty-grants gate、writeScope×command 拒否、`ToolCtx.command`（resolved + pin map）、`registered_definition` arm、`execute_inner` の declaration arm と managed flag 一般化。
- `src/invocation.rs`: `InvocationToolRuntimeOptions.command`、`inner.command`、parity check。
- `src/access.rs`・`src/scheduler.rs`: 無変更（`is_mutating_tool`・pool name で自動追従）。

**`crates/cheaper-runtime`**
- `src/run/mod.rs`: `InvocationToolRuntimeOptions` 構築（~:281）と `ToolRuntimeOptions` 構築（~:429）に `command: serde_json::to_value(&policy.command)` を配線。
- `src/dev/fixture.rs`、`tests/run_dev_seam.rs`、`tests/ollama_e2e.rs`: `RuntimePolicy` literal に `command` field。

**`crates/cheaper-cli`**
- `resources/policy.yaml`: `command: {maxConcurrent: 1, grants: {}}` + `tools.allow` に `command`。
- `src/workflow_run.rs`: `ensure_invocation` の `InvocationToolRuntimeOptions` に `command` を配線（`process` と同じ serialize 経路）。`ToolJobExecutor` の scope 構築は `names: vec![name]` が共通のため grants gate が自動的に効く。
- `docs/guides/command.md`（新規）または `docs/guides/managed-process.md` 拡張: trust / sandbox 限界、ambient PATH provenance と per-scope pin の残余 TOCTOU、argv pattern 言語、config-driven executable の capability 限界と grant `env` の注入変数注意、`process.tasks` の project extend との divergence、writeScope 非両立。`configuration.md`・`workers.md` に cross-link。
- `docs/guides/configuration.md`: `:93-135` が builtin `policy.yaml` を verbatim embed しているため、`command` section と `tools.allow` の `command` を同期更新する。
- `docs/guides/workers.md`: gate 記述（`:37-38`・`:99`）を `tools` 宣言 ∩ `tools.allow` ∩ `command.grants` の三重構造に合わせて更新（`command` では `tools.allow` が exposure toggle であり authorization でない点を明記）。

**`crates/cheaper-workflow`**
- `src/resource_inference.rs`: 無変更（generic tool 分岐が catalog `resources()` 経由で保守的 manifest を得る）。挙動を固定する test を追加。
- `crates/cheaper-workflow/tests/*.rs` の `InvocationToolRuntimeOptions`/`ToolRuntimeOptions` 構築点に `command` field。

**tests / fixtures**
- `crates/cheaper-tools/tests/proc_runtime.rs`: `exposes_eight_schemas` → nine、`options()` の `ToolRuntimeOptions` に `command: None`。
- 全 `ToolRuntimeOptions` / `InvocationToolRuntimeOptions` / `RuntimePolicy` 構築 site（`characterization.rs`・`differential.rs`・`proc_invocation.rs`・`crates/cheaper-rig-adapter/tests/adapter.rs:314` 等）に `command` field。`proc_process.rs` は `create_process_tool`/`ProcessToolOptions` を直接使い `ToolRuntimeOptions` を構築しないため field 追加の対象外。
- `crates/cheaper-core/tests/golden/`: `worker_validate.rs` に command entry、`policy_schema.rs` の golden mode mapping に `"project-partial"` 分岐（`:15-18`）、project-partial grant 拒否・user-partial 受理の fixture。
- golden fixture 再生成: `RuntimePolicy` に `"command"` key が加わるため、serialize 済み `expected.ok.policy` を持つ `fixtures/policy/{load,restrict,schema}.json` を全件更新。`fixtures/vocabulary.json` の `toolNames` / canonical 全件 / `known` 配列は 8→9 要素へ。`policy/merge.json` は raw JSON merge のみを扱うため変更不要。
- 新規 integration test（`crates/cheaper-tools/tests/proc_command.rs`）: grant gate、argv match、metachar/interpreter/cwd 拒否、timeout/abort/nonzero/output-cap/tree-cleanup（Windows/POSIX 差分は既存 `proc_process.rs` の fixture pattern を踏襲）。

### (l) migration / back-compat

- `command` section を持たない既存 policy（user/project 双方）は `resolve_command_policy(None)` で empty grants（=deny）へ normalize され、load は変わらず成功する。
- 既存 `process` contract・test は不変。shared `proc` への移動は `tools::process` の `pub use` で外部公開 path を維持し、process の挙動・error shape・details field は変えない。
- builtin の `tools.allow` 追加は新 vocabulary のみで、既存 policy の `process` 許可が `command` に転用されることはない（issue の「権限自動付与禁止」を満たす）。

## Data and Control Flow

- Worker 経路: worker `tools` 宣言 ∩ `tools.allow` ∩（scope 生成時）nonempty grants + pin 成功 → `definitions()` に `command` が expose → model が `{name, args}` を呼ぶ → `validate_tool_input`（`INVALID_INPUT`）→ `command_operation_declaration`（Exclusive + pool `command` + root reads/writes claim）→ scheduler admission → grant resolve + argv match + metachar + cwd/prepare + pin 再検証（`POLICY_VIOLATION`）→ `proc::run`（spawn → capture → timeout/abort → cleanup）→ `CanonicalToolResult`（`{grant, argv, state, exitCode, signal, durationMs, stdout, stderr}`）。
- Workflow `kind: tool` 経路: `tools.allow` admission（cli `workflow_run` acquire lease）→ `create_worker_scope`（`names: [command]`）→ 同上の registration gate → `scope.execute` → 同一 canonical execute。worker tools 宣言 gate は workflow job に適用しない。
- pin は scope（tool runtime）構築ごとにその時点の ambient PATH で行われ（per-scope、invocation をまたぐ cache なし）、scope parity は declared-shape resolved policy の equality で検査、実行は pin 済み path + spawn 前再検証のみを使う — model 入力経路に executable/cwd/env は一切乗らない。

## Non-Functional Constraints

- `command` 子 process は OS sandbox 無しで動作し、`filesystem.denyPaths`・workspace boundary・worker writeScope は子の副作用を bound しない。docs に必須記載。
- pin 済み executable は spawn 前に candidate 検証（deny basename・workspace-boundary 含む）を再適用する（pin 後差替えへの fail-closed）。pin と spawn の間の PATH 変更は pin path に影響しないが、pin が古い ambient PATH で行われ得る残余 TOCTOU は accepted limitation（(b) 参照）。`meta.changes` は実測 write/edit のみ、`savings.directlyComparable` は `command` 使用 worker で常に false。
- `command` の concurrency cap は独自 pool `"command"`（`command.maxConcurrent`、default 1）で process pool と独立させる。

## Risks and Trade-offs

- project 側 narrowing field を一切提供しない（Option A）：project admin が grant を authoring できないのは意図した制約であり、`process.tasks` extend 許可との divergence として docs に明記する。
- bare name の PATH 解決は pin 時点の ambient PATH を信頼する — attacker が PATH を制御できる環境では別 executable を pin し得る。`process` task と同じ前提であり、user-facing docs で provenance を明記する。
- argv pattern は trailing `*` を持たない限り arg 個数まで pin するが、bound できるのは argv の shape のみ — config 駆動の executable（git hooks/filter/external-diff、cargo/make/npm build script、`git -c alias.x=!cmd` 等）は workspace 管理下の content を実行し得る。真に固定した invocation は `process.tasks` へ誘導し、per-grant `env` hardening（`GIT_CONFIG_NOSYSTEM=1` 等 — repo `.git/config` は無効化しない点に注意）を docs で案内する。
- executable deny は deny-list 方式のため抜け道（exotic interpreter・exec wrapper 亜種、将来の新 runtime）が残り得る — exec-capable binary への grant は shell 認可と等価であり、deny list は containment ではなく known-bad への fail-fast として扱う。static list を core policy 層に集約し、発見時は deny set を拡張する運用とする。
- grant の env は明示 authorization として扱う（task.env と同規則）— credential 値を policy file に書くこと自体は trusted author の責務とする。加えて grant `env` の値は trusted author の content だが injection 変数（`LD_PRELOAD`・`DYLD_INSERT_LIBRARIES`・`NODE_OPTIONS`・`GIT_CONFIG_GLOBAL` 等）を運び得る — env key の deny list は設けず、trusted author の責務として docs に警告を明記する。

## Deferred Design Concerns

- interpreter opt-in knob（per-grant `allowInterpreter` 等）: 本 iteration は無条件 deny。issue の「既定で禁止」を満たした上で、interpreter 経由が必要な legit 要求が出た場合に別 spec で設計する。risk: 正当な interpreter 利用（python 製 build tool 等）は現行では process task か shell で代替するしかない。
- project 側 narrowing field: Option A で本 scope から除外済み（ADR consequence どおり将来の明示的 workspace trust 設計に委ねる）。
- grant の `cwd` に absolute path / 複数 workspace 対応は要望が出れば別途設計（現行は `process` task と同じく workspace-relative のみ）。

## Verification Notes

- [ ] `process` 既存 test（unit + `proc_process.rs` + golden）が無変更で pass（AC1）。
- [ ] grant 空の runtime で `command` expose が `POLICY_VIOLATION` で fail-closed（worker scope + workflow tool job 両経路）。`names` に `Command` を含まない runtime は解決不能 grant を持つ policy でも dormant で構築できる test。
- [ ] grant authoring key を含む project `policy.yaml` が load 時 `InvalidRuntimePolicy` で拒否、同内容の user policy は受理される focused test。
- [ ] `create_worker_scope` の grant re-resolve parity test — declared-shape resolved policy（`to_value` → re-resolve → `PartialEq`）が一致しない scope を reject。
- [ ] `{name, args}` 以外の input key（`executable`/`cwd`/`env` 等）を持つ input が `INVALID_INPUT` で reject される test。
- [ ] interpreter executable（python/perl/ruby/node.exe、python3.13t 等の versioned name 含む）・exec wrapper/loader（env/xargs/sudo/docker/ld-linux 等）・shell/shim・workspace-resident executable・metachar token・argv pattern 不一致・cwd escape が実行前に reject される test。
- [ ] timeout/abort/nonzero/output-cap/tree-cleanup が Windows/POSIX で検証（shared `proc::run` 経由、`process` と同一実装である構造確認含む）。
- [ ] `savings.directlyComparable` false・`meta.changes` が未観測副作用を主張しない・独自 pool `command` の cap が効くこと。
- [ ] `mise run check`（fmt / clippy / `cargo test --workspace` dev-seam 付き / doc-id check）が pass。

## Refine 注記（issue #169）

権限モデル再設計（#151 分解 #163–#168）により、本 doc の以下の記述は現行の真実と異なる。本 design は決定時点の記録として本文は保持する。

- `:104`・`:113`（§(a)）— `argv_shell_re` metachar check・pattern 文字列自体の metachar 検査を含む token 検証 → call-time の内容審査は #163 で除去。metachar を含む token も pattern 適合なら executable の literal argv として渡る（残存 bounds: token ≤4096・args ≤256 は input schema 層の `INVALID_INPUT`）。
- `:119-123`（§(b)）— workspace-resident executable 二段拒否・`native()` candidate 検証・`(d)` interpreter deny 適用の pin 手順 → 内容審査系（native magic・workspace-resident・interpreter deny）は #163 で除去。pin 契約（B1/B2 — load 時 1 回 + spawn 前再検証）は維持。
- `:139-158`（§(d)）— interpreter denial 節全体（deny set 拡張・`command_denied_executable_re()`・3 適用点）→ #163 で mandatory 層から除去、#164 で opt-in `security.execDenyPatterns`（deterministic executable deny-list・既定 `[]`・basename stem 完全一致・policy resolve / pin / spawn 前再検証の 3 適用点は存続）へ降格。
- `:209`（§(j)）・`:259`・`:300` — error 写像表・integration test・Verification の metachar/interpreter/shell/shim/workspace-resident 拒否項目 → 同上。deny 該当は `execDenyPatterns` 宣言時のみ拒否側。

**supersede ではない** — pin 契約（B1/B2）・argv pattern 言語（A6）・Exclusive root manifest（C7）・route-context 注入・trusted-layer 境界は引き続き authoritative。

governing: [ADR-034L6ihibvbjmSnKmSffWH](../adr/documentation-honesty-sweep-and-migration-guide.md)（canonical 文言・訂正方式）・[SPEC-034L6i3XmfCaWRHAiw8TTT](../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md) R5・[migration guide](../guides/migration.md)
