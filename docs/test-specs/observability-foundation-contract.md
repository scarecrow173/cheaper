---
id: "TSPEC-034DWcP8v4SUuuui8hvhnT"
type: "test-spec"
status: "approved"
title: "Observability foundation contract"
created: "2026-10-11"
updated: "2026-10-12"
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
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/observability-foundation-logging-and-instrumentation-wiring.md"
    - "docs/plans/implement-observability-foundation.md"
    - "docs/tasks/observability-contract-tests-and-verification.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies:
    - "docs/specs/observability-foundation-scope.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Observability foundation contract

## Purpose

observability foundation(dual-sink logging + span による stage
timing + flag/env activation)が
SPEC-034DUFyMl47o924O1KW8Ya の acceptance criteria と
DESIGN-034DV9ypzmxdAobGSJ6Cm6 が確定した出力・activation
contract を満たすことを固定する意図を記録する。

本契約の核心は 2 つの不変条件の両立である: (1) stdout は
envelope 専用のままで、logging 有効化が stdout に 1 byte も追加
しないこと。(2) 既定 OFF で ~zero overhead を保ちつつ、明示的
有効化では file(`<workspace>/.cheaper/logs/`)と stderr の dual
sink に instrumentation event が届くこと。hook 経路では stderr
channel が target native contract(windsurf deny guidance 等)の
一部であるため、stderr sink を構造的に無効化する非対称も本
contract の一部である。

observability の output contract(sink・format・activation 規則)
が変更されるか、logging 基盤自体が置き換えられた場合にのみ退役
する。metadata-only redaction 規約は raw payload capture の
opt-in 機構が導入されるまで唯一の挙動であり、その導入時に本
spec は改訂される。

## Feature

`cheaper-observability` crate の subscriber init・sink 構築と、
`--log`/`--log-format` flag・`CHEAPER_LOG`/`CHEAPER_LOG_FORMAT`
env による activation、および全 crate の `tracing` span/event
instrumentation による run pipeline stage timing と内部状態の
観測可能性。`cheaper` binary を tempdir cwd + 注入 env で spawn
する subprocess 経路(`CARGO_BIN_EXE_cheaper` + `env_clear`)を
file sink assertion の正とする。

## Rules

- Rule: stdout envelope exclusivity — logging 有効化(flag/env
  いずれの経路でも)は stdout に 0 byte の追加を生じさせない。
  envelope は「envelope 1 JSON + 末尾改行 1 つのみ」・
  `schemaVersion: 1` の byte-parity を維持し、fmt layer の
  writer に `std::io::stdout` を渡す経路を作らない
  (hard invariant)。
- Rule: activation matrix — 既定は OFF(subscriber 未 install・
  `.cheaper/logs/` 未作成)。`CHEAPER_LOG` env・`--log <level>`
  flag のいずれかが level を与えると有効化し、優先順位は
  flag > env で level・format の双方に適用される。format は
  `CHEAPER_LOG_FORMAT`/`--log-format` の `text|json`、未指定は
  text。不正な level/format 値は fail-open — warnings に積み、
  level は OFF・format は text 扱いとし、command を logging
  都合で失敗させない。init 警告は非 hook 経路で `io.stderr` の
  `warning:` 行として出る(`--quiet` 時は抑制)。
- Rule: dual sink — 有効化された command は同一 event を
  `<log_root>/.cheaper/logs/cheaper.log.*` と stderr
  (`std::io::stderr` 直書き・`with_ansi(false)`・1 event = 1 行)
  の両 sink に出力する。hook 経路は `stderr_enabled=false` で
  stderr sink を構造的に無効化し file sink のみを有効とする —
  tracing 行・init `warning:` 行は hook の stderr に出さず、
  init 警告は install 済み subscriber へ `tracing::warn!` で
  畳んで file sink にのみ残す(subscriber 未 install 時は
  drop — fail-open)。
- Rule: format contract — text format は両 sink で
  human-readable な行(JSON parse 不能な形式)を出力し、json
  format は file・stderr の各行が機械 parse 可能な JSON object
  である。format は両 sink 共通(sink 別 format は要件外)。
- Rule: stage timing — 両 `fmt::Layer` の
  `with_span_events(FmtSpan::CLOSE)` により、`cheaper.*` span の
  close event が `time.busy`/`time.idle` を field に持つ。stage
  分解は span tree(`cheaper.run` 親 + stage 子 span)から得られ、
  site 毎の `Instant` + `elapsed_ms` 二重実装は置かない。
- Rule: coverage — cli dispatch(`cheaper.cli.dispatch`・
  `cheaper.cli.command`)、run pipeline stages(`cheaper.run` +
  stage 子 span)、hook(`cheaper.hook`)、tool internals
  (`cheaper.tool` span + `tool.name` field・
  `cheaper.tool.process` event)、auth/catalog I/O boundary
  (`cheaper.auth`・`cheaper.auth.store`・`cheaper.catalog`)が
  log 有効時にそれぞれ観測可能な event を file sink に出力する。
  workflow(`cheaper.workflow`)・adapter
  (`cheaper.adapter.session`)・devin provider
  (`cheaper.provider.devin`)・core の resolve/load event も
  instrumentation 配置表の一部。
- Rule: secrets safety — credential(api_key/oauth token/JWT)・
  `system_prompt`/`prompt`・tool input/output 本体は field 値に
  しない。記録可能なのは metadata-only allowlist(`*.len`・
  `*.count`/`turns`/`tool_calls`・`*.id`・`name`/`code`/`tier`
  等の分類値)のみ。file path は allowlist 外(OS username を
  含み得る識別情報)で、file 参照は basename の `*.name`
  (例: `file.name = "credentials.json"`)に留める。file sink・
  stderr 双方から credential 値・prompt 本文・tool payload が
  grep 0 件である scan assertion で固定する。
- Rule: `--quiet` rule — `--quiet` は `io.stderr` diagnostics
  (error line・`model:` line・`usage-log:` line・`warning:` line)
  と init 警告を抑制するが、flag/env var による明示的な log
  有効化は抑制しない(確定済みルール)。`--quiet` + log 有効化の
  組合せで stderr に diagnostics が出ず、file sink に log event
  が出る。
- Rule: retention — file roller は `Rotation::DAILY`・
  `filename_prefix("cheaper.log")`・`max_log_files(7)`(直近
  7 file ≒ 7 日分)。prefix 固定は retention の削除対象判定が
  prefix 一致を要求するため。「m 個残したいなら m+1」
  semantics は実測で確認し結果を記録する。
- Rule: repeat-init — `set_global_default` は process 当たり一度
  しか成功しない。install 済みでの `init` 再呼出しは first
  subscriber wins で `(None, warnings + "subscriber already
  installed")` を返して継続する(fail-open — 後から init を
  呼んだ経路を error 化しない)。初回が OFF 解決(install 無し)
  なら後続呼出しは install 可能。
- Rule: log root anchoring と fail-open — `.cheaper/logs/` は
  workspace root に anchor される。`--cwd` が command 意味を持つ
  経路(run/run alias/workflow/models/count-tokens)でのみ argv
  `--cwd` を lenient scan して使い、それ以外(usage・auth 等)
  では `io.cwd` を使う。dir 作成/書込み不可時は fail-open で
  stderr-only + warning に継続し、command は失敗しない。
- Rule: off-state overhead — subscriber 未 install 時の emit は
  `tracing` の no-subscriber fast path で数 ns に収まり、
  benchmark scenario(tool 実行を含む run pipeline 経由)の
  median wall-clock で (a) instrumentation 導入前 HEAD vs
  (b) 導入後 `CHEAPER_LOG` 未設定の delta が許容範囲(目安:
  median ±3% 未満)であることを記録する。(c) `CHEAPER_LOG=debug`
  arm の計測には `benchmarkEnvironment` の `CHEAPER_LOG*` env
  passthrough と per-arm env/flag 注入受け口の harness 拡張が
  前提となる。
- Rule: subprocess isolation — file sink を assert する test は
  全て `CARGO_BIN_EXE_cheaper` + `env_clear` spawn の
  subprocess isolation とする(`set_global_default` の process
  当たり一度制約により、同一 test binary 内の複数 init は
  repeat-init contract に乗り test 毎の独立 subscriber を取れ
  ない)。in-process の `run_cli` test は repeat-init の
  warning/exit code と OFF 経路の確認に限定する。

## Examples

- Example: `CHEAPER_LOG`/`--log` 未設定の subprocess(既定 OFF)
  で `cheaper` を spawn すると `<ws>/.cheaper/logs/` が作成され
  ず、stdout envelope・exit code が導入前と byte-parity —
  `tests/envelope.rs`・`tests/regression/` 55 case が無変更
  pass することがそのまま証拠。
- Example: tempdir cwd + `env_clear` + `CHEAPER_LOG=debug` +
  `CHEAPER_CONFIG_DIR`=tempdir で `CARGO_BIN_EXE_cheaper` を
  spawn した command が `<ws>/.cheaper/logs/cheaper.log.*` を
  生成し、同一 event が stderr にも出る(dual sink)。
- Example: `CHEAPER_LOG=warn` env + `--log debug` argv の spawn
  で file に debug level event が出る(flag > env の level
  override)。`CHEAPER_LOG_FORMAT=json` env + `--log-format text`
  argv で file 行が JSON ではない text 形式になる(flag > env
  の format override)。
- Example: `CHEAPER_LOG_FORMAT=json` の spawn で file の全行が
  `serde_json` parse 可能。`--log-format text`(または format
  未指定)で JSON ではない human-readable 行が file・stderr
  双方に出る。
- Example: dev-seam binary(`required-features = ["dev-seam"]`
  target)に `CHEAPER_TEST_FAKE_STREAM=<fixture>` +
  `CHEAPER_LOG=debug` で tool call を含む run を spawn すると、
  file に `cheaper.run` span tree の close event(`time.busy`)
  と stage span 名・`cheaper.tool`/`cheaper.tool.process` event
  が出る。同 log file と stderr から credential 値・prompt 本文・
  tool payload 本体を grep して 0 件(secrets scan assertion)。
- Example: `--quiet` + `CHEAPER_LOG` env の spawn で stderr に
  error/model/usage-log/warning diagnostics 行が出ず、file に
  log event が出る。
- Example: hook invocation を `CHEAPER_LOG` 有効 + invalid
  `CHEAPER_LOG_FORMAT` 値で spawn しても、stderr に tracing
  行・`warning:` init 警告が出ず(親 agent との stdin/stdout
  protocol・target native stderr を汚さない)、file に
  `cheaper.hook` event が出る。
- Example: `CHEAPER_LOG` 有効で `cheaper auth status`・
  `cheaper models` を spawn すると file にそれぞれ
  `cheaper.auth`・`cheaper.catalog` event 行が出る
  (run pipeline 以外の計測点)。
- Example: in-process で `run_cli` を 2 回呼ぶと 2 回目は
  `warning:` 行(subscriber already installed)を出しつつ正常
  exit code で継続する — file 内容は assert せず、subscriber
  は 1 回目の activation 設定のまま(first subscriber wins)。
- Example: `.cheaper/logs/` を作成できない anchor(read-only
  workspace 等)で `CHEAPER_LOG` 有効な非 hook command を
  spawn すると、stderr-only で継続し `warning:` 行に理由が出て
  command は失敗しない。
- Example: `node benchmarks/run.mjs --scenario
  single-simple-tool-call` の median wall-clock を
  (a) instrumentation 導入前 HEAD・(b) 導入後 `CHEAPER_LOG`
  未設定・(c) 導入後 `CHEAPER_LOG=debug`(harness env
  passthrough + per-arm 注入受け口経由)の 3 点で比較し、
  (a)→(b) delta が ±3% 未満の結果を `benchmarks/results/` の
  JSON として記録する。

## Guarantees

- envelope byte-parity・exit code・usage.jsonl・hook contract・
  `--quiet` semantics・`panic = "unwind"`・panic hook 経路は
  log ON/OFF いずれでも不変 — OFF 既定で既存 suite が無変更
  pass することがそのまま証拠となる。
- `.cheaper/logs/` は root `.gitignore` の bare `logs` pattern
  で cover される(design で確認済み)— consumer の workspace
  が log file で汚染されない。
- file sink assertion は subprocess 分離で実装され、global
  subscriber の process 当たり一度制約による flake が構造的に
  排除される。dev-seam fixture 経路は
  `required-features = ["dev-seam"]` target に分離され、
  non-seam test は feature 有無で gate されない。
- hook 経路は stderr sink 無効 + init 警告不混入で、target
  native stderr contract への干渉経路を作らない。
- `cheaper-observability` は workspace 内 crate へ依存せず
  (everything → cheaper-observability → tracing-subscriber の
  一方向)、`cheaper-core` は `tracing` emit のみで tokio/rig を
  引き込まない(`cargo tree` で確認可能)。

## Non-goals

- 最適化の実施・CPU sampling / flamegraph・memory profiling —
  本 spec は stage timing の証拠収集基盤のみを対象とし、
  measured delta の改善自体は検証しない。
- raw payload capture の opt-in 機構(spec で deferred)—
  metadata-only が唯一の挙動であり、本 spec が verify するのは
  「payload を記録しないこと」の方向。
- OTLP 等の外部 backend への送出・sink 別 format・
  per-startup rotation・hook 経路 file sink の opt-out。
- instrumentation の個別 field 値の業務的正しさ — span 名・
  canonical field naming 規約の遵守と観測可能性は contract
  test の assert 範囲だが、各 subsystem の意味的正しさは
  個別 instrumentation task の検証領域。
- log 有効時の性能 upper bound — (c) arm は観測・記録のみで
  閾値 gate は設けない。
- `__grep-eval` 内部 subprocess の subscriber install —
  `run_cli` に到達しない経路で tool 側 emit は no-op のまま。
- 実行結果の記録 — pass/fail・計測値は impl-doc experiment
  log・`benchmarks/results/`・CI evidence の領域。

## Risk

- stdout exclusivity が破れると envelope consumer(親 agent の
  tool call protocol)が silent に壊れ、envelope byte-parity を
  検証する術自体が logging に汚染される。
- activation matrix・precedence がずれると「`--log` を付けた
  のに出ない」「env で level を変えたのに反映されない」調査
  不能状態になり、証拠収集基盤の信頼性そのものが失われる。
- secrets scan が欠けると credential・prompt・tool payload が
  file sink に永続化され、workspace artifact 経由の流出経路が
  regression として復活する。
- subprocess isolation を外すと test が global subscriber 制約
  で順序依存 flake になり、repeat-init 挙動と本物の file sink
  assert が混ざって判別不能になる。
- retention・prefix がずれると `.cheaper/logs/` が無制限に肥大
  し、consumer workspace を汚染する。
- hook 経路で stderr sink が有効だと tracing 行が target
  native stderr(windsurf deny guidance)に混入し、hook
  contract が壊れる。
- off-state overhead の計測経路が無いと「既定 OFF で ~zero
  overhead」の主張が未検証のまま残り、将来の instrumentation
  追加による劣化を検出できない。
