---
id: "TASK-034KJu8k1x8fWOfRemmi3r"
type: "task"
status: "todo"
title: "除去の grep-pin・keep-boundary・mise run check 最終検証"
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
        path: "docs/tasks/remove-core-policy-content-audit.md"
      - type: "file"
        path: "docs/tasks/remove-tools-content-audit.md"
      - type: "file"
        path: "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
      - type: "file"
        path: "docs/tasks/verify-content-audit-removal.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/exec-contract-content-audit-removal.md"
  implemented-by: []
  depends-on:
    - "docs/plans/exec-contract-content-audit-removal.md"
    - "docs/tasks/repurpose-cli-tests-and-comment-churn.md"
  blocks: []
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
  defers: []
  deferred-by: []
---

# 除去の grep-pin・keep-boundary・mise run check 最終検証

## Work

plan `docs/plans/exec-contract-content-audit-removal.md`
(PLAN-034KJtLyNUE46XjtrMOG60)の T4。T1-T3 で完了した除去が
design の Verification Notes・spec の Acceptance Criteria 通りに
成立していることを、grep-pin・観測 test の存在確認・full
verification pipeline で閉じる。product code は変更しない
(検証と doc status 更新のみ)。

### 1. name-disappearance grep

```bash
grep -rn "is_shell_process_executable\|native_magic_supported\|\
within_workspace\|is_relative_executable_path\|script_ext_re\|\
shell_executable_re\|command_denied_executable_re\|\
is_denied_command_executable" crates/
```

8 名全てが `crates/` から消失していること。許容 residual:
`within_workspace` は `walker.rs`/`normalization.rs` の無関係な
test 名(`..._under_within_workspace`)のみ hit してよい —
除去対象は `command/mod.rs` の fn とその呼出しであり、test 名の
残存は許容する(design Verification Notes)。

### 2. stale-vocabulary grep(spec R10)

```bash
grep -rniE "deny|workspace-resident|shim|native magic" crates/
```

残存 hit を全て確認し、除去した審査への言及が残らないこと。
許容 hit は (a) `denyPaths`/`PathDenylist`/`denied_declared_path`
等の denylist 機構(E2 — executable deny とは別物)、(b)
hook/permission 判定の `deny` verdict・capability/empty-grants
由来の `denied` test 名、(c) 「shim」の非該当用法(`serde_yml`
shim・PATH shim・self-reshim)、(d) `route/resolve.rs:279-281`
の viability comment(前提は issue 4 完了まで一時的に偽 —
known-stale として残置し comment churn 対象外)のみ。

### 3. keep-boundary 残置確認

- `argv_shell_re` 定義(`crates/cheaper-core/src/policy/process.rs`)
  と `route/resolve.rs:285` の参照が残置されている。
- `tokenize_shell`(`hook/routing_policy.rs`)・`resolved != pinned`
  ・wildcard 構造検査(`ArgPattern::compile`/`matches`)・
  `relative()`・env NUL/dup 検査・`PathDenylist`/
  `denied_declared_path`・win32 `.exe/.com`・X_OK・
  `spawn_failure_is_async`・`env_clear`/stdin null が変わらず
  残る。
- 削除した審査の別条件での再実装・`tokenize_shell` の拒否転用・
  #164 用の seam/flag/stub が diff に含まれない(spec R11 — diff
  目視 + grep の副産物として確認)。

### 4. 観測 pin の存在確認

- `\0` degradation: `"a\0b"` を含む argv が `POLICY_VIOLATION`
  でなく `TOOL_EXECUTION_FAILURE` + `details.spawnCode=
  "SPAWN_ERROR"` として観測される test が存在する
  (`proc_command.rs` 再用途 — message 非依存の code+spawnCode
  pin)。
- `\0` executable 不縮退: `\0` を含む declared executable が pin
  層 `POLICY_VIOLATION` のまま失敗する fail-closed test が存在する。
- 受理 pin: `python`/`ssh`/`tar`/`bash`/`.sh`/workspace-resident
  binary・symlink basename・bare `node` 宣言が opt-in deny 未指定
  で受理される test が存在する(resolve 層は名前で deterministic、
  pin 層は workspace-resident file・`.sh`・symlink で pin)。
- `a*b` repurpose determinism: `load.rs` unit・`load.json`
  golden・`run_dev_seam` case・`proc_degrade` spec・regression
  case が `argv:["a*b"]`(または `git`+`timeout:"5x"`)で
  PATH 非依存の `INVALID_RUNTIME_POLICY` を pin する。
- pin-equality 再設計: `spawn_time_revalidation` 相当が path 差替
  (symlink retarget・realpath 不一致・file 消失)を B3 非依存で
  検出し、内容差替非検出を positive assert する。

### 5. full verification・status 更新

- `mise run check` が pass すること。
- T1-T4 の task status を `done`・plan status を `completed` へ
  更新する(実測結果を task の Verification 欄へ記録)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/exec-contract-content-audit-removal.md`
  (PLAN-034KJtLyNUE46XjtrMOG60 — T4)
- `docs/designs/exec-contract-content-audit-removal.md`
  (DESIGN-034KIsSCvVMAxGDZY1dQTd — Verification Notes・
  Keep-Boundary Pins が AUTHORITATIVE)
- `docs/specs/exec-contract-content-audit-removal.md`
  (SPEC-034KI3mWcW92kMljP5xoa8 — Acceptance Criteria)
- `docs/test-specs/exec-contract-content-audit-removal.md`
  (TSPEC-034KJtUqMNXjkbjsbz5YPa — 全 Rules の最終確認)

## Dependencies

- `docs/tasks/repurpose-cli-tests-and-comment-churn.md`(T3) —
  全除去・churn が完了していること。

## Affected Files

- `docs/tasks/remove-core-policy-content-audit.md`・
  `docs/tasks/remove-tools-content-audit.md`・
  `docs/tasks/repurpose-cli-tests-and-comment-churn.md`・
  `docs/tasks/verify-content-audit-removal.md`(status 更新)
- `docs/plans/exec-contract-content-audit-removal.md`(status 更新)
- 観測 pin 欠落が見つかった場合のみ、該当 test file へ pin 追加
  (本来は T1-T3 の責務 — 本 task で新規 test を書くのは
  gap 検出時に限る)

## Done When

- [ ] name-disappearance grep が許容 residual を除き 0 hit
- [ ] stale-vocabulary grep の残存 hit が allowlist(a)-(d) のみ
- [ ] keep-boundary の各 site が不変で残存(removed symbol の
      import/call 残骸・審査の再実装が無い)
- [ ] `\0` degradation・受理 pin・`a*b` determinism・pin-equality
      再設計の各 test が存在し pass
- [ ] `mise run check` exits 0
- [ ] 4 task + plan の status が更新されている

## Out of Scope

- product code の追加変更(除去漏れが見つかった場合は該当 task
  へ差し戻す — T4 で大規模な除去をやり直さない)
- issue #164(opt-in deny)・issue 4(viability 整理)・issue 7
  (docs sweep)の作業
- `route/resolve.rs`・`tokenize_shell`・`argv_shell_re` 定義への
  変更(残置確認のみ)

## Verification

- [ ] 上記 §1-§5 の全項目が pass(コマンド実測を記録)
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で本 issue の新規 doc 6 件(plan・test-spec・task×4)が
      error 0
