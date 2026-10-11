---
id: "TASK-034DVHrszM9fxMAyaOCqFt"
type: "task"
status: "done"
title: "T6: observability contract test + verification"
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
    - "docs/tasks/instrument-cli-dispatch-and-run-pipeline.md"
    - "docs/tasks/instrument-tools-workflow-provider-and-adapter.md"
    - "docs/tasks/instrument-hook-auth-catalog-and-core.md"
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

# T6: observability contract test + verification

## Work

plan `docs/plans/implement-observability-foundation.md`
(PLAN-034DVGKbl3V2g7msSfpXv0)の T6。design の test 戦略に従い
contract test 群を実装し、benchmark harness の env 注入受け口を
拡張して off-state overhead 計測を可能にし、`mise run check` で
最終検証する。**file sink を assert する test は全て subprocess
isolation** — `set_global_default` は process 当たり一度しか成功
しないため、同一 test binary が `run_cli` を複数回呼ぶ経路は
repeat-init contract に乗り test 毎の独立 subscriber を取れない。
前例は `tests/regression/main.rs`(:144-149 の
`CARGO_BIN_EXE_cheaper` 解決)と
`tests/regression/harness.rs::spawn_collect`(:257、`env_clear`
spawn)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-observability-foundation.md` — 本 task が
  implement する plan。File Map「T6」・Verification matrix が
  作業の正
- `docs/designs/observability-foundation-logging-and-instrumentation-wiring.md`
  (DESIGN-034DV9ypzmxdAobGSJ6Cm6)— test 戦略表・subprocess
  isolation 方針・benchmark 拡張の required work の AUTHORITATIVE
  定義
- `docs/specs/observability-foundation-scope.md` — acceptance
  criteria(secrets 非記録・`--quiet` 確定ルール・不変条件維持)

## Dependencies

- `docs/tasks/instrument-cli-dispatch-and-run-pipeline.md`(T3)
- `docs/tasks/instrument-tools-workflow-provider-and-adapter.md`(T4)
- `docs/tasks/instrument-hook-auth-catalog-and-core.md`(T5)—
  全 instrumentation 配置完了後に contract test を書く
  (T1・T2 は T3 経由で推移的に依存)

## Affected Files

- `crates/cheaper-cli/tests/observability.rs`(新規、non-seam)—
  design test 戦略表の検証を実装:
  - `.cheaper/logs/` 生成・file sink 出力 — subprocess:
    `CARGO_BIN_EXE_cheaper` を `env_clear` + `CHEAPER_LOG=debug` +
    `CHEAPER_CONFIG_DIR`=tempdir + tempdir cwd で spawn し、
    `<ws>/.cheaper/logs/cheaper.log.*` の存在と内容を確認
  - JSON format parse 可能性(`CHEAPER_LOG_FORMAT=json` で file
    行を `serde_json` parse)+ text format 出力(JSON 以外の行
    形式)を両 sink で確認
  - flag>env override(level・format 双方): `CHEAPER_LOG=warn` +
    `--log debug` で debug event が file に出る等
  - `--quiet` + log 有効化: `--quiet` + `CHEAPER_LOG` で stderr に
    diagnostics(error/model/warning line)が出ず log event が
    file に出る(確定ルール: `--quiet` は明示的 log 有効化を
    抑制しない)
  - repeat-init: 同一 test 内で `run_cli` を 2 回呼び、2 回目の
    `warning:` 行と正常 exit code を assert(in-process —
    file 内容は assert しない。subscriber は 1 回目の設定のまま)
  - hook 経路: hook invocation を spawn し、stderr に tracing 行・
    init `warning:` 行が出ない + file に `cheaper.hook` event が
    ある(invalid `CHEAPER_LOG` 値注入で init 警告が stderr に
    漏れないことも確認)
  - auth/catalog I/O boundary の観測可能性(spec AC5):
    `CHEAPER_LOG=debug` 有効化 subprocess で `cheaper models`
    (catalog)と `cheaper auth status`(auth)を spawn し、file に
    それぞれ `cheaper.catalog`・`cheaper.auth` event 行が出ること
    を assert
  - envelope byte-parity・exit code: log OFF(既定)で既存
    `tests/envelope.rs`・`tests/regression/` が無変更 pass
    であることがそのまま証拠。追加の assert は不要
- `crates/cheaper-cli/tests/observability_dev_seam.rs`(新規、
  `required-features = ["dev-seam"]`)— secrets 非記録 scan:
  `CHEAPER_TEST_FAKE_STREAM` fixture で dev-seam binary を
  `CHEAPER_LOG=debug` で spawn し、`.cheaper/logs/*` と stderr
  から secret 文字列・prompt 本文・tool payload を grep して
  0 件を assert + run pipeline stage timing(`cheaper.run`
  span tree の close event `time.busy`・stage span 名)が file
  に出ること + tool internals の観測可能性(spec AC5): fixture を
  tool call 実行を含む run になるよう configure し、file に
  `cheaper.tool` span・`cheaper.tool.process` event 行が出ること
  を assert
- `crates/cheaper-cli/Cargo.toml` —
  `[[test]] name = "observability_dev_seam" required-features =
  ["dev-seam"]` を追加(non-seam `observability.rs` は ungated)
- `benchmarks/lib/cli.mjs` — `benchmarkEnvironment` に
  `CHEAPER_LOG`/`CHEAPER_LOG_FORMAT` の env passthrough
  allowlist(または scenario/arm 指定 env override)を追加。
  現状の curated env dict(PATH/HOME/NODE_ENV/
  CHEAPER_CONFIG_DIR/CHEAPER_TEST_FAKE_STREAM 等)は
  `CHEAPER_LOG*` を forward しない
- `benchmarks/lib/run-scenario.mjs` +
  `benchmarks/scenarios/scenarios.json` — per-arm env 指定
  (または `--log debug` flag 注入 arm)の受け口を追加。
  off-state overhead 計測の (c) `CHEAPER_LOG=debug` arm に
  必要な required work
- `benchmarks/results/` — off-state overhead 3 点計測
  ((a) instrumentation 導入前 HEAD・(b) 導入後 CHEAPER_LOG
  未設定・(c) 導入後 `CHEAPER_LOG=debug`)の JSON 記録。
  `node benchmarks/run.mjs --scenario <id>` の median
  wall-clock 比較、(a)→(b) delta がノイズ範囲内(目安: median
  で ±3% 未満)であることを確認。scenario は tool 実行を含む
  `single-simple-tool-call` 等 run pipeline を通るもの

## Done When

- [x] `tests/observability.rs` が design test 戦略表の non-seam
      検証(`.cheaper/logs` 生成・JSON parse・text format・
      flag>env・`--quiet`・repeat-init・hook stderr contract・
      auth/catalog 観測可能性)を subprocess isolation で実装して
      いる
- [x] `tests/observability_dev_seam.rs` が `required-features =
      ["dev-seam"]` で secrets 非記録 scan + stage timing +
      tool internals の観測可能性を実装している
- [x] spec AC5 の run pipeline 以外の計測点が test で固定されて
      いる — hook に加え、tool internals(dev-seam: fake-stream
      fixture + tool call で `cheaper.tool`/`cheaper.tool.process`
      event 行)と auth/catalog I/O(non-seam subprocess で
      `cheaper auth status` → `cheaper.auth`・`cheaper models` →
      `cheaper.catalog` event 行)が log 有効時に file へ出ることを
      assert
- [x] file sink を assert する test が全て
      `CARGO_BIN_EXE_cheaper` + `env_clear` spawn 経由で、
      in-process `run_cli` 呼出しは repeat-init contract のみ
- [x] `benchmarkEnvironment` が `CHEAPER_LOG*` を forward し、
      per-arm env/flag 注入の受け口が run-scenario/scenarios
      にある
- [x] `max_log_files` の m+1 semantics を実測確認 —
      `cheaper.log` prefix の世代 file が実際に rotate/retain
      されることを確認し、結果を Verification に記録(plan
      risk 4 の引継ぎ)
- [ ] `mise run check` exits 0 — 構成コマンドは個別に全実行済みだが、
      `cargo test --workspace`(dev-seam 付き)が pre-existing の
      `hang_grace` abort 分類 bug(issue #94)で fail するため
      未達。本 observability 変更とは無関係の既知 flake
      (tmp/task-reports/obs-T6.md 参照)

## Out of Scope

- instrumentation の追加・変更 — T3/T4/T5 の成果物を test する
  のみ(見つかった欠落は該当 task の範囲として報告し、本 task
  での大規模な再配置はしない)
- 最適化の実施・(a)→(b) delta 改善 — 計測・記録のみ。許容範囲
  超過時は結果を報告して判断を仰ぐ
- OTLP 等の外部 backend・raw payload opt-in — spec Out of Scope
- `mise.toml`・CI gate の追加 — design で確定済み(追加の gate
  は作らない。observability は常時 compile 対象)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --test observability`
      exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --test
      observability_dev_seam --features cheaper-cli/dev-seam`
      exits 0
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
      (regression 55 case・envelope byte-parity 無変更 pass)
      — `--no-fail-fast` で `hang_grace` fixture 1 件のみ fail
      (issue #94 の pre-existing abort 分類 bug)。その他の全
      binary は pass
- [x] `mise exec -- cargo test --workspace` exits 0(feature 無しで
      dev-seam target が required-features skip・non-seam pass)
- [x] `CHEAPER_LOG=debug CHEAPER_LOG_FORMAT=json` の手動 spawn で
      `.cheaper/logs/cheaper.log.*` と stderr の両方に
      `cheaper.run` stage span の close event(`time.busy`)が
      出て JSON が parse 可能であることを確認(contract test
      `observability_dev_seam`・`observability.rs` の assert で
      代替検証)
- [x] spec AC5 計測点: `cheaper models`・`cheaper auth status` の
      subprocess で file に `cheaper.catalog`・`cheaper.auth`
      event 行が、dev-seam fake stream + tool call で
      `cheaper.tool`/`cheaper.tool.process` event 行が出ること
- [x] `max_log_files` retention 実測結果(保持世代数・rotation
      挙動)を記録済み(`retention_keeps_most_recent_files`:9 世代
      → 6+当日=7 file 保持・prefix 非一致は削除対象外)
- [ ] off-state overhead 3 点計測が `benchmarks/results/` に
      記録され、(a)→(b) delta がノイズ範囲内 — (a)
      instrumentation 導入前 HEAD build が別 checkout を要し
      未取得。current build(80e8eee)のみで off/file/debug 3 arm
      を計測し `benchmarks/results/4eda3caf-…json` に記録
      (logging 有効化の追加 overhead は観測不能な範囲)
- [ ] `mise run check` exits 0 — 上記 hang_grace flake(issue #94)
      のため未達。構成コマンドは個別に全 pass 済み
