---
id: "PLAN-034KJtLyNUE46XjtrMOG60"
type: "plan"
status: "approved"
title: "exec-contract cleanup — 内容審査を mandatory 層から除去 (issue #163)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
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
  implements:
    - "docs/specs/exec-contract-content-audit-removal.md"
  implemented-by:
    - "docs/tasks/remove-core-policy-content-audit.md"
    - "docs/tasks/remove-tools-content-audit.md"
    - "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
    - "docs/tasks/verify-content-audit-removal.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/exec-contract-content-audit-removal.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/exec-contract-content-audit-removal.md"
    - "docs/designs/exec-contract-content-audit-removal.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/discovery/exec-contract-content-audit-removal.md"
  derived-by:
    - "docs/tasks/remove-core-policy-content-audit.md"
    - "docs/tasks/remove-tools-content-audit.md"
    - "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
    - "docs/tasks/verify-content-audit-removal.md"
  verifies: []
  verified-by:
    - "docs/test-specs/exec-contract-content-audit-removal.md"
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/discovery/permission-model-responsibility-redesign.md"
    - "crates/cheaper-core/src/policy/process.rs"
    - "crates/cheaper-core/src/policy/command.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
    - "crates/cheaper-tools/src/tools/proc/executable.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
  defers: []
  deferred-by: []
---

# exec-contract cleanup — 内容審査を mandatory 層から除去 (issue #163) Implementation Plan

## Goal

DESIGN-034KIsSCvVMAxGDZY1dQTd の Removal Contract に従い、
content-audit クラス 7 項目(A8・A10・B3・B4・B5・B6・F3)と A9 deny
list の mandatory 適用点を、実行契約の mandatory 層(grant/task
resolve → pin → spawn 前再検証 → argv 認可)から一括除去する
(ADR-034KI3r2xdthh6rKDFY517 Option A + predicate 定義も call site
と共に削除)。除去後に cheaper が強制するのは**宣言された実行契約
のみ** — shell-free argv spawn・grant/task 認可・pin 同一性・
wildcard 構造・cwd/env 宣言境界・lifecycle 境界・`PathDenylist`。
審査の再実装・補償 check は行わない(spec R11)。

## Scope

単一の implementation stream。「mandatory 層からの内容審査除去」という
1 つの behavioral 変更に test churn が横断的に付随する構成であり、
除去単位・disposition・test 張替えは design が行単位で固定済み —
分割するのは commit の論理境界(crate 層)のみで、plan は 1 つに収める。

|| In | Out |
|---|---|---|
| `policy/process.rs`・`policy/command.rs` の審査 predicate・regex 定数・task loop/resolve/`ArgPattern::compile` 適用点の除去 | `argv_shell_re` 定義と `route/resolve.rs:285` の viability 参照(残置 — issue 4 scope) |
| `tools/command/mod.rs`・`tools/proc/executable.rs` の `within_workspace`・pin pre-filter・verify 再査・`authorize_args` metachar loop・`native()` magic/shim・front-door・bare `node` reject の除去 | pin 取得(`resolve_process_executable` PATH 検索→canonical realpath)・`native_executable` 再解決・`resolved != pinned` pin-equality・wildcard 構造検査(B1/B2/A6) |
| error message・doc comment の誠実化(`unsupported()`・`unsupported_empty_command`・pin 導入 comment・各 fn doc — deny/residency/metachar 言及の除去、spec R10) | `PathDenylist`/`denyPaths`・`denied_declared_path`(E2 — executable deny とは別機構)・`route/resolve.rs:279-281` の comment(known-stale として残置 — issue 4 が整理) |
| 観測差分の pin 化(`\0` argv の POLICY_VIOLATION → spawn 層 TOOL_EXECUTION_FAILURE 縮退) | `\0` 以外の metachar の取扱い(literal argv として子へ届く — shell-free spawn では data)・declared executable 内 `\0`(realpath 失敗で pin 層 POLICY_VIOLATION のまま — 縮退しない) |
| Test/Fixture Churn Plan の全項目(golden fixture・unit・integration・dev-seam・regression の更新・削除・再用途化) | `spawn_time_revalidation` が検出しない範囲(同 path の内容差替は B3 除去で検出対象外 — design 受理済み trade-off) |
| #164 のための適用点構造の保全(3 適用点の自然な再挿入位置 — if-block 丸ごと除去で残す) | `security.execDenyPatterns` schema・opt-in deny predicate・seam/flag/stub(#164 scope — 本 issue は一切残さない) |

## Upstream Documents

- `docs/specs/exec-contract-content-audit-removal.md`
  (SPEC-034KI3mWcW92kMljP5xoa8、approved — R1-R12・acceptance criteria)
- `docs/adr/exec-contract-content-audit-removal.md`
  (ADR-034KI3r2xdthh6rKDFY517、accepted — Option A 一括除去 +
  predicate 定義削除・棄却 option の理由)
- `docs/discovery/exec-contract-content-audit-removal.md`
  (DISC-JLHFz6x4ZuhzYZrpfrG2QV、resolved — 実測 file:line
  inventory・keep list・観測差分の ground truth)

## Design Inputs

- `docs/designs/exec-contract-content-audit-removal.md`
  (DESIGN-034KIsSCvVMAxGDZY1dQTd、approved — AUTHORITATIVE な HOW:
  4 file 別 Removal Contract・error contract 変化・`\0` degradation
  path・Test/Fixture Churn Plan 全 row・Keep-Boundary Pins・#164
  境界・Verification Notes)。本 plan・task は design の行指定を
  転記するだけで、除去対象・残置対象を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-core/src/policy/process.rs` | `shell_executable_re`(:117-125)・`script_ext_re`(:127-130)・`is_shell_process_executable`(:141-148)削除、task loop の 2 適用点(:272-280)削除、`argv_shell_re`(:134-137)は残置し doc comment を viability 参照へ訂正、unit test 更新 | T1。`relative()`・env dup/NUL(:290-308)は残す |
| `crates/cheaper-core/src/policy/command.rs` | `command_denied_executable_re`(:160-189)・`is_denied_command_executable`(:196-207)・`is_relative_executable_path`(:214-223)削除、`ArgPattern::compile` の metachar 検査(:249-251)+doc(:242-244)・resolve の 2 適用点(:372-381)削除、import(:23-28)整理、unit test 更新 | T1。wildcard 構造検査(:252-271)・env/cwd 規則は残す |
| `crates/cheaper-core/src/policy/load.rs` | module doc(:13)・Partial validate comment(:151-152)の「deny list」言及を restate、`invocation_denied_executable_keeps_inner_code_with_layer`(:527-545)を `{"executable":"python","argv":["a*b"]}` spec へ再用途化 | T1。wildcard 構造違反で code+layer を deterministic に維持 |
| `crates/cheaper-core/tests/golden/policy_process.rs` | `is_shell_process_executable` import・`"isShellProcessExecutable"` op arm 削除 | T1 |
| `crates/cheaper-core/tests/golden/fixtures/policy/process.json` | shell/script/metachar 4 case を受理 expected へ更新・rename、`isShellProcessExecutable` op 14 件削除 | T1 |
| `crates/cheaper-core/tests/golden/fixtures/policy/load.json` | denied executable case を `a*b` spec へ再用途化・case rename(expected 不変) | T1 |
| `crates/cheaper-core/tests/golden/fixtures/policy/schema.json` | `sh` task case を受理 expected(`{"ok":{"policy":...}}`)へ更新・case rename | T1 |
| `crates/cheaper-tools/src/tools/command/mod.rs` | `within_workspace`(:167-170)・pin pre-filter(:213-219)・pin の deny/resident 再査(:230-244)・verify の deny/resident 再査(:267-278)・`authorize_args` metachar loop(:468-475)削除、`verify_pinned_executable` の `root` 引数除去+call site 追随、import(:24-29)整理、module/fn doc restate | T2。`resolve_process_executable`・`native_executable`・`resolved != pinned`・argv pattern match・pin 不在 fail-closed は残す |
| `crates/cheaper-tools/src/tools/proc/executable.rs` | `native_magic_supported`(:74-86)・magic-byte read(:108-117)・`native()` shim 判定(:92-94)・front-door shim 判定(:143-145)・bare `node` reject(:163-173)削除、`unsupported()`(:35-42)・`resolve_process_executable` doc(:128-137)・設計固定ルール doc(:6-11)restate、import(:13,17)整理、`native_bytes` fixture rename | T2。win32 `.exe/.com`・X_OK・workspace-relative 解決+denylist・PATH candidate loop は残す |
| `crates/cheaper-tools/src/tools/process/prepare.rs` | `unsupported_empty_command`(:140-145)の message を `unsupported()` と同規約で restate | T2 |
| `crates/cheaper-tools/src/registry.rs` | pin 導入 comment(:1202-1207)を「pin 不能の grant は fail」へ訂正 | T2 |
| `crates/cheaper-tools/tests/proc_command.rs` | 7 test の更新・再用途化(declared deny→受理 pin・resolved basename→受理・workspace resident→受理・fail-closed message・metachar→literal argv+`\0` degradation pin・spawn revalidation→path 差替再設計・shared predicate metachar assert) + header(:12-18)・:404 comment | T2。B4/B5/A9 受理 pin・`\0` degradation 観測点 |
| `crates/cheaper-tools/tests/proc_degrade.rs` | `pin_failure_stays_hard` の message assert・`resolve_policy_errors_stay_hard`(:272-284)の spec を `{"executable":"node","argv":["a*b"]}` へ更新 | T2 |
| `crates/cheaper-tools/tests/proc_process.rs` | `node_task_is_an_explicit_policy_violation` を PATH 解決 pin へ再用途化・header(:3-8) | T2。mise toolchain node 前提 |
| `crates/cheaper-tools/tests/proc_invocation.rs` | `self_grant` doc(:581-584)の前提記述を訂正 | T2。test 本体不変 |
| `crates/cheaper-tools/tests/differential.rs` | header(:9-15)・`translate_process_spec` comment(:613)の「Rust は node を PATH 解決しない」記述を除去 | T2。`expectRust` 機構自体は tiktoken pin で継続使用 |
| `crates/cheaper-tools/tests/fixtures/process.json` | `"bare node task is an explicit Rust error"` case(:803-841)の `expectRust` 除去・base expected 復帰・case rename | T2 |
| `crates/cheaper-cli/src/policy.rs` | module doc(:8-9)・`write_validated` doc(:245-247)の「deny list」言及を除去後の検査内容へ restate | T3 |
| `crates/cheaper-cli/tests/integration/policy.rs` | `policy_grant_add_rejects_invalid_spec_without_writing` の `{"executable":"python"}` spec を `{"executable":"git","timeout":"5x"}` へ置換 | T3 |
| `crates/cheaper-cli/tests/run_dev_seam.rs` | `run_fake_stream_grant_deny_listed_executable_rejected`(:907)を `a*b` spec へ再用途化+rename、`self_grant` doc(:359-362)訂正 | T3 |
| `crates/cheaper-cli/tests/regression/cases/run-grant-deny.json` + `expected/run-grant-deny.json` | spec を `a*b` 化・name/files_note を invalid-spec 趣旨へ変更し `run-grant-invalid-spec.json` へ rename(expected 内容不変 — code+layer のみ pin) | T3。削除+追加として manifest に列挙 |
| `crates/cheaper-cli/tests/workflow.rs`(:505)・`workflow_dev_seam.rs`(:166) | 「workspace-resident 拒否に掛からない」記述を pin determinism の趣旨へ訂正 | T3 |

## Implementation Sequence

- [ ] TASK-034KJtXX2QajBjkAgOJRtm (T1): cheaper-core policy 層の
      内容審査除去 — `process.rs`/`command.rs`/`load.rs` の
      predicate・適用点・import 除去+doc restate、同 file の unit
      test 更新・削除・再用途化、golden driver(`policy_process.rs`)
      + 3 golden fixture の更新。`cargo test -p cheaper-core` で閉じる
- [ ] TASK-034KJu31OgzJMwk2vb4KJz (T2): cheaper-tools command/proc
      層の内容審査除去 — `command/mod.rs`・`proc/executable.rs` の
      審査分岐・helper・import 除去+message/doc restate、
      `prepare.rs`/`registry.rs` の churn、同 crate の unit・
      integration test・fixture の更新・再用途化(受理 pin・
      `\0` degradation・path 差替 pin を含む)。`cargo test -p
      cheaper-tools` で workspace compile を回復させて閉じる
- [ ] TASK-034KJu3PCBQDeImZEcznWv (T3): cheaper-cli の churn —
      `policy.rs` doc restate、integration `policy.rs` spec 置換、
      `run_dev_seam.rs` case 再用途化+doc、regression `run-grant-deny`
      →`run-grant-invalid-spec` rename、`workflow.rs`/
      `workflow_dev_seam.rs` comment 訂正。cli test target で閉じる
- [ ] TASK-034KJu8k1x8fWOfRemmi3r (T4): grep-pin(消失・旧語彙
      allowlist・keep-boundary)・`\0` degradation pin・`a*b`
      determinism の最終確認、`mise run check`、task status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 core policy 層除去 | — | T2 |
| T2 tools 層除去 | T1 | T3 |
| T3 cli churn | T2 | T4 |
| T4 最終検証 | T3 | — |

critical path は T1 → T2 → T3 → T4 の完全直列。並列実行可能な
stream は無い — T1 が core の pub symbol を消すため T2 完了まで
`cargo build -p cheaper-tools`/`cheaper-cli` は compile 不可
(T1 境界の既知の transient で、T1 の検証は `cargo test -p
cheaper-core` に scope)。T2 で workspace compile が回復し、cli の
旧挙動観測 test(3 箇所: integration `policy.rs`・dev-seam deny
case・regression deny case)だけが T3 まで残る。各 task は 1 commit
で、file-local の unit test・doc comment は同 commit で追随する。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | T1 commit 境界で cheaper-tools/cli が compile 不可(`is_shell_process_executable`・`is_denied_command_executable`・`is_relative_executable_path` の import 残留) | high(必発) | low | 既知の transient — T1 の検証を `cargo test -p cheaper-core` に scope し、T2 が import 除去で即回復させる直列構成 |
| 2 | `node`/PATH 解決の環境依存 — bare `node` positive pin は PATH に node が無い環境で unsupported 経路へ倒れる | low | med | mise toolchain(node 26)前提を task・TSPEC に明記 — `mise run check`/`mise exec` 経路では保証。前提が崩れた場合は該当 pin を cfg-gate/削除 |
| 3 | `\0` argv の error 分類縮退(POLICY_VIOLATION → TOOL_EXECUTION_FAILURE+`spawnCode="SPAWN_ERROR"`)を観測する pin が platform 差で脆い | low | med | design が spawn 層経路を固定済み — `proc_command` の再用途 test で code+spawnCode のみ assert(message 非依存)。declared executable 内 `\0` は縮退しないことを fail-closed test で別 pin |
| 4 | pin 内容差替の検出範囲縮小(B3 除去で同 path の非 native 化は `resolved == pinned` のまま検出不能) | high(必発・受理済み) | low | `spawn_time_revalidation` を path 差替(symlink retarget・file 消失)検出+内容差替非検出の positive assert へ再設計 — design 受理済み trade-off を test で明文化 |
| 5 | 旧語彙残存(spec R10 違反)や keep 対象(`argv_shell_re`・`relative()`・env NUL・wildcard 構造・pin-equality)の誤削除 | med | med | T4 の grep-pin(消失名・旧語彙 allowlist・keep-boundary)を検証契約として TSPEC 化 — 機械的に確認可能 |
| 6 | 除去した審査の補償的再実装・`tokenize_shell` の拒否転用(spec R11 違反) | low | high | 再実装禁止を plan・task・TSPEC の Non-goals/Guarantees に明記。`route/resolve.rs`・`argv_shell_re`・`tokenize_shell` は触れないことを Out of Scope で固定 |
| 7 | route viability の一時的不整合(metachar argv は exec 受理・route 不発) | high(必発・受理済み) | low | issue 4 の整理対象として意図的残置 — `resolve.rs:279-281` comment は known-stale のまま触らない(design 記録済み) |
| 8 | mandatory deny net の一時的喪失(`python`/`ssh`/`tar` 等が opt-in 無しで受理) | high(必発・受理済み) | med | ADR Consequences 受理済みの降格順序(mandatory 除去 → #164 opt-in 再配置)。`security.execDenyPatterns`・seam/flag/stub は本 issue では実装しない |
| 9 | data-loss / breaking-change | — | — | behavioral breaking change(migration 告知は issue 7 docs 作業)。schema・feature flag・依存追加は無し |
| 10 | external dependency | — | — | 新規 crate 依存ゼロ(design 明記)。検証は unit/integration/golden/regression + grep のみ |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step(migration・外部 format 固定)は無い — regression case
の rename も file pair の復元で再現可能。

## Task Breakdown

- T1 `docs/tasks/remove-core-policy-content-audit.md`
  (`TASK-034KJtXX2QajBjkAgOJRtm`) — `policy/process.rs`・
  `policy/command.rs`・`policy/load.rs` の predicate・適用点・import
  除去+restate、unit test 更新・削除・再用途化、golden driver+
  fixture 更新。
  verified by `docs/test-specs/exec-contract-content-audit-removal.md`
- T2 `docs/tasks/remove-tools-content-audit.md`
  (`TASK-034KJu31OgzJMwk2vb4KJz`) — `tools/command/mod.rs`・
  `tools/proc/executable.rs`・`tools/process/prepare.rs`・
  `registry.rs` の審査分岐・helper・import 除去+restate、同 crate
  test・fixture の更新・再用途化。
  verified by `docs/test-specs/exec-contract-content-audit-removal.md`
- T3 `docs/tasks/repurpose-cli-tests-and-comment-churn.md`
  (`TASK-034KJu3PCBQDeImZEcznWv`) — cheaper-cli の doc/test/
  regression churn。
  verified by `docs/test-specs/exec-contract-content-audit-removal.md`
- T4 `docs/tasks/verify-content-audit-removal.md`
  (`TASK-034KJu8k1x8fWOfRemmi3r`) — grep-pin・keep-boundary・
  `\0`/`a*b` 観測点確認・`mise run check`・status 更新。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      で unit test(更新・再用途化済み)+ golden driver/fixture が
      pass(T1)
- [ ] `mise exec -- cargo test -p cheaper-tools --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
      で unit・proc_* integration・differential・fixture が
      pass(T2 — workspace compile 回復)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration --test regression --test run_dev_seam --test workflow --test workflow_dev_seam`
      が pass(T3)
- [ ] grep 消失確認: `is_shell_process_executable`・
      `native_magic_supported`・`within_workspace`・
      `is_relative_executable_path`・`script_ext_re`・
      `shell_executable_re`・`command_denied_executable_re`・
      `is_denied_command_executable` が `crates/` から消失
      (`within_workspace` は `walker.rs`/`normalization.rs` の
      無関係な `..._under_within_workspace` test 名 residual を
      許容)(T4)
- [ ] 旧語彙 grep(spec R10): `deny|workspace-resident|shim|native magic`
      の残存 hit が allowlist(E2 denylist 機構・hook/permission の
      `deny` verdict・capability/empty-grants 由来 `denied` test 名・
      非該当「shim」用法・`route/resolve.rs:279-281` known-stale
      comment)内に収まること(T4)
- [ ] keep-boundary 残置確認: `argv_shell_re` 定義・`route/resolve.rs:285`
      参照・`tokenize_shell`・`resolved != pinned`・wildcard 構造
      検査・`relative()`・env NUL/dup・`PathDenylist`・`.exe/.com`・
      X_OK が不変で残ること(T4)
- [ ] `\0` argv が `TOOL_EXECUTION_FAILURE`+`spawnCode="SPAWN_ERROR"`
      へ縮退する観測 test・`a*b` repurpose が PATH 非依存で
      deterministic な test が存在すること(T4)
- [ ] `mise run check` が pass(T4)
- [ ] `node scripts/check-doc-ids.mjs` が pass(全 task)
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で新規 doc の error 0(全 task)

## Open Questions / Gaps

- なし — Removal Contract・Churn Plan・keep boundary の全 site は
  design/discovery が file:line 精度で固定済み。`verify_pinned_executable`
  の縮小签名(`root` 除去)・`\0` degradation 経路・`a*b` repurpose
  の determinism も design で確定しており、実装者の判断余地は
  rename 後の test/case 命名と restate 後の message 語尾のみ。
- `node` positive pin の mise toolchain 前提は Risk#2 に記録 —
  toolchain が PATH を継承しない経路で test が走る場合のみ
  cfg-gate を検討する(本 plan では想定しない)。

## Self-Review

- [x] Every spec requirement maps to a task.(R1/R7→T1、R2-R6→T2、
      R8/R9→T1-T3 の非変更確認+T4 grep-pin、R10→各 task の restate+T4
      旧語彙 grep、R11→全 task の Out of Scope、R12→T1)
- [x] No placeholders remain.(未確定事項は Open Questions へ明示)
- [x] File paths, relations, status values, and terminology are
      consistent.(design `changes` manifest と同一 path 集合)

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation —
  完全直列 DAG のため task 単位の逐次実行+各 task の scoped
  verification checkpoint が適合する。
- Independent work streams: なし(T1→T2→T3→T4 直列。T1 境界では
  downstream crate が compile 不可の既知 transient)
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 焦点: 除去対象と keep boundary の境界(line 指定は design が
  AUTHORITATIVE — task が転記する範囲で一致するか)、`\0`
  degradation の観測 pin が code+spawnCode のみ assert し message
  非依存か、`a*b` repurpose が全経路で PATH 非依存か、T1→T2 の
  compile transient が scoped verification で正当に閉じているか。
- 明示仮定: test の positive pin(bare `node`・workspace-resident・
  `.sh`・symlink basename)は除去後の最終挙動を前提に T2 で記述
  する — T1 単独状態では core deny が残るため
  `declared_denied_executables` 系は T1 では触らない。
- 未解決依存: issue #164(opt-in deny 再配置)・issue 4(viability
  整理)・issue 7(docs sweep)は本 plan の完了条件に含まない —
  `route/resolve.rs`・`argv_shell_re`・`tokenize_shell` に触れない
  ことが本 issue の責務境界。
- 上流: SPEC-034KI3mWcW92kMljP5xoa8 /
  ADR-034KI3r2xdthh6rKDFY517 / DESIGN-034KIsSCvVMAxGDZY1dQTd /
  DISC-JLHFz6x4ZuhzYZrpfrG2QV。
