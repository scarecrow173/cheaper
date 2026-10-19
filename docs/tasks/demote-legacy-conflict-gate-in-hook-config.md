---
id: "TASK-034IM7QWavO611vU5ZdhmT"
type: "task"
status: "done"
title: "Demote legacy conflict gate in hook_config.rs and update Rust integration tests"
created: "2026-10-19"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
  implemented-by: []
  depends-on:
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
  blocks:
    - "docs/tasks/update-conflict-demotion-docs-and-mjs-test.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-legacy-conflict-warning-contract.md"
  references:
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  defers: []
  deferred-by: []
---

# Demote legacy conflict gate in hook_config.rs and update Rust integration tests

## Work

plan の T1: `crates/cheaper-cli/src/hook_config.rs` で legacy conflict gate を
blocker から warning へ降格し、`tests/integration/hook_config.rs` の contract
test を新規約へ更新する。文言は design の Wording Contract の確定文字列を
そのまま使う(独自の言い換えをしない)。

### Code 変更(`crates/cheaper-cli/src/hook_config.rs`)

- [x] `run_verify`(:668-682)— `blocker = true` を除去。per-file
      `legacy-conflict:` finding push と `legacyConflicts` 常時 populate
      (:679-682)は維持し、finding 文言を新契約へ置き換える:
      `legacy-conflict: {f} contains a cheaper hook registration (safe to keep — duplicate firings are deduplicated at eval time for identical requests; remove that registration to converge on a single registration)`
- [x] `run_verify` `remediation`(:730-738)— `!quiet && blocker` gate は
      維持し文言のみ置き換える:
      `run \`cheaper hook-config sync\` to converge the registration; see docs/guides/distribution.md for the APM lifecycle wiring`
- [x] `run_sync`(:757-768)— `invalid_request` early return を除去し、
      `let conflicts = legacy_conflicts(&root_dir, &config_path);` を write
      前位置で保持(降格 comment 付き)。upsert 継続後、result 組立(:850
      付近、`changed`/`matcher` insert 直後・quiet gated `note` の**外**)
      で `legacyConflicts`(string array、conflict 無しでも常時)を
      insert し、conflict 検出時のみ `warning` を insert(R1/R2 — `note`
      の quiet gate :852-858 を複製しない)。`warning` 文言:
      `legacy cheaper hook registrations remain (see legacyConflicts); coexisting is safe — duplicate firings are deduplicated at eval time for identical requests, so removal is needed only to converge on a single registration. A pre-dedup cheaper binary on PATH still evaluates each firing independently (duplicate evaluation only — the verdict is identical); see docs/guides/distribution.md`
- [x] `run_remove`(:932-938)— `residualRegistrations` key・emit 条件は
      不変のまま `note` 文言のみ置き換える:
      `remove deleted only the canonical entry; other files still contain cheaper registrations (safe to keep — duplicate firings are deduplicated at eval time for identical requests); run \`cheaper hook-config verify\` to inspect`
- [x] stale comment 修正 — `legacy_conflicts` :377 の「parse 不能 file は
      verify の finding で別途報告」を実動作(verify は sibling の parse
      失敗を報告しない best-effort skip)へ修正。`run_sync` doc-comment
      :742 を「canonical own entry の冪等 upsert + legacy conflict の
      informational 報告」系の降格後責務表現へ修正。:757-759 の「legacy
      gate:」comment は gate 本体と一緒に置き換わるため別途 touch しない。
      test file `crates/cheaper-cli/tests/integration/hook_config.rs:4` の
      header doc-comment「legacy gate」も :377/:742 と併せて降格後表現
      (「legacy conflict reporting」等)へ更新する
- [x] `run_event_stdin`(:973-1053)はコード変更なし — verify code
      passthrough 構造で降格を継承する(検出規則
      `CONFLICT_PATTERN`・`KNOWN_LEGACY_FILES`・`legacy_conflicts()` も
      不変)

### Test 変更(`crates/cheaper-cli/tests/integration/hook_config.rs`)

`fixture_with_cheaper_on_path` PATH shim パターンを再利用。

- [x] :132 `hook_config_sync_fails_on_legacy_conflict` を
      `hook_config_sync_warns_on_legacy_conflict` へ rewrite —
      `.windsurf/hooks.json` legacy 登録 fixture は流用。`sync` →
      `code == 0`・`env["ok"] == true`・`result.legacyConflicts` が len 1
      で windsurf path を含む・`result.warning` が非空 string・config
      file が存在し own entry(canonical matcher + `OWN_COMMAND`)が
      書き込まれている。`verify` → `code == 0`・`result.clean == true`・
      `legacyConflicts` populate・findings に `legacy-conflict:` prefix
      entry・全 finding が "duplicate firing is unsafe" を含まない
- [x] 新規 `hook_config_event_stdin_converges_on_legacy_conflict` —
      conflict setup + `{"event":"post-install","packages":["cheaper"]}`
      → `code == 0`・config 書込み・`result.sync.legacyConflicts` 非空・
      `result.sync.warning` 存在・`result.verify.clean == true`。
      続けて `{"event":"post-update","packages":["cheaper"]}` →
      `code == 0` + `config_path(&fx).exists()` と own entry(canonical
      matcher + `OWN_COMMAND`)の存在を assert
- [x] 新規 post-uninstall 判定不能 × conflict-only residual — own entry
      書込み済み + conflict 残りで `{"event":"post-uninstall"}`(packages
      欠落)→ `code == 0`・`result.action == "skipped"`・
      `result.verify.clean == true`(降格前は exit 2)
- [x] 新規 `hook_config_sync_quiet_keeps_conflict_fields` — conflict
      setup + `sync --quiet` → `code == 0`・`result.legacyConflicts` 非空・
      `result.warning` 存在・`result.note` 不在(R2 guard)
- [x] retained error — conflict 併存下で malformed `.devin/config.json` /
      未知 matcher の modified own entry / duplicate own entry の各条件で
      `sync` が `code == 2`・`env["ok"] == false`(1 test に fixture 再
      setup でまとめても良い)
- [x] verify blocker retention — conflict + own entry missing →
      `code == 2`・`clean == false`・findings が `missing` と
      `legacy-conflict` の両方を含む;own entry に余分な key(または未知
      matcher)→ `code == 2`・`modified:` prefix finding;own entry
      group 2 件 → `code == 2`・`duplicate:` finding(absent-config・
      malformed は既存 test :62-76・:222-254 が担保)
- [x] remove residual — conflict setup + own entry 書込み済み → `remove`
      → `code == 0`・`result.residualRegistrations` 非空・`result.note` が
      新文言("deduplicated at eval time" を含む)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md (T1 節)
- docs/test-specs/hook-config-legacy-conflict-warning-contract.md (test
  contract — Rules/Examples と test case 対応)
- docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md
  (Wording Contract — 確定文言)
- docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md (R1-R8)

## Dependencies

- 前提 task なし(plan の先頭 slice)。T2
  (`update-conflict-demotion-docs-and-mjs-test.md`)は本 task の着地が前提 —
  mjs test が新 contract の binary を spawn し、docs が確定文言を引用する。

## Affected Files

- modify: `crates/cheaper-cli/src/hook_config.rs`
- modify: `crates/cheaper-cli/tests/integration/hook_config.rs`

Out of scope(本 task では触らない): `scripts/test/devin-hooks-install.test.mjs`
・docs 2 file(T2)、`run_event_stdin` のコード、検出規則・所有境界・
`install-devin-hooks.ps1`。

## Done When

- [x] 上記 code 変更 6 項目と test 変更 7 項目が実装され、新規約
      (conflict で sync/verify/lifecycle が exit 0 収束・informational
      報告)と retained error/blocker(exit 2 維持)が test で固定される
- [x] finding・`warning`・`note`・`remediation` の文言に "duplicate firing
      is unsafe" が残っていない
- [x] 既存 `hook_config::` test 群を含む integration target に回帰が無い

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` が exit 0
- [x] `mise exec -- cargo fmt --check` が exit 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` が exit 0
- [x] `grep -n "duplicate firing is unsafe" crates/cheaper-cli/src/hook_config.rs` が 0 件
