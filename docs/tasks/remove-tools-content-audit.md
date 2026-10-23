---
id: "TASK-034KJu31OgzJMwk2vb4KJz"
type: "task"
status: "done"
title: "cheaper-tools command/proc 層の内容審査を除去"
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
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/executable.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/exec/prepare.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/exec-contract-content-audit-removal.md"
  implemented-by: []
  depends-on:
    - "docs/plans/exec-contract-content-audit-removal.md"
    - "docs/tasks/remove-core-policy-content-audit.md"
  blocks:
    - "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
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
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/proc/executable.rs"
    - "crates/cheaper-tools/src/tools/exec/prepare.rs"
  defers: []
  deferred-by: []
---

# cheaper-tools command/proc 層の内容審査を除去

## Work

plan `docs/plans/exec-contract-content-audit-removal.md`
(PLAN-034KJtLyNUE46XjtrMOG60)の T2。cheaper-tools の
`command/mod.rs`・`proc/executable.rs` から content-audit
分岐・helper・import を design の Removal Contract(B3・B4・B5・
A9 適用点 2/3・A10 runtime・F3)通りに除去し、
`prepare.rs`/`registry.rs` の message/comment churn と同 crate の
unit・integration test・fixture を張り替える。T1 が消した core
symbol の import 除去で workspace compile を回復させる。
design(DESIGN-034KIsSCvVMAxGDZY1dQTd)の行指定をそのまま実装する
— 除去対象・残置対象を再解釈しない。

### 1. `tools/command/mod.rs`

- `within_workspace()`(:167-170)削除 — caller は pin
  post-realpath・verify の 2 箇所のみ。
- pin pre-filter `is_relative_executable_path`(:213-219)削除 —
  `"workspace-resident executables are not grantable"` error ごと
  除去。relative declared は `resolve_workspace_path` の escape
  検査が引き継ぐ。
- `resolve_process_executable` 呼出し(:220-229)は**残す** —
  pin の決定的解決経路(denylist 検査は executable.rs 内で残る)。
- `is_denied_command_executable`(:230-237、A9 適用点 2)削除 —
  `"resolved executable is interpreter or exec-capable"` error ごと。
- `within_workspace` post-realpath(:238-244)削除。
- `native_executable` 呼出し(:255-260)・`resolved != pinned`
  pin-equality(:261-266、`"pinned executable path changed since
  pin"`・B2 TOCTOU 対策本体)は**残す**。
- `verify_pinned_executable` 内の `is_denied_command_executable`
  (:267-272、A9 適用点 3)・`within_workspace`(:273-278)削除。
- `verify_pinned_executable` 签名(:254)から `root` 引数を**除去**
  (唯一の用途は `within_workspace` で dead param 化)。
  `check_command_execution` の call site(:453)も追随。
- `authorize_args` の `argv_shell_re` loop(:468-475)削除 —
  `"argument contains forbidden shell metacharacters"` error ごと。
  pattern match(:476-487 — `grant.argv: None` なら無条件受理・
  `Some` なら compile + `matches` 照合のみ)は**残す**。
- import(:24-29)から `is_denied_command_executable`・
  `is_relative_executable_path`・`argv_shell_re` を外す。
  `use cheaper_core::nodepath`(24 行目)も除去 — consumer は
  `within_workspace`(:168-169)と削除する
  `within_workspace_boundary` test(:869,878)のみで dead import 化。
- pin map 不在の fail-closed(`"no pinned executable"` :451-458)
  は**残す**。
- module doc(:1-17)・各 fn doc comment(:172-205, 248-253,
  403-427, 447-450, 516-518)の deny/residency/metachar 言及を除去
  後の実態へ restate — :447-450 の「deny/residency の同一
  predicate」は `native_executable` 再解決 + path 同一性のみの
  記述へ。

### 2. `tools/proc/executable.rs`

- `native_magic_supported()`(:74-86、B3)削除 — 唯一の caller は
  `native()` 内 magic 判定。
- `native()` 内 `is_shell_process_executable`(:92-94、F3)削除。
- win32 `.exe/.com` 拡張子要求(:99-104)・X_OK(:105-107)は
  **残す** — spawn viability(OS が spawn 可能か)。
- magic-byte read + 判定(:108-117)削除 — `use std::io::Read`
  (13 行目)はこの読み込み専用のため import ごと除去。
- `use cheaper_core::policy::process::is_shell_process_executable`
  (17 行目)を import から除去 — consumer は `native()` :92 と
  front-door :143 で共に除去。`nodepath`・`PathDenylist`・
  `realpath`/`resolve_workspace_path`/`SymlinkPolicy` は残る。
- `native_executable()` wrapper(:124-126)は**残す** — doc
  comment の「native magic」言及は訂正。
- front-door `is_shell_process_executable`(:143-145、F3)削除 —
  declared `sh`/`bash`/`*.sh` は PATH・path 解決へ進む。
- workspace-relative 解決 + denylist 検査(:146-157)は**残す** —
  `resolve_workspace_path`(workspace 境界)と `PathDenylist` は宣言契約。
- bare `node` reject(:163-173、B5)削除 — `node` は PATH 解決の
  通常経路へ。
- PATH candidate loop(:174-189)は**残す** — 内部の `native()`
  のみ縮小。
- `unsupported()` message(:35-42)を restate —
  `"{subject} requires a supported native executable; shell and
  script shims are unsupported"` は実態と乖離。
  `"{subject} could not be resolved to a spawnable executable"`
  系へ置き換え(`native`/`shell`/`shim` 語彙を残さない — 残る
  失敗条件は realpath/is_file/win32 ext/X_OK = spawn viability)。
- `resolve_process_executable` doc(:128-137)の bare `node` 固定
  ルール記述を除去して restate。
- 設計固定ルール doc comment(:6-11)の「Rust は bare `node` を
  PATH 解決しない」記述を削除し、B5 除去後の通常解決を記述。
- `native_bytes()` test fixture(:246-252)は magic-byte 内容審査の
  除去で fixture 名・前提が stale 化するため rename・restate
  (file 内容はもはや検査対象ではなく、実在性・実行 bit だけが
  意味を持つ)。

### 3. `tools/process/prepare.rs`・`registry.rs`

- `prepare.rs` — `resolve_process_executable` 呼出し(:127-136)
  は F3 の実行経路としてそのまま残るが、
  `unsupported_empty_command`(:140-145)の message が
  `"supported native executable; shell and script shims are
  unsupported"` を含むため `unsupported()` と同じ文言規約
  (`"{subject} could not be resolved to a spawnable executable"` 系)
  で restate。
- `registry.rs` — pin 導入 comment(:1202-1207)の「pin 不能・
  deny 該当・workspace-resident の grant は runtime 構築自体を
  `POLICY_VIOLATION` で fail」を「pin 不能の grant は fail」へ
  訂正。

### 4. unit test(src 内 `#[cfg(test)]`)

- `command/mod.rs` —
  `authorize_args_rejects_metachar_before_pattern_match` を**再用途化**
  (metachar token が rest marker pattern で受理されること・pattern
  不一致だけが POLICY_VIOLATION である pin へ)、
  `authorize_args_without_pattern_accepts_any_count` を**更新**
  (`"a;b"` 受理へ — metachar reject 部を除去)、
  `pin_rejects_relative_executable_path_before_resolution` を
  **再用途化**(tempdir 内 `bin/tool(.exe)` を実在させて declared
  `"bin/tool"` の pin 成功 pin へ — B4 positive pin)、
  `pin_rejects_absolute_path_inside_workspace` を**再用途化**
  (workspace 内 absolute path の pin 成功 pin へ)、
  `pin_fails_closed_on_unresolvable_or_denied_executable` を**更新**
  (unresolvable の fail-closed 部は維持、`python3` resolve reject
  の副 assert を除去し受理へ転換)、
  `verify_pinned_rejects_missing_swapped_or_workspace_paths` を
  **更新**(missing path は引き続き error= restate 後 message、
  workspace 内 file は `resolved == pinned` で検査通過、pin map
  不在の execute fail-closed は維持。rename:
  `verify_pinned_rejects_missing_accepts_workspace_paths` 系)、
  `within_workspace_boundary` **削除**。
- `executable.rs` — `bare_node_is_an_explicit_policy_violation` を
  **再用途化**(`search_path: Some(bin_dir)`(tempdir に
  `node`/`node.exe` を配置)で `node` が解決される positive pin へ。
  `Some("")`(PATH 遮断)は引き続き unsupported error。rename:
  `bare_node_resolves_via_search_path` 系)、
  `error_subject_is_parameterized_by_caller` を**更新**
  (`"node"` を使った subject pin を missing executable
  (例 `"missing-tool"`)へ置き換えて subject 引数の文言規約のみ
  pin)、`refuses_script_shims_and_executable_text` を**再用途化**
  (platform 分岐 pin: unix では +X 付き shim/text は**解決される**
  — B3/B6 除去の positive pin。win32 では `.cmd`/`script`(非
  `.exe/.com`)が引き続き viability で reject、`.exe` 拡張子の
  text file は解決される)、
  `nonexistent_or_unsupported_magic_fails_closed` を**更新**
  (`missing` は引き続き fail-closed、`wrong`(非 magic 内容 +X)は
  解決される positive pin、`short` は両 platform で解決される。
  rename: `unresolvable_fails_closed_regardless_of_content` 系)、
  `fails_closed_when_search_path_has_no_supported_executable` は
  **維持**(`"node.exe"`・`"missing-tool"` は PATH 解決不能で
  引き続き unsupported — assertion 変更なし)。

### 5. integration test・fixture(`crates/cheaper-tools/tests/`)

- `proc_command.rs` —
  `declared_denied_executables_fail_policy_resolution` を**再用途化**
  (`resolve_command_policy` で `python`/`ssh`/`tar`/`env`/`bash`/
  `deploy.sh` が受理される pin へ — policy resolve は PATH 非依存で
  deterministic)、`pinned_resolved_basename_deny_rejects`(cfg unix)
  を**再用途化**(symlink `tool`→実 file `python3`(current_exe
  複製)の pin が受理される positive pin へ)、
  `workspace_resident_executables_are_rejected` を**再用途化**
  (`./tool` relative・absolute inside-workspace の双方が pin・
  runtime 構築を通る pin へ)、`unpinnable_executable_fails_closed`
  を**更新**(`POLICY_VIOLATION` は維持、message assertion を
  restate 後文言へ)、`shell_metacharacters_are_rejected` を
  **再用途化**((a) metachar token は literal argv として子へ届く
  (fixture `inspect` の sentinel args で pin)、(b) `"a\0b"` は
  `TOOL_EXECUTION_FAILURE` + `spawnCode="SPAWN_ERROR"` へ縮退
  することを pin — **`\0` degradation の観測点**)、
  `spawn_time_revalidation_detects_swapped_executable` を**再設計**
  (B3 非依存の path 差替 pin: cfg(unix)で pinned file を symlink
  へ置換し realpath 不一致 → `"path changed since pin"`、全
  platform で pinned file 削除 → `"no longer resolves"`。内容差替
  (同 path 非 native 化)は検出対象外であることを positive
  assert)、`check_command_execution_shares_execute_predicate` を
  **更新**(metachar sub-assertion(`["test","a;rm"]` →
  POLICY_VIOLATION)は `"test","*"` pattern に適合して受理へ転換。
  unknown grant・pattern mismatch・pin 不在の各 assert は維持)。
  header comment(:12-18)・:404 の `is_shell_process_executable`
  言及 comment も restate。
- `proc_degrade.rs` — `pin_failure_stays_hard_even_when_opted_in`
  を**更新**(message assertion を restate 後文言へ)、
  `resolve_policy_errors_stay_hard_even_when_opted_in`(:272-284)
  を**更新**(spec を `{"executable":"node","argv":["a*b"]}`
  へ変更 — `{"executable":"node"}` は除去後に policy resolve を
  通り pin へ進むため環境依存になる。wildcard 構造違反で
  `resolve_command_policy` の `InvalidRuntimePolicy` が opt-in で
  も hard のままであることを PATH 非依存で pin)。
- `proc_process.rs` — `node_task_is_an_explicit_policy_violation`
  を**再用途化**(`["node","-e",<script>]` task が PATH 解決され
  実 `node` で実行される pin へ — mise toolchain の node 前提。
  rename: `node_task_resolves_via_path` 系)。header comment(:3-8)
  の「deny/workspace-resident/PATH 解決しない」記述も訂正。
- `proc_invocation.rs` — `self_grant` doc comment(:581-584)の
  「absolute native executable」「workspace-resident 拒否に掛から
  ない」記述を「pin は ambient PATH 非依存で deterministic」の
  趣旨へ訂正(test 本体不変)。
- `differential.rs` — header(:9-15)・`translate_process_spec`
  comment(:613)の「Rust は `node` を PATH 解決しない」「bare
  `node` は expectRust pin」記述を除去(`expectRust` 機構自体は
  tiktoken pin で継続使用)。
- `fixtures/process.json` — `"bare node task is an explicit Rust
  error"` case(:803-841)の `expectRust` block を除去し base
  expected(実 `node` 実行 → "bare node" stdout)へ戻す。case 名は
  `"bare node task resolves via PATH"` 系へ rename(mise toolchain
  の node が PATH 上にある前提 — `mise run check` 経路では保証)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/exec-contract-content-audit-removal.md`
  (PLAN-034KJtLyNUE46XjtrMOG60 — T2)
- `docs/designs/exec-contract-content-audit-removal.md`
  (DESIGN-034KIsSCvVMAxGDZY1dQTd — Removal Contract
  `command/mod.rs`/`proc/executable.rs`/message churn 表・`\0`
  degradation path・integration churn 表が AUTHORITATIVE)
- `docs/specs/exec-contract-content-audit-removal.md`
  (SPEC-034KI3mWcW92kMljP5xoa8 — R2-R6・R10)
- `docs/test-specs/exec-contract-content-audit-removal.md`
  (TSPEC-034KJtUqMNXjkbjsbz5YPa — acceptance・`\0` degradation・
  pin-equality 再設計・bare `node`/script 受理境界 Rules)

## Dependencies

- `docs/tasks/remove-core-policy-content-audit.md`(T1) — core の
  predicate 除去が先行。T1 で消えた pub symbol の import 除去を
  含み、本 task 完了で workspace compile が回復する。
- `node` positive pin は mise toolchain(node 26)が PATH 上にある
  前提 — `mise run check`/`mise exec` 経路では保証される。

## Affected Files

- `crates/cheaper-tools/src/tools/command/mod.rs`(modify)
- `crates/cheaper-tools/src/tools/proc/executable.rs`(modify)
- `crates/cheaper-tools/src/tools/process/prepare.rs`(modify)
- `crates/cheaper-tools/src/registry.rs`(modify)
- `crates/cheaper-tools/tests/proc_command.rs`(modify)
- `crates/cheaper-tools/tests/proc_degrade.rs`(modify)
- `crates/cheaper-tools/tests/proc_process.rs`(modify)
- `crates/cheaper-tools/tests/proc_invocation.rs`(modify)
- `crates/cheaper-tools/tests/differential.rs`(modify)
- `crates/cheaper-tools/tests/fixtures/process.json`(modify)

## Done When

- [x] `within_workspace`・`native_magic_supported`・
      `is_denied_command_executable`・`is_relative_executable_path`・
      `is_shell_process_executable`・`argv_shell_re`・
      `std::io::Read`・`nodepath` が `crates/cheaper-tools` の
      src/import から消失(helper・call site・import 全て —
      `nodepath`/`io::Read` の消失範囲は §Work の指定通り
      `command/mod.rs`・`proc/executable.rs` の審査 consumer のみ。
      `executable.rs` は PATH 解決用に `nodepath` を残置し、
      `grep/scan.rs`・`walker.rs` 等の無関係 consumer も残置)
- [x] `pin_command_grant` が realpath 解決と pin 取得のみを行い、
      `verify_pinned_executable(name, pinned)` が `resolved !=
      pinned` の pin-equality のみを強制する(`root` 引数なし)
- [x] `native()` が spawn viability(realpath・is_file・win32
      `.exe/.com`・X_OK)のみを判定し、`resolve_process_executable`
      が shim/front-door・bare `node` 固定 reject を持たない
- [x] `authorize_args` が argv pattern 照合のみを行う
- [x] `unsupported()`・`unsupported_empty_command`・pin 導入
      comment が restate 後文言になり、tools 側の stale 語彙が
      残らない
- [x] `cargo test -p cheaper-tools` が pass し workspace compile
      が回復する

## Out of Scope

- `crates/cheaper-core` 側の除去(T1)・`crates/cheaper-cli` 側の
  doc/test/regression churn(T3)
- `PathDenylist`/`denyPaths`・`denied_declared_path`(E2 —
  credential path deny。executable deny とは別物 — 変更しない)
- `spawn_failure_is_async`・`env_clear`・stdin null・process
  group・`proc/spawn.rs` 全般(spawn API 層 — 不変)
- `route/resolve.rs`・`tokenize_shell`・`argv_shell_re` 定義
  (issue 4 scope — 触れない)
- `security.execDenyPatterns`・opt-in deny の再配置・seam/flag/
  stub(issue #164 scope)
- ENOEXEC → libc `/bin/sh` fallback の対策 — OS/libc 既定動作
  (issue 7 で docs 化)

## Verification

- [x] `mise exec -- cargo test -p cheaper-tools`
      exits 0(unit・proc_* integration・differential・fixture 全 pass —
      single-crate では dev-seam feature は適用外のため
      workspace feature 指定なしで実測)
- [x] `mise exec -- cargo clippy --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      exits 0(dead import・dead param なし — `mise run check` 内)
- [x] `grep -rn "within_workspace\|native_magic_supported\|is_denied_command_executable\|is_relative_executable_path\|is_shell_process_executable" crates/cheaper-tools/src/`
      — 許容 residual のみ: `walker.rs` の `*_under_within_workspace`
      test 名 3 件(審査 helper とは無関係の symlink walker 規約
      test — task T4 allowlist 相当)
- [x] `mise run check` 内の
      `cargo clippy --workspace --all-targets` +
      `cargo test --workspace` が exits 0(T1 の compile
      transient が解消 — `cargo build` と同等の全 target 検証)

## Verification results

2026-10-23 実施(Windows / `feature/issue-163-exec-contract-cleanup`)。
T2 commit: `cce415b refactor(tools): remove mandatory content audits
from command/proc`。

| Command | Result |
|---------|--------|
| `mise exec -- cargo test -p cheaper-tools` | exit 0(lib 258・characterization 10・differential 11・proc_command 33・proc_degrade 9・proc_invocation 14・proc_process 9・proc_runtime 7 — 全 pass) |
| `cargo clippy --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` | exit 0(`mise run check` 内で実測) |
| name-disappearance grep(`crates/cheaper-tools/src/`) | 審査 symbol 0 hit — residual は `walker.rs` の `*_under_within_workspace` test 名 3 件のみ(無関係) |

T4 gap-fill として `proc_command.rs` の
`unpinnable_executable_fails_closed` に `\0` 入り declared
executable の pin 層 `POLICY_VIOLATION` 観測を追加済み(design
§b 指定の観測点 — T4 commit に含む)。
