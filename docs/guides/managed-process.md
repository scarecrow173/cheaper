# Managed process

`process` は policy で事前登録した named task を実行する canonical tool。Worker と Workflow は同じ `{"kind":"tool","tool":"process","input":{"task":"test"}}` を使う。Worker では `tools` と policy の `tools.allow` に `process` が必要で、task が空なら expose されない（`cheaper.optionalTools` opt-in の worker は未登録へ縮退して run が継続し、opt-in していない worker では scope 構築が `POLICY_VIOLATION` で失敗する — [workers ガイド](./workers.md) 参照）。

```yaml
process:
  maxConcurrent: 1
  timeout: 2m
  maxOutputBytes: 65536
  killGrace: 2s
  heartbeat:
    interval: 30s
  tasks:
    test:
      command: [node, node_modules/vitest/vitest.mjs, run]
      cwd: .
      inputs: ["src/**", "test/**"]
      outputs: []
      env:
        CI: "true"
```

`command` は固定 argv 配列。呼出し input は `task` のみ — model から任意 command/argv は指定できない。宣言した `command` 配列は内容審査なしに literal argv として spawn される — shell・script file・演算子を含む要素も executable の引数としてそのまま渡る（cheaper は shell を介さないため `|`・`;` は制御構文として解釈されない）。executable 名・argv の正当性は task 作者の責務。cwd は既存 workspace 相対 directory。inputs/outputs は cwd に関わらず workspace root 相対で、literal output は exact、末尾 `/**` は subtree ownership として正規化する。部分 wildcard output は ownership を確定できず拒否される。

resolve 時に `process.tasks` は `command.grants` と**単一 `exec` grant namespace** を共有する `fixed:true` grant へ畳まれる（issue #165）— executable は `command[0]`、argv は `command[1..]` の literal（`command` tool の argv pattern 契約とは別物で compile 対象外）、lifecycle 既定値（timeout/killGrace/maxOutputBytes）は `process` section から畳み込まれる。両 section の名が衝突すると policy load は `InvalidRuntimePolicy` + `details.{grant,task}` で fail closed する。`process` surface は `fixed:true` grant のみに到達でき、`command` surface は `!fixed` grant のみに到達できる（unknown はそれぞれ "unknown or unauthorized process task" + `details.task` / "unknown or unauthorized command grant" + `details.grant`）。

executable 名・内容の審査は行わない — spawn viability（realpath・is_file・実行 bit）のみを判定する。win32 では CreateProcess が直接 spawn できる `.exe`/`.com` のみ解決する（`.cmd`/`.bat`/`.ps1` は viability で `POLICY_VIOLATION` — 内容審査ではなく OS の制約）。pnpm 等は native executable と JS entrypoint の argv にする。unix では +X file を内容無審査で解決し、bare command は host PATH の絶対 directory を検索する — bare `node` も PATH 上の実 `node` へ解決される。**POSIX で shebang を持たない +X file の実行が ENOEXEC となった際に libc（glibc `execvpe` 系）が `/bin/sh` で retry するのは OS/libc 既定動作であり cheaper の保証対象外（musl は retry しない残留差分あり）。** 見つからなければ `POLICY_VIOLATION` で拒否する。

子 env は task.env で明示した値のみ。Windows は OS 用 SystemRoot を補完し、Windows が親から補完する必須 env（PATH/USERPROFILE/TEMP 等）は未指定なら空値にする。親 credential、NODE_OPTIONS、NODE_V8_COVERAGE 等の自動伝播は継承しない（spawn は env_clear 相当）。PATH 等が子にも必要なら task.env に明示する。stdin は ignore。

Task の I/O 宣言から ResourceManifest と Workflow producer/consumer edge を導出する。Workflow compiler には同じ policy で構築した `ToolCatalog`（resolved `exec` policy 込み）と `WorkspaceResourceInference` を渡す。unknown task は fail closed、明示 resources の underclaim や outputs 外の producer/write overclaim を拒否する。既存 process declaration seam は input.task と一致しなければ拒否する。

worker が `writeScope` を宣言する場合、task の writeScope coverage は `outputs` の tri-state で決まる: `outputs` 宣言値（nonempty）は `outputs ⊆ writeScope` の coverage 必須（超過は `POLICY_VIOLATION` "process outputs exceed worker writeScope"）、`outputs: []` は「副作用を宣言しない」契約で coverage は trivially 成立、`outputs` 未宣言は副作用が resource 層へ不可視で coverage 証明不能のため `POLICY_VIOLATION` で fail closed（writeScope 非宣言なら通常実行）。`command` tool（`!fixed` grant）は writeScope 下の worker では構築時 gate で `"command is incompatible with worker writeScope"` と拒否される（[workers ガイド](./workers.md) 参照）。

Invocation 全 scope で global operation cap と `process.maxConcurrent` pool を共有する。output 無しの task は shared、output 有りは exclusive。ResourceManager の claims と pool 内の提出順で admission を行い、claim conflict がない他 tool は pool 待ちの process を追い越せる。running capacity と resource lease は tree cleanup を含む実際の settle まで保持する。

成功 details は task/state/exitCode/signal と stdout/stderr の text/bytes/capturedBytes/truncated。両 stream 合計 `process.maxOutputBytes` raw bytes（既定65536、設定上限16MiB）を保持し、上限後も drain する。UTF-8 表示も同じ byte cap 内に収める。content は token 計測の長い文字列処理を避けるため64文字単位で折り返す表示で、正確な stream text は details にある。nonzero/spawn failure は bounded details を持つ TOOL_EXECUTION_FAILURE。

`timeout` の既定は2分で、native spawn から root exit までを測る。queue、prepare、admission の待ち時間は含まない。timeout は TIMEOUT / operation `timed_out` となり、外側 abort が先なら元の reason / `cancelled` を保持する。`killGrace` は既定2秒。POSIX detached process group に SIGTERM を送り、grace 内に終了したら force-kill せず、残存 group に SIGKILL を送る。Windows は creation identity を持つ bounded CIM snapshot で管理下の子孫を追い、taskkill `/T` → grace → `/T /F` とする。Windows console app の graceful termination は best effort で、POSIX と同じ graceful delivery は保証しない。

これらの設定は top-level `process.*` のみで、task ごとの override はない。trusted user の replace 後に既定値を補完し、project は timeout、maxOutputBytes、killGrace、heartbeat.interval を min で制限する。duration は正の有限値で Node timer の上限以下。heartbeat.interval は最大観測間隔なので、project は短縮できるが長くしたり無効化したりできない。

heartbeat は既定30秒ごとに runtime 自身の stderr に bounded JSON diagnostic を出す。task/PID/elapsedMs/outputBytes/phase を含み、backpressure 時は drop する。child stderr、tool result、Worker context に混ぜず、LLM turn や wake を生成しない。executor が全 timer を所有し、handler settle で停止する。Worker/Workflow は poll しない。

正常/nonzero root exit、timeout、abort、scope/invocation disposal で tree と pipes/listeners/timers を回収する。scope dispose はその owner のみ停止する。managed process の dispose は generic `abortGraceMs` を超え得るが、killGrace と OS helper の各2秒 deadline/output cap による bounded cleanup と actual settle を待つ。cleanup failure は診断し、既にある abort/timeout reason を上書きしない。全出力の Artifact Store handoff は streaming writer contract が必要なため延期し、切り捨てた出力を追加 buffer しない。

Named task 登録が authorization boundary で、project policy は任意 executable を事前認可できる。信頼できる project の task のみ許可する。project policy 由来の task は `scope:"project"` grant として解決され（provenance は merge 前の project task 名集合から記録される）、deep-merge で project task が同名の trusted task を shadow しても `scope:"project"` が保存される — route-eligible な `scope:"trusted"` には昇格しない。`security.execDenyPatterns` は task executable にも `command` grant と同じ 3 箇所（policy resolve の declared spec 照合・pin 時の resolved basename 照合・spawn 前再検証）で均一適用され、resolve 該当は `InvalidRuntimePolicy` + `details.{grant,executable,task}` で拒否される。宣言は scheduler/ownership の入力であり、cheaper は OS sandbox ではない。canonical file tool（write/edit/ls/read/find/grep）の workspace boundary・`filesystem.denyPaths`・worker `writeScope` は cheaper 所有の tool surface で強制されるが、`process` が spawn した子プロセスの fs 副作用は OS sandbox 無しでは強制・完全観測できない。denyPaths と writeScope の検証は declared I/O/cwd に限られ、`.cheaper/**` を deny しても process の未宣言 access を制限できない。未宣言 access を観測済みとみなさず、mutation journal もその変更を網羅しない。spawn viability と ENOEXEC の区別は executable 解決の節（上記）を参照。

管理下の parent/child/grandchild は正常 root exit 後も残存検査して終了する。POSIX の setsid 等による group escape は containment 対象外。Windows は初回 identity snapshot 前に中間親が終了し ancestry を失った子孫を追えない（宣言済み W1 platform limitation）。これは adversarial escape とは別の制約で、native Job Objects は将来判断に延期する。長寿命 service、container/remote sandbox、poll/status API はこの tool の対象外。
