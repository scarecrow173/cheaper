---
id: "DESIGN-034KIsSCvVMAxGDZY1dQTd"
type: "design"
status: "approved"
title: "exec-contract cleanup — 内容審査を mandatory 層から除去"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
  changes:
    added:
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/cases/run-grant-invalid-spec.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-grant-invalid-spec.json"
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/process.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/executable.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_process.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/process.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-tools/tests/fixtures/process.json"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/workflow_dev_seam.rs"
    deleted:
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/cases/run-grant-deny.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/run-grant-deny.json"
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
    - "../specs/exec-contract-content-audit-removal.md"
    - "../adr/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/exec-contract-content-audit-removal.md"
    - "../adr/exec-contract-content-audit-removal.md"
    - "../discovery/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../../crates/cheaper-core/src/policy/process.rs"
    - "../../crates/cheaper-core/src/policy/command.rs"
    - "../../crates/cheaper-tools/src/tools/command/mod.rs"
    - "../../crates/cheaper-tools/src/tools/proc/executable.rs"
    - "../../crates/cheaper-cli/src/route/resolve.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
  defers: []
  deferred-by: []
---

# exec-contract cleanup — 内容審査を mandatory 層から除去

## Context

issue #163（#151 実装分解 1/7・起点 issue）。親 design
（DESIGN-034KuDkWOmNXiBddEQ4XWK・approved）が分類した content-audit
クラス 8 項目のうち、remove 決定の 7 項目（A8・A10・B3・B4・B5・
B6・F3）と A9 deny list の **mandatory 適用点**を、実行契約の
mandatory 層（grant/task resolve → pin → spawn 前再検証 → argv
認可）から一括除去する。除去単位・deny predicate の disposition・
`argv_shell_re` の残置は ADR-034KI3r2xdthh6rKDFY517 で確定済み
（Option A 一括除去 + predicate 定義も call site と共に削除）。
実測 site inventory は DISC-JLHFz6x4ZuhzYZrpfrG2QV が file:line
精度で固定済み。

本 design は「どの行を消し、何を残し、test をどう張り替えるか」の
実装契約を固定する。除去後に cheaper が強制するのは**宣言された
実行契約のみ** — shell-free argv spawn・grant/task 認可・pin
同一性・wildcard 構造・cwd/env 宣言境界・lifecycle 境界・
`PathDenylist`。内容審査の再実装・補償 check の追加は spec R11
で禁止される。

## Scope

### In Scope

- `crates/cheaper-core/src/policy/process.rs`・`policy/command.rs`、
  `crates/cheaper-tools/src/tools/command/mod.rs`・
  `tools/proc/executable.rs` の審査分岐・helper・regex 定数の除去
  （Removal Contract の各 row）。
- `crates/cheaper-tools/src/tools/process/prepare.rs`・
  `crates/cheaper-tools/src/registry.rs` の error message・doc
  comment の誠実化（message churn）。
- error 分類変化（`\0` argv の POLICY_VIOLATION → spawn 層
  TOOL_EXECUTION_FAILURE 縮退）の pin 化。
- Test/Fixture Churn Plan の全項目（golden fixture・unit・
  integration・dev-seam・regression の更新・削除・再用途化）。
- 後続 issue #164 のための適用点構造の保全（下記「#164 境界」）。

### Out of Scope

- **issue #164** — `security.execDenyPatterns` schema・opt-in deny
  predicate の実装・builtin 旧既定値の docs 記録。
- **issue 4 scope** — route viability 層の審査依存整理
  （`route/resolve.rs:285` の `argv_shell_re` unsafe 分類・
  `is_interactive`/`CWD_CHANGING_COMMANDS` 等 viability 内
  curated 分類の見直し）。viability 経路自体は不変。
- **issue 3/5/6/7** — grant/task merge・substitute mode・trusted
  flag 主体・docs 全体 sweep。
- Keep-Boundary Pins に列挙する全 site（`tokenize_shell`・pin
  同一性・wildcard 構造・`relative()`・env NUL/dup・lifecycle・
  `PathDenylist`・`.exe/.com`・X_OK・`spawn_failure_is_async`）。
- 削除した審査を別条件で再実装する一切の変更（spec R11）。

## Components and Boundaries

### 除去 vs 残置の境界原則

除去対象は「宣言された executable・argv の**内容**を審査して
reject する」分岐のみ。残るのは (a) spawn viability（OS が spawn
可能か）、(b) 宣言契約の強制（wildcard 構造・argv 照合・cwd/env
境界・pin 同一性・denylist・lifecycle）の 2 クラス。判別に迷う
場合は「reject の根拠が file/名の内容か、宣言と実体の一致か」で
分ける — 後者は全て残す。

### Removal Contract — `crates/cheaper-core/src/policy/process.rs`

| site | lines(現行) | item | disposition |
|------|------------|------|-------------|
| `shell_executable_re()` | 117-125 | A8/B6 | **削除** — 唯一の caller は `is_shell_process_executable` |
| `script_ext_re()` | 127-130 | B6 | **削除** — 同上 |
| `argv_shell_re()` | 134-137 | A10 残渣 | **残置** — 唯一の残存 caller は `cheaper-cli` `route/resolve.rs:285`。doc comment（「cheaper-tools の `command` execute が cross-crate で参照するため `pub`」）は stale になるため「`route/resolve.rs` viability が参照する間 `pub` を維持」へ訂正 |
| `is_shell_process_executable()` | 141-148 | A8/F3 | **削除** — caller は process task loop・command.rs resolve・executable.rs の 2 箇所・golden driver のみで全て除去 |
| task loop `is_shell_process_executable` 呼出し | 272-277 | F3 | **削除** — error `"shell or script shim executable is forbidden"` ごと除去 |
| task loop `argv_shell_re` 呼出し | 278-280 | A10 | **削除** — error `"shell syntax in argv is forbidden"` ごと除去 |
| `relative()` | 153-174 | A7 | **残す** — cwd/inputs/outputs の workspace-relative 検査 |
| env dup/NUL 検査 | 290-308 | A7 | **残す** |

### Removal Contract — `crates/cheaper-core/src/policy/command.rs`

| site | lines(現行) | item | disposition |
|------|------------|------|-------------|
| `command_denied_executable_re()` | 160-189 | A9 | **削除** — builtin 定数 regex。#164 は operator 宣言 pattern を compile する**別 predicate** を要し再利用価値なし（ADR Option D 棄却・YAGNI） |
| `is_denied_command_executable()` | 196-207 | A9 | **削除** — 3 適用点と共に消える |
| `is_relative_executable_path()` | 214-223 | B4 | **削除** — 唯一の caller は pin pre-filter |
| `ArgPattern::compile` の `argv_shell_re` 検査 | 249-251 | A10 | **削除** — error `"shell syntax in argv pattern is forbidden"` ごと除去。doc comment（242-244）の metachar 言及も訂正 |
| `ArgPattern::compile` wildcard 構造検査 | 252-271 | A6 | **残す** — bare `*` 位置・prefix `*`・複数 `*` の構文規約 |
| `is_shell_process_executable` in `resolve_command_policy` | 372-376 | A8 | **削除** — error 同上 |
| `is_denied_command_executable` in `resolve_command_policy` | 377-381 | A9 適用点 1 | **削除** — error `"interpreter or exec-capable executable is forbidden"` ごと除去 |
| import 行 | 23-28 | — | `argv_shell_re`・`is_shell_process_executable` を import から外す（`bounded_duration`・`relative`・`str_vec`・`ProcessHeartbeat`・`DEFAULT_*`・`MAX_PROCESS_OUTPUT_BYTES` は残る）。`use crate::nodepath`（23 行目）も除去 — 唯一の consumer は `is_relative_executable_path` の :219-220 で dead import 化する |
| env dup/NUL・`relative`・`resolve_command_grant`・`project_grant`・`command_grant_resources` | 各所 | A7/C9/E6 | **残す** |

### Removal Contract — `crates/cheaper-tools/src/tools/command/mod.rs`

| site | lines(現行) | item | disposition |
|------|------------|------|-------------|
| `within_workspace()` | 167-170 | B4 | **削除** — caller は pin post-realpath・verify の 2 箇所のみ |
| `is_relative_executable_path` pre-filter | 213-219 | B4 | **削除** — `"workspace-resident executables are not grantable"` error ごと除去 |
| `resolve_process_executable` 呼出し | 220-229 | B1 | **残す** — pin の決定的解決経路（denylist 検査は executable.rs 内で残る） |
| `is_denied_command_executable` | 230-237 | A9 適用点 2 | **削除** — `"resolved executable is interpreter or exec-capable"` error ごと除去 |
| `within_workspace` post-realpath | 238-244 | B4 | **削除** — error 同上 |
| `native_executable` 呼出し | 255-260 | B2 | **残す** — 再解決 vehicle |
| `resolved != pinned` pin-equality | 261-266 | B2 | **残す** — TOCTOU 対策本体（`"pinned executable path changed since pin"`） |
| `is_denied_command_executable` | 267-272 | A9 適用点 3 | **削除** |
| `within_workspace` | 273-278 | B4 | **削除** |
| `verify_pinned_executable` 签名 | 254 | — | `root` 引数を**除去**（唯一の用途は `within_workspace` で dead param 化）。`check_command_execution` の call site も追随 |
| `authorize_args` の `argv_shell_re` loop | 468-475 | A10 | **削除** — `"argument contains forbidden shell metacharacters"` error ごと除去 |
| `authorize_args` の pattern match | 476-487 | A6 | **残す** — `grant.argv: None` なら無条件受理・`Some` なら compile + `matches` 照合のみ |
| import 行 | 24-29 | — | `is_denied_command_executable`・`is_relative_executable_path`・`argv_shell_re` を import から外す。`use cheaper_core::nodepath`（24 行目）も除去 — consumer は `within_workspace` の :168-169 と削除する `within_workspace_boundary` test（:869,878）のみで dead import 化する |
| pin map 不在の fail-closed（`"no pinned executable"`） | 451-458 | C4 系 | **残す** |
| module doc・各 fn doc comment・`check_command_execution` 内 comment | 1-17, 172-205, 248-253, 403-427, 447-450, 516-518 | — | deny/residency/metachar 言及を除去後の実態へ restate（:447-450 の「deny/residency の同一 predicate」は `native_executable` 再解決 + path 同一性のみの記述へ） |

### Removal Contract — `crates/cheaper-tools/src/tools/proc/executable.rs`

| site | lines(現行) | item | disposition |
|------|------------|------|-------------|
| `native_magic_supported()` | 74-86 | B3 | **削除** — 唯一の caller は `native()` 内 magic 判定 |
| `is_shell_process_executable` in `native()` | 92-94 | F3 | **削除** |
| win32 `.exe/.com` 拡張子要求 | 99-104 | spawn viability | **残す** — Windows は `.exe`/`.com` 以外を直接 CreateProcess 不能 |
| X_OK | 105-107 | spawn viability | **残す** |
| magic-byte read + 判定 | 108-117 | B3 | **削除** — `use std::io::Read`（13 行目）はこの読み込み専用のため import ごと除去 |
| import 行 | 13, 17 | — | `use std::io::Read`・`use cheaper_core::policy::process::is_shell_process_executable`（17 行目 — consumer は `native()` :92 と front-door :143 で共に除去）を import から外す。`nodepath`・`PathDenylist`・`realpath`/`resolve_workspace_path`/`SymlinkPolicy` は残る |
| `native_executable()` wrapper | 124-126 | B2 | **残す** — pin 再検証の再解決 vehicle。doc comment の「native magic」言及は訂正 |
| front-door `is_shell_process_executable` | 143-145 | F3 | **削除** — declared `sh`/`bash`/`*.sh` は PATH・path 解決へ進む |
| workspace-relative 解決 + denylist 検査 | 146-157 | B1/E2 | **残す** — `resolve_workspace_path`（workspace 境界）と `PathDenylist` は宣言契約 |
| bare `node` reject | 163-173 | B5 | **削除** — `node` は PATH 解決の通常経路へ |
| PATH candidate loop | 174-189 | B1 | **残す** — 内部の `native()` のみ縮小 |
| `unsupported()` message | 35-42 | — | **restate** — `"{subject} requires a supported native executable; shell and script shims are unsupported"` は実態と乖離。`"{subject} could not be resolved to a spawnable executable"` 系へ置き換え（`native`/`shell`/`shim` 語彙を残さない — 残る失敗条件は realpath/is_file/win32 ext/X_OK = spawn viability） |
| `resolve_process_executable` doc | 128-137 | — | **restate** — bare `node` 固定ルール記述を除去 |
| 設計固定ルール doc comment | 6-11 | B5 | **restate** — 「Rust は bare `node` を PATH 解決しない」の記述を削除し、B5 除去後の通常解決を記述 |

### Removal Contract — message/comment churn 関連 site

- `crates/cheaper-tools/src/tools/process/prepare.rs:127-136` —
  `resolve_process_executable` 呼出しは F3 の実行経路としてそのまま
  残るが、`unsupported_empty_command`（140-145）の message が
  `"supported native executable; shell and script shims are
  unsupported"` を含むため `unsupported()` と同じ文言規約で
  restate。
- `crates/cheaper-tools/src/registry.rs:1202-1207` — pin 導入の
  comment「pin 不能・deny 該当・workspace-resident の grant は
  runtime 構築自体を `POLICY_VIOLATION` で fail」を「pin 不能の
  grant は fail」へ訂正。
- `crates/cheaper-tools/tests/proc_command.rs:12-18`・
  `proc_process.rs:3-8`・`crates/cheaper-cli/tests/workflow.rs:505`・
  `workflow_dev_seam.rs:166`・`crates/cheaper-tools/tests/
  differential.rs:9-15,613` の header comment — 「deny/
  workspace-resident/PATH 解決しない」等の前提記述を訂正
  （test 本体の変更が伴うのは proc_command/proc_process/
  differential のみ）。
- `crates/cheaper-cli/src/policy.rs:8-9`（module doc）・
  `:245-247`（`write_validated` doc）— 「deny list」の言及を
  除去後の検査内容（argv pattern compile・env/cwd 規則）へ
  restate。
- `crates/cheaper-core/src/policy/load.rs:13`（module doc）・
  `:151-152`（Partial validate comment）— 同じく「deny list」
  言及を除去後の検査内容へ restate。
- `crates/cheaper-cli/tests/run_dev_seam.rs:359-362`・
  `crates/cheaper-tools/tests/proc_invocation.rs:581-584`
  （`self_grant` doc）— 「workspace-resident 拒否に掛からない」
  「absolute native executable」等の前提記述を「pin は ambient
  PATH 非依存で deterministic」等の除去後の趣旨へ訂正。
- `crates/cheaper-tools/src/tools/proc/executable.rs:246-252`・
  `crates/cheaper-tools/src/tools/command/mod.rs:628-634` の
  `native_bytes()` test fixture — magic-byte 内容審査の除去で
  fixture 名・前提が stale 化するため rename・restate（file
  内容はもはや検査対象ではなく、実在性・実行 bit だけが意味を
  持つ）。
- `crates/cheaper-cli/src/route/resolve.rs:279-281` —
  「grant 側は metachar を拒否するため安全性分類へ倒す」の
  前提は本 issue で一時的に偽になるが、viability 層の審査依存
  整理は issue 4 の scope。**本 issue では comment を修正
  しない**（known-stale としてここに記録するのみ）。

### #164 境界（何を残し何を消すか）

ADR-034KI3r2xdthh6rKDFY517 の決定に従い、deny predicate
**機械は全て削除**する:

- 削除 — `command_denied_executable_re`・
  `is_denied_command_executable`・3 適用点・関連 error message・
  `denied_executables_cover_interpreters_wrappers_and_versioned_names`
  unit test（builtin 一覧の docs 移管は #164/#169 の scope）。
- #164 が再利用するのは **適用点構造のみ** — `resolve_command_policy`
  の declared basename 段階・`pin_command_grant` の resolved
  basename 段階・`verify_pinned_executable` の spawn 前再検証
  という 3 つの差し込み位置。本 issue は if-block を丸ごと消して
  自然な再挿入点を残すだけで、seam・flag・stub は一切残さない。
- `verify_pinned_executable(name, pinned)` の縮小签名（root 除去）は
  #164 の deny pattern 照合が `root` を要しないため問題ない
  （opt-in deny は basename match であり workspace residency を
  復活させない）。

## Error Contract Changes

### 消える error variant

**`INVALID_RUNTIME_POLICY`**（policy resolve 層）:

- `"shell or script shim executable is forbidden"` —
  `process.rs` task loop・`command.rs` resolve の 2 箇所。
- `"interpreter or exec-capable executable is forbidden"` —
  `command.rs` resolve（A9 適用点 1）。
- `"shell syntax in argv is forbidden"` — `process.rs` task loop。
- `"shell syntax in argv pattern is forbidden"` —
  `ArgPattern::compile`（A10）。

**`POLICY_VIOLATION`**（pin/verify/execute 層）:

- `"workspace-resident executables are not grantable"` —
  pin pre-filter・pin post-realpath・verify の 3 箇所。
- `"resolved executable is interpreter or exec-capable"` —
  pin・verify の 2 箇所。
- `"argument contains forbidden shell metacharacters"` —
  `authorize_args`。
- `"{subject} task \"node\" is unsupported: the Rust runtime has no
  Node.js host executable"` — `resolve_process_executable`（B5）。

### 残るが文言を変える variant

- `unsupported()`（executable.rs）・`unsupported_empty_command`
  （prepare.rs）— code は `POLICY_VIOLATION` のまま。message を
  `"{subject} could not be resolved to a spawnable executable"`
  系へ restate。観測側で message assertion を持つのは
  `unpinnable_executable_fails_closed`・
  `pin_failure_stays_hard_even_when_opted_in`。
- `"pinned executable no longer resolves to a supported native
  executable"` → `"pinned executable no longer resolves to a
  spawnable executable"` 系へ restate（native→spawnable 語彙）。

### 変わらない variant

- `POLICY_VIOLATION`: unknown grant/task・argv pattern mismatch・
  `"no pinned executable"`・`"pinned executable path changed since
  pin"`・denylist 由来の `denied_declared_path`・WorkspaceEscapeError
  （declared relative path の escape）。
- `INVALID_RUNTIME_POLICY`: schema 違反・duration 上限・wildcard
  構造違反（`a*b`/`**`/非末尾 `*`）・`relative()` workspace-
  relative 規約・env dup/NUL。

### `\0` argv の degradation path（error 分類の縮退）

現行: `authorize_args` の metachar check が `\0` を含む token を
`POLICY_VIOLATION` で事前拒否。除去後の観測経路:

```
authorize_args（pattern match のみ → argv 省略/rest marker で受理）
  → execute → ProcPrepared.argv → build_command → Command::args
  → tokio Command::spawn → io::Error{kind: InvalidInput,
    raw_os_error: None}（std の "nul byte found in provided data"）
  → spawn_failure_is_async = false（raw_os_error 非保持）
  → spawn_code → "SPAWN_ERROR"（kind が NotFound/PermissionDenied 以外）
  → TOOL_EXECUTION_FAILURE "managed process spawn failed"
     + details.spawnCode="SPAWN_ERROR" + cause（io error）
```

- `\0` **以外**の metachar（`'\r'`・`'\n'`・`|&;<>`・`` ` ``・`$(`）は
  literal argv として子へ届き spawn 自体は成功する（shell-free
  spawn では単なるデータ）。拒否が残るのは NUL のみであり、それも
  OS/Rust 側の argv 制約であって cheaper の policy 層ではない。
- `env` の NUL 規約（`process.rs:304`・`command.rs:415`）と
  `relative()` の `'\0'` 拒否は**別規約として維持** — 混同しない。
- **declared executable 内の `\0` は縮退しない** — `\0` を含む
  executable は realpath が失敗し、restate 後の `unsupported()`
  message で pin 層 `POLICY_VIOLATION` のまま失敗する（spawn
  層へは到達しない）。argv token の `\0` と異なり error 分類は
  不変。この pin は `unpinnable_executable_fails_closed` 系の
  fail-closed test（`\0` 入り executable spec → `POLICY_VIOLATION`
  + restate 後 message）で観測可能。
- **argv pattern 内の literal `\0`** — `ArgPattern::compile` は
  wildcard 構造のみを検査するため `\0` を含む pattern 要素は
  compile を通り、それに match した arg は上記 argv token と
  同じく spawn 層で失敗する。
- この分類変化を観測するのは `proc::run` 経由の tool error →
  `command::execute` → integration test（下記 Churn Plan で
  `shell_metacharacters_are_rejected` の再用途先として pin 化）。

## Test / Fixture Churn Plan

凡例: **削除** / **更新**（assertion・expected の書換え）/
**再用途**（前提・assertion を別の残存契約へ張り替え、case の
存在意義を維持）。

### golden fixture・golden driver

| case/site | 内容 | disposition |
|-----------|------|-------------|
| `policy/process.json` "invalid shell executable" | `["bash","-c","x"]` task → INVALID_RUNTIME_POLICY | **更新** — resolve 受理へ転換し expected を resolved policy の成功 shape へ。case 名も `"shell executable task is accepted"` 系へ rename |
| 同 "invalid script shim" | `["run.sh"]` task | **更新** — 同上（`.sh` 受理 pin） |
| 同 "invalid argv shell syntax" | `["echo","a|b"]` argv | **更新** — 同上（metachar argv 受理 pin） |
| 同 "invalid argv subshell" | `["echo","$(x)"]` argv | **更新** — 同上 |
| 同 `isShellProcessExecutable` op 14 件 | predicate 参照 | **削除** — predicate ごと消える |
| `tests/golden/policy_process.rs` | import・op 分岐（L1-3, 22-24） | **更新** — `is_shell_process_executable` import・`"isShellProcessExecutable"` op arm を除去 |
| `policy/load.json` "invocation denied executable keeps inner code" | invocation `python` grant → INVALID_RUNTIME_POLICY + `layer=invocation` | **再用途** — spec を `{"executable":"python","argv":["a*b"]}` へ変更（wildcard 構造違反で同一 code+layer を deterministic に維持）。case 名は `"invocation invalid grant spec keeps inner code"` 系へ rename。expected 不変 |
| `policy/schema.json` "process shell task rejected pre-merge" | `mode:complete`・`op:validatePolicySchema` で `command:["sh"]` task（case は :714-774）→ INVALID_RUNTIME_POLICY | **更新** — F3 除去で `validate_policy_schema` 内の `resolve_process_policy` が受理するため golden mismatch になる。expected を `{"ok":{"policy":<validated input echo>}}` の success shape（`validatePolicySchema` op は resolved ではなく validated 入力を echo する）へ更新し、case 名は `"process shell task accepted pre-merge"` 系へ rename |
| `tools/tests/fixtures/process.json` "bare node task is an explicit Rust error" | `expectRust` divergence pin（803-841 行目 — `POLICY_VIOLATION` patch） | **更新** — `expectRust` block を除去し base expected（実 `node` 実行 → "bare node" stdout）へ戻す。case 名は `"bare node task resolves via PATH"` 系へ rename。前提: mise toolchain の node が PATH 上にあり `cargo test` が ambient PATH を継承する（`mise run check` 経路では保証される） |

### unit test（src 内 `#[cfg(test)]`）

| test | file | disposition |
|------|------|-------------|
| `shell_executable_detection` | process.rs | **削除** |
| `rejects_shell_executables_and_argv_syntax` | process.rs | **再用途** — `bash` task・metachar argv task が受理される positive pin へ（rename: `accepts_shell_executables_and_argv_metachars` 系） |
| `denied_executables_cover_interpreters_wrappers_and_versioned_names` | command.rs | **削除** — builtin 一覧の docs 移管は #164/#169 |
| `resolve_rejects_denied_and_shell_executables` | command.rs | **再用途** — `python3`/`python3.13t`/`env`/`ld-linux-x86-64.so.2`/`bash`/`deploy.sh` が受理される declared-name acceptance pin へ（spec AC「python/ssh/tar/.sh を含む grant が拒否されない」の resolve 層 pin） |
| `argv_pattern_compile_and_match` | command.rs | **更新** — `bad` list から `"a|b"`・`"x;y"`・`"v$(i)"` を除去（literal token として受理されることを assert へ追加可）。`"a*b"`・`"**"`・`"*a"`・非末尾 bare `*`・resolve 経由 `a*b` の INVALID_RUNTIME_POLICY は維持 |
| `relative_executable_path_prefilter` | command.rs | **削除** |
| `invocation_denied_executable_keeps_inner_code_with_layer` | load.rs（:527-545） | **再用途** — `{"executable":"python"}` spec は受理されるようになり `unwrap_err` が panic するため、spec を `{"executable":"python","argv":["a*b"]}`（wildcard 構造違反 → `ArgPattern::compile` の INVALID_RUNTIME_POLICY）へ変更し、inner code 保持 + `details.layer="invocation"` + `"invalid command policy"` を PATH 非依存で deterministic に pin。rename: `invocation_invalid_grant_spec_keeps_inner_code_with_layer` 系。`load.rs:13`・`:151-152` の「deny list」comment は上記 churn 対象 |
| `bare_node_is_an_explicit_policy_violation` | executable.rs | **再用途** — `search_path: Some(bin_dir)`（tempdir に `node`/`node.exe` を配置）で `node` が解決される positive pin へ。`Some("")`（PATH 遮断）は引き続き unsupported error（rename: `bare_node_resolves_via_search_path` 系） |
| `error_subject_is_parameterized_by_caller` | executable.rs | **更新** — `"node"` を使った subject pin を missing executable（例 `"missing-tool"`）へ置き換えて subject 引数の文言規約のみ pin |
| `refuses_script_shims_and_executable_text` | executable.rs | **再用途** — `pnpm.cmd`/`script`(+X) の解釈を platform 分岐で pin: unix では +X 付き shim/text は**解決される**（B3/B6 除去の positive pin）、win32 では `.cmd`/`script`（非 `.exe/.com`）が引き続き viability で reject（`.exe` 拡張子の text file は解決される） |
| `nonexistent_or_unsupported_magic_fails_closed` | executable.rs | **更新** — `missing` は引き続き fail-closed。`wrong`（非 magic 内容 +X）は解決される positive pin へ。`short` は内容判定をやめたため両 platform で解決される（rename: `unresolvable_fails_closed_regardless_of_content` 系） |
| `fails_closed_when_search_path_has_no_supported_executable` | executable.rs | **維持** — `"node.exe"`（literal `node` と別名）・`"missing-tool"` は PATH 解決不能で引き続き unsupported。assertion 変更なし |
| `authorize_args_rejects_metachar_before_pattern_match` | command/mod.rs | **再用途** — metachar token が rest marker pattern で受理されること・pattern 不一致だけが POLICY_VIOLATION である pin へ |
| `authorize_args_without_pattern_accepts_any_count` | command/mod.rs | **更新** — `"a;b"` 受理へ（metachar reject 部を除去） |
| `pin_rejects_relative_executable_path_before_resolution` | command/mod.rs | **再用途** — tempdir 内 `bin/tool(.exe)` を実在させて declared `"bin/tool"` の pin 成功 pin へ（B4 positive pin。workspace-relative 解決は `resolve_workspace_path` 経由で escape 検査は残る） |
| `pin_rejects_absolute_path_inside_workspace` | command/mod.rs | **再用途** — workspace 内 absolute path の pin 成功 pin へ |
| `pin_fails_closed_on_unresolvable_or_denied_executable` | command/mod.rs | **更新** — unresolvable executable の fail-closed 部は維持。`python3` resolve reject の副 assert を除去し受理へ転換 |
| `verify_pinned_rejects_missing_swapped_or_workspace_paths` | command/mod.rs | **更新** — missing path は引き続き error（restate 後 message）。workspace 内 native file は検査通過（`resolved == pinned`）へ転換。pin map 不在の execute fail-closed は維持（rename: `verify_pinned_rejects_missing_accepts_workspace_paths` 系） |
| `within_workspace_boundary` | command/mod.rs | **削除** |

### integration test

| test | file | disposition |
|------|------|-------------|
| `declared_denied_executables_fail_policy_resolution` | proc_command.rs | **再用途** — `resolve_command_policy` で `python`/`ssh`/`tar`/`env`/`bash`/`deploy.sh` が受理される pin へ（policy resolve は PATH 非依存で deterministic。pin 層の受理は workspace-resident case で別途 pin） |
| `pinned_resolved_basename_deny_rejects` | proc_command.rs (cfg unix) | **再用途** — symlink `tool`→実 file `python3`（current_exe 複製）の pin が受理される positive pin へ（resolved basename が旧 deny 名でも pin される） |
| `workspace_resident_executables_are_rejected` | proc_command.rs | **再用途** — `./tool` relative・absolute inside-workspace の双方が pin・runtime 構築を通る pin へ |
| `unpinnable_executable_fails_closed` | proc_command.rs | **更新** — `POLICY_VIOLATION` は維持、message assertion を restate 後文言へ |
| `shell_metacharacters_are_rejected` | proc_command.rs | **再用途** — (a) metachar token は literal argv として子へ届く（fixture `inspect` の sentinel args で pin）、(b) `"a\0b"` は `TOOL_EXECUTION_FAILURE` + `spawnCode="SPAWN_ERROR"` へ縮退することを pin（`\0` degradation の観測点） |
| `spawn_time_revalidation_detects_swapped_executable` | proc_command.rs | **再用途** — B3 非依存の **path 差替** pin へ再設計: cfg(unix) では pinned file を symlink へ置換し realpath 不一致 → `"path changed since pin"`。全 platform で pinned file 削除 → `"no longer resolves"`。**内容差替（同 path 非 native 化）は検出対象外**であることを positive assert（差替 file が検査を通り実行まで進むこと — 実 spawn しないなら check が通ることを pin） |
| `check_command_execution_shares_execute_predicate` | proc_command.rs | **更新** — metachar sub-assertion（`["test","a;rm"]` → POLICY_VIOLATION）は `"test","*"` pattern に適合して受理へ転換。unknown grant・pattern mismatch・pin 不在の各 assert は維持 |
| `pin_failure_stays_hard_even_when_opted_in` | proc_degrade.rs | **更新** — message assertion を restate 後文言へ |
| `resolve_policy_errors_stay_hard_even_when_opted_in` | proc_degrade.rs（:272-284） | **更新** — `{"executable":"node"}` は除去後に policy resolve を通り pin へ進むため環境依存の失敗になる（`node` が PATH にあれば runtime 構築が成功し `unwrap_err` が panic、無ければ expected の `InvalidRuntimePolicy` ではなく pin の `POLICY_VIOLATION`）。spec を `{"executable":"node","argv":["a*b"]}`（wildcard 構造違反）へ変更し、`resolve_command_policy` の `InvalidRuntimePolicy` が opt-in でも hard のままであることを PATH 非依存で pin |
| `node_task_is_an_explicit_policy_violation` | proc_process.rs | **再用途** — `["node","-e",<script>]` task が PATH 解決され実 `node` で実行される pin へ（mise toolchain の node 前提 — `mise run check` 経路では保証。rename: `node_task_resolves_via_path` 系）。代替として case 削除も可だが、B5 除去の end-to-end pin として維持を推奨 |

### dev-seam / regression / CLI integration

| test | file | disposition |
|------|------|-------------|
| `run_fake_stream_grant_deny_listed_executable_rejected` | run_dev_seam.rs | **再用途** — `--grant` spec を `{"name":"x","executable":"python","argv":["a*b"]}` へ変更（invocation 層の shared validation で INVALID_RUNTIME_POLICY + `details.layer=invocation` を deterministic に維持。`python` 単体 spec は受理→pin が PATH 依存になるため使わない）。rename: `run_fake_stream_grant_invalid_spec_rejected` 系。assertion（ok:false・code・layer・exit 3）は不変 |
| `cases/run-grant-deny.json` + `expected/run-grant-deny.json` | regression | **再用途（rename）** — `extraArgs` の spec を `{"name":"deny-me","executable":"python","argv":["a*b"]}`・name/files_note を invalid-spec 趣旨へ変更し、file pair を `run-grant-invalid-spec.json` へ rename。expected の内容は不変（code+`layer` のみ pin で message 非依存）。単純削除も可だが、invocation 層の spec 検証 regression pin を残すため再用途を推奨 |
| `policy_grant_add_rejects_invalid_spec_without_writing` | integration/policy.rs | **更新** — `{"executable":"python"}` spec を `{"executable":"git","timeout":"5x"}`（bounded duration 違反 → INVALID_RUNTIME_POLICY）へ置き換え。`{"argv":["a*b"]}`・`{"argv":["log"]}`（schema 違反）は維持 |
| `workflow.rs:505`・`workflow_dev_seam.rs:166` の comment | — | **更新** — 「workspace-resident 拒否に掛からない」記述を削除（pin は依然 ambient PATH 非依存で deterministic という趣旨のみ残す） |
| `differential.rs` header・`translate_process_spec` comment | — | **更新** — 「Rust は `node` を PATH 解決しない」「bare `node` は expectRust pin」記述を除去。`expectRust` 機構自体は tiktoken pin で継続使用 |

### 非依存（変更不要 — 確認済み）

- `crates/cheaper-workflow/tests/process.rs` — 実 `node` の absolute
  path 使用。bare `node` 非該当。
- `benchmarks/scenarios/scenarios.json` — `{node}` →
  `process.execPath` 置換で非該当。
- `route.rs`・`route_run.rs`・`hook.rs` — grant executable は
  test binary 自身（非 deny・workspace 外）。`proc_invocation.rs`
  の test 本体も同じ前提で不変だが、`self_grant` doc comment
  （:581-584）の「workspace-resident 拒否」記述のみ comment
  churn 対象（上記「message/comment churn 関連 site」参照）。
- `resources/policy.yaml` — grants/tasks 空。
- `route/resolve.rs` tests — `argv_shell_re` viability 分類は不変
  （`a;rm` → UnsafeCommand の assert は残る）。grant は test
  binary で pin 経路不変。

## Keep-Boundary Pins（変更しない site）

| site | path | tag | 不変理由 |
|------|------|-----|----------|
| `tokenize_shell` | `crates/cheaper-cli/src/hook/routing_policy.rs:1122`（caller: `route/run.rs:66`・`route_cmd.rs:142`） | D1 | hook 文字列解析の correctness — 自前の metachar/unsafe token 判定で `argv_shell_re` 非依存。拒否ではなく viability 分類 |
| PATH 検索→canonical realpath | `executable.rs` `resolve_process_executable` | B1 | pin の決定性 |
| `resolved != pinned` | `command/mod.rs:261-266` | B2 | TOCTOU 対策本体 |
| `native_executable` wrapper | `executable.rs:124-126` | B2 | 再解決 vehicle |
| wildcard 構造検査・`ArgPattern::matches` | `command.rs:252-302`・`command/mod.rs:476-487` | A6 | argv pattern 言語（宣言制約） |
| `relative()`（cwd/inputs/outputs） | `process.rs:153-174` | A7 | 宣言境界 |
| env dup/NUL 検査 | `process.rs:290-308`・`command.rs:401-419` | A7 | env 宣言境界（argv とは別規約） |
| lifecycle bound（timeout/killGrace/heartbeat/maxOutputBytes）・nonempty grants/tasks gate・writeScope×command gate | `process.rs`・`command.rs`・`registry.rs` | C3/C4/C5 | bounded lifecycle・fail-closed admission |
| `PathDenylist`/`denyPaths`・`denied_declared_path`・`resolve_process_executable` の denylist 引数 | `deny_paths.rs`・`proc/mod.rs`・`executable.rs:151-155`・`prepare.rs:71-95` | E2 | credential path deny — executable deny とは別物 |
| `.exe/.com` win32 拡張子・X_OK | `executable.rs:99-107` | spawn viability | OS が spawn 可能かの判定 |
| `spawn_failure_is_async` の ENOEXEC/エラー写像 | `proc/spawn.rs` | C2 | spawn 層の OS error shape |
| `argv_shell_re` 定義 + `route/resolve.rs:285` UnsafeCommand 分類・`is_interactive`・`CWD_CHANGING_COMMANDS` | `process.rs:134-137`・`resolve.rs:282-288` | D2/issue 4 | viability 審査依存の整理は issue 4 — 本 issue では一切触れない |
| viability → `check_command_execution` 委譲 | `resolve.rs:304-335` | D2 | routing≠authorization — predicate の中身が変わるだけで経路不変 |
| `env_clear`・stdin null・process group・CREATE_NO_WINDOW | `proc/spawn.rs` | C1/C2 | spawn API 層の shell-free・lifecycle |

## Data and Control Flow

除去後の grant 実行経路:

```
resolve_command_policy ── declared name を内容審査しない（受理）
  → ArgPattern::compile（wildcard 構造のみ検査 — metachar literal 受理）
  → cwd relative()・env dup/NUL・lifecycle 畳み込み
pin_command_grant ── resolve_process_executable のみ
  （PATH 検索 / workspace-relative 解決+denylist / absolute →
   native() = realpath → is_file → win32 .exe/.com → X_OK →
   canonical path を pin。relative declared は pre-filter せず
   resolve_workspace_path の escape 検査が引き継ぐ）
check_command_execution ── grant 解決 → authorize_args
  （argv pattern match のみ）→ cwd prepare → pin map 引き →
  verify_pinned_executable（native_executable 再解決 +
  resolved != pinned のみ）→ CheckedCommand
execute ── ProcPrepared → proc::run（env_clear・stdin null・
  lifecycle）→ spawn
```

task 経路: `resolve_process_policy` は task command/argv を内容
審査せず、cwd/inputs/outputs の `relative()`・env 規約のみ強制。
`prepare` は manifest・writeScope・denylist の後に
`resolve_process_executable` で解決する（front-door shim・bare
`node` 固定ルールなし）。

## Non-Functional Constraints

- **破壊的変更**: mandatory deny list・shim/script/resident/metachar
  reject の除去は behavioral breaking change。migration 告知は
  issue 7 の docs 作業で行い、本 issue は code・test のみ。
- **正直性**: error message・doc comment に「native」「shell/
  script shim」「denied」「workspace-resident」等の旧語彙を残さ
  ない（spec R10）。「shell-free spawn」は **spawn API 層でのみ
  真** — ENOEXEC 時の libc `/bin/sh` fallback は OS/libc 既定
  動作で cheaper の保証対象外（issue 7 で明文化予定、本 design
  では Risks に記録）。
- **再実装禁止**: 削除した審査を別条件・別層で再実装しない
  （spec R11）。`tokenize_shell` の実行拒否への転用も禁止。
- **grep で検証可能**: 除去 helper は名前を残さない
  （`is_shell_process_executable`・`native_magic_supported`・
  `within_workspace`・`is_relative_executable_path`・
  `script_ext_re`・`shell_executable_re`・
  `command_denied_executable_re`・`is_denied_command_executable`
  が `crates/` から消失）。

## Risks and Trade-offs

- **内容差替の検出範囲の縮小** — B3 除去で「同 path の file 内容が
  非 native へ差し替わった」ケースは pin-equality で検出不能に
  なる（`resolved == pinned` のまま）。path/symlink retarget の
  検出は維持される。これは parent ADR で受理済みの trade-off
  であり、`spawn_time_revalidation_detects_swapped_executable`
  の再設計で「検出する差替 = path/リンクの差替」に pin を
  張り替える。
- **`\0` argv の error 分類縮退** — POLICY_VIOLATION →
  TOOL_EXECUTION_FAILURE（spawn 層）。拒否自体は維持されるが
  consumer が code を見ている場合の観測差分として `spawnCode=
  "SPAWN_ERROR"` pin で明文化する（ADR Consequences 受理済み）。
- **ENOEXEC → libc `/bin/sh` fallback** — shebang 無し +X text
  file が execvpe 系で `/bin/sh` 解釈され得る。spawn API 層の
  shell-free は維持されるが fallback は OS/libc 既定動作。
  narrower な is-native gate を残す選択肢もあったが、形式審査は
  内容と無関係かつ bypass 可能なため「保護らしき何か」を残す
  より正直に記述する（親 design B3 補足・issue 7 で docs 化）。
- **pin-equality の windows 検証限界** — symlink retarget の
  `resolved != pinned` は windows で権限依存のため unix のみで
  pin。win32 でも pinned file 削除による `native()` → None →
  `"no longer resolves"` の fail-closed 経路は platform 共通で
  検証可能（B1 側の pin）。
- **`node`/PATH 解決の環境依存** — `node` が PATH に無い環境では
  bare `node` task は unsupported で失敗する（受理しないのでは
  なく解決不能）。mise toolchain は node 26 を管理しており
  `mise run check`/`mise exec` 経路では PATH 上に存在する —
  fixture・unit test の positive pin はその前提で記述する。
  toolchain 前提が崩れる場合は該当 pin を削除・cfg-gate する。
- **route viability の一時的不整合** — exec 側の metachar reject
  を消しても viability の UnsafeCommand 分類（`resolve.rs:285`）
  は残るため、metachar argv は command tool 層では受理可能なのに
  route 不発になるギャップが残る。これは issue 4 の整理対象と
  する意図的な残置であり、本 issue では触れない。
- **mandatory deny net の一時的喪失** — `python`/`ssh`/`tar` 等が
  opt-in 無しで受理される。#164 の `security.execDenyPatterns`
  が補完を担うが、#163 単独では deny 能力は無い — parent
  design/ADR が受理した降格順序（mandatory 除去 → opt-in 再配置）。
- **workspace-resident binary の宣言実行** — 「宣言した operator
  の責任」に割り切る（parent design B4 rationale）。denylist
  （E2）による宣言 path deny は維持される。

## Deferred Design Concerns

- **`security.execDenyPatterns` opt-in profile** — issue #164。
  operator 宣言 pattern を compile する別 predicate を同 3 適用点
  （resolve・pin・spawn 前再検証）へ再配置する。builtin 旧既定値の
  docs 記録・migration 例は #164/#169。
- **viability 層の審査依存整理** — issue 4。`resolve.rs:285` の
  `argv_shell_re` unsafe 分類・viability 評価順の見直し。残置中に
  viability と exec 層の受理範囲が diverge する（metachar argv は
  exec 受理・route 不発）。
- **docs 全体の正直性 sweep** — issue 7。「shell-free」
  「boundary/sandbox」表現と ENOEXEC fallback・内容差替非検出・
  破壊的変更の migration guide。sweep は少なくとも以下を列挙
  対象に含めること: `docs/guides/` 全体（特に `command.md` の
  deny list・metachar・workspace-resident 記述と
  `managed-process.md` の shim・native magic・bare `node` 固定
  ルール記述）、
  `docs/test-specs/invocation-scoped-command-grants.md`（deny
  list 前提の Rule・AC 記述 — invocation layer が「deny list
  を迂回しない」とする保証は spec として書き直しが必要）、
  および consumer 配布物の
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md:353`
  （`policy grant add` の "deny list" 記述 — repo 内 source の
  修正だけでは配布済み consumer へ届かない）。
- **platform 差の明文化** — issue 7。除去後も Windows の
  `.bat`/`.cmd`/`.ps1` は残置する `.exe`/`.com` viability rule
  （`executable.rs:99-104` — CreateProcess が直接 spawn できる
  拡張子の制約）で pin 時に失敗し続ける。script/shim が解決
  されるのは unix の +X file のみであり、docs は「script が
  実行可能になる」と platform 非依存に読める記述を避ける
  必要がある（`refuses_script_shims_and_executable_text` の
  platform 分岐 pin と整合させる）。
- **deny 一覧の migration 資料化** — `command_denied_executable_re`
  の builtin 一覧を「旧既定値」として docs へ移す作業は
  #164/#169。本 issue では一覧ごと削除するのみ。

## Verification Notes

- [ ] grep 除去確認（`crates/` から消失すること）:

  ```bash
  grep -rn "is_shell_process_executable\|native_magic_supported\|\
within_workspace\|is_relative_executable_path\|script_ext_re\|\
shell_executable_re\|command_denied_executable_re\|\
is_denied_command_executable" crates/
  ```

  注: `within_workspace` は `walker.rs`/`normalization.rs` の
  無関係な test 名(`..._under_within_workspace`)にも hit する。
  除去対象は `command/mod.rs` の fn とその呼出しのみ —
  test 名の残存は許容する。

- [ ] 旧語彙の残存確認（spec R10 — error message・doc comment・
  test comment の正直性）:

  ```bash
  grep -rniE "deny|workspace-resident|shim|native magic" crates/
  ```

  残存 hit を全て確認し、除去した審査への言及が残らないこと。
  許容する残存 hit は (a) `denyPaths`/`PathDenylist`/
  `denied_declared_path` 等の denylist 機構（E2 — executable
  deny とは別物）、(b) hook/permission 判定の `deny` verdict・
  capability/empty-grants 由来の `denied` test 名、(c) 「shim」
  の非該当用法（`serde_yml` shim・PATH shim・self-reshim）、
  (d) `route/resolve.rs:279-281` の viability comment（前提は
  issue 4 完了まで一時的に偽 — known-stale として残置し
  comment churn 対象外）のみ。

- [ ] 残置確認: `argv_shell_re` 定義（`process.rs`）と
  `route/resolve.rs` の参照、`tokenize_shell`・`resolved != pinned`・
  wildcard 構造検査・`relative()`・env NUL・`PathDenylist`・
  `.exe/.com`・X_OK が変わらず残ること。
- [ ] golden・unit・integration・dev-seam・regression の各 test
  target が通ること:

  ```bash
  mise exec -- cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam
  mise exec -- cargo test -p cheaper-tools --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam
  mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam
  ```

- [ ] 宣言 executable として `python`/`ssh`/`tar`/`bash`/`.sh`/
  workspace-resident binary を含む grant が opt-in deny 未指定で
  policy resolve・pin で拒否されない test が存在すること
  （resolve 層は名前で deterministic、pin 層は workspace-resident
  file・`.sh`・symlink basename で pin）。
- [ ] `\0` を含む argv が `POLICY_VIOLATION` でなく spawn 層
  `TOOL_EXECUTION_FAILURE` + `spawnCode="SPAWN_ERROR"` として
  観測される test が存在すること。
- [ ] pin-equality の spawn 前再検証が path 差替（symlink
  retarget・realpath 不一致・file 消失）を B3 非依存で検出する
  test に再設計されていること。
- [ ] `mise run check` pipeline が通ること。
