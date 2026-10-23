---
id: "IMPL-034KKIy3KVgaEjtuv3WTrb"
type: "impl"
status: "completed"
title: "Exec-contract content-audit removal issue 163"
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
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_process.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/process.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/command/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/executable.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/proc/mod.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/tools/process/prepare.rs"
      - type: "file"
        path: "crates/cheaper-tools/src/registry.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_command.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_degrade.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_process.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/differential.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/fixtures/process.json"
      - type: "file"
        path: "crates/cheaper-cli/src/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/run_dev_seam.rs"
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
  implements:
    - "docs/tasks/remove-core-policy-content-audit.md"
    - "docs/tasks/remove-tools-content-audit.md"
    - "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
    - "docs/tasks/verify-content-audit-removal.md"
  implemented-by: []
  depends-on:
    - "docs/plans/exec-contract-content-audit-removal.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/exec-contract-content-audit-removal.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/exec-contract-content-audit-removal.md"
  references:
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Exec-contract content-audit removal issue 163

## Summary

issue #163(#151 実装分解 1/7・起点 issue)— 「宣言された exec の
内容を審査して reject」する content-audit クラス 7 項目(A8・
A10・B3・B4・B5・B6・F3)と A9 deny list の mandatory 適用点を、
実行契約の mandatory 層(grant/task resolve → pin → spawn 前
再検証 → argv 認可)から一括除去した(ADR-034KI3r2xdthh6rKDFY517
Option A — predicate 定義も call site と共に削除)。除去後に
cheaper が強制するのは**宣言された実行契約のみ** — shell-free
argv spawn・grant/task 認可・pin 同一性・wildcard 構造・
cwd/env 宣言境界・lifecycle 境界・`PathDenylist`。審査の再
実装・補償 check は行わない(spec R11)。

1. `cheaper-core`(T1): `policy/process.rs` の
   `shell_executable_re`・`script_ext_re`・
   `is_shell_process_executable` と task loop 2 適用点を削除し、
   `argv_shell_re` の doc を viability 参照へ訂正。
   `policy/command.rs` の `command_denied_executable_re`・
   `is_denied_command_executable`・`is_relative_executable_path`
   と `ArgPattern::compile` の metachar 検査・resolve 2 適用点を
   削除。`policy/load.rs` の doc restate + unit test を `a*b`
   wildcard spec へ再用途化。golden driver + 3 fixture を追随。
2. `cheaper-tools`(T2): `command/mod.rs` の `within_workspace`・
   pin pre-filter・pin/verify の deny+resident 再査・
   `authorize_args` metachar loop を削除し、
   `verify_pinned_executable` から `root` 引数を除去。
   `proc/executable.rs` の `native_magic_supported`・magic-byte
   read・`native()` shim 判定・front-door 判定・bare `node`
   reject を削除。`prepare.rs` message・`registry.rs` pin 導入
   comment を restate。
3. `cheaper-cli`(T3): `policy.rs` doc restate、integration・
   dev-seam の旧 deny 観測 test を `argv ["a*b"]` wildcard 構造
   違反(deterministic `INVALID_RUNTIME_POLICY`)・
   `{"executable":"git","timeout":"5x"}` 等の PATH 非依存 spec
   へ再用途化、regression pair を `run-grant-deny` →
   `run-grant-invalid-spec` rename。
4. 検証(T4): 8 symbol の消失 grep・旧語彙 allowlist sweep・
   keep-boundary 残置確認・`mise run check`。gap 検出で
   `\0` executable の pin 層 `POLICY_VIOLATION` 観測を
   `unpinnable_executable_fails_closed` へ追加し、stale grant
   name `"deny-me"` を `"x"` へ rename。

## Context

TASK-034KJtXX2QajBjkAgOJRtm(T1)・TASK-034KJu31OgzJMwk2vb4KJz(T2)・
TASK-034KJu3PCBQDeImZEcznWv(T3)・TASK-034KJu8k1x8fWOfRemmi3r(T4)。
PLAN-034KJtLyNUE46XjtrMOG60 / DESIGN-034KIsSCvVMAxGDZY1dQTd /
SPEC-034KI3mWcW92kMljP5xoa8 / ADR-034KI3r2xdthh6rKDFY517 /
TSPEC-034KJtUqMNXjkbjsbz5YPa に従う。実測 file:line inventory・
keep list・観測差分の ground truth は DISC-JLHFz6x4ZuhzYZrpfrG2QV
で確定済み — 親 chain(#151)が「cheaper の責務は宣言された実行
契約の強制であり content の審査ではない」と確定した設計判断の
code 側落とし込み。本 IR は #151 docs への cross-chain lineage
を持たず、chain 内 doc 集合で閉じる。

## Implementation

### Changes

- `crates/cheaper-core/src/policy/process.rs`:
  `shell_executable_re`(:117-125)・`script_ext_re`(:127-130)・
  `is_shell_process_executable`(:141-148)・task loop の 2 適用点
  (:272-280)を削除。`argv_shell_re`(:134-137)は `route/resolve.rs:285`
  の viability 分類が現役参照するため残置し、doc comment を
  viability 参照の趣旨へ訂正。`relative()`・env dup/NUL 検査
  (:290-308)は不変で残置。
- `crates/cheaper-core/src/policy/command.rs`:
  `command_denied_executable_re`(:160-189)・
  `is_denied_command_executable`(:196-207)・
  `is_relative_executable_path`(:214-223)・`ArgPattern::compile`
  の metachar 検査(:249-251)+doc(:242-244)・resolve の 2 適用点
  (:372-381)を削除、import 整理。wildcard 構造検査(:252-271)・
  env/cwd 規則は残置。
- `crates/cheaper-core/src/policy/load.rs`: module doc(:13)・
  Partial validate comment(:151-152)の「deny list」言及を
  restate、`invocation_denied_executable_keeps_inner_code_with_layer`
  (:527-545)を `{"executable":"python","argv":["a*b"]}` spec へ
  再用途化(wildcard 構造違反で code+layer を deterministic に
  維持)。`policy/schema.rs` は golden 更新に伴う追随のみ。
- `crates/cheaper-core/tests/golden/`: `policy_process.rs` の
  `isShellProcessExecutable` op arm 削除、`process.json` の
  shell/script/metachar 4 case を受理 expected へ更新+同 op
  14 件削除、`load.json` denied case を `a*b` spec へ再用途化、
  `schema.json` `sh` task case を受理 expected へ更新。
- `crates/cheaper-tools/src/tools/command/mod.rs`:
  `within_workspace`(:167-170)・pin pre-filter(:213-219)・
  pin の deny/resident 再査(:230-244)・verify の deny/resident
  再査(:267-278)・`authorize_args` metachar loop(:468-475)を
  削除。`verify_pinned_executable` の `root` 引数を除去し call
  site を追随。`resolve_process_executable`・`native_executable`・
  `resolved != pinned`(:217)・argv pattern match・pin 不在
  fail-closed は残置。
- `crates/cheaper-tools/src/tools/proc/executable.rs`:
  `native_magic_supported`(:74-86)・magic-byte read(:108-117)・
  `native()` shim 判定(:92-94)・front-door shim 判定(:143-145)・
  bare `node` reject(:163-173)を削除。`unsupported()`(:35-42)・
  `resolve_process_executable` doc(:128-137)・設計固定ルール
  doc(:6-11)を restate。win32 `.exe/.com`・X_OK・
  workspace-relative 解決+denylist・PATH candidate loop は残置。
- `crates/cheaper-tools/src/tools/process/prepare.rs`:
  `unsupported_empty_command`(:140-145)の message を
  `unsupported()` と同規約で restate。
- `crates/cheaper-tools/src/registry.rs`: pin 導入 comment
  (:1202-1207)を「pin 不能の grant は fail」へ訂正。
- `crates/cheaper-tools/tests/`: `proc_command.rs` 7 test の
  更新・再用途化(declared deny→受理・workspace resident→受理・
  metachar→literal argv 受理+`\0` degradation pin・
  `spawn_time_revalidation` を path 差替検出+内容差替非検出へ
  再設計)。`proc_degrade.rs` spec を `{"executable":"node",
  "argv":["a*b"]}` へ更新。`proc_process.rs` の
  `node_task_is_an_explicit_policy_violation` を PATH 解決 pin
  へ再用途化。`proc_invocation.rs` `self_grant` doc 訂正。
  `differential.rs` + `fixtures/process.json` の bare `node`
  `expectRust` 特例を除去。
- `crates/cheaper-cli/src/policy.rs`: module doc(:8-9)・
  `write_validated` doc(:245-247)の「deny list」言及を除去後の
  検査内容へ restate。
- `crates/cheaper-cli/tests/`: `integration/policy.rs` の
  invalid-spec spec を `{"executable":"git","timeout":"5x"}` へ
  置換。`run_dev_seam.rs` の
  `run_fake_stream_grant_deny_listed_executable_rejected` を
  `a*b` spec へ再用途化+rename、`self_grant` doc 訂正。
  regression pair `run-grant-deny.json` →
  `run-grant-invalid-spec.json` rename(expected 内容不変 —
  code+layer のみ pin)。`workflow.rs`(:505)・
  `workflow_dev_seam.rs`(:166)の「workspace-resident 拒否」
  記述を pin determinism の趣旨へ訂正。

### Decisions During Implementation

- predicate 定義も call site と共に削除(ADR Option A)—
  `command_denied_executable_re` は builtin 定数 regex であり、
  #164 は operator 宣言 pattern を compile する**別 predicate**
  を要するため再利用価値なし(Option D 棄却・YAGNI)。#164 用の
  seam/flag/stub は一切残さない。
- repurpose 先は PATH 非依存で deterministic — `argv ["a*b"]`
  wildcard 構造違反(`INVALID_RUNTIME_POLICY`+`details.layer=
  "invocation"` 維持)または `timeout:"5x"` parse failure。実
  executable 名に依存しないため環境差で脆くならない。
- T1→T2 の compile transient は scoped verification で処理 —
  T1 が core pub symbol を消すため T2 完了まで downstream crate
  は compile 不可(既知の直列依存)。T1 は `cargo test -p
  cheaper-core` で閉じ、T2 で workspace compile を回復。
- T4 gap-fill — design 指定の `\0` 入り declared executable が
  pin 層 `POLICY_VIOLATION` で fail closed する観測が不足して
  いたため `proc_command.rs::unpinnable_executable_fails_closed`
  に `"a\0b"` spec の assert を追加。併せて regression case 内
  の stale grant name `"deny-me"`(executable-deny 時代の残存)
  を `"x"` へ rename(expected には echo されず挙動不変)。

## Related Experiments

### Adopted

なし(known-solution task。design が Removal Contract・Churn
Plan・keep boundary を file:line 精度で固定済みのため
Experiment Log は未作成)。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-core`: exit 0(lib 216・
  golden 22 全 pass — T1)
- `mise exec -- cargo test -p cheaper-tools`: exit 0(lib 258・
  characterization 10・differential 11・proc_command 33・
  proc_degrade 9・proc_invocation 14・proc_process 9・
  proc_runtime 7 全 pass — T2)
- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`:
  exit 0(integration 288・regression 1・run_dev_seam 38・
  workflow 18・workflow_dev_seam 5 全 pass — T3)
- name-disappearance grep(T4): `is_shell_process_executable`・
  `native_magic_supported`・`within_workspace`・
  `is_relative_executable_path`・`script_ext_re`・
  `shell_executable_re`・`command_denied_executable_re`・
  `is_denied_command_executable` の 8 symbol が `crates/` から
  消失。`within_workspace` residual は `walker.rs`/
  `normalization.rs` の無関係な `*_under_within_workspace`
  test 名のみ(許容)。
- stale-vocabulary grep(T4・spec R10): `deny|workspace-resident|
  shim|native magic` の残存 hit が allowlist(`PathDenylist`/
  `denyPaths`/`denied_declared_path` の E2 denylist 機構・hook
  verdict・empty-grant `denied` test 名・`serde_yml`/PATH shim・
  `route/resolve.rs:279-281` known-stale viability comment)内に
  収まる。
- keep-boundary 残置確認(T4): `argv_shell_re` 定義
  (`policy/process.rs:120`)+ `route/resolve.rs:11,285` 参照・
  `tokenize_shell`(`hook/routing_policy.rs:1122`)・
  `resolved != pinned`(`command/mod.rs:217`)・`relative()`・
  env dup/NUL・`PathDenylist`・win32 `.exe/.com`+X_OK・
  `spawn_failure_is_async`・`env_clear`+`Stdio::null` — 全残存、
  `route/resolve.rs` diff 無し。審査の再実装・`tokenize_shell`
  拒否転用・#164 seam/flag/stub が diff に無いことも確認
  (spec R11)。
- 観測差分 pin: `\0` を含む argv は `POLICY_VIOLATION` でなく
  `TOOL_EXECUTION_FAILURE`+`details.spawnCode="SPAWN_ERROR"` へ
  縮退(`proc_command.rs::metachar_args_pass_through_and_nul_degrades_to_spawn_error`
  — message 非依存の code+spawnCode pin)。`\0` 入り declared
  executable は realpath 失敗で pin 層 `POLICY_VIOLATION` の
  まま fail closed(縮退しない — `unpinnable_executable_fails_closed`
  の `"a\0b"` case、T4 追加)。`python`/`ssh`/`tar`/`bash`/`.sh`/
  workspace-resident/symlink basename/bare `node` の受理 pin
  も存在。
- `mise run check`: exit 0(biome・`cargo fmt --check`・
  clippy --all-targets・version/doc-id check・
  `cargo test --workspace` 50 test binary 全 pass・
  `node --test`・lockfile/apm-payload check — T4 実測)
- `node scripts/check-doc-ids.mjs`: pass(1120 files — `mise run
  check` 内で実測)
- `node .agents/skills/doc-status/scripts/audit_docs.js --json`:
  findings error 0(本 issue 6 doc は reciprocal-relation
  warning のみ — non-blocking)

証跡 commit: `8b08adf`(discovery)→ `2f5370f`(spec/ADR)→
`9cd6bf5`(design)→ `f9342c4`(design review 反映)→
`c74c20c`(spec/ADR/design 承認)→ `9502c67`(plan)→
`f7a5ea4`(test-spec)→ `e45225c`(task breakdown)→
`981a4f4`(T1 core)→ `cce415b`(T2 tools)→ `44c9df7`(T3 cli)→
`57383fc`(T4 pin+rename)→ `fdfc85e`(status 更新)。

## Risks

- **mandatory deny net の一時的喪失** — `python`/`ssh`/`tar` 等が
  opt-in 無しで受理される。親 ADR が受理した降格順序
  (mandatory 除去 → opt-in 再配置)であり、#164 の
  `security.execDenyPatterns` が補完を担う。#163 単独では deny
  能力は無い。
- **route viability との一時的不整合** — exec 側の metachar
  reject を消しても viability の UnsafeCommand 分類
  (`resolve.rs:285`)は残るため、metachar argv は command tool
  層では受理可能なのに route 不発になる diverge が残る
  (issue 4 の整理対象として意図的残置、`resolve.rs:279-281`
  comment は known-stale)。
- **pin 内容差替の検出範囲縮小** — B3 除去で同 path の非
  native 化は `resolved == pinned` のまま検出不能(design 受理
  済み trade-off、`spawn_time_revalidation` 再設計 test で
  明文化)。
- **bare `node` positive pin の mise toolchain 前提** — PATH に
  node が無い環境では unsupported 経路へ倒れる。mise
  toolchain(node 26)を `mise run check`/`mise exec` 経路で
  前提。前提が崩れた場合は該当 pin を cfg-gate/削除。
- **ENOEXEC → libc `/bin/sh` fallback** — shebang 無し +X text
  file が execvpe 系で `/bin/sh` 解釈され得るのは OS/libc
  既定動作であり cheaper の保証対象外(design で明文化済み、
  issue 7 の docs 化で扱う)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #163)由来の未分類・未処理の follow-up はない。
下記は全て spec「Out of Scope」・design「Deferred Design
Concerns」・test-spec「Non-goals」・各 task「Out of Scope」で
scope 外確定・記録済みの defer であり、親 chain(#151)の後続
分解 issue が追跡するため新規 chain は起こさない。

- **`security.execDenyPatterns` opt-in profile** — issue #164。
  operator 宣言 pattern を compile する別 predicate を同 3
  適用点(resolve・pin・spawn 前再検証)へ再配置する。#163 が
  保全した適用点構造(if-block 丸ごと除去で残した再挿入位置)
  を再利用する。builtin 旧既定値の docs 記録・migration 例は
  #164/#169。
- **route viability 層の審査依存整理 + grant-set 投影** —
  issue 4(#166)。`resolve.rs:285` の `argv_shell_re` unsafe
  分類・viability 評価順の見直し。substitute mode 宣言時の
  検査対象 retarget は issue 5(#167)の scope。
- **docs 全体の正直性 sweep + breaking-change migration
  guide** — issue 7(#169)。「shell-free」「boundary/sandbox」
  表現と ENOEXEC fallback・内容差替非検出の明文化。sweep
  対象に `docs/guides/` 全体(特に `command.md` の deny
  list/metachar/workspace-resident 記述と `managed-process.md`
  の shim/native magic/bare `node` 固定ルール記述)、
  `docs/test-specs/invocation-scoped-command-grants.md`(deny
  list 前提の Rule・AC)、consumer 配布物の
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md:353`
  (`policy grant add` の "deny list" 記述 — repo 内 source の
  修正だけでは配布済み consumer へ届かない)が design で列挙
  済み。
- **platform 差の明文化** — issue 7(#169)。Windows の
  `.bat`/`.cmd`/`.ps1` は残置する `.exe`/`.com` viability rule
  で pin 時に失敗し続けるため、docs は「script が実行可能に
  なる」と platform 非依存に読める記述を避ける必要がある。
- **deny 一覧の migration 資料化** — `command_denied_executable_re`
  の builtin 一覧を「旧既定値」として docs へ移す作業は
  #164/#169。本 issue では一覧ごと削除するのみ。
