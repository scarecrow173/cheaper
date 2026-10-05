# Managed process

`process` は policy で事前登録した named task を実行する canonical tool。Pi Worker と Workflow は同じ `{"kind":"tool","tool":"process","input":{"task":"test"}}` を使う。Worker では `tools` と policy の `tools.allow` に `process` が必要で、task が空なら expose されない。

```yaml
process:
  maxConcurrent: 1
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

Windows の `.cmd`/`.bat`/`.ps1` shim は実行しない。pnpm 等は native `node` と JS entrypoint の argv にする。native PE/ELF/Mach-O executable のみ解決し、bare command は host PATH の絶対 directory を検索する。通常の Node 実行では native `process.execPath` を使い、SEA では host PATH の Node を解決する。見つからなければ拒否する。

子 env は task.env で明示した値のみ。Windows は OS 用 SystemRoot を補完し、libuv が親から補う PATH/USERPROFILE/TEMP 等は未指定なら空値にする。親 credential、NODE_OPTIONS、Node の NODE_V8_COVERAGE 自動伝播を継承しない。PATH 等が子にも必要なら task.env に明示する。stdin は ignore。

Task の I/O 宣言から ResourceManifest と Workflow producer/consumer edge を導出する。Workflow compiler には同じ policy を与えた `createToolCatalog({process: policy.process})` と `WorkspaceResourceInference` を渡す。unknown task は fail closed、明示 resources の underclaim や outputs 外の producer/write overclaim を拒否する。既存 process declaration seam は input.task と一致しなければ拒否する。

Invocation 全 scope で global operation cap と `process.maxConcurrent` pool を共有する。output 無しの task は shared、output 有りは exclusive。#61 の lease 導入前は output task を保守的に serialize する。pool 飽和で FIFO 先頭が待つ間は後続の他 tool も待つ。running capacity は実際の settle まで保持する。

成功 details は task/state/exitCode/signal と stdout/stderr の text/bytes/capturedBytes/truncated。両 stream 合計65536 raw bytes を保持し、上限後も drain する。UTF-8 表示も同じ byte cap 内に収める。content は token 計測の長い文字列処理を避けるため64文字単位で折り返す表示で、正確な stream text は details にある。nonzero/spawn failure は bounded details を持つ TOOL_EXECUTION_FAILURE。abort は元の reason を保持し、POSIX process group または Windows taskkill tree を終了させる。

Named task 登録が authorization boundary で、project policy は任意 executable を事前認可できる。信頼できる project の task のみ許可する。宣言は scheduler/ownership の入力であり OS sandbox や tracer ではない。denyPaths と writeScope の検証は declared I/O/cwd に限られ、`.cheaper/**` を deny しても process の未宣言 access を制限できない。未宣言 access を観測済みとみなさず、mutation journal もその変更を網羅しない。

Cancel は管理下の parent/descendant を終了させるが、setsid や re-parent で管理下を離れた子孫には届かない。正常終了後の子孫残留検査も行わない。長寿命 service、container/remote sandbox、poll/status API はこの tool の対象外。
