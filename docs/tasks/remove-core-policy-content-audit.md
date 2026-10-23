---
id: "TASK-034KJtXX2QajBjkAgOJRtm"
type: "task"
status: "todo"
title: "cheaper-core policy 層の内容審査 predicate・適用点を除去"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/163"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/process.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/command.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_process.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/process.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/exec-contract-content-audit-removal.md"
  implemented-by: []
  depends-on:
    - "docs/plans/exec-contract-content-audit-removal.md"
  blocks:
    - "docs/tasks/remove-tools-content-audit.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/exec-contract-content-audit-removal.md"
  references:
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/discovery/exec-contract-content-audit-removal.md"
    - "crates/cheaper-core/src/policy/process.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
---

# cheaper-core policy 層の内容審査 predicate・適用点を除去

## Work

plan `docs/plans/exec-contract-content-audit-removal.md`
(PLAN-034KJtLyNUE46XjtrMOG60)の T1。cheaper-core の policy 層
(`process.rs`・`command.rs`・`load.rs`)から content-audit
predicate・regex 定数・適用点を design の Removal Contract
(A8・A9・A10・B4・B6・F3 相当行)通りに除去し、同 crate の unit
test・golden driver・golden fixture を張り替える。
design(DESIGN-034KIsSCvVMAxGDZY1dQTd)の行指定をそのまま実装する
— 除去対象・残置対象を再解釈しない。

### 1. `policy/process.rs`

- `shell_executable_re()`(:117-125)削除 — 唯一の caller は
  `is_shell_process_executable`。
- `script_ext_re()`(:127-130)削除 — 同上。
- `argv_shell_re()`(:134-137)は**残置** — 唯一の残存 caller は
  `cheaper-cli` `route/resolve.rs:285`。doc comment(「cheaper-tools
  の `command` execute が cross-crate で参照するため `pub`」)を
  「`route/resolve.rs` viability が参照する間 `pub` を維持」へ
  訂正する。
- `is_shell_process_executable()`(:141-148)削除 — caller は
  process task loop・`command.rs` resolve・`executable.rs`・golden
  driver のみで全て除去される。
- task loop の 2 適用点を削除 — `is_shell_process_executable`
  呼出し(:272-277、error `"shell or script shim executable is
  forbidden"` ごと)と `argv_shell_re` 呼出し(:278-280、error
  `"shell syntax in argv is forbidden"` ごと)。
- **残す**: `relative()`(:153-174、cwd/inputs/outputs の
  workspace-relative 検査)・env dup/NUL 検査(:290-308)・
  `resolve_process_task`/`process_task_resources`/`str_vec`/
  `bounded_duration`・`DEFAULT_*`/`MAX_PROCESS_OUTPUT_BYTES`。

### 2. `policy/command.rs`

- `command_denied_executable_re()`(:160-189)削除 — builtin 定数
  regex。#164 は operator 宣言 pattern を compile する別 predicate
  を要し再利用価値なし(ADR Option D 棄却・YAGNI)。
- `is_denied_command_executable()`(:196-207)削除 — 3 適用点と
  共に消える。
- `is_relative_executable_path()`(:214-223)削除 — 唯一の
  caller は tools の pin pre-filter(T2 で除去)。
- `ArgPattern::compile` の `argv_shell_re` 検査(:249-251)削除 —
  error `"shell syntax in argv pattern is forbidden"` ごと除去。
  doc comment(:242-244)の metachar 言及も訂正。wildcard 構造検査
  (:252-271 — bare `*` 位置・prefix `*`・複数 `*` の構文規約)
  は**残す**。
- `resolve_command_policy` の 2 適用点を削除 —
  `is_shell_process_executable`(:372-376)と
  `is_denied_command_executable`(:377-381、error `"interpreter
  or exec-capable executable is forbidden"` ごと)。
- import(:23-28)から `argv_shell_re`・
  `is_shell_process_executable` を外す。`use crate::nodepath`
  (23 行目)も除去 — 唯一の consumer は
  `is_relative_executable_path`(:219-220)で dead import 化する。
- **残す**: env dup/NUL・`relative`・`resolve_command_grant`・
  `project_grant`・`command_grant_resources`・bounded_duration。

### 3. `policy/load.rs`

- module doc(:13)・Partial validate comment(:151-152)の
  「deny list」言及を除去後の検査内容(argv pattern compile・
  env/cwd 規則等)へ restate。
- `invocation_denied_executable_keeps_inner_code_with_layer`
  (:527-545)を**再用途化** — `{"executable":"python"}` spec は
  受理されるようになり `unwrap_err` が panic するため、spec を
  `{"executable":"python","argv":["a*b"]}`(wildcard 構造違反 →
  `ArgPattern::compile` の `INVALID_RUNTIME_POLICY`)へ変更し、
  inner code 保持 + `details.layer="invocation"` +
  `"invalid command policy"` を PATH 非依存で deterministic に
  pin する。rename: `invocation_invalid_grant_spec_keeps_inner_code_with_layer`
  系。

### 4. unit test(src 内 `#[cfg(test)]`)

- `process.rs` — `shell_executable_detection` **削除**、
  `rejects_shell_executables_and_argv_syntax` を**再用途化**
  (`bash` task・metachar argv task が受理される positive pin へ。
  rename: `accepts_shell_executables_and_argv_metachars` 系)。
- `command.rs` —
  `denied_executables_cover_interpreters_wrappers_and_versioned_names`
  **削除**(builtin 一覧の docs 移管は #164/#169)、
  `resolve_rejects_denied_and_shell_executables` を**再用途化**
  (`python3`/`python3.13t`/`env`/`ld-linux-x86-64.so.2`/`bash`/
  `deploy.sh` が受理される declared-name acceptance pin へ)、
  `argv_pattern_compile_and_match` を**更新**(`bad` list から
  `"a|b"`・`"x;y"`・`"v$(i)"` を除去し literal token 受理を
  assert へ追加可。`"a*b"`・`"**"`・`"*a"`・非末尾 bare `*`・
  resolve 経由 `a*b` の `INVALID_RUNTIME_POLICY` は維持)、
  `relative_executable_path_prefilter` **削除**。

### 5. golden driver・fixture(`crates/cheaper-core/tests/`)

- `golden/policy_process.rs` — `is_shell_process_executable`
  import と `"isShellProcessExecutable"` op arm(L1-3, 22-24)を
  除去。
- `golden/fixtures/policy/process.json` — `"invalid shell
  executable"`(`["bash","-c","x"]`)・`"invalid script shim"`
  (`["run.sh"]`)・`"invalid argv shell syntax"`(`["echo","a|b"]`)・
  `"invalid argv subshell"`(`["echo","$(x)"]`)の 4 case を resolve
  受理の成功 expected へ**更新**し case 名を受理系へ rename。
  `isShellProcessExecutable` op 14 件は**削除**。
- `golden/fixtures/policy/load.json` — `"invocation denied
  executable keeps inner code"` を spec `{"executable":"python",
  "argv":["a*b"]}` へ**再用途化**(wildcard 構造違反で同一
  code+layer を deterministic に維持。case 名は
  `"invocation invalid grant spec keeps inner code"` 系へ rename、
  expected 不変)。
- `golden/fixtures/policy/schema.json` — `"process shell task
  rejected pre-merge"`(`mode:complete`・`op:validatePolicySchema`・
  `command:["sh"]`、case は :714-774)の expected を受理
  (`{"ok":{"policy":<validated input echo>}}` — op は resolved
  ではなく validated 入力を echo)へ**更新**し、case 名を
  `"process shell task accepted pre-merge"` 系へ rename。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/exec-contract-content-audit-removal.md`
  (PLAN-034KJtLyNUE46XjtrMOG60 — T1)
- `docs/designs/exec-contract-content-audit-removal.md`
  (DESIGN-034KIsSCvVMAxGDZY1dQTd — Removal Contract
  `policy/process.rs`/`policy/command.rs` 表・error contract・
  golden churn 表が AUTHORITATIVE)
- `docs/specs/exec-contract-content-audit-removal.md`
  (SPEC-034KI3mWcW92kMljP5xoa8 — R1・R7・R8・R10・R12)
- `docs/test-specs/exec-contract-content-audit-removal.md`
  (TSPEC-034KJtUqMNXjkbjsbz5YPa — name-disappearance・
  `a*b` determinism・keep-boundary Rules)

## Dependencies

- なし(起点 task)。design の行指定は現行コードと一致済み。
- 既知の transient: 本 task で core の pub symbol
  (`is_shell_process_executable`・`is_denied_command_executable`・
  `is_relative_executable_path`)を消すため、T2 が import/call
  site を除去するまで `cargo build -p cheaper-tools`/`-p
  cheaper-cli` は compile 不可。本 task の検証は
  `cargo test -p cheaper-core` に scope する(cheaper-core は
  単独で compile・test 可能)。

## Affected Files

- `crates/cheaper-core/src/policy/process.rs`(modify)
- `crates/cheaper-core/src/policy/command.rs`(modify)
- `crates/cheaper-core/src/policy/load.rs`(modify)
- `crates/cheaper-core/tests/golden/policy_process.rs`(modify)
- `crates/cheaper-core/tests/golden/fixtures/policy/process.json`
  (modify)
- `crates/cheaper-core/tests/golden/fixtures/policy/load.json`
  (modify)
- `crates/cheaper-core/tests/golden/fixtures/policy/schema.json`
  (modify)

## Done When

- [ ] `shell_executable_re`・`script_ext_re`・
      `is_shell_process_executable`・`command_denied_executable_re`・
      `is_denied_command_executable`・`is_relative_executable_path`
      が `crates/cheaper-core/src/` から消失(定義・caller・import
      全て)
- [ ] `argv_shell_re` 定義が残り、doc comment が viability 参照の
      趣旨へ訂正されている
- [ ] `resolve_command_policy`・`resolve_process_policy` task
      loop・`ArgPattern::compile` が shell/shim・deny・metachar で
      reject せず、wildcard 構造・`relative()`・env dup/NUL・
      lifecycle 規約を維持する
- [ ] golden 4 系(driver・process.json・load.json・schema.json)
      が受理 expected・`a*b` 再用途化へ張り替わり
      `cargo test -p cheaper-core` が pass
- [ ] `load.rs` unit test が `a*b` spec で inner code+
      `layer="invocation"` を PATH 非依存で pin する

## Out of Scope

- `crates/cheaper-tools` 側の call site・import・test(T2)
- `crates/cheaper-cli` 側の doc/test/regression churn(T3)
- `argv_shell_re` 定義の削除・`route/resolve.rs` の viability
  分類・`tokenize_shell`(issue 4 scope — 一切触れない)
- `relative()`・env dup/NUL・wildcard 構造検査・denylist・
  lifecycle(keep site — 変更しない)
- 削除した審査の別条件・別層での再実装、#164 用の seam/flag/
  stub(spec R11)

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      exits 0(unit test + golden target 全 pass)
- [ ] `mise exec -- cargo clippy -p cheaper-core --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings`
      exits 0(dead import・dead code なし)
- [ ] `grep -rn "is_shell_process_executable\|script_ext_re\|shell_executable_re\|command_denied_executable_re\|is_denied_command_executable\|is_relative_executable_path" crates/cheaper-core/`
      が 0 hit
