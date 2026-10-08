---
id: "IMPL-034CLj96KIHERKMfbd1Qe3S"
type: "impl"
status: "completed"
title: "Phase 2: port cheaper-tools"
created: "2026-10-07"
updated: "2026-10-07"
owners: []
relations:
  source: []
  changes:
    added:
      - type: "directory"
        path: "crates/cheaper-tools/"
      - type: "file"
        path: "docs/impl/ir/phase-2-port-cheaper-tools.md"
      - type: "file"
        path: "scripts/dump-tool-fixtures.mjs"
    modified:
      - type: "file"
        path: "Cargo.toml"
      - type: "file"
        path: "Cargo.lock"
      - type: "file"
        path: "crates/cheaper-core/src/lib.rs"
      - type: "file"
        path: "crates/cheaper-core/src/js.rs"
      - type: "file"
        path: "crates/cheaper-core/src/errors.rs"
      - type: "file"
        path: "crates/cheaper-core/src/token/count.rs"
      - type: "file"
        path: "crates/cheaper-core/src/worker/schema.rs"
      - type: "file"
        path: "crates/cheaper-core/src/workspace/paths.rs"
      - type: "file"
        path: "crates/cheaper-core/src/workspace/deny_paths.rs"
      - type: "file"
        path: "docs/tasks/phase-2-port-cheaper-tools.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/phase-2-port-cheaper-tools.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/phase-1-port-cheaper-core.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/phase-2-port-cheaper-tools.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Phase 2: port cheaper-tools

## Summary

plan の Task Breakdown T2-1(crate skeleton + 基盤小物)〜 T2-7
(runtime glue)を実施した。`crates/cheaper-tools` を `cheaper-core`
path 依存 + plan 承認済み依存のみで workspace に追加し、
`limits.rs`・`line_framer.rs`・`walker.rs`・`artifact.rs` の基盤、
`operation.rs`(abort/operation manager)・`scheduler.rs`・
`resources.rs`・`access.rs`・`accounting.rs`・`catalog.rs`・
`definition.rs`・`journal.rs`・`execution_metrics.rs` の runtime
層、canonical 8 tool(read/write/edit/ls/grep/find/count-tokens/
process)実装、そして `registry.rs`(TS `registry.ts` 相当の
`ToolRuntime`)・`invocation.rs`(TS `invocation.ts` 相当の
`InvocationToolRuntime`)まで unit/integration test 付きで移植した。
T2-8 として TS runtime 本体から生成した deterministic fixture 群
(`scripts/dump-tool-fixtures.mjs` → `crates/cheaper-tools/tests/
fixtures/*.json`、9 file / 107 case)を `tests/differential.rs` で
replay し、全 case が一致することを Windows 実機で確認した。

## Context

TASK-034BgKiZhENYXeNMRvxyFA(`docs/tasks/phase-2-port-cheaper-tools.md`)/
PLAN-034BepYuOf4htZRfdkZTYc(`docs/plans/phase-2-port-cheaper-tools.md`)
の T2-1 を実装。前提は Phase 1 の `cheaper-core`
(`resolveWorkspacePath`/`PathDenylist`/`SymlinkPolicy`/
`isInsideWorkspaceRoot`/`WorkspaceEscapeError` 等)。

## Implementation

### Changes

- `Cargo.toml`(root virtual manifest): `[workspace.dependencies]` に
  `tokio`(features: rt/process/fs/time/sync/io-util/macros の plan 最小
  集合)・`tokio-util`・`regress`・`unicode-normalization`・`similar`・
  `globset`・`fastrand`・`libc` を追加。`=` exact pin は rig-family
  規則が適用される crate のみで、本追加分は全て通常 semver。
- `crates/cheaper-tools/`(lib): `Cargo.toml`(`libc` は
  `cfg(unix)` target dep)・`src/lib.rs`(全 module 宣言 +
  `ToolError` 型)・`src/limits.rs`・`src/line_framer.rs`・
  `src/walker.rs`・`src/artifact.rs`・`src/access.rs`
  (`AccessObserver` のみ先行定義)・残 10 file + `tools/` 12 file の
  stub。`tempfile` は dev-dep へも追加せず、test の tmpdir は
  `std::env::temp_dir` + `fastrand` で生成(承認済み依存のみ使用)。
- `crates/cheaper-core/src/lib.rs`: `nodepath` を `pub mod` に変更。
  tool 層の `toDisplay` が `path.relative`/`path.sep` の win32 字句
  意味論(case-insensitive 比較含む)を必要とし、Node `path` 互換は
  core 責務の設計固定ルールに従う。
- `crates/cheaper-core/src/workspace/paths.rs`: `realpath` を `pub` に
  変更(walker の cycle 防止 canonical key・symlink target 解決)。
- `crates/cheaper-core/src/js.rs`: `pub(crate)` → `pub`。
  `locale_compare`/`code_unit_compare`/`js_trim`/`js_string` 等の
  JS-compat helper を cheaper-tools の後続 slice から再利用可能にする
  (表現ドリフト禁止: 再実装させない)。
- `crates/cheaper-core/src/worker/schema.rs`: `strict_schema_check`
  に `covers_type` を追加(integer ⊆ number の keyword 要求方向
  修正。詳細は Decisions)。
- `crates/cheaper-core/src/workspace/deny_paths.rs`:
  `PathDenylist` に `Clone` derive。
- `crates/cheaper-tools/` T2-2 〜 T2-7: `operation.rs`(AbortSignal/
  AbortController/AbortReason・operation 状態機械・
  `OperationManager`)・`scheduler.rs`(admission/pool/並行度)・
  `resources.rs`(lease/claim/parent 検証)・`access.rs`
  (`ResourceAccess`/workspace scope)・`accounting.rs`(`ToolBudget`/
  `ToolMeter`)・`catalog.rs`・`definition.rs`・`journal.rs`・
  `execution_metrics.rs`・`tools/`(read/write/edit/ls/grep/
  grep_eval/find/count_tokens/boundary/process/process_executable/
  process_tree)・`registry.rs`・`invocation.rs`。integration test は
  `tests/proc_runtime.rs`・`tests/proc_invocation.rs`・
  `tests/proc_process.rs`。

### Decisions During Implementation (Slice 1: T2-1)

- 定数の型: byte・行数・文字数の上限は buffer 長との比較に使うため
  `usize`、`_MS` の時間上限は `Duration::from_millis` へ渡す `u64`
  とした。値は `limits.ts` と同一(表現ドリフト防止)。
- `LineFramer` は `Buffer[]` 保持の代わりに grow する `Vec<u8>` 一本に
  畳んだ(TS は確定時に concat するだけで意味差なし)。decode は
  `String::from_utf8_lossy` = WHATWG UTF-8 U+FFFD 置換。callback は
  boxed `FnMut`(`Send` 付き)。
- `walkFiles` の async generator は **pull ベース `Iterator`**
  (`WalkFiles`)として再現した。`next()` が高々 1 dirent ぶんの FS
  処理を行い、呼出し側の early break が walk 全体を止める
  (grep/find の `for await` + `break` parity)。再帰 `walkDir` は
  明示 stack(`Vec<DirFrame>` + `HashSet` ancestors)へ機械展開し、
  `ReadDir` の遅延 entry yield で「incremental opendir reads」を
  保持した。`observe` は Rust 側で全て同期 FS のため callback も
  同期 `Fn`(`access::AccessObserver` type alias を access.rs stub に
  先行定義)。
- observe/列挙エラーの伝播: `Item = Result<WalkedFile, ToolError>` で、
  `Err` を 1 度返すと iterator が fuse する(generator の throw =
  列挙終了 parity)。`dir.read()` 失敗は ancestors を戻して
  `on_issue` 通知後に当該 dir を閉じる。
- FS アクセスは `cheaper-core` と同じ `std::fs` 同期実装(tokio
  依存は許可されるが walker に async I/O は不要)。`realpath` は
  core の `dunce::canonicalize` wrapper(verbatim prefix 除去 =
  Node `fs.realpath` 相当)を公開して再利用。
- `ToolError = Box<dyn Error + Send + Sync>` を crate 全体の throw
  表現として `lib.rs` に定義。`rethrow_as_denied` は
  `WorkspaceEscapeError` downcast → `DeniedError::access()` に写像、
  それ以外は passthrough(TS の rethrow 相当)。`DeniedError` は deny
  3 定数専用の plain error で、`is_access_denied_error` は TS 同様に
  message 一致(`DENIED_MESSAGES` 集合)で判定する
  (rejectedToolCalls 分類の contract)。
- `Dirent.name` は `OsString::to_string_lossy`(Node の UTF-8 lossy
  decode 相当)。`entry.file_type()` の失敗経路は Node Dirent に
  存在しないため `unreadable` issue + skip に写した。
- artifact port は TS の sync interface をそのまま同期 trait 化し、
  失敗可能経路は `Result<_, ToolError>`(TS throw 相当)。
  `ArtifactData` は `string | Uint8Array` の enum、`ReadSource` は
  `#[serde(tag="kind")]` の discriminated union、
  `ArtifactMetadata` は `Omit<…,"preview"> & {preview?}` の key 順を
  struct field 順で保持した。

### Decisions During Implementation (runtime glue: T2-7)

- `CanonicalToolDefinition.execute` は boxed future を返す型にした
  (`Pin<Box<dyn Future<Output = Result<CanonicalToolResult, ToolError>>
  + Send>>`)。sync tool は `Arc<T>` を capture した async block で
  adapt する(`process` だけが真に async)。`prepare_arguments` も
  `Arc` で保持し、public/external definition が同じ hook を借用
  なしで共有できるようにした。
- `AbortSignal::any` の composite は `SignalInner.sources:
  Vec<AbortSignal>` に source を保持する。weak listener 登録だけでは
  caller が source handle を drop した時点で伝播経路が消えるため
  (call signal と root signal の合成が落ちる実障害を修正)。keep は
  read されない liveness のみが目的で、`Debug` で `sources.len()`
  を出して warning を抑制した。
- active operation の binding は `tokio::task_local!` で
  `AsyncLocalStorage` を近似し、`observeAccess` は実行前後の両方で
  operation が `Running` のままかを検証する(TS と同じ 2 点検査)。
- managed process は `AtomicUsize` + `Notify` で追跡し、drain が
  count 0 を待つ。TS の `Set<Promise>` + `Promise.allSettled`
  相当。遅延 settle(late settlement)は許容し、bounded dispose 後も
  `finished_at` が記録される挙動を test で固定した。
- `registry` の tool 登録は runtime option の `names` 順を保持し、
  lookup は canonical `ToolName` 経由。`process` だけ
  `process_operation_declaration`、他は mutation 有無で
  shared/exclusive access を宣言する。
- `invocation` は `OperationManager`/`ToolScheduler`/`ResourceManager`
  を全 owner で共有し、owner 重複・空 owner・root/symlink/
  process policy 不一致・偽造 parent lease を worker scope 生成時に
  reject。1 owner の dispose は sibling に影響しない
  (owner-local operation 追跡)。
- `cheaper-core` `worker::schema::strict_schema_check` の keyword
  type 要求で `type:"integer"` が `number` 要求(`minimum` 等)を
  満たさず canonical schema(read/grep/find)が compile 失敗して
  いた。AJV は integer を number の subtype として扱うため、
  context-narrowing 用 `includes_type` とは別方向の `covers_type`
  を追加して修正(所有 boundary = cheaper-core で修正)。
- `PathDenylist` に `Clone` を derive(boundary operations が
  caller の denylist と `ResourceAccess` 構築の双方で同一 list を
  必要とする)。

### Decisions During Implementation (differential fixtures: T2-8)

- `scripts/dump-tool-fixtures.mjs` は built `dist` の
  `createToolRuntime` + `runtime-pi` の `createPiToolHandlers` を実際に
  駆動し、case ごとに deterministic な tmp workspace を構築して
  `expected`(normalized result/error)と `state`(budget/meter/journal/
  operations/accesses)を `<ROOT>`/`<OUT>`/`<BASE>` token で scrub して
  書き出す。`errno` 除去・schema error は sorted unique `instancePath`
  のみ保持・時刻/pid 排除により再生成は byte 同一(SHA-256 で検証済)。
- `tests/differential.rs` は fixture を再構築して replay する。
  `expectRust` は設計固定ルール由来の divergence を pin する sparse
  override(`ops` は op index → entry 置換、`state` は deep-merge)。
  現在の pin は 5 件: bare `node` task(POLICY_VIOLATION — Rust は
  Node host executable を持たない設計固定ルール)、`count_tokens` の
  tokenizer identity(js-tiktoken → tiktoken-rs、token 数は同一)、
  schema validator の union-branch `instancePath` 集合差(ajv →
  jsonschema、2 case)、grep invalid pattern の error wording(V8 →
  regress、semantic は同一)。
- process fixture の `node -e` script は `src/bin/diff_fixture_child`
  (bin target)へ翻訳する。libtest re-exec は `running N tests`
  banner が stdout を汚染して byte 一致の `details.stdout` 比較に
  使えないため test binary ではなく bin を採った(`env!(
  "CARGO_BIN_EXE_diff_fixture_child")` で参照)。
- `runParallel` replay は member 順の noop-waker poll で TS の
  `members.map(execute)` 同期 prefix(admission/operation 登録順)を
  再現し、その後に `tokio::spawn` で実 waker を登録する。先に spawn
  せず `then` effect(drain 等)を走らせると、in-flight op が noop
  waker から起きず drain が settle を永遠に待つデッドロックになる
  (TS では promise が event loop で独立に進行する)。
- fs error の observable parity: `cheaper_core::errors::NodeIoError`
  を新設し、caller へ伝播する fs 失敗を `{code, message:
  "{code}: {uv_strerror}, {syscall} '{path}'"}`(Node `UVException`
  shape)で wrap する。raw `io::Error` は locale 依存 message を持ち
  path/syscall/`code` を保持しないため、`describeError` parity
  (op-record `error:{name,message,code}`)に到達できなかった。
  対象: `count_file_tokens`(stat/open)、`boundary.rs`(stat/scandir/
  mkdir/open/rename/access)、`read.rs`(stat/open/read)、
  `process.rs`(cwd stat)。`describe_error_object` は source chain
  を走査して `NodeIoError` の `code` を拾う(`CountTokensError::Io`
  のような enum wrap 経路を通すため)。
- `edit.rs` の `error_code_string` は `NodeIoError.code()` を優先する
  (`access` 失敗 → `Error code: ENOENT.` の文言が raw io::Error への
  kind 写像ではなく Node `error.code` に揃う)。

### Decisions During Implementation (review fixes: glob matcher)

- `glob.rs` に `path.posix.matchesGlob`(内部は minimatch v10.2.5、
  `{nonegate, nocomment, windowsPathsNoEscape, nocase: platform}`)の
  UTF-16 忠実 port を新設し、`find.rs` の自前 matcher と `grep.rs`
  の `globset` filter を同じ `glob::matches_glob` に統一した。
  `globset` は workspace 依存から除去。
- 実装は regex 生成ではなく、minimatch の AST(parse → `#flatten`
  (adopt/adoptWithSpace/usurp)→ `#fillNegs`)→ part 列 token 化 →
  memo 化 DP という構造を直接写した。`matchOne`/`#matchGlobstar`/
  file 側 `levelTwoFileOptimize`(`.`/空 segment 除去・`..` 解決)も
  再現。UTF-16 code unit で評価し、POSIX named class(`[[:name:]]`)
  が必要とする Unicode 範囲は `glob_tables.rs`(Node 自身の
  `/[\p{...}]/u` を全 code point で舐めて生成、`gen-glob-tables.cjs`
  が codegen)に持つ。
- `*`/`?` 反復と extglob は continuation frame arena + memo で実装。
  `!` extglob は `#fillNegs` を AST 上で忠実に再現(`!` 以降の part を
  祖先 partlist まで遡って各 alternative の末尾へ copyIn)。これに
  より `!(a|b)*` の `*` が alternative 内で `noEmpty` 扱い
  (`a[^/]+?`)になる等、単純な動的 continuation では出せない
  挙動も一致する。alternative の `isStart` は JS の
  `#parentIndex`(全 alt が `|` 検出時生成のため全て 0)を反映して
  「親 ext が isStart なら全 alt が isStart」。
- 差分検証: `src/bin/glob_probe.rs` + `probe-glob.mjs` で
  `path.posix.matchesGlob` と約 29 万 pair(系統 corpus + seeded fuzz)
  を比較し bool 一致 0 mismatch。differential fixture として
  `glob.json`(`{p, g, expected}` 172 case)を dump script に追加。
- Accepted divergence: minimatch が不正な JS regex を生成する
  pattern(例: negated class 内 `\p{...}` — ` [![:alpha:]]`)で Node
  は `SyntaxError` を投げる。Rust API は例外経路を持たないため
  `matches_glob` は bool を返す。fixture では `expected: null` として
  replay 側で skip する。

### Decisions During Implementation (review fixes: abort / UTF-16 / fs access)

- `AbortReason::aborted()` を追加し、`controller.abort()` の reasonless
  呼出し相当の全経路(registry/invocation の `abort(None)`、
  `drain()` 内の暗黙 abort、`dispose_inner`)を DOMException
  `AbortError`/"This operation was aborted" へ揃えた。
  `cancelAll`/`cancelOwner`/`cancel` の既定 `Error("Operation
  cancelled")` は `AbortReason::cancelled()` のまま(TS と同じ
  2 系統の既定値を維持)。
- abort listener の registration/drain race を修正:
  `SignalInner.listeners` を `{map, drained}` にし insert と drain を
  同一 mutex で serialize。dispatch 済みへの `on_abort` は JS と同じく
  二度と呼ばれないため listener は map に入れず即 drop する
  (leak 防止)。bounded race 回帰テスト(200 iteration × 50 listener)
  を `operation.rs` に追加。
- `src/js16.rs` に UTF-16 code unit helper(`utf16_len`/`slice_utf16`)
  を集約し、grep の private copy を移動。`find.rs` の skip-path
  truncation は `chars().count()`(Unicode scalar)から UTF-16 unit
  計測へ修正し、`edit.rs` `count_occurrences` の空 needle 経路
  (`split("").length - 1`)も UTF-16 unit へ修正(astral char で
  error message の occurrence 数が変わる)。Accepted divergence:
  `slice(0, n)` が surrogate pair の途中に切れる場合、JS は lone
  surrogate を残すが Rust `String` は表現不能のため手前で切る
  (backoff)。fixture では boundary 一致の unit 数のみを検証する。
- `process.rs` heartbeat の `elapsedMs` は TS `Math.round(performance
  .now() - startedAt)` 相当の最近接丸めへ(切り捨て `as_millis()` を
  修正)。
- `boundary.rs` `check_read_write`(`ops.access` = Node `fs.access(path,
  R_OK|W_OK)`): Unix は `libc::access(2)` を直接呼び real uid/gid・
  `EROFS`・dir への R_OK|W_OK 通過まで一致させた。Windows は
  `access(2)` 相当 syscall が無いため r/w open 近似を維持するが、
  readonly attribute 由来の失敗は Node(libuv)が返す `EPERM` を
  `node_io_error_with_code`(cheaper-core に code 指定 ctor を追加)
  で再現。残る Windows 近似差異: real-uid 評価・ACL 細粒度
  (write-only file への R_OK 拒否等)。
- fixture 追加: `edit.json` read-only file(EPERM)+ astral 空 needle
  occurrence 数、`grep.json` UTF-16 truncation(line + skip path)、
  `runtime.json` reasonless `abort-runtime`/`drain`(AbortError)。
  dump script の `files` spec に `{content, mode}`(chmodSync)を追加し、
  replay 側も同形状を実装(Windows は write bit 皆無を readonly
  属性へ写像、`TempBase` drop で解除してから削除)。

### Decisions During Implementation (parity fixes: V-A〜V-J)

実機 probe(Node v26.10.0 / Windows)で検証した 10 件の乖離を修正した。

- **V-A(pattern 長上限)**: `glob::matches_glob` を `Result<bool,
  ToolError>` に変更し、pattern の UTF-16 unit 数が 65,536 を超えると
  `PatternTooLongError`(`"pattern is too long"`、differential では
  `TypeError` に normalize)を返す。length check は minimatch の
  `assertValidPattern` と同じく **pattern** のみに適用し、長い path は
  制限しない。`find`/`grep` は file 単位の遅延評価を維持するため、
  空の tree では throw しない(lazy)。fixture: `find.json`
  "overlong pattern throws through execute" / "overlong pattern is
  lazy on empty tree"、`grep.json` 同型 2 case。
- **V-B(brace sentinel)**: brace-expansion の escape sentinel を PUA
  (U+E000/U+E001)から Unicode scalar 範囲外の `u32` 値
  (`0x110000`/`0x110001`)へ変更。入力中の literal PUA char が sentinel
  と衝突して brace 構造と誤認される経路を閉じた。fixture:
  `glob.json` の U+E000/U+E001 literal case。
- **V-C(数値 brace range)**: `{a..b}` の端点・step を `f64`(JS
  number 意味論)で扱う。parse は `str::parse::<f64>`(correctly
  rounded = `parseInt` 相当 — 逐次 `a*10+d` fold は末尾 bit が
  ずれ得る)、生成値の stringify は `format_number`
  (`String(i)` 相当、`2^63` → `"9223372036854776000"`、`1e21` 超は
  exponential)。i64 overflow panic を排除し巨大 range も
  `MAX_EXPAND` cap で打ち切る。fixture: `glob.json` の
  `{9223372036854775808..}` / `{1000000000000000000000..}` 系 case。
- **V-D(abort listener race)**: `request` で queue 挿入後の abort
  listener 登録と同一 lock 内で signal を再検査し、pre-check →
  登録の隙間に abort が落ちても waiter が孤立しないようにした。
  test: `abort_landing_before_listener_registration_still_rejects`。
- **V-E/V-F(callback を `Inner` mutex 外で実行)**: user callback
  (`respond`/`start`/`can_start`)を `VecDeque<Pending>` に defer し、
  `drain_pending` が lock を離して実行する。再入 `release`/`notify`/
  `request` は deadlock しない。`while let` scrutinee の
  `MutexGuard` temporary が loop body 中も生存する pre-2024 edition
  の落とし穴は、pop を別 statement に分けて解消(再帰 drain で
  `Respond` が後続 `CanStart` 評価より先に処理される順序も維持)。
  `can_start` panic は `catch_unwind` で捕捉し verdict を lock 内で
  再検証した上で `resume_unwind` する。mutex poison 対策として
  `lock_inner` は poison を recover して継続する。test:
  `start_and_can_start_callbacks_may_reenter_manager`、
  `start_panic_rejects_and_keeps_manager_healthy`。
- **V-G(`wait_idle` lost wakeup)**: `tokio::sync::Notify` の
  `notify_waiters` は登録前通知を失うため、`Notified` future を
  `enable()` で先に登録してから active count を確認する(tokio
  1.53+ API)。
- **V-H(`ready()` rejection memoize)**: TS の `readyPromise`
  memoization に倣い `OnceCell<Result<ResourceLease, AbortReason>>`
  に Result 自体を格納。一度 reject された `ready()` は retry せず
  同じ rejection を返す。`ready_requested: AtomicBool` で sync 呼出し
  検出(`readiness !== undefined` parity)を維持。
- **V-I(owner 登録の atomicity)**: `create_worker_scope` の重複
  owner 判定は早期 fast path(`contains`)に加え、runtime 構築成功の
  commit 時点で `HashSet::insert` の戻り値を authoritative 判定に
  する — check-then-insert の TOCTOU 窓で同時生成された同じ owner を
  reject する。構築失敗時は owner を消費・登録しない。
- **V-J(spawn error shape)**: `Command::spawn` の raw OS error を
  libuv `uv_translate_sys_error` 名へ写す `spawn_code` を追加
  (unix: libc errno 表、win32: `2|3|161→ENOENT`、`5|32|82→EACCES`、
  `193→EFTYPE`、unmapped→`"UNKNOWN"` — 全て Node 実測値)。
  message shape は `spawn_failure_is_async` で platform 別に選択:
  unix は exec 失敗が error pipe 経由で常に非同期 `error` event のため
  raw errno を持つ全失敗を `"managed process failed" + spawnCode`
  (cause 無し)、win32 は `CreateProcessW` 失敗が sync throw だが
  ENOENT だけ Node が非同期扱いする実測に倣い ENOENT のみ failed
  shape、その他は `"managed process spawn failed" + spawnCode +
  cause`。raw errno を持たない構築 error も spawn failed 経路。
  **残留差分**: (1) musl 環境では Node が ENOEXEC を sync throw
  し得る(musl `execvp` は glibc と異なり ENOEXEC shell retry を
  しない/glibc は error pipe で非同期化)ため shape が逆転し得る。
  (2) unix で表外の errno は symbolic name が取れず `"SPAWN_ERROR"`
  で近似(libuv は `-errno` を既知名に写す)。(3) Windows の
  `ERROR_BAD_EXE_FORMAT` は実測では `EFTYPE` に写る(libuv table)。
  test: `spawn_code_maps_raw_errno_to_libuv_names`、
  `spawn_failure_async_shape_follows_node`、
  `spawn_errno_failure_reports_libuv_style_code`(win32 execute-level)。

### Plan ambiguities / notes for later slices

- plan の依存許可リストに `dunce`/`tempfile` が無い。walker の
  canonicalize は core `realpath`(dunce wrapper)を pub 化して再利用
  した。test tmpdir は `fastrand` で代替。
- `js.rs` の helper は `pub(crate)` だったため `pub` に変更した
  (edit の fuzzy・ls の sort・process の表示 fold 等が必要とする)。
- `AccessObserver` は `walker` が型を必要とするため `access.rs` stub に
  先行定義した(T2-3 で残りを埋める)。sync `Fn` 型で `FnMut` ではない
  — access 層が内部可変(Mutex 等)を持つ前提。
- `sniff_text_file` は TS の単発 `fh.read` を単発 `File::read` で再現
  (partial read もそのまま判定)。

## Related Experiments

### Adopted

<!-- slice 1 では experiment log 無し -->

### Rejected

<!-- Record rejected experiment logs and why they were rejected. -->

## Validation

- `cargo test -p cheaper-tools` — 286 passed / 0 failed
  (lib 249 + differential 11 + proc_invocation 10 + proc_process 9 +
  proc_runtime 7 — proc_* は fixture child 経由のため実行時間が長い)。
  differential は fixture 10 file(read 18/grep 22/find 12/ls 9/write 9/
  edit 15/count_tokens 4/process 9/runtime 14 + glob 172 pair)を全て
  replay(Windows 実機)。
- `cargo test -p cheaper-core` — 179 + 22 passed / 0 failed
  (`node_io_error_with_code` 追加の回帰確認)。
- `cargo clippy -p cheaper-tools --all-targets -- -D warnings` — green。
- `cargo clippy -p cheaper-core --all-targets -- -D warnings` — green。
- `cargo fmt --check` — green。
- `node scripts/dump-tool-fixtures.mjs` — fixture 10 file 再生成。
- glob matcher 差分 probe(`probe-glob.mjs` + `src/bin/glob_probe.rs`):
  seeded fuzz 3 seed × 約 9.7 万 pair で `path.posix.matchesGlob` と
  bool 完全一致(0 mismatch、Node SyntaxError case は別集計)。
- `pnpm -C packages/tool-runtime test` — 172 passed / 1 skipped
  (pre-existing skip)/ 0 failed。
- `pnpm -C packages/runtime-pi test` — 387 passed / 0 failed。

## Risks

differential fixture・実機 probe・review cycle で confirm 済みの残存
divergence(全て IR 本体・code comment・plan に記録済みの受容残差):

- regress 由来の regex 方言差(JS `new RegExp` 非 unicode vs
  `regress::Regex`): `\u{...}` は JS Annex B では identity escape
  `u` + `{...}`(quantifier/literal class)として解釈されるが regress
  は u flag 無しでも Unicode escape として扱う。`i` flag の simple
  folding も JS が折らない I↔ı(dotless i)・s↔ſ(long s)等を regress
  icase は fold する(grep_eval.rs の dialect probe 内
  `DIVERGENCE(known, regress)` comment に固定)。
- UTF-16 unit 系の残差: `slice(0, n)` が surrogate pair の途中に
  切れる入力では JS は lone surrogate を残すが Rust `String` は
  表現不能のため手前までで切る(backoff)。fixture では boundary
  一致の unit 数のみ検証。
- minimatch が不正な JS regex を生成する pattern(negated class 内の
  `\p{...}` 等)は Node が `SyntaxError` を投げるが、Rust API は例外
  経路を持たないため `matches_glob` は bool を返す。fixture では
  `expected: null` で skip。
- spawn error shape: musl 環境では Node が ENOEXEC を sync throw
  し得る(glibc は error pipe で非同期化されるが musl `execvp` は
  ENOEXEC shell retry をしない)ため shape が逆転し得る。unix の
  表外 errno は symbolic name が取れず `"SPAWN_ERROR"` 近似、
  Windows `ERROR_BAD_EXE_FORMAT` は実測 `EFTYPE`(libuv table)。
- `fs.access` 相当の Windows 近似差異: real-uid 評価・ACL 細粒度
  (write-only file への R_OK 拒否等)は r/w open 近似のまま残る。
- grep eval helper の heap cap(TS `resourceLimits.
  maxOldGenerationSizeMb`、`GREP_WORKER_MAX_OLD_SPACE_MB`=64)は v1
  **未実装**(Job Object `ProcessMemoryLimit` / `setrlimit` が候補、
  実現可否は plan Risk #7 の open question)。v1 は byte cap +
  timeout kill のみが契約。
- differential pin 5 件: bare `node` task の `POLICY_VIOLATION`
  (Rust runtime は Node host executable を持たない設計固定ルール)、
  `count_tokens` の tokenizer identity(js-tiktoken → tiktoken-rs、
  token 数は同一)、schema validator の union-branch `instancePath`
  集合差(ajv → jsonschema、2 case)、grep invalid pattern の error
  wording(V8 → regress、semantic は同一)。

## Follow-ups

記録済み residual を除く残作業は無い。heap cap の実現可否は plan
Risk #7 の open question として残るが v1 契約外(上記 Risks 参照)。
`cheaper __grep-eval` 内部 mode の production 起動配線は plan 通り
Phase 4 の CLI wiring が担う。
