---
id: "DESIGN-034A8ubkLAdLl7ICQZRYM5"
type: "design"
status: "approved"
title: "managed process canonical executor の設計"
created: "2026-10-04"
updated: "2026-10-05"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/59"]
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/managed-process-canonical-tool.md"
    - "../adr/managed-process-named-task-lifecycle.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  defers: []
  deferred-by: []
---

# Managed process canonical executor

## Context

共有 ADR D1/D2/D4/D5/D7 に従い、Pi 非依存の canonical process を既存 invocation に追加する。process は tool job のままであり、CLI/Workflow engine の追加実装は行わない。overview は読むが変更しない。

## Scope

### In Scope

- (A) core named task policy/schema/validation/resource 宣言。(B) bounded spawn/tree kill と scheduler pool、adapter 接続。

### Out of Scope

- shell、tracing、sandbox、独立 process job、poll/status、依存/manifest/root script 変更。

## Components and Boundaries

- core `policy/process.ts`: ProcessTask/ProcessPolicy、schema、resolveProcessPolicy、shell argv と task 名/cwd/env を検証する。schema.ts の optional process に接続する。legacy omission は normalize/load により maxConcurrent=1/tasks={} へ補完してから project restriction する。
- task name は lowercase ASCII identifier。command は非空 argv 配列で、各引数に NUL/newline、shell 演算子（pipe、ampersand、semicolon、redirection、backtick、$()）を許可しない。argv0 の sh/bash/cmd/powershell 等 shell と Windows script shim は拒否。command/input/env の field 長は schema と resolver で制限する。
- inputs/outputs は workspace-relative pattern 配列。cwd の変更で I/O の基準を変えず、I/O は workspace root 相対に統一する。literal output は exact、末尾 /** は subtree。core normalizeResourceManifest を再利用し、partial output glob は ownership error。writeScope がある場合、canonical output claim は exact 許可 path 内でなければ拒否。
- tool-runtime `process.ts`: metadata/input schema、named task 解決、workspace cwd/stat/deny、resource normalization と実行前 validation、native executable resolver、fixed 64KiB combined capture、exit diagnostics、tree cleanup。spawn を直接 tool definition へ閉じ込め、外部任意 command API を公開しない。
- catalog は optional ProcessPolicy を clone し、process.resources(input) で同じ core 宣言 helper を使う。未知 task は拒否。input additionalProperties:false により command/argv 注入を拒否する。
- registry は process だけ task の宣言から OperationDeclaration を導出。outputs が無ければ shared、あれば exclusive。`pool:{name:'process',maxConcurrent}` を scheduler に渡す。
- scheduler は invocation-owned pool counter/cap を持ち、FIFO admission 時に global/exclusive/pool の全条件を検査する。実 settle の finally で両 capacity を解放する。queued abort の既存 reason と budget/meter 1回を保持する。
- invocation は process policy を construction 時に snapshot し scope に固定する。scope の process 省略は invocation policy を継承、明示指定の不一致は fail closed。standalone runtime は options.process を resolver で検証する。names に process が含まれ tasks が空なら、定義を expose する前に POLICY_VIOLATION を返す。metadata catalog の取得自体は exposure/認可ではない。
- core isMutatingTool は process を conservatively mutation とし、savings の直接比較を避ける。runtime の task-specific readonly access は registry の宣言 seam が所有する。
- runtime-pi は runWorker の policy.process を owned invocation/ツール options へ渡す。existing capability intersection で process を許可した Worker にだけ expose する。
- Workflow inference は既存 item.process seam を維持し、未指定時は policy-aware catalog process.resources を使う。task が解決できたら source.authority は task.outputs から正規化した produces claim。process 入力 task と seam task の不一致は拒否する。未知 task/不明 policy は unknownAuthority 相当で authoritative scope を認証できず、compile を fail closed にする。producer authority と underclaim validation は既存アルゴリズムを再利用する。

## Data and Control Flow

- canonical execute → strict input validate → named task declaration → accounting → OperationManager queue → scheduler global + access + pool admission → resource/cwd validation → native spawn（stdin ignore）→ drain/capture → child close + cleanup → canonical result または TOOL_EXECUTION_FAILURE。境界拒否は既存 accounting 内で一度記録する。
- executable: node は process.execPath が実体名 node/node.exe の時のみそれを使用（SEA の cheaper.exe を Node として再起動しない）。それ以外の node/ bare command は trusted host PATH の native executable を候補として解決。Windows は .exe/.com のみで .cmd/.bat/.ps1 に fallback しない。relative executable は validated workspace path。absolute executable は task policy が認可した OS executable として許可する。resolver unit test では SEA execPath と native Node PATH 候補を注入して確認する。
- env は task.env だけ、Windows は SystemRoot を OS 用に補完し、libuv が自動継承する required env は未指定時に空値で遮断する。NODE_V8_COVERAGE は own undefined で Node の自動補完を遮断する。親 PATH/credential/NODE_OPTIONS は継承しない。task.env は明示 authorization として扱う。
- POSIX detached process group を SIGKILL、Windows SystemRoot/System32/taskkill.exe /PID n /T /F を shell:false/windowsHide:true で呼ぶ。cancel listener は close/cleanup 後に除去。cleanup failure は成功にせず、child error/close と cancel first reason を区別する。
- 結果 details = task/state/exitCode/signal/stdout/stderr。各 stream は text/bytes/capturedBytes/truncated。nonzero exit は同じ bounded details を持つ error。cancel は原 reason を reject するため manager の cancelled state を保存する。
- content の表示は64文字で折り返し、bounded でも高密度の長い文字列で同期 BPE 計測が停滞するのを避ける。正確な stream text は details に保持する。

## Non-Functional Constraints

- capture は合計65536 raw bytes、text もUTF-8再符号化でその cap 内。stream は上限後も drain。process tree の kill helper に bounded deadline を置き、待機による orphan は許さない。
- input/task policy を snapshot し、caller の後続 mutation で認可 argv/metadata を変更できない。
- resource 宣言は信頼する scheduler 入力で OS-level access 制限ではない。実行後未宣言 access を観測済みとしない。

## Risks and Trade-offs

- Windows shim 非対応: node と package の JS entrypoint を explicit argv で設定し、cmd /c に逃げない。
- process policy の task は project でも認可できる。trusted maxConcurrent cap は既存 min merge を再利用する。未知の process access は core predicate に従い mutation 側へ縮退する。
- #61 の lease が未実装なので output task の overlap は conservatively serialize。#61 landing 後の ordering は claim conflict ベースであり、pool 飽和で待つ先頭 process を resource conflict の無い後続 tool が追い越してよい。pool 内 process 間の提出順は維持する。
- argv のメタ文字拒否により、正規の引数内に pipe/semicolon 等を含める task も拒否する。回避目的の shell wrapper は許可せず、必要な処理は信頼された script へ移す。
- 宣言外 I/O/外部 race は検出不能。setsid/re-parent 孫へ kill tree は届かず、正常終了後の孫残留検査もない。project policy の task は任意 executable を事前認可できるため、project policy を信頼することが前提。.cheaper/** を denyPaths に指定しても宣言/検証対象への拒否に限られ、未宣言の process access を制限できない。これらを guide に明記する。

## Deferred Design Concerns

<!-- Intentionally deferred future work. Link the deferred draft doc via relations.defers. -->

non-goals の API を追加しない。#61 の ResourceManager は共有 ADR に従う別 issue の責務。

## Verification Notes

- [x] coordinator 承認: msg_126e0d3e23d0、spec-approval / adr-acceptance / design-approval。hot-file 範囲は core vocabulary/policy/schema/load/worker predicate、builtin policy ×2、core/index.ts と tool-runtime/index.ts の export 行。
- [x] policy、tool-runtime、runtime-pi、workflow の focused suite と pnpm lint/typecheck/build を実行する。
- [x] cancellation は実 PID/孫 PID の生存確認を行い、output cap は実大量/binary出力で検証する。
- [x] resource-inference.test.ts の既存 process 未宣言ケース（review時314–318行）を、policy/catalog から解決不能なら fail closed、解決可能なら canonical manifest になる契約に更新する。
