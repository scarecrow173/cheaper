---
id: "SPEC-034KI3mWcW92kMljP5xoa8"
type: "spec"
status: "proposed"
title: "exec-contract cleanup — 内容審査を mandatory 層から除去"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
    - "../discovery/exec-contract-content-audit-removal.md"
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
    - "../adr/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../discovery/permission-model-responsibility-redesign.md"
  defers: []
  deferred-by: []
---

# exec-contract cleanup — 内容審査を mandatory 層から除去

## What Should Be Built

mandatory 実行契約層（grant/task の resolve → pin → spawn 前再検証 →
argv 認可）から「宣言された executable・argv の**内容を審査して
reject**」する 7 項目の制限を除去する cleanup。対象は親 design
（#151）が content-audit クラスとして整理した A8・A10・B3・B4・
B5・B6・F3、および A9 deny list の **mandatory 適用点**（opt-in
profile 化は issue #164）。除去後、cheaper が強制するのは
「宣言された実行契約」のみ — shell-free argv spawn・grant/task
認可・pin 同一性・lifecycle 境界・filesystem/declared-resource
契約。

除去する内容審査（実測 site は
`../discovery/exec-contract-content-audit-removal.md` の
Removal Site Inventory 参照）:

- **A8** — grant executable の shell/shim 分類 reject
  （`policy/command.rs` の `is_shell_process_executable` 適用）
- **A10** — `argv_shell_re` による metacharacter reject（宣言
  `ArgPattern::compile`・runtime `authorize_args`・task argv の
  3 適用点。regex 定義自体は route viability が現役参照するため
  残置 — issue 4 scope）
- **B3** — native executable の magic-byte 検査（MZ/ELF/Mach-O）
- **B4** — workspace-resident executable の reject（宣言 path
  pre-filter・realpath 後・spawn 前再査）
- **B5** — bare `node` の固定 reject
- **B6** — `.cmd/.bat/.ps1/.sh` 拡張子の reject
- **F3** — process task の native/shim 検査（task executable への
  shim predicate 適用・`native()` 内 shim 判定・bare `node` 等
  `resolve_process_executable` 経路）
- **A9 適用点** — `is_denied_command_executable` の resolve・
  pin・spawn 前再検証 3 適用点（builtin deny predicate 定義の
  disposition は ADR に従う）

## Why Now

現行の内容審査は「危なそうな executable・argv を cheaper が審査
して止める」設計であり、親 ADR（ADR-034KuqZelggTIb8Z8K9QB8・
accepted）で以下が確定済み:

- cheaper の責務は**宣言された実行契約の強制**であり、宣言され
  た executable の content を審査することではない。
- 内容審査は false positive を生み（宣言済みの正当な `.sh`・
  `python`・workspace 内 helper を拒否）、bypass は無限に存在
  するため完全性が保証できない。
- 同一責務が policy resolve・pin・spawn 前再検証・viability の
  複数層へ分散しており、1 つの behavioral 変更に横断的な修正を
  要する構造上の問題がある。

issue #163 は #151 実装分解の起点（1/7）であり、後続の opt-in
deny profile（#164）・grant/task merge・viability 整理の全てが
本 cleanup の除去結果を前提にする。

## Users and Value

- **operator / workflow 作者** — 宣言した executable（`.sh`
  helper・`python`・workspace 内 tool・bare `node` 等）が
  cheaper 側の内容判定で拒否されなくなり、policy が「宣言した
  ものがそのまま実行契約になる」読みやすいモデルへ戻る。
- **runtime 保守者** — 認可層が「契約の強制」のみを担い、
  content audit の判断分岐・compensating check が 1 箇所で済む
  構造になる。deny 能力が必要な operator は issue #164 の
  opt-in profile で**決定論的**に有効化できる。
- **route/hook 層** — viabililty 分類は authorization と切り離
  され、pin 成否・contract 成否の観測のみを残す整理が可能に
  なる（issue 4 前提）。

## Scope

### In Scope

- A8・A10・B3・B4・B5・B6・F3 の call site・helper・審査分岐の
  除去（Removal Site Inventory の各 row）。
- A9 deny list の 3 mandatory 適用点の除去
  （`is_denied_command_executable` 呼出し。builtin regex
  `command_denied_executable_re`・predicate 定義自体の
  disposition は ADR-034KI3r2xdthh6rKDFY517 に従う）。
- 除去に伴う `argv_shell_re` への限定処置: exec 側 3 適用点を
  除去し、regex 定義と `crates/cheaper-cli/src/route/resolve.rs:285`
  の viability 参照は**残置**する（issue 4 が viability 側審査
  依存を整理する）。
- `native()` からの審査分岐除去後、spawn viability 判定
  （realpath・is_file・win32 `.exe/.com` 拡張子・X_OK）は残す。
- error message・doc comment の誠実化
  （「shell and script shims are unsupported」「denied/resident
  /metacharacter」等、実態と乖離する表現の restate）。
- Affected Tests / Fixtures Inventory の全項目の更新・削除・
  再用途化。
- 観測差分の明文化（`\0` を含む argv の error 分類変化・pin
  内容差替の検出範囲・ENOEXEC fallback 注記 — discovery 参照）。

### Out of Scope

- **issue #164** — `security.execDenyPatterns` opt-in profile の
  新規実装・schema・opt-in deny の再適用。本 issue が除去した
  mandatory 挙動の「再配置」先であり、本 issue 内では実装しない。
- **issue 4 scope** — route viability 層の審査依存整理
  （`resolve.rs:285` の `argv_shell_re` 参照含む viability 側
  unsafe 分類の見直し）。
- **grant/task merge・trusted flag・docs 全体 sweep** — 後続の
  分解 issue の scope。
- `tokenize_shell`・pin 取得・pin-equality・wildcard 構造検査・
  `relative()`・lifecycle・`PathDenylist`/`denyPaths` 等 Keep
  Site Inventory の全項目 — 変更しない。
- OS sandbox・実アクセス tracing — 親 chain でも非目標。

## Requirements

- **R1** — `resolve_command_policy` は shell/shim・deny・metachar
  の観点で declared executable・argv pattern を reject しない。
  wildcard 構造検査（A6）と env/cwd/argv-pattern の契約検査は
  維持する。
- **R2** — `pin_command_grant` は realpath 解決と pin 取得のみを
  行い、resolved basename の deny・workspace 内所在・relative
  path pre-filter・shell/shim で拒否しない。`resolve_process_executable`
  の PATH 検索→canonical realpath の決定的 pin は維持する。
- **R3** — `verify_pinned_executable` は `resolved != pinned` の
  pin-equality（B2・TOCTOU）のみを強制し、deny・workspace 内
  所在・内容審査を再検証しない。
- **R4** — `native()` は spawn viability（realpath・is_file・
  win32 `.exe/.com`・X_OK）のみを判定し、magic-byte 検査
  （`native_magic_supported`）と shim 判定を除去する。
- **R5** — `resolve_process_executable` は shim/front-door・
  bare `node` 固定 reject を持たず、PATH 解決した executable を
  返す。bare `node` が PATH に存在すれば解決され、存在しなければ
  解決不能として通常の unsupported 経路で失敗する。
- **R6** — `authorize_args` は metacharacter で runtime argv を
  reject せず、argv pattern 照合（A6）のみを行う。NUL・改行等
  契約外入力は spawn 層の OS エラーとして観測される（error 分類
  の変化は許容。`env` の NUL 規約は別途維持）。
- **R7** — `resolve_process_policy` の task 検証ループは
  executable の shell/shim・argv metachar で task を reject せず、
  cwd/inputs/outputs の workspace-relative 検査（A7）と env
  規約のみを強制する。
- **R8** — `argv_shell_re` regex 定義と `route/resolve.rs:285` の
  viability unsafe 分類は本 issue では変更しない。
- **R9** — `tokenize_shell`・`PathDenylist`/`denyPaths`・
  `check_command_execution` の pin-equality 経路・lifecycle 契約
  の code site は変更しない。
- **R10** — error message・doc comment は除去後の実態を正直に
  記述する（「unsupported native or shell/script executable」
  等の旧表現を残さない）。
- **R11** — 削除した審査を**別の補償条件で再実装しない**
  （責務分離の意図を破壊しない）。deny 能力の再配置は issue
  #164 の opt-in profile が担う。
- **R12** — A9 predicate 定義（`command_denied_executable_re`・
  `is_denied_command_executable`）の disposition は
  ADR-034KI3r2xdthh6rKDFY517 の決定に従う。

## Acceptance Criteria

- [ ] A8・A10・B3・B4・B5・B6・F3 の全適用点が
  `resolve_command_policy`・`resolve_process_policy`・
  `pin_command_grant`・`verify_pinned_executable`・
  `authorize_args`・`native()`・`resolve_process_executable`
  から除去され、`is_shell_process_executable`・
  `native_magic_supported`・`within_workspace`・
  `is_relative_executable_path`・`script_ext_re` の審査 helper
  が残存しないこと（grep で確認可能）。
- [ ] 宣言 executable として `python`・`ssh`・`tar`・`.sh`・
  workspace-resident binary を含む grant が、opt-in deny
  profile を指定しない構成で policy resolve・pin で拒否
  されないこと（unit または integration test で固定）。
- [ ] bare `node` を宣言した process task が B5 固定 reject を
  受けず、PATH 上に node があれば解決されること。
- [ ] `tokenize_shell` の hook 文字列解析・pin 取得・
  `resolved != pinned` の spawn 前再検証・argv pattern 照合・
  `relative()`・lifecycle 契約の既存 contract test が変更
  なく通ること。
- [ ] `spawn_time_revalidation_detects_swapped_executable` 相当の
  pin-equality 検証が、B3 除去後も path 差替（symlink retarget・
  realpath 不一致）を検出することを固定する test に再設計され
  ていること。
- [ ] Affected Tests / Fixtures Inventory に挙げた golden
  fixture（`policy/process.json`・`policy/load.json`・
  `fixtures/process.json`）・unit test・integration test・
  dev-seam case が新挙動へ更新・削除・再用途化されていること。
- [ ] `\0` を含む argv が policy 層 `POLICY_VIOLATION` でなく
  spawn/OS 層のエラーとして観測されることが test または記録
  で明文化されていること。
- [ ] `route/resolve.rs:285` の `argv_shell_re` viability 分類が
  変更されず残っていること。
- [ ] `mise run check` pipeline が通ること。

## Deferred Design Concerns

- **viability 側の metachar unsafe 分類**（`resolve.rs:285` —
  `argv_shell_re`）— 本 issue では残置。routing≠authorization
  の整理は issue 4 が担う。残置のため `argv_shell_re` regex
  定義は削除しない。
- **opt-in deny profile** — `security.execDenyPatterns` の schema・
  適用フック・builtin 一覧の docs 移管は issue #164 の scope。
  本 issue が空けた mandatory 適用点を前提に再配置される。
- **builtin deny 一覧の docs 記録** — 削除される場合の旧既定値
  の migration 例は issue #164/#169 の docs 作業で扱う。
- **route viability の挙動変化** — workspace-resident・shim・
  deny 該当の declared grant が pin を通るようになり NoGrant →
  Runnable になる経路が増える。分類の見直しは issue 4。

## Sources and Relations

- 起点 issue: #163（#151 実装分解 1/7）
- 本 spec の派生元: `../discovery/exec-contract-content-audit-removal.md`
  （実測 site inventory・test/fixture 棚卸し・観測差分の
  ground truth）
- 決定根拠（`references`/`related` のみ — cross-chain lineage は
  張らない）:
  - `../designs/permission-model-responsibility-redesign.md`
  - `../specs/permission-model-responsibility-redesign.md`
  - `../adr/enforce-declared-contracts-over-content-audit.md`
  - `../discovery/permission-model-responsibility-redesign.md`
- 兄弟 ADR: `../adr/exec-contract-content-audit-removal.md`
- 依存する後続 issue: #164（opt-in deny profile の再配置先）
