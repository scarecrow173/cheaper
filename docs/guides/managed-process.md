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

`command` は固定 argv 配列。呼出し input は `task` のみで、任意 command/argv の指定、shell、shell 演算子、script shim を拒否する。pipe、semicolon 等は正規の引数内でも拒否するため、複雑な処理は信頼した script に記述する。cwd は既存 workspace 相対 directory。inputs/outputs は cwd に関わらず workspace root 相対で、literal output は exact、末尾 `/**` は subtree ownership として正規化する。部分 wildcard output は ownership を確定できず拒否される。

Windows の `.cmd`/`.bat`/`.ps1` shim は実行しない。pnpm 等は native executable と JS entrypoint の argv にする。native PE/ELF/Mach-O executable のみ解決し、bare command は host PATH の絶対 directory を検索する。Rust runtime には Node host executable が存在しないため、bare `node` は PATH 解決せず `POLICY_VIOLATION` の明示 error で拒否する（PATH 上に `node` が実在しても同じ）。Node 製 tool を task 化する場合は node binary の絶対 path を command に使う。見つからなければ拒否する。

子 env は task.env で明示した値のみ。Windows は OS 用 SystemRoot を補完し、Windows が親から補完する必須 env（PATH/USERPROFILE/TEMP 等）は未指定なら空値にする。親 credential、NODE_OPTIONS、NODE_V8_COVERAGE 等の自動伝播は継承しない（spawn は env_clear 相当）。PATH 等が子にも必要なら task.env に明示する。stdin は ignore。

Task の I/O 宣言から ResourceManifest と Workflow producer/consumer edge を導出する。Workflow compiler には同じ policy で構築した `ToolCatalog`（process policy 込み）と `WorkspaceResourceInference` を渡す。unknown task は fail closed、明示 resources の underclaim や outputs 外の producer/write overclaim を拒否する。既存 process declaration seam は input.task と一致しなければ拒否する。

Invocation 全 scope で global operation cap と `process.maxConcurrent` pool を共有する。output 無しの task は shared、output 有りは exclusive。ResourceManager の claims と pool 内の提出順で admission を行い、claim conflict がない他 tool は pool 待ちの process を追い越せる。running capacity と resource lease は tree cleanup を含む実際の settle まで保持する。

成功 details は task/state/exitCode/signal と stdout/stderr の text/bytes/capturedBytes/truncated。両 stream 合計 `process.maxOutputBytes` raw bytes（既定65536、設定上限16MiB）を保持し、上限後も drain する。UTF-8 表示も同じ byte cap 内に収める。content は token 計測の長い文字列処理を避けるため64文字単位で折り返す表示で、正確な stream text は details にある。nonzero/spawn failure は bounded details を持つ TOOL_EXECUTION_FAILURE。

`timeout` の既定は2分で、native spawn から root exit までを測る。queue、prepare、admission の待ち時間は含まない。timeout は TIMEOUT / operation `timed_out` となり、外側 abort が先なら元の reason / `cancelled` を保持する。`killGrace` は既定2秒。POSIX detached process group に SIGTERM を送り、grace 内に終了したら force-kill せず、残存 group に SIGKILL を送る。Windows は creation identity を持つ bounded CIM snapshot で管理下の子孫を追い、taskkill `/T` → grace → `/T /F` とする。Windows console app の graceful termination は best effort で、POSIX と同じ graceful delivery は保証しない。

これらの設定は top-level `process.*` のみで、task ごとの override はない。trusted user の replace 後に既定値を補完し、project は timeout、maxOutputBytes、killGrace、heartbeat.interval を min で制限する。duration は正の有限値で Node timer の上限以下。heartbeat.interval は最大観測間隔なので、project は短縮できるが長くしたり無効化したりできない。

heartbeat は既定30秒ごとに runtime 自身の stderr に bounded JSON diagnostic を出す。task/PID/elapsedMs/outputBytes/phase を含み、backpressure 時は drop する。child stderr、tool result、Worker context に混ぜず、LLM turn や wake を生成しない。executor が全 timer を所有し、handler settle で停止する。Worker/Workflow は poll しない。

正常/nonzero root exit、timeout、abort、scope/invocation disposal で tree と pipes/listeners/timers を回収する。scope dispose はその owner のみ停止する。managed process の dispose は generic `abortGraceMs` を超え得るが、killGrace と OS helper の各2秒 deadline/output cap による bounded cleanup と actual settle を待つ。cleanup failure は診断し、既にある abort/timeout reason を上書きしない。全出力の Artifact Store handoff は streaming writer contract が必要なため延期し、切り捨てた出力を追加 buffer しない。

Named task 登録が authorization boundary で、project policy は任意 executable を事前認可できる。信頼できる project の task のみ許可する。宣言は scheduler/ownership の入力であり OS sandbox や tracer ではない。denyPaths と writeScope の検証は declared I/O/cwd に限られ、`.cheaper/**` を deny しても process の未宣言 access を制限できない。未宣言 access を観測済みとみなさず、mutation journal もその変更を網羅しない。

管理下の parent/child/grandchild は正常 root exit 後も残存検査して終了する。POSIX の setsid 等による group escape は containment 対象外。Windows は初回 identity snapshot 前に中間親が終了し ancestry を失った子孫を追えない（宣言済み W1 platform limitation）。これは adversarial escape とは別の制約で、native Job Objects は将来判断に延期する。長寿命 service、container/remote sandbox、poll/status API はこの tool の対象外。
