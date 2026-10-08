---
id: "PLAN-034BepYuOf4htZRfdkZTYc"
type: "plan"
status: "approved"
title: "Phase 2 port cheaper-tools"
created: "2026-10-06"
updated: "2026-10-06"
owners: []
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
  implemented-by: []
  depends-on:
    - "docs/plans/phase-1-port-cheaper-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/plans/migrate-cheaper-runtime-to-rust-rig.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/rust-migration-run-semantics.md"
    - "docs/test-specs/rust-migration-differential-golden.md"
  references: []
  defers: []
  deferred-by: []
---

# Phase 2 port cheaper-tools Implementation Plan

## Goal

`packages/tool-runtime`(src ≈4.5k 行)と `runtime-pi/src/tools` の boundary
operations / Pi helper 接着部を、`cheaper-core` のみに依存し tokio 依存を
許可した `crates/cheaper-tools` crate へ移植し、canonical tool 8 種
(read/write/edit/ls/grep/find/count_tokens/process)・tool scheduler・
budget/accounting・mutation journal・artifact port を TS 版と意味論同一で
成立させる。

## Scope

- 単一の実装 stream とし、分割しない。crate 1 個・依存 `cheaper-core` のみの
  境界が設計書で確定済みのため。
- In scope:
  - `crates/cheaper-tools` 全モジュール(scheduler・resources・operation・
    access・accounting・journal・artifact port・catalog・registry・
    invocation・builtin tool 8 種・walker/line-framer/limits)。
  - write/edit/ls の self-contained 実装(現行は Pi helper
    `createWriteToolDefinition`/`createEditToolDefinition`/`createLsToolDefinition`
    + `runtime-pi/src/tools/operations.ts` の境界 ops を注入する構成。
    Rust では Pi が無いため tool 本体も cheaper-tools が所有する)。
  - grep の `regress` crate 採用 + eval subprocess 隔離。
  - process tool の固定ルール完全 port(設計書 §tool 移植の固定ルール)。
  - unit test・TS fixture 差分検証・Windows 実機検証。
- Out of scope(別 Phase の責務):
  - Rig `Tool`/`PortableTool` trait 接着・`createSandboxedTools` 相当の
    session 露出(Phase 3 `cheaper-rig-adapter`)。
  - artifact store 実体(workflow、Phase 4。本 Phase は
    reader/publisher port の trait 定義のみ)。
  - CLI dispatch・envelope(Phase 4)。grep eval helper の production 起動
    mode 配線も Phase 4(Open Questions 参照)。
  - provider 側に返す tool definition の transport 変換(Phase 3)。

## Upstream Documents

- `docs/specs/rust-rig-native-runtime-migration-for-cheaper-cli.md`
- `docs/adr/migrate-the-cheaper-cli-runtime-to-rust-and-rig.md`
- `docs/plans/migrate-cheaper-runtime-to-rust-rig.md`(親 plan・共通ルール)
- `docs/plans/phase-1-port-cheaper-core.md`(前提: core 型/API 供給元)

## Design Inputs

- `docs/designs/rust-rig-runtime-migration-architecture.md`
  - crate 表 `cheaper-tools` 行(責務列挙・tokio 許可)
  - §tool 移植の固定ルール: grep JS RegExp 方言は tool contract・`regress`
    第一候補 + eval subprocess 隔離・`limits.ts` budget port・Windows path
    canonicalize 規則(core 責務)・process tool 列挙完全化(timeout/
    heartbeat/killGrace/task resource declaration/PID reuse 照合・
    `node` task は PATH 解決せず明示 error)
  - §Rig 適用の意味論固定: invalid/unknown tool call は runtime 未到達で
    `toolCalls`/`rejectedToolCalls` 不計上。計数写像は
    「実行到達=toolCalls・boundary/limit 拒否=rejectedToolCalls」
- `docs/designs/canonical-tool-runtime.md`(runtime/scope/lifetime・budget
  外側/meter 内側 1 回適用・拒否計数規則)
- `docs/designs/bounded-tool-scheduler.md`(FIFO admission・shared/
  exclusive・pool cap・queued cancel first-reason・late settle release)
- `docs/designs/managed-process-canonical-tool.md`(process 責務全列挙)
- `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`
  (boundary/deny/writeScope)

## File Map

`crates/cheaper-tools/`。依存は `cheaper-core`(path)+ tokio
(rt/process/fs/time/sync 系)+ `tokio-util`(CancellationToken)+
`serde`/`serde_json` + `regress` + `unicode-normalization`(edit fuzzy 用
NFKC)+ line diff 実装(`similar` 第一候補、不一致時は jsdiff 直接 port)
+ glob matcher(`globset` 第一候補、`path.posix.matchesGlob` 互換検証要)
+ tmp 名乱数用の軽量 RNG(`rand` or `fastrand`)。POSIX 側 process 制御は
`libc`/`rustix` 系 cfg 依存(kill(-pid)、setrlimit)。`AbortSignal`/
`AbortController`/`AbortSignal.any` は reason を運ぶ内部小 struct として
CancellationToken + `OnceCell<reason>` で再実装(registry の
cancelled/timed_out 分類と first-cause 保存が contract)。active operation
参照は `AsyncLocalStorage` 相当を `tokio::task_local!` で再現。

| Rust path | 移植元(TS) | 責務 |
| --- | --- | --- |
| `src/lib.rs` | `index.ts` | 公開面 re-export。crate 外 API の唯一の入口 |
| `src/limits.rs` | `limits.ts` | 全上限定数を verbatim port(表現ドリフト禁止: 第二の定義を持たない) |
| `src/line_framer.rs` | `line-framer.ts` | byte 上限付き行フレーマ。`split("\n")` parity(LF 終端/空入力で末尾空行 emit)・oversized 破棄モード |
| `src/walker.rs` | `walker.ts` | deny message 3 定数(ACCESS/POLICY/SCOPE)・`rethrow_as_denied`・`is_access_denied_error`(rejectedToolCalls 判定源)・`sniff_text_file`・`walk_files`(`.git`/`node_modules` skip・symlink policy・denylist 存在秘匿・祖先 canonical で cycle 防止・opendir 逐次 read) |
| `src/artifact.rs` | `artifact.ts` | `ArtifactReader`/`ArtifactPublisher` trait + reference/read request/result/metadata 型(store 実体は workflow 側、ここは port のみ) |
| `src/operation.rs` | `operation.ts` | `OperationManager` 六状態機械(queued/running/completed/failed/cancelled/timed_out + transitions 表)・cancel first-reason・owner 単位 cancel・drain(grace→unsettled detach)・monotonic clock 注入 |
| `src/execution_metrics.rs` | `execution-metrics.ts` | `ExecutionMetrics` 集計(byState/byTool/maxConcurrency/operationMs/activeMs/overlapMs/queueMs) |
| `src/resources.rs` | `resources.ts` | `ResourceManager`・`ResourceLease`・FIFO queue 単一仲裁点・parent lease borrow・claim conflict/cover 判定・copyClaims 正規化。pump は Mutex 保護の同期関数で「提出順 atomic 調停」を再現(prepare async 完了・release・notify・abort で再入) |
| `src/scheduler.rs` | `scheduler.ts` | `ToolScheduler`・`OperationDeclaration`(access/pool/resources/resourceScoped/parentLease/prepare/readiness)・global maxOperations・exclusive 排他・pool cap・canStart・start/late settle の両 capacity 解放 |
| `src/access.rs` | `access.ts` | `ResourceAccess`(prepare/operation/observe/snapshot)・writeScope claim・manifest 認可・support directory 判定・`MAX_ACCESS_OBSERVATIONS` |
| `src/accounting.rs` | `accounting.ts` | `create_tool_budget`(admission 時 `toolCalls++`・limit 超過 `rejectedToolCalls++`+`limitHit`+`onLimit`・実行 throw が access denied なら `rejectedToolCalls++`)・`create_tool_meter`(content[].text の `countTextTokens` 集計) |
| `src/journal.rs` | `journal.ts` | `MutationJournal`(captureBefore 冪等・finalize で diff 計数・差分無し除外・path sort)・`FileChange`/`ChangesSummary` |
| `src/definition.rs` | `definition.ts` | `CanonicalToolDefinition`(`prepare_arguments` hook field を追加)・`validate_tool_input`(INVALID_INPUT・details{tool,errors}、core `compileSchema` 経由) |
| `src/catalog.rs` | `catalog.ts` | 8 tool の input/output JSON schema(TypeBox 生成物を literal JSON 化)・`resources()` 宣言関数・result schema・`create_tool_catalog` |
| `src/tools/boundary.rs` | `runtime-pi/src/tools/operations.ts` | resolve + observeAccess + denylist の共有 ops・同 dir tmp + rename 原子 write・mkdir support 判定・`resolve_for_write` |
| `src/tools/read.rs` | `read.ts` | bounded read(LF byte 走査・WHATWG UTF-8 置換の byte 会計・byteInLine・`truncatedBy`/continuation・画像拡張子拒否・artifact 3 mode) |
| `src/tools/write.rs` | Pi `write.js` + operations.ts | mkdir 親生成 + atomic write + `Successfully wrote to ${path}` 出力・Pi description 文字列を verbatim port |
| `src/tools/edit.rs` | Pi `edit.js`/`edit-diff.js` + operations.ts | BOM 分離・line ending 検出/正規化/復元・`applyEditsToNormalizedContent`(exact→fuzzy NFKC 正規化・unique/非 overlap/no-change 検証)・`applyReplacementsPreservingUnchangedLines`・`generateDiffString`/`generateUnifiedPatch`(context=4)・`prepareEditArguments` normalization(edits JSON 文字列・単一 object・legacy oldText/newText) |
| `src/tools/ls.rs` | Pi `ls.js` + `truncate.js` | readdir + case-insensitive sort + `/` suffix + entry limit(500)+ `truncateHead` 50KB byte cap + notice 文言 |
| `src/tools/find.rs` | `find.ts` | `path.posix.matchesGlob` 相当 matcher・maxResults・有界 skip footer |
| `src/tools/grep.rs` | `grep.ts` | walk→sniff→chunk read→framer→batch eval の streaming・per-file/total eval budget・scan deadline・output caps・`B >= 3*L` invariant・有界 footer・`[truncated: ...]` marker |
| `src/tools/grep_eval.rs` | `grep.ts` の `GrepEvalWorker`/worker source | eval subprocess protocol(init/batch/result/error、長 prefix JSON lines over stdio)+ supervisor(watchMs = min(perFile 残・total 残・deadline 残)で timeout kill・output byte cap・heap cap)+ `helper_main()` 入口。engine は `regress::Regex`(非 unicode mode = JS `new RegExp` 意味)+ `i` flag。spawn 対象は `current_exe` + 内部 mode env/argv(option で差替可、test は helper 差替で検証) |
| `src/tools/count_tokens.rs` | `count-tokens.ts` | core `countFileTokens` 呼出し + details{path,tokens,bytes,tokenizer} |
| `src/tools/process.rs` | `process.ts` | named task 解決・prepare(cwd/deny/writeScope 検査)・minimal env(Windows required env 空値遮断・SystemRoot 補完)・spawn(stdin ignore/stdout-stderr pipe/windowsHide/POSIX detached)・64KiB combined capture・timeout(parseDuration)+`onTimeout`・heartbeat stderr JSON・details{task,state,exitCode,signal,stdout,stderr}・64 文字 fold 表示・`processOperationDeclaration`(pool + access 導出) |
| `src/tools/process_executable.rs` | `process-executable.ts` | shell/shim 拒否・native magic 検査(MZ `4d5a`/ELF `7f454c46`)・Windows `.exe/.com` のみ・relative=workspace 検証・absolute=許可。**`node` は PATH 解決せず POLICY_VIOLATION 明示 error(設計固定ルール: execPath fast-path の等価物なし)** |
| `src/tools/process_tree.rs` | `process-tree.ts` | POSIX: process group SIGTERM→live poll→SIGKILL・/proc zombie 判定。Windows: powershell CIM snapshot(pid/parent/created)+ taskkill /T /F・PID reuse 防止(creation timestamp 照合・bornBefore 規則)。helper timeout 10s/output 1MB |
| `src/registry.rs` | `registry.ts` | `create_tool_runtime`/`ToolRuntime`(ready/observeAccess/accesses/execute/operations/abort/drain/dispose)・definitions 構築(injection seam は廃止、8 種 native)・execute pipeline(prepare_arguments→validate→declaration→accounting→scheduler→task_local→meter)・process exposure 空 tasks 拒否・managed process settle 追跡 |
| `src/invocation.rs` | `invocation.ts` | `InvocationToolRuntime`(acquireJobLease/createWorkerScope/operations/executionMetrics/abort/drain/dispose)・scope 整合検査(root/symlink/process policy/owner/parentLease) |
| `tests/*.rs` | tool-runtime/test + runtime-pi/test/tools | 各モジュール統合 test・scheduler/operation 系は fake clock・deferred handler で決定的再現 |
| `tests/fixtures/` + `scripts/dump-tool-fixtures.mjs` | —(新規) | TS registry で CanonicalToolResult を JSONL dump する開発用 driver と共有 fixture 定義。Rust 側 `tests/differential.rs` が replay 比較(配布物非依存) |

## Implementation Sequence

- [ ] `crates/cheaper-tools` skeleton を cargo workspace に追加する(member 登録・`cheaper-core` path 依存・上記 crate 依存宣言・`lib.rs` 空公開面)
- [ ] `limits.rs` に `limits.ts` の全定数を同名で移植する
- [ ] `line_framer.rs` に `line-framer.ts` を移植する(`split("\n")` parity・oversized 破棄の unit test を含む)
- [ ] `walker.rs` に `walker.ts` を移植する(deny message 定数・sniff・walk の unit test を含む)
- [ ] `artifact.rs` に `artifact.ts` の port trait・型を移植する
- [ ] `execution_metrics.rs` に `execution-metrics.ts` を移植する
- [ ] `operation.rs` に `operation.ts` を移植する(六状態遷移表・cancel 分類・drain/detach の unit test を `operation.test.ts` 対応で含む)
- [ ] `resources.rs` に `resources.ts` を移植する(FIFO 仲裁・parent borrow の unit test を `resources.test.ts` 対応で含む)
- [ ] `scheduler.rs` に `scheduler.ts` を移植する(global/exclusive/pool/readiness/queued cancel の unit test を `scheduler.test.ts`・`tool-scheduler.test.ts` 対応で含む)
- [ ] `access.rs` に `access.ts` を移植する(prepare/operation/observe/support dir の unit test を含む)
- [ ] `accounting.rs` に `accounting.ts` を移植する(toolCalls/rejectedToolCalls 計数 4 分類の unit test を含む)
- [ ] `journal.rs` に `journal.ts` を移植する(diff 実装を `similar` 試行、行差分計数の unit test を含む)
- [ ] `definition.rs` + `catalog.rs` に `definition.ts`/`catalog.ts` を移植する(8 tool schema JSON・resources 宣言・validate の unit test を含む)
- [ ] `tools/boundary.rs` に `operations.ts` の境界 ops を移植する
- [ ] `tools/read.rs` に `read.ts` を移植する(`read.test.ts` の byte 会計・continuation・artifact case を unit test 化)
- [ ] `tools/ls.rs` + truncate ロジックを Pi `ls.js`/`truncate.js` から移植する
- [ ] `tools/write.rs` を Pi `write.js` + boundary ops から移植する(description 文字列・出力文言を verbatim)
- [ ] `tools/edit.rs` + edit-diff を Pi `edit.js`/`edit-diff.js` から移植する(fuzzy/CRLF/BOM/overlap/dup/no-change・`prepareEditArguments` の unit test を含む)
- [ ] `tools/find.rs` に `find.ts` を移植する
- [ ] `tools/count_tokens.rs` に `count-tokens.ts` を移植する
- [ ] `tools/grep_eval.rs` に eval subprocess protocol・supervisor・`helper_main` を新設する(`regress` 採用・watch/heap/output cap の unit test を含む)
- [ ] `tools/grep.rs` に `grep.ts` を移植する(batch 組立・budget/deadline・footer の unit test を `grep.test.ts` 対応で含む)
- [ ] `tools/process_executable.rs` に `process-executable.ts` を移植する(`node` 明示 error 化を含む)
- [ ] `tools/process_tree.rs` に `process-tree.ts` を移植する(POSIX/Windows 両分岐)
- [ ] `tools/process.rs` に `process.ts` を移植する(`process.test.ts` 対応の unit test を含む)
- [ ] `registry.rs` に `registry.ts` を移植する(8 種 native 構築・execute pipeline・`runtime.test.ts` 対応の unit test を含む)
- [ ] `invocation.rs` に `invocation.ts` を移植する(`invocation.test.ts` 対応の unit test を含む)
- [ ] `lib.rs` の公開面を `index.ts` export 集合と対応付けて確定する
- [ ] `scripts/dump-tool-fixtures.mjs` + `tests/fixtures` + `tests/differential.rs` の差分基盤を構築する
- [ ] Windows 実機で `cargo test -p cheaper-tools` 全パスと process/grep 手動 smoke を完了する

## Dependencies and Risks

- 前提依存: Phase 1 `cheaper-core` 完了。本 crate が消費する core API:
  `resolveWorkspacePath`/`canonicalizeWorkspaceRoot`/`isInsideWorkspaceRoot`/
  `WorkspaceEscapeError`・`PathDenylist`・`SymlinkPolicy`・
  `normalizeResourceManifest`/`claimContains`/`claimsConflict`/`resourceKey`/
  `ResourceClaim`/`ResourceManifest`/`ResourceManifestInput`/
  `ResourceManifestDeclaration`/`ResourceScope`・`resolveConcurrencyPolicy`/
  `ConcurrencyPolicy`・`resolveProcessPolicy`/`resolveProcessTask`/
  `processTaskResources`/`isShellProcessExecutable`/`MAX_PROCESS_OUTPUT_BYTES`・
  `isMutatingTool`/`ToolName`/`TOOL_NAMES`・`compileSchema`・
  `countFileTokens`/`countTextTokens`/`TOKENIZER_IDENTITY`・`parseDuration`・
  `CheaperError`/ERROR_TABLE。
- Risks:

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `regress` が JS RegExp 方言(lookbehind・backreference・named group・`\p` 等)をカバーし切れず tool contract 破壊 | med | high | dialect fixture corpus で先に検証。不足が確定したら設計固定ルールに従い「JS 方言 subset への制限」を spec/contract 変更として正式起案(暗黙の破壊禁止) |
| 2 | eval subprocess 隔離のコスト(worker_thread より spawn が重い)・protocol 設計ミス | med | med | helper を常駐型(init 1 回・複数 batch 処理)にし batch 毎 spawn しない。timeout kill・output cap は初版から実装 |
| 3 | Windows 差分(canonicalize verbatim prefix・taskkill/powershell・PID reuse・minimal env・`.exe/.com`・path 表記) | high | high | Windows golden fixture を必須化し、Windows 実機検証を完了条件にする。TS 実装の Windows 分岐を先に仕様化してから port |
| 4 | npm `diff` と Rust diff(`similar`)の tie-break 差で `meta.changes`/`details.diff` がずれる(differential golden の厳密比較対象) | med | med | fixture で差分検出。不一致時は jsdiff `diffLines`/`createTwoFilesPatch` を直接 port(≈数百行)に切替 |
| 5 | `path.posix.matchesGlob`(minimatch 系)と `globset` の意味差(extglob・brace・`!` 等) | med | med | find/grep glob fixture で検証。不足時は matcher 差替え、または contract 変更として起案 |
| 6 | AbortSignal/microtask/pump 意味の再現ずれ(提出順・cancel first-reason・late settle release) | med | high | 状態機械・admission・drain の決定的 test(fake clock/deferred handler)で pin。differential の scheduler 系列 fixture も追加 |
| 7 | helper heap cap の Windows 実現性(Job Object) | low | med | 不達なら batch byte cap + timeout kill のみを v1 とし、差分を spec/plan に明記して判断に回す |

## Task Breakdown

- T2-1 skeleton + 基盤小物(limits/line_framer/walker/artifact)
- T2-2 scheduling core(execution_metrics/operation/resources/scheduler)
- T2-3 scope・計数・journal(access/accounting/journal)
- T2-4 catalog/definition + fs tools(boundary/read/ls/write/edit/find/count_tokens)
- T2-5 grep(grep_eval → grep)
- T2-6 process(process_executable → process_tree → process)
- T2-7 runtime 接合(registry/invocation/lib)
- T2-8 差分基盤 + Windows 実検証 + 差分解消

並行可能: T2-2・T2-4 は T2-1 後に並行可。T2-5・T2-6 は相互独立。
T2-7 は T2-2/T2-3/T2-4/T2-5/T2-6 完了後、T2-8 は T2-7 後。

## Verification

- [ ] `cargo test -p cheaper-tools` 全パス(各 TS test file の意味を写した
      unit/integration test。fake clock・deferred handler による決定的再現を含む)
- [ ] `cargo clippy -p cheaper-tools -- -D warnings`・`cargo fmt --check` 通過
- [ ] TS fixture 差分: `scripts/dump-tool-fixtures.mjs` で TS registry の
      `execute` 結果を JSONL dump し、Rust `tests/differential.rs` が同 fixture を
      replay して CanonicalToolResult(content/details/error 分類)を比較。
      fixture matrix: read(offset/limit/byteInLine/binary/image/artifact/
      不存在)・grep(方言 pattern 各種/context/glob/maxResults/output cap/
      footer/timeout 注入)・find(glob/skip footer)・ls(limit/byte cap/
      dir suffix/empty)・write(新規/nested mkdir/overwrite/scope deny)・
      edit(single/multi/fuzzy/CRLF/BOM/overlap/duplicate/not-found/
      no-change)・count_tokens・process(success/nonzero/output cap/
      timeout/tree kill/unknown task/`node` task 明示 error)・scheduler/
      budget(maxToolCalls/boundary deny/serialized/per-resource/pool/
      queued abort/drain grace)・journal(created/modified/no-change 除外)。
      normalize 規則: 時刻・tmp 名乱数・pid を除外して比較
- [ ] 計数写像の fixture pin: budget 内実行到達=`toolCalls`、boundary
      拒否・limit 超過=`rejectedToolCalls`、runtime 未到達 invalid call=
      不計上、の 4 分類が TS dump と一致
- [ ] Windows 実機(runner または開発 Windows ホスト)で `cargo test` 全
      パス + process taskkill tree kill・PID reuse 非誤爆・minimal env・
      grep eval timeout kill の smoke を実施

## Open Questions / Gaps

- `regress` の方言カバレッジ(実利用 pattern で不足が無いか)は fixture
  corpus 作成時に確定する。不足時は設計固定ルール通り spec/contract 変更を
  起案する(実装側で黙って subset にしない)。
- glob matcher 選定(`globset` で `path.posix.matchesGlob` を再現できるか、
  extglob/negation 等の不足時の扱い)は T2-4 で確定。
- helper heap cap の Windows 実現方式(Job Object `ProcessMemoryLimit`)の
  可否と、不達時に v1 scope として何を契約化するか。
- eval helper の production dispatch: `current_exe` + 内部 mode
  (`cheaper __grep-eval` 相当)を Phase 4 の CLI wiring で接続する前提。
  Phase 2 では helper 差替え option + test binary で動作検証を完結させる。
- npm `diff` ↔ `similar` の一致率は T2-3 で実測し、不一致なら jsdiff 直接
  port に切替える(実装手順に分岐条件として組込済み)。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: subagent-capable implementation(T2-2/T2-4 以降は task 単位で並行 dispatch 可)
- Independent work streams: T2-2・T2-4(T2-1 完了後に並行)、T2-5・T2-6(相互独立)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: Phase 1 `cheaper-core` 完了済み(cancellation reason・ResourceClaim
  等の Rust 型表現は Phase 1 の決定に追従)。core API が未確定の field 名を
  本 plan で新規発明していないか確認のこと。
- レビュー重点:
  - 計数写像(accounting の 4 分類)と scheduler の FIFO/cancel
    first-reason/late settle が TS 実装の意味を忠実に再現しているか。
  - process tool 固定ルール(`node` 明示 error・PID reuse 照合・minimal
    env・output cap)の抜け漏れ。
  - grep subprocess 隔離の protocol/kill/heap cap 設計と、`regress` 方言
    検証の起案手続きが守られているか。
  - write/edit/ls が Pi helper 由来の description 文言・出力 format・
    `prepareEditArguments` を含めて parity しているか。
  - Windows 分岐を先に仕様化せず写経した箇所が残っていないか。
