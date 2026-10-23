---
id: "DISC-JLHFz6x4ZuhzYZrpfrG2QV"
type: "discovery"
status: "resolved"
title: "exec-contract cleanup — 内容審査 7 項目を mandatory 層から除去（#163）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
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
  derives-from: []
  derived-by:
    - "../specs/exec-contract-content-audit-removal.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../designs/exec-contract-content-audit-removal.md"
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

# exec-contract cleanup — 内容審査を mandatory 層から除去（issue #163）

## Exploration Goal

issue #163（#151 実装分解 1/7・起点 issue）の除去 scope を実コード
file:line 精度で固定する。「宣言された exec の内容を審査して
reject」する制限（content-audit クラス）を mandatory 層から除去
する設計判断は親 chain（#151）で確定済み（ADR-034KuqZelggTIb8Z8K9QB8
accepted・design approved）。本調査の完了条件:

1. issue 記載の各除去 site が現行コードと一致するか検証し、
   issue 略記と実 path の差異を確定する。
2. 各検査を観測する caller/test/fixture を全量棚卸しし、
   churn 範囲を固定する。
3. keep 対象（`tokenize_shell`・pin + spawn 前再検証・
   pin-equality）の触れてはいけない code site を特定する。

## Key Issues and Assumptions

- **[確認済] issue の path 略記と実 path のずれ** — issue 本文の
  `command/mod.rs` は `crates/cheaper-cli/src/command/mod.rs` ではなく
  `crates/cheaper-tools/src/tools/command/mod.rs`、
  `executable.rs` は `crates/cheaper-tools/src/tools/proc/executable.rs`。
  `policy/command.rs`・`policy/process.rs` は
  `crates/cheaper-core/src/policy/` 配下で一致。
- **[確認済] A9 deny list の mandatory 適用点は本 issue の除去範囲** —
  issue の表は 7 項目（A8・A10・B3・B4・B5・B6・F3）のみ挙げるが、
  AC が「`python`/`ssh`/`tar` 宣言 grant が拒否されない」を要求し、
  churn 対象に `run_fake_stream_grant_deny_listed_executable_rejected`
  を明示、さらに issue #164 の body が
  「deny list の mandatory 除去**後**に opt-in profile として再配置」
  を deps に持つ。よって `is_denied_command_executable` の 3 適用点
  （`command.rs` resolve・`command/mod.rs` pin・同 spawn 前再検証）
  の除去は本 issue の scope。
- **[確認済] `argv_shell_re` は削除しない** — regex 定義自体は
  `crates/cheaper-cli/src/route/resolve.rs:285`（viability の
  unsafe-command 分類）が現役で参照する。exec 側の 3 適用点のみ除去
  し、viability 側の審査依存整理は issue 4 の scope。
- **[確認済] pin-equality は path 同一性の再検証であって内容同一性では
  ない** — `verify_pinned_executable` は `native_executable` で再解決
  した canonical path と pin 値を比較する（B2）。B3 除去で
  `native()` の magic-byte 検査が消えるため、「同じ path の file
  内容が非 native へ差し替わった」ケースは検出対象外になる
  （path/symlink retarget の検出は維持）。
- **[前提] 残す検査は spawn viability であって内容審査ではない** —
  `native()` 内の realpath・is_file・win32 `.exe/.com` 拡張子・
  X_OK は「OS が spawn 可能か」の判定であり残す。Windows は
  `.exe`/`.com` 以外を直接 CreateProcess できない。
- **[前提] deny predicate 定義の disposition** —
  `command_denied_executable_re`・`is_denied_command_executable` は
  builtin 定数 regex であり、issue #164 の opt-in profile は
  operator 宣言 pattern を compile する別 predicate を要するため
  再利用価値が無い。削除してよいかは ADR で記録する。

## Removal Site Inventory（実測 file:line — 2026-10-23 時点）

### `crates/cheaper-core/src/policy/process.rs`

| site | lines | item | 内容 |
|------|-------|------|------|
| `shell_executable_re()` | 117-125 | A8/B6 | shell 名 regex 定義 |
| `script_ext_re()` | 127-130 | B6 | `.cmd/.bat/.ps1/.sh` regex 定義 |
| `argv_shell_re()` | 134-137 | A10 | metachar regex 定義 — **削除しない**（`route/resolve.rs:285` が参照）|
| `is_shell_process_executable()` | 141-148 | A8/F3 | shell/shim predicate |
| task loop `is_shell_process_executable` 呼出し | 272-277 | F3 | task executable 審査 |
| task loop `argv_shell_re` 呼出し | 278-280 | A10 | task argv metachar 審査 |

### `crates/cheaper-core/src/policy/command.rs`

| site | lines | item | 内容 |
|------|-------|------|------|
| `argv_shell_re` in `ArgPattern::compile` | 249-251 | A10 | 宣言 pattern の metachar 審査（wildcard 構造検査 252-271 は A6 として残す）|
| `is_shell_process_executable` in `resolve_command_policy` | 372-376 | A8 | declared basename の shell/shim 審査 |
| `is_denied_command_executable` in `resolve_command_policy` | 377-381 | A9 適用点 1 | declared basename の deny 審査 |
| `command_denied_executable_re()` | 160-189 | A9 | builtin deny regex 定義（disposition は ADR）|
| `is_denied_command_executable()` | 196-207 | A9 | deny predicate（同上）|
| `is_relative_executable_path()` | 214-223 | B4 | workspace-relative 宣言 path の pre-filter helper |

### `crates/cheaper-tools/src/tools/command/mod.rs`

| site | lines | item | 内容 |
|------|-------|------|------|
| `within_workspace()` | 167-170 | B4 | canonical path の workspace 内判定 helper |
| `is_relative_executable_path` pre-filter (a) | 213-219 | B4 | 宣言 path の委譲前 reject |
| `is_denied_command_executable` | 230-237 | A9 適用点 2 | pin 時 resolved basename の deny 再検査 |
| `within_workspace` post-realpath (b) | 238-244 | B4 | realpath 後の workspace 内 reject |
| `native_executable` 呼出し | 255-260 | B2 | **残す**（再解決 vehicle。内部の審査は executable.rs 側で除去）|
| `resolved != pinned` pin-equality | 261-266 | B2 | **残す**（TOCTOU 対策本体）|
| `is_denied_command_executable` | 267-272 | A9 適用点 3 | spawn 前再検証の deny |
| `within_workspace` | 273-278 | B4 | spawn 前再検証の workspace 再査 |
| `argv_shell_re` in `authorize_args` | 468-475 | A10 | runtime argv の metachar reject（pattern match 476-487 は A6 として残す）|
| import 行 | 25-29 | — | 除去に伴う import 整理 |

### `crates/cheaper-tools/src/tools/proc/executable.rs`

| site | lines | item | 内容 |
|------|-------|------|------|
| `native_magic_supported()` | 74-86 | B3 | magic-byte 判定定義 |
| `is_shell_process_executable` in `native()` | 92-94 | F3 | candidate 内の shim 検査 |
| win32 `.exe/.com` 拡張子要求 | 99-104 | — | **残す**（spawn viability — Windows は `.exe`/`.com` 以外を直接 spawn 不能）|
| X_OK | 105-107 | — | **残す**（spawn viability）|
| magic-byte read + 判定 | 108-117 | B3 | `native()` 内の magic 検査 |
| `native_executable()` | 124-126 | B2 | **残す**（pin 再検証の再解決 wrapper）|
| front-door `is_shell_process_executable` | 143-145 | F3 | declared command の shim 審査 |
| bare `node` reject | 163-173 | B5 | `node` 固定ルール |
| `unsupported()` message | 35-42 | — | 「shell and script shims are unsupported」の表現訂正（message churn）|
| 設計固定ルール doc comment | 6-11 | — | bare `node` ルール記述の訂正 |

### 関連する呼出し・記述 site（message/comment churn）

- `crates/cheaper-tools/src/tools/process/prepare.rs:127-136` —
  `resolve_process_executable` 呼出し（F3 の実行経路）。同:140-145
  `unsupported_empty_command` の message が shim 表現を含む。
- `crates/cheaper-tools/src/registry.rs`（`pin_command_grants` 呼出し
  周辺・content-gate comment）— 「deny 該当・workspace-resident」を
  謳う comment の訂正。
- `crates/cheaper-tools/src/tools/command/mod.rs` module doc（L1-17）・
  `pin_command_grant`/`verify_pinned_executable`/`authorize_args`/
  `check_command_execution` の doc comment — deny/residency/metachar
  言及の訂正。

## Keep Site Inventory（触れない code site）

| site | path | item | 理由 |
|------|------|------|------|
| `tokenize_shell` | `crates/cheaper-cli/src/hook/routing_policy.rs`（自前の metachar/unsafe token 判定、`argv_shell_re` 非依存） | D1 | hook 文字列の解析 correctness — 拒否ではなく viability 分類 |
| `resolve_process_executable` の PATH 検索→canonical realpath | `crates/cheaper-tools/src/tools/proc/executable.rs:146-190` | B1 | pin の決定性 — 除去は内部の審査分岐のみ |
| `verify_pinned_executable` の `resolved != pinned` | `crates/cheaper-tools/src/tools/command/mod.rs:261-266` | B2 | TOCTOU 対策本体 |
| `native_executable` wrapper | `executable.rs:124-126` | B2 | 再解決 vehicle |
| `ArgPattern::compile` の wildcard 構造検査 | `command.rs:252-271` | A6 | argv pattern 言語（宣言制約）|
| `ArgPattern::matches` / `authorize_args` の pattern match | `command.rs:280-302`・`command/mod.rs:476-487` | A6 | argv 照合 |
| `relative()`（cwd/inputs/outputs の workspace-relative 検査）| `process.rs:153-174` | A7 | 宣言境界 |
| env dup/NUL 検査 | `process.rs:290-308`・`command.rs:401-419` | A7 | env 宣言境界（NUL は argv とは別規約で残る）|
| lifecycle bound（timeout/killGrace/heartbeat/maxOutputBytes）| `process.rs`・`command.rs` 各所 | C3 | bounded lifecycle |
| nonempty grants/tasks gate（C4）・writeScope×command gate（C5）| `registry.rs` | C4/C5 | fail-closed admission |
| `PathDenylist`/`denyPaths`（E2）| `deny_paths.rs`・`proc/mod.rs`・`resolve_process_executable` の `denylist` 引数 | E2 | credential path deny — **executable deny list とは別物**。混同注意 |
| `spawn_failure_is_async` の ENOEXEC 分類 | `proc/spawn` 系 | C2 | spawn 層の OS error shape |
| `argv_shell_re` regex 定義 | `process.rs:134-137` | A10 残渣 | `route/resolve.rs:285` が現役参照（issue 4 で整理）|
| route viability の `check_command_execution` 委譲 | `route/resolve.rs:304-335` | D2 | routing≠authorization — predicate の中身が変わるだけで経路は不変 |

## Affected Tests / Fixtures Inventory

### golden fixture・golden driver

- `crates/cheaper-core/tests/golden/fixtures/policy/process.json`
  - `resolveProcessPolicy` error case 4 件 — "invalid shell
    executable"（F3）・"invalid script shim"（B6）・"invalid argv
    shell syntax"・"invalid argv subshell"（A10）→ 受理へ転換する
    ため更新または削除。
  - `isShellProcessExecutable` op case 14 件 — predicate 削除に伴い
    case 削除。
- `crates/cheaper-core/tests/golden/policy_process.rs` —
  `is_shell_process_executable` import・op 分岐（L1-3, 22-24）の除去。
- `crates/cheaper-core/tests/golden/fixtures/policy/load.json` —
  case "invocation denied executable keeps inner code"（invocation
  `python` grant の `INVALID_RUNTIME_POLICY` pin — A9 適用点 1）。
- `crates/cheaper-tools/tests/fixtures/process.json` — case "bare
  node task is an explicit Rust error"（B5 の `expectRust` divergence
  pin）。differential driver `tests/differential.rs` は `<NODE>`
  task を fixture child へ翻訳しており他 case は非依存。

### unit test（src 内 `#[cfg(test)]`）

- `crates/cheaper-core/src/policy/process.rs` —
  `shell_executable_detection`・`rejects_shell_executables_and_argv_syntax`。
- `crates/cheaper-core/src/policy/command.rs` —
  `resolve_rejects_denied_and_shell_executables`・
  `relative_executable_path_prefilter`・部分:
  `denied_executables_cover_interpreters_wrappers_and_versioned_names`
  （`is_shell_process_executable` 使用行）・`argv_pattern_compile_and_match`
  （metachar compile 拒否 sub-case — wildcard 構造拒否は残る）。
- `crates/cheaper-tools/src/tools/proc/executable.rs` —
  `bare_node_is_an_explicit_policy_violation`・
  `error_subject_is_parameterized_by_caller`（`"node"` を検査対象に
  使用）・`refuses_script_shims_and_executable_text`・
  `nonexistent_or_unsupported_magic_fails_closed`・
  `fails_closed_when_search_path_has_no_supported_executable`
  （`"node.exe"` を含む）。

### integration test

- `crates/cheaper-tools/tests/proc_command.rs` —
  `declared_denied_executables_fail_policy_resolution`（A8+A9）・
  `pinned_resolved_basename_deny_rejects`（A9 適用点 2）・
  `workspace_resident_executables_are_rejected`（B4）・
  `shell_metacharacters_are_rejected`（A10）・
  `spawn_time_revalidation_detects_swapped_executable`（B3 経由 —
  内容差替は検出対象外へ。path 差替は維持）・
  `check_command_execution_shares_execute_predicate`（metachar
  sub-assertion）・module doc header（L15-16）。message 断言:
  `unpinnable_executable_fails_closed`。
- `crates/cheaper-tools/tests/proc_degrade.rs` —
  `pin_failure_stays_hard_even_when_opted_in`（message 断言 —
  `unsupported()` 文言 restate の影響範囲）。
- `crates/cheaper-tools/tests/proc_process.rs` —
  `node_task_is_an_explicit_policy_violation`（B5）・header
  comment（L3-4）。
- `crates/cheaper-cli/tests/run_dev_seam.rs` —
  `run_fake_stream_grant_deny_listed_executable_rejected`（A9 —
  `python` `--grant` が `INVALID_RUNTIME_POLICY` でなくなる）。
- `crates/cheaper-cli/tests/regression/cases/run-grant-deny.json` +
  `tests/regression/expected/run-grant-deny.json` — 同上の
  regression pin（環境依存になるため削除または再用途化）。
- `crates/cheaper-cli/tests/integration/policy.rs` —
  `policy_grant_add_rejects_invalid_spec_without_writing` の
  `{"executable":"python"}` spec case（`a*b` argv case は wildcard
  構造検査が残るため有効のまま）。
- comment churn のみ: `crates/cheaper-cli/tests/workflow.rs`、
  `crates/cheaper-cli/tests/workflow_dev_seam.rs`、
  `crates/cheaper-tools/src/registry.rs`。

### 非依存（検証済み・変更不要）

- `crates/cheaper-workflow/tests/process.rs` — 実 `node` の
  **absolute path** を使うため bare `node` reject 非該当。
- `benchmarks/scenarios/scenarios.json` — `{node}` →
  `process.execPath`（absolute）置換で非該当。
- `crates/cheaper-cli/tests/integration/{route,route_run,hook}.rs`・
  `crates/cheaper-tools/tests/proc_invocation.rs` — grant
  executable は test binary 自身（workspace 外 native exe）。
- `crates/cheaper-cli/resources/policy.yaml` — grants/tasks 空。

## 観測差分（removal 後の挙動変化）

1. `\0`・`\r`・`\n`・`|&;<>`・`` ` ``・`$(` を含む argv — 現行
   `POLICY_VIOLATION` 事前拒否（A10）→ spawn 層の OS/NUL エラーへ
   縮退。拒否自体は維持されるが error 分類が変わる。
2. pin 済み executable の**内容**差替（同 path の非 native 化）—
   現行 `native_executable` の magic-byte で検出 → B3 除去で検出
   対象外。path/symlink retarget（`resolved != pinned`）は B2
   として検出維持。
3. shebang 無し +X text file の spawn — ENOEXEC 時に glibc
   `execvpe` が `/bin/sh` fallback し得る。spawn API 層の
   shell-free は維持されるが、fallback は OS/libc 既定動作で
   cheaper の保証対象外（design B3 補足）。
4. route viability — 宣言 executable が workspace-resident・
   shim・deny 該当の grant が `pin_command_grant` を通るようになり
   NoGrant → Runnable へ変わる経路が生じる。viability の metachar
   unsafe 分類（`resolve.rs:285`）は `argv_shell_re` 残置により
   現行維持（issue 4 の整理対象）。
5. `unsupported()` / pin 失敗系の error message —「shell and
   script shims are unsupported」等の文言は実態と乖離するため
   restate が必要（誠実性）。

## Open Questions

- `command_denied_executable_re`・`is_denied_command_executable`
  （A9 predicate 定義）を issue #164 まで残置するか、call site と
  共に削除するか — ADR で記録（issue #164 は operator 宣言 pattern
  を compile する別 predicate を要するため直接再利用は無い）。
- `denied_executables_cover_...` unit test の builtin 一覧は
  issue #164/#169 の migration 資料として docs へ移す対象。

## Promotion Candidates

- [x] spec-doc: `../specs/exec-contract-content-audit-removal.md`
- [x] adr-doc: `../adr/exec-contract-content-audit-removal.md`

## Sources and Relations

- 起点: issue #163（#151 実装分解 1/7）
- 上流: 親 chain（#151）の design・spec・ADR・discovery — 全て
  `references` のみ（cross-chain lineage は張らない）
