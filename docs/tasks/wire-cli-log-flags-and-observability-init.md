---
id: "TASK-034DVH7HJKxbc1zEF7638b"
type: "task"
status: "done"
title: "T2: cli --log/--log-format flag 配線と init 呼出し"
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
  implements:
    - "docs/plans/implement-observability-foundation.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-observability-foundation.md"
    - "docs/tasks/create-cheaper-observability-crate.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# T2: cli --log/--log-format flag 配線と init 呼出し

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T2。`cheaper-cli` に
`--log`/`--log-format` の strict parse 受理と、`run_cli` 先頭での
`cheaper_observability::init` 呼出し(guard 寿命・hook 経路の
`stderr_enabled: false`・init 警告の `--quiet`/hook 抑制ルール)
を配線する。activation 解釈自体は observability crate の責務で、
cli は flag/env 値を生のまま `Activation` に載せるだけ。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T2」が作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— activation surface・init
  呼出し点・`resolve_log_root`・hook stderr 判断の AUTHORITATIVE
  定義(「activation surface」「init 呼出し点と guard 寿命」節に
  実装疑似コードあり)
- `docs/adr/logging-output-and-activation-contract.md` — flag 命名・
  flag > env 優先順位・`--quiet` 確定ルールの根拠

## Dependencies

- `docs/tasks/create-cheaper-observability-crate.md`(T1)— init
  呼出し先の crate が前提
- `docs/tasks/instrument-cli-dispatch-and-run-pipeline.md`(T3)を
  block する — `src/cli.rs` を共有するため直列(T2 完了後に T3)

## Affected Files

- `crates/cheaper-cli/Cargo.toml` — `cheaper-observability =
  { path = "../cheaper-observability" }` + `tracing.workspace`
- `crates/cheaper-cli/src/args.rs`:
  - `STRING_OPTIONS` に `"log"`・`"log-format"` を追加し、
    `Values::set_string` に受理 arm を追加(値は `Values` field に
    保持しない — 消費は `global_log_flags` が担い、parse 済み値と
    activation 値の二重表現を避ける)
  - `workflow` 両 action の literal allowed-token 表
    (args.rs:501-517 付近の `allowed: &[&str]`)に `"log"`・
    `"log-format"` を追加。他 command には新規 rejection guard を
    置かない(field unchecked = globally accepted で global
    option 化が成立)。`ParsedCli` 各 variant の args struct にも
    載せない
  - lenient accessor `global_log_flags(argv) -> (Option<String>,
    Option<String>)` を追加:`--name value`/`--name=value`・`--`
    terminator・last-wins のみを解釈する軽量 scan(strict parse が
    後で検証するため)。`run_cli` が init 前に呼ぶ
  - 同じ lenient scan 規約を共有する bool 版
    `global_flag_present(argv, "quiet") -> bool` を追加 —
    `argv.iter().any(== "--quiet")` の素朴 scan は `--` 以降の
    positional `--quiet` を誤検出するため。bool option は
    `--quiet` 単独形のみ受理(`--quiet=…` は strict parse が弾く)
- `crates/cheaper-cli/src/cli.rs` — `run_cli` 先頭(`hook` 早期
  分岐より前、`parse_cli_args` より前)で init:
  - `is_hook` 判定・`global_log_flags(argv)`・`io.env` closure 経由
    で `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` 読取り(`std::env::var`
    ではない — test が `EnvLookup` で注入できる同一経路)
  - `resolve_log_root(argv, io)`: `--cwd` が command 意味を持つ
    経路(run・run alias・workflow・models・count-tokens)でのみ
    argv の `--cwd`(`--cwd value`/`--cwd=value`)を lenient scan。
    この scan は `global_log_flags`/`global_flag_present` と同一の
    `--` terminator 規約を共有する(design の単一 lenient-scan
    規約 — `--` 以降の positional `--cwd` を誤検出しない)。
    `usage`・`auth` 等では `io.cwd`。解決は
    `canonicalize_workspace_root` 規約(run 経路の workspace root
    解決と同一)、失敗時 `io.cwd` fallback(fail-open)
  - `stderr_enabled: !is_hook` で `Activation` を組み立て
    `cheaper_observability::init` を呼ぶ
  - `_log_guard` は `run_cli` ローカルに束縛 — dispatch・record
    usage 完了後の return 時に drop → non_blocking writer flush。
    tokio runtime は `run_cli` より外側・長命で、guard drop は
    runtime 生存中に完了する
  - hook 経路: init 警告を `io.stderr` に流さず install 済み
    subscriber へ `tracing::warn!` で畳む(未 install/None 時は
    drop — fail-open で hook 応答を汚さない)後、既存の
    `execute_hook` 早期 return
  - 非 hook 経路: `global_flag_present(argv, "quiet")` で判定し、
    `!quiet` の時だけ `io.stderr` へ `warning: {w}` 行を流す
  - HELP literal(cli.rs:405-442)の `Options:` 節に
    `--log <level>`・`--log-format <text|json>` の行を追加 —
    user-facing の global flag(全 command で受理)であり help
    記載が必須
- `crates/cheaper-cli/tests/regression/expected/cli-help-command.json`
  — help 系 regression golden を refresh(HELP 更新に伴う差分を
  再生成)
- `crates/cheaper-cli/tests/args.rs` — `--log`/`--log-format` の
  strict parse 受理(run・workflow・global option としての他
  command)+ `global_log_flags`/`global_flag_present` の lenient
  scan(`--` 以降は flag として扱わない・last-wins)の test 追加

## Done When

- [x] `--log <level>`・`--log-format <text|json>` が strict parse で
      受理され、workflow 両 action を含む全 command で Unknown
      option / "not valid for workflow" を返さない
- [x] `run_cli` 先頭で design 疑似コード通りの init 呼出しが配線
      され、`_log_guard` が run_cli 終了まで生きる
- [x] `resolve_log_root` が `--cwd` 対象 5 経路(run/run alias/
      workflow/models/count-tokens)でのみ argv `--cwd` を使い、
      それ以外は `io.cwd` を使う
- [x] hook 経路で `stderr_enabled: false` が渡され、init 警告が
      `io.stderr` に出ない(subscriber へ `tracing::warn!` で畳む)
- [x] 非 hook 経路で `--quiet` 時は init 警告が抑制され、非 quiet
      時は `warning:` 行が `io.stderr` に出る
- [x] `global_log_flags`・`global_flag_present`・`resolve_log_root`
      の `--cwd` lenient scan が同一の `--` terminator 規約を共有
      し、`--` 以降の positional を誤検出しない(`global_log_flags`
      は last-wins — `tests/args.rs` の追加 test で規約を固定)
- [x] HELP literal の `Options:` 節に `--log`・`--log-format` が
      追加され、help 系 regression golden
      (`tests/regression/expected/cli-help-command.json`)が
      refresh されている(実測の結果、当該 golden は HELP literal
      を render しないため refresh 不要と判定 — tmp/task-reports/
      obs-T2.md に記録)

## Out of Scope

- span/event instrumentation(`cheaper.cli.dispatch` 等)— T3/T4/T5
  の範囲。本 task は activation 配線のみ
- `.cheaper/logs` file 内容の assert・subprocess contract test —
  T6 の範囲
- envelope・exit code・`io.stderr` diagnostics 形式・`--quiet`
  semantics・usage.jsonl の変更 — 不変条件
- `tracing` emit の `io.stderr`/`CliWrite` 経由への統合 —
  design で棄却済み(tracing stderr sink は `std::io::stderr` 直書き)

## Verification

- [x] `mise exec -- cargo build -p cheaper-cli` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli` exits 0(既存 suite
      無変更 pass + args test 追加分)
- [x] `mise exec -- cargo tree -p cheaper-cli` に
      `cheaper-observability` が現れる
- [x] log 未指定の既定経路で regression suite・envelope test が
      無変更 pass(subscriber 未 install・`.cheaper/logs` 未作成を
      確認)
- [x] `git grep -n '"log"' -- crates/cheaper-cli/src/args.rs` で
      `STRING_OPTIONS` と workflow 両 action の `allowed` への
      追加を確認(追加漏れの diff 網羅確認)
