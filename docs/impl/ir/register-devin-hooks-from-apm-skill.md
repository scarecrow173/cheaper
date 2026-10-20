---
id: "IMPL-034Gu2UGm6tPXaR2rvwH0U"
type: "impl"
status: "completed"
title: "Register Devin hooks from APM skill"
created: "2026-10-15"
updated: "2026-10-21"
owners: ["scarecrow173"]
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
    - "docs/tasks/register-devin-hooks-from-apm-skill.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/normalize-tools-and-add-devin-hook-capability.md"
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
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Register Devin hooks from APM skill

## Summary

T2: APM-distributed の Windows PowerShell 5.1 helper
`install-devin-hooks.ps1` を新規実装した。preview 既定・`-Apply` で
`.devin/config.json` の `hooks.PreToolUse` に canonical
`cheaper hook pre-tool-use --target devin` entry を追加する。
capability handshake（`cheaper hook targets --json` の
`schemaVersion == 1` + case-sensitive `devin` membership）、
`Get-JsonShape` semantic 保全 gate、known legacy conflict scan、
reparse/path 検査、temp + `File.Replace`/`File.Move` の atomic write を
実装。`SKILL.md` は lead-authored exact copy（Devin section 追加 +
Claude bullet 差し替え）のみ適用、`docs/guides/distribution.md` に同一
workflow を追記。

## Context

- Task: `docs/tasks/register-devin-hooks-from-apm-skill.md`
  （`TASK-034GtBzKmwPf5z06kAahym`）
- Plan: `docs/plans/implement-native-tool-normalization-and-devin-bootstrap.md`
  （`PLAN-034GtndRTaEqU3lA6Tnquu`、T2 slice）
- Spec: `docs/specs/case-insensitive-native-tools-and-devin-registration.md`
  （approved）
- ADR: `docs/adr/apm-distributed-devin-native-hook-bootstrap.md`
  （accepted）
- Design: `docs/designs/devin-native-hooks-and-apm-bootstrap.md`
  （approved、`## JSON Preservation Helper` が `Get-JsonShape` の
  canonical source）
- Test-spec: `docs/test-specs/apm-distributed-devin-registration-preservation.md`
  （approved）
- 前提実装: T1 IR
  `docs/impl/ir/normalize-tools-and-add-devin-hook-capability.md`
  （capability contract の提供側）

Graph route は original `task-graph-to-implementation` edge を継続
（T1 → T2、commit baseline は T1 IR 保存値を保持、新 edge 未作成）。

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
  （新規、ASCII・comment 無し）:
  - `param([string]$ProjectRoot = (Get-Location).Path, [switch]$Apply)`。
    preview 既定は CLI 不要・file/dir 作成なし・sanitized な変更予告のみ。
  - root/`.devin`/`config.json` の各 path component を `ReparsePoint`
    attribute で検査し、書込み直前に再検査。write は
    `$ProjectRoot/.devin/config.json` のみ。
  - 既存 config は `ConvertFrom-Json` 後に `ConvertTo-Json -InputObject
    -Depth 100 -Compress` の再 serialize を `Get-JsonShape`（design の
    canonical function を一字変えず移植、冒頭で
    `Add-Type -AssemblyName System.Runtime.Serialization`）で raw と
    `-ceq` 比較。重複 key・escaped duplicate・case collision・precision
    loss・exponent 表記差異は拒否し bytes を保全。
  - schema gate: root object・`hooks` object・`PreToolUse` array・group
    object 以外は非破壊 error。
  - own entry 規則: canonical matcher+command の完全一致は no-op
    （bytewise）、変更済み・extra attribute・複数 own entry は拒否。
  - conflict scan: `.windsurf/hooks.json`・`.devin/hooks.json`・
    `.devin/hooks.v1.json`・`.devin/config.local.json`・
    `.claude/settings.json`・`.claude/settings.local.json`・primary
    config（own entry を除外）を走査し、`command`/`powershell`/`bash`/
    `windows`/`linux`/`osx` 配下の cheaper `pre-read-code`/
    `pre-tool-use` command を検出したら relative path だけ報告して停止。
    `pre-run-command` のみの shell-only legacy は非 conflict。
    unparseable な known file は非破壊 error。
  - `-Apply`: `Get-Command cheaper.exe, cheaper -CommandType Application`
    で解決 → `hook targets --json` を 1 回実行（exit 0・JSON object・
    integer `schemaVersion == 1`・全 string `targets`・`-ccontains
    'devin'`）→ 直前に原 bytes の Base64 比較と reparse 再検査 →
    canonical entry を append して temp + `File.Replace`（既存）/
    `File.Move`（新規）で書込み、`finally` で自作 temp/backup のみ
    cleanup。`.devin` は全 validation 通過後にのみ作成。
    `.devin/hooks.v1.json` は生成しない。
  - error 出力は全て fixed sanitized message + `cargo install` 等の
    install/update guidance。raw JSON・capability stdout/stderr・
    exception 文・config 値は出力しない。
- `scripts/test/devin-hooks-install.test.mjs`: installer 部分を追加
  （Windows `powershell.exe` full-path spawn、非 Windows は
  `describe` skip。fixturebin `cheaper.cmd` stub + PATH 隔離で
  capability 応答を制御。preview/no-file/no-CLI・sentinel 非出現・
  canonical 追加・deep/Unicode 保全・bytewise idempotence・capability
  拒否各種・malformed/ambiguous JSON 拒否・own entry 変更/複製/
  extra attribute 拒否・known legacy conflict・shell-only 非 conflict・
  junction 拒否を網羅）。既存 matcher 2 test は維持。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`: plan 記載の
  lead-authored exact copy で `### Devin CLI native registration
  (Windows)` section を追加（powershell 行は fenced code block）し、
  既存 Claude bullet 4 行を lead-authored 差し替え copy で置換。
- `docs/guides/distribution.md`: 「Devin CLI の native hook 登録
  （Windows）」節を追記（new/existing consumer 分離・preview/apply・
  capability 要件・supplemental registration・legacy conflict・`/hooks`・
  whole-file policy 不変）。

### Decisions During Implementation

- PS 5.1 の `[IO.File]::Replace(src, dst, $null)` は第 3 引数が空文字化
  して `ArgumentException`（パスの形式が無効）になるため、明示的な
  backup path を渡し `finally` で除去する形にした（atomic replace 自体は
  維持）。
- PS 5.1 では `-is [PSCustomObject]` が任意の値に対して `True` を返す
  既知の quirk があるため、object 判定は `GetType().FullName -eq
  'System.Management.Automation.PSCustomObject'` の
  `Test-JsonObject` helper に統一。
- fixed sanitized error contract に沿い、`.devin` が regular file の
  場合・`Get-Item`/`ReadAllBytes`/`CreateDirectory`/atomic write の
  IO 例外も全て固定 message へ変換（raw exception 文を出さない）。
- installer test の数値 fixture は frozen probe
  （`tmp/issue107-json-runtime-probe.ps1`、design Review Record 記載の
  結果）と同じ値を使う — `9007199254740993`/`0.1` は PS 5.1 で
  Int64/Decimal として lossless roundtrip するため保全 gate を通り、
  「拒否」を期待する test contract とは一致しない。probe の
  `123456789012345678901234567890`（Double 化で損失）と
  `0.12345678901234567890123456789` に修正した。
- `SKILL.md` の文面は一切生成せず、plan に記録された lead-authored
  copy のみ挿入/差し替え。
- lead review（Reviewer B）で confirmed された source-level failure
  path（B1〜B8）を 1 回の rework round で修正した。新規 native hook
  group schema の具体化は approved design の native contract の
  詳細化であり方針変更なし。主要な修正:
  - Apply 時は既存 canonical entry でも capability handshake を
    通すよう no-op return を handshake 成功後へ移動（旧 CLI/CLI 不在
    では apply が失敗する）。
  - `Get-ExactKey`（case-sensitive 完全一致）を追加し、root
    `hooks`・event `PreToolUse`・capability `schemaVersion`/`targets`
    の既存 property 名が exact case でなければ保全 error とする
    （`{"Hooks":{}}` 等の誤 case key へ登録して Devin が読まない
    状態を防止）。capability response の未知追加 field は許可。
  - `Assert-NativeHookGroup` を追加し、`PreToolUse` の各 group を
    schema 検査（group は object、`matcher` は省略可・あれば string、
    `hooks` は Object[] 必須・各 item は object、`type` は
    `"command"`/`"prompt"` で対応する `command`/`prompt` string 必須。
    空文字の新規制約は追加せず、unknown props は保持）。own
    canonical 判定は strict 2-props のまま。
  - `Assert-PathNoReparse` を ancestor 対応へ拡張し、root の drive
    root までの各既存 component と known legacy config file path の
    reparse point を検査（global file crawl はしない）。harness 側も
    `runInstaller` に `binRoot` を追加し、junction root 配下の
    consumer でも fixture `cheaper.cmd` が解決できるよう修正（従来は
    「CLI not found」による vacuous pass だった）。
  - concurrency guard test を追加: capability stub が config を外部
    書換してから応答する fixture で、pre-write bytes re-check が外部
    変更後 bytes を保全し entry 未追加で失敗することを検証（code
    追加なし、既存 pre-write check で通過）。
  - `installHint` を distribution.md の canonical update 手順と一致
    させ `--locked --force cheaper-cli` へ修正。
  - capability schema の厳格化 test を拡充（`1.0`/`"1"`/scalar
    targets/non-string member/root array/miscased `SchemaVersion` は
    非破壊 error、sentinel の追加 field は許可しつつ新 config に
    sentinel を書かない）。
  - Node test は `SystemRoot` から full path を構築し、Windows 上で
    `powershell.exe` 不在なら skip ではなく fail。
  - PS 5.1 では `if` 式経由の代入が単一要素配列を unwrap するため、
    capability field の取出しは直接代入に変更（sentinel payload で
    `targets: ["devin"]` が string 化して `tOk` を落としていた）。
- final verification 中の lead 追記で、`apm install` の lockfile replay に
  対する scoped `apm update` 手順を lead-authored paragraph として
  `SKILL.md`（Devin section）・plan 内 skill 引用・`distribution.md`
  （Japanese copy）の 3 箇所へ機械挿入。code/API/dep 変更なし、helper
  hash inputs 不変のため APM gate 再実行なし。

## Related Experiments

### Adopted

なし（experiment log は作成していない。`Get-JsonShape` は approved
design `## JSON Preservation Helper` の frozen source を機械移植）。

### Rejected

なし。

## Validation

- red（script 未作成時）: `mise exec -- node --test
  scripts/test/devin-hooks-install.test.mjs` → exit 1、15 tests /
  7 pass / 8 fail（log: `tmp/issue107-t2-node-red.log`）。script
  missing のための behavioral red。
- 中間 debug: `[IO.File]::Replace` の `$null` backup 引数による
  ArgumentException（`tmp/issue107-t2-node-green1.log` / `green2`）と
  PS 5.1 `-is [PSCustomObject]` quirk、および数値 fixture の frozen
  probe 値への整合（`tmp/issue107-t2-node-green3.log`）を修正。
- green: 同コマンド → exit 0、15 passed / 0 failed
  （log: `tmp/issue107-t2-node-green4.log`、biome format 後
  `tmp/issue107-t2-node-green-final.log`、sanitized-error hardening 後
  `tmp/issue107-t2-node-green5.log`、bytes/reparse re-check を
  serialize 直後・write 直前へ移動した最終 file で
  `tmp/issue107-t2-node-green6.log`）。
- B1〜B8 rework round: harness `binRoot` 修正で vacuous pass を解消
  した後の red（ancestor junction・exact-case・group schema・capability
  schema 等）は `tmp/issue107-t2-node-red3.log`。残 1 fail（sentinel
  payload の accept 期待に対し exit 1）は `if` 式経由代入による
  単一要素配列 unwrap が script 側 bug であり、直接代入へ修正して
  解消（test contract は「未知 field 許可・sentinel 非出現」を維持）。
  最終 green: 同コマンド → exit 0、**21 passed / 0 failed**
  （log: `tmp/issue107-t2-node-green10.log`）。
- `mise exec -- biome check --write
  scripts/test/devin-hooks-install.test.mjs` → organizeImports/format/
  unused import 修正後、`biome check` exit 0。
- 手動 APM integration gate: fixture
  `tmp/issue107-apm-consumer/`（`apm.yml` に `path: ../../packages/apm`
  local dep）で `mise exec -- apm install --target agent-skills` →
  exit 0（log: `tmp/issue107-apm-gate.log`、policy discovery の
  warning 1 件のみ・admission bypass なし）。source script への
  hardening 変更を行ったため、許可された「source 変更時のみ再 install」
  の条件で同一 fixture へ再実行（log: `tmp/issue107-apm-gate2.log`、
  `gate3`、`gate4`、capability unwrap 修正後 `tmp/issue107-apm-gate5.log`）
  → いずれも exit 0。
  最終 deployed
  `.agents/skills/cheaper-usage/scripts/install-devin-hooks.ps1` の
  SHA256 = `338e671bd3688a02e93e246f80386d2232db45cc64904208ef44c6d28020e2fc`
  で最終 source と一致、`.windsurf`/`.claude`/`.devin` 未生成を機械
  確認。consumer root・`apm.lock.yaml`・log は tmp に保持。
- cargo hook test は T1 未変更のため未再実行（最終 gate の
  `cargo test --workspace` で全 pass 済み）。
- 最終 gate `mise run check`（`CHEAPER_E2E_OLLAMA=0`、実行 process
  のみ）実施: `tmp/issue107-final-check.log`（biome 段で stop）→
  修正後 `tmp/issue107-final-check2.log` で全 stage exit 0。
  本差分起因の修正: test file の format 差分・fixture 未 cleanup
  （`afterEach` で `rmSync` 追加、junction 残留の biome warning
  解消）・`apm-install-targets.test.mjs` claude smoke の旧 matcher
  literal を新 canonical matcher へ更新・`hook.rs` の `format!`
  literal を `.to_string()` へ（clippy）。
- lead review: T2 全差分 + green10/gate5 証拠 ACCEPT（B1〜B4
  confirmed P1 解消、B5〜B8 適用、Reviewer A O-B は UNVERIFIED
  nonblocking で推測修正なし）。pending
  task-graph-to-implementation edge の commit gate は lead commit
  待ち（完了 checkpoint は lead commit 後）。

## Risks

- helper は supplemental registration。`apm uninstall` は native entry
  を除去しない（`SKILL.md`/`distribution.md` に明記済み）。
- conflict scan は project 内の既知 file に限定。global/ancestor の
  hook source は `/hooks` での利用者確認に依存（design の deferred
  concern と一致）。
- 初回適用は既存 config の format を `ConvertTo-Json` 正規化する
  （semantic 保全 gate 通過後のみ）。byte-level の format 保持ではない。
- #107 の範囲評価（`offset`/`limit`/`max_results` semantics）は
  T1/T2 いずれの scope 外で未解決のまま。
- POSIX installer・native shell tool 登録は design の deferred concern
  のまま。

## Follow-ups

- lead が全 T2 diff（`tmp/issue107-t2.diff`）と証跡を review 済みで
  ACCEPT。
- 最終 gate は `mise run check` の初回 invocation が biome 段で
  失敗（exit 1）し、修正後に未実行残 stage を mise.toml 順で継続して
  全 pass を確認（単一 invocation の exit 0 ではない。log:
  `tmp/issue107-final-check.log` / `tmp/issue107-final-check2.log`）。
- `docs/guides/parent-agent-integration.md` の旧 matcher 記述 drift
  は lead-authored copy で解消済み（Claude/Codex bullet 更新 +
  Devin CLI・agent-skills・その他 target bullet 追加）。
- plan status は completed。pending task-graph-to-implementation
  edge の commit gate は lead commit 待ち（完了 checkpoint は
  lead commit 後）。

## Evidence 保存状態（2026-10-21 追記）

当時の raw log・diff artifact(`tmp/issue107-t2.diff`・
`tmp/issue107-final-check*.log` 等)は履歴 label であり参照前提
ではない。登録 wire・matcher・config shape の durable な正本は
[design `devin-native-hooks-and-apm-bootstrap`](../../designs/devin-native-hooks-and-apm-bootstrap.md)
と
[discovery `bounded-hook-read-request-contract`](../../discovery/bounded-hook-read-request-contract.md)
に保持される。最終 gate の段階別結果は本 IR の Follow-ups に
記録済み、raw log 自体は不要。現行 test path:
`scripts/test/devin-hooks-install.test.mjs`・
`crates/cheaper-cli/tests/integration/hook.rs`。
