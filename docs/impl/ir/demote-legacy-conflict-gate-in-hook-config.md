---
id: "IMPL-034IMWFzhYCEc3udAKl2xZ"
type: "impl"
status: "completed"
title: "Demote legacy conflict gate in hook config"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "docs/tasks/demote-legacy-conflict-gate-in-hook-config.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/demote-legacy-conflict-gate-in-hook-config.md"
  implemented-by: []
  depends-on: []
  blocks:
    - "docs/tasks/update-conflict-demotion-docs-and-mjs-test.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-legacy-conflict-warning-contract.md"
  references:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Demote legacy conflict gate in hook config

## Summary

plan T1: `crates/cheaper-cli/src/hook_config.rs` の legacy conflict gate を
blocker から warning へ降格した(issue #128 — dedup 導入で「重複発火は
unsafe」という gate 前提が除去されたため)。`run_verify` は conflict を
informational `legacy-conflict:` finding として報告し `clean` は blocker
系 finding のみから導出、`run_sync` は early return を除去して upsert を
継続し result に `legacyConflicts`(常時)+ `warning`(conflict 時のみ)を
quiet gate の外で insert、`run_remove` は `residualRegistrations` +
note の構造を維持して文言のみ新規約へ整合した。文言は design の
Wording Contract の確定文字列をそのまま使用(独自言い換え無し)。

## Context

- task: `docs/tasks/demote-legacy-conflict-gate-in-hook-config.md`
  (TASK-034IM7QWavO611vU5ZdhmT)
- plan: `docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md`
  (T1 — code + Rust integration test の先頭 slice)
- spec: `docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (SPEC-034IKGkW4dQEVw6QigMFxQ — R1-R11・exit code 表・AC)
- design: `docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (DESIGN-034ILuADFr70oZoUROR13q — touch point 単位の実装青写真・
  Wording Contract)

## Implementation

### Changes

- `crates/cheaper-cli/src/hook_config.rs`
  - `run_verify`(conflict 処理)— `blocker = true` を除去し per-file
    `legacy-conflict:` finding を Wording Contract 文言("safe to keep —
    duplicate firings are deduplicated at eval time for identical
    requests; remove that registration to converge on a single
    registration")へ置換。`legacyConflicts` 常時 populate は不変。
  - `run_verify` `remediation` — `!quiet && blocker` gate を維持し文言のみ
    "to converge the registration" 形へ置換(conflict 解消を sync 前提の
    ように読ませない)。
  - `run_sync` — `invalid_request` early return を除去。conflict scan は
    design の scan-before-write 決定どおり write 前位置に保持(観測結果は
    同一・verify の read-only semantics と対称)。result 組立で
    `changed`/`matcher` insert 直後・quiet gated `note` の外に
    `legacyConflicts`(空 array でも常時)+ `warning`(conflict 検出時のみ)
    を insert — `note` の quiet gate を複製していない(R1/R2)。
  - `run_remove` — `residualRegistrations` key・emit 条件は不変で note
    文言のみ新規約へ整合("safe to keep — duplicate firings are
    deduplicated at eval time for identical requests")。
  - stale comment 修正 — `legacy_conflicts` の parse-skip comment を
    実動作(verify は sibling の parse 失敗を finding として報告しない
    best-effort skip)へ修正、`run_sync` doc-comment を降格後責務
    表現(「冪等 upsert + legacy conflict の informational 報告」)へ。
  - `run_event_stdin` はコード変更なし — verify code passthrough 構造で
    降格を継承(post-install/post-update は conflict-only で exit 0、
    post-uninstall packages 判定不能は conflict-only residual で
    exit 2→0)。検出規則(`CONFLICT_PATTERN`・`KNOWN_LEGACY_FILES`・
    `legacy_conflicts()`)も不変。
- `crates/cheaper-cli/tests/integration/hook_config.rs`
  - header comment の「legacy gate」を降格後表現(legacy conflict
    reporting)へ更新。
  - `hook_config_sync_fails_on_legacy_conflict` →
    `hook_config_sync_warns_on_legacy_conflict` へ rewrite(sync →
    exit 0・`ok:true`・`legacyConflicts` len 1・`warning` 非空・config
    書込み済み own entry;verify → exit 0・`clean:true`・
    `legacy-conflict:` finding・全 finding が "duplicate firing is
    unsafe" を含まない)。
  - 新規 test 6 件: `hook_config_event_stdin_converges_on_legacy_conflict`
    (post-install + post-update × conflict で exit 0・
    `result.sync.legacyConflicts`/`warning`・`result.verify.clean`・
    own entry 存在)、
    `hook_config_post_uninstall_undetermined_converges_on_conflict`
    (packages 欠落 × conflict-only residual → exit 0・
    `action:"skipped"`・`verify.clean`)、
    `hook_config_sync_quiet_keeps_conflict_fields`(`--quiet` で
    `legacyConflicts`/`warning` 持続・`note` 不在の R2 guard)、
    `hook_config_sync_retained_errors_with_conflict`(conflict 併存下の
    malformed/未知 matcher modified/duplicate own で exit 2・`ok:false`)、
    `hook_config_verify_blockers_retained_with_conflict`(conflict +
    missing/modified/duplicate で exit 2・`clean:false`・両 finding 併発)、
    `hook_config_remove_reports_residual_conflict`(
    `residualRegistrations` 非空・note が "deduplicated at eval time"
    を含む)。
  - 共通 helper `write_legacy_conflict`・`write_config_with_groups`・
    `assert_own_entry_present` を追加。
- `docs/tasks/demote-legacy-conflict-gate-in-hook-config.md` — status
  `done`、checklist 全項目 checked。

### Decisions During Implementation

- wording は design Wording Contract の確定文字列を verbatim で使用し、
  Rust の `\` 行継続で 1 行 JSON-safe 文字列として組立てた(独自の
  言い換え・wrap での単語分割変更は無し)。
- sync の `legacyConflicts`/`warning` insert は `changed`/`matcher`
  insert の直後・quiet gated `note` の外に置いた — `note` の quiet
  gate を複製しないことが R2 guard(test `hook_config_sync_quiet_keeps_
  conflict_fields` で固定)。
- 降格の副次変化(design 確定済み)は意図どおり残した: conflict +
  config 不在では `absent-config` blocker が `legacy-conflict` と併発し
  exit 2、`sweep_orphans` が conflict 有り sync でも実行される。

## Related Experiments

### Adopted

なし(既知の実装 task のため Experiment Log は作成していない)。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook_config::` — 13 passed / 0 failed(exit 0)。
- `mise exec -- cargo clippy -p cheaper-cli --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0(警告無し)。
- `cargo fmt --all -- --check` — clean。
- `grep -n "duplicate firing is unsafe" crates/cheaper-cli/src/hook_config.rs`
  — 0 件(test file は文言非含有 assert のみで検出)。
- 上記 test run に加え `--test integration` 全体(13 hook_config を含む
  target 全 test)で回帰無しを確認。

## Risks

- conflict の見落とし — blocker でなくなり exit code に現れなくなるため、
  consumer が warning を見落とすリスクは残る(ADR で受容済み)。
  `warning`・`legacy-conflict:` finding・`residualRegistrations` note で
  可観測性は維持。
- PATH 上の旧 binary(dedup 無し)では重複登録が firing 毎に独立評価される
  — 影響は duplicate evaluation のみで verdict は同一(docs mitigation は
  T2 の `docs/guides/distribution.md` 更新で担保)。
- `install-devin-hooks.ps1:5` の「legacy conflict gate」header comment は
  wrapper scope 外のため stale のまま残す — drift として design に記録済み。

## Follow-ups

- T2 `docs/tasks/update-conflict-demotion-docs-and-mjs-test.md` —
  `docs/guides/distribution.md`・`packages/apm/.apm/skills/cheaper-usage/
  SKILL.md` の文言整合と `scripts/test/devin-hooks-install.test.mjs`
  :269-285 の warns 版 assert 書き換え(本 task で確定した result field・
  finding 文字列を引用する)。
