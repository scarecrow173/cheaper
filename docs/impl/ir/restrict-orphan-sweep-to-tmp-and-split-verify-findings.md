---
id: "IMPL-o1iX554igoTSDCyr03s9Bh"
type: "impl"
status: "completed"
title: "Restrict orphan sweep to .tmp and split verify orphan findings"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
    - "https://github.com/scarecrow173/cheaper/issues/128"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md"
  implemented-by: []
  depends-on:
    - "docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  blocks:
    - "docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md"
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-orphan-sweep-ownership-contract.md"
  references:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Restrict orphan sweep to .tmp and split verify orphan findings

## Summary

plan の T1: `crates/cheaper-cli/src/hook_config.rs` の `sweep_orphans`
の削除 predicate を `{name}.` prefix + `.tmp` suffix のみへ narrow
し(issue #137 — `.bak` は cheaper が一切生成しない foreign file で
あり、sync・`--event-stdin` lifecycle の自動収束が user の手動
backup を同意なく削除していた)、`run_verify` の `orphan:` finding
文言を `.tmp`(atomic write 残留 — 次回 sync が自動回収)と
`.bak`(cheaper 管理外 — 削除判断は user)で区分した。
`tests/integration/hook_config.rs` に orphan contract test 6 本と
helper `write_orphan_file` を追加した。文言は design の Wording
Contract の確定文字列をそのまま使用(独自言い換え無し)。

## Context

- task: `docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md`
  (TASK-034IQdP0etbaZJqUzaP6sl)
- plan: `docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (T1 — code + Rust integration test の先頭 slice)
- spec: `docs/specs/hook-config-orphan-sweep-ownership-boundary.md`
  (SPEC-034IOh5OATJ4AcprA5u47p — R1-R6)
- design: `docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (DESIGN-034IPoKswcH3LaUZdnb0XT — Detailed Design・Wording Contract・
  Test Design)
- test-spec: `docs/test-specs/hook-config-orphan-sweep-ownership-contract.md`
  (TSPEC-034IQa6tpizWQ1s8A9Dvh0 — test contract)

## Implementation

### Changes

- `crates/cheaper-cli/src/hook_config.rs`
  - `sweep_orphans` — is_orphan predicate を
    `fname.ends_with(".tmp") || fname.ends_with(".bak")` から
    `fname.ends_with(".tmp")` のみへ narrow し、「`.bak` は cheaper が
    生成しない(所有境界外)」旨の inline comment を併記。signature・
    戻り値 `Vec<String>`・`remove_file` 成功時のみ `removed` へ
    push する条件・`read_dir` 失敗時の empty 返却は不変。
  - `sweep_orphans` doc comment を 2 箇所訂正 — (a) 対象記述を
    `{name}.` prefix + `.tmp` suffix(`config.json.*.tmp` 相当)へ
    (旧記述の `config.json.<pid>.tmp`/`*.bak` 列挙は stale: glob は
    pid 成分を要求せず `.bak` は削除しない)、(b) 戻り値説明を
    「verify の finding 用」から「sync result の `orphansRemoved`
    field 用(verify が使う走査は `find_orphans`)」へ。
  - `run_verify` の `orphan:` finding を `o.ends_with(".tmp")` で
    区分 — `.tmp` 側は
    `orphan: {o} (leftover atomic-write temp; auto-removed by the next \`hook-config sync\`)`、
    else 側(`.bak`)は
    `orphan: {o} (not managed by cheaper; remove manually if unwanted)`。
    `orphan:` prefix・findings の string array 形式・informational
    扱い(`clean` flag に影響しない)・`orphans` field populate は
    不変。`.bak` 向けの "safe to delete" 表現は廃止。直上の inline
    comment(`// orphan temp/bak。`)を区分後 contract に整合する
    内容へ更新。
  - `find_orphans`(`.tmp`/`.bak` glob)・`run_sync`(sweep 呼出位置・
    `orphansRemoved` 組立)・`run_event_stdin`・`write_config_atomic`
    はコード変更なし — 新 contract は predicate 経由で自動継承。
- `crates/cheaper-cli/tests/integration/hook_config.rs`
  - helper `write_orphan_file(fx, name)` を追加 — `.devin/` dir 作成 +
    任意名 file 書込みで sibling orphan 候補を配置する。
  - 新規 test 6 件:
    `hook_config_sync_sweeps_tmp_orphan`(`config.json.12345.tmp` →
    sync → 削除済み・`orphansRemoved` に `.tmp` 名・exit 0)、
    `hook_config_sync_preserves_bak_files`(`config.json.bak` +
    `config.json.manual.bak` → sync → 両 file 残存・`orphansRemoved`
    が `.bak` 名を一切含まない — 削除 0 件時の key 不在を許容)、
    `hook_config_sync_sweeps_only_tmp_among_mixed`(`.tmp` + `.bak`
    混在 → `.tmp` のみ削除・`orphansRemoved` が `.tmp` 名のみ)、
    `hook_config_verify_reports_bak_orphan_informational`(converged
    config 前提で `config.json.bak` → verify → `orphans` +
    `orphan:` finding に file 名・file 非削除・finding が
    "not managed by cheaper" を含み "safe to delete" を含まない・
    `.bak` 単独で `clean:true`・exit 0)、
    `hook_config_verify_tmp_orphan_wording_and_readonly`(
    `config.json.<x>.tmp` → verify → finding が `auto-removed` +
    `sync` を含む・verify 後も `.tmp` 残存の read-only pin)、
    `hook_config_event_stdin_preserves_bak`(`config.json.bak` →
    post-install / post-update を `fx.stdin.data` 差替えで連続実行 →
    両実行 exit 0・`.bak` 残存・`result.sync.orphansRemoved` に
    `.bak` 名を含まない)。
- `docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md`
  — status `done`、checklist 全項目 checked。

### Decisions During Implementation

- wording は design Wording Contract の確定文字列を verbatim で使用
  し、既存 finding と同じく Rust の `\` 行継続で 1 行文字列として
  組立てた(独自の言い換え無し)。
- `.bak` informational verify test は converged config
  (`write_config_with_groups(&fx, &[own_entry(CANONICAL_MATCHER)])`)
  を前提化 — config 不在だと `absent-config` blocker で
  `clean:false`・exit 2 となり、`.bak` orphan 単独では `clean:true`・
  exit 0 を保つこと(task/test-spec の必須条件)を pin できないため。
- `.tmp` wording test にも同じ converged config 前提を敷いた —
  absent-config blocker 下でも finding 検証は可能だが、clean な
  verify 経路の read-only pin として deterministic にするため。

## Related Experiments

なし(既知の実装 task のため Experiment Log は作成していない)。

## Validation

- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  hook_config` — 21 passed / 0 failed(新規 6 件を含む、exit 0)。
- `mise exec -- cargo clippy -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --all-targets --
  -D warnings` — exit 0(警告無し)。
- `mise exec -- cargo fmt --all -- --check` — clean。
- `grep -n "safe to delete" crates/cheaper-cli/src/hook_config.rs`
  — 0 件。

## Risks

- 外部 tool 由来の `config.json.*.bak` は cheaper の掃除責務から
  外れた — verify の informational `orphan:` finding で可視性は
  維持されるが、exit code には現れない(ADR で受容済みの
  trade-off)。
- glob は pid 成分を要求しないため、`config.json.tmp` 等の非 pid
  命名の外部 `.tmp` file は post-fix でも sweep されうる — spec
  Deferred Design Concerns に記録済みの残存リスク(strict
  `config.json.<pid>.tmp` narrowing は deferred)。
- verify ↔ sync の報告乖離は意図的に永続する: verify は `.bak`
  orphan を報告し続けるが sync は一切除去しない — 新 finding 文言
  が乖離の理由そのものを説明する役割を担う。

## Follow-ups

- T2 `docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md`
  — 歴史的 doc 3 箇所(spec R7 amend: bounded spec R17・対応 design・
  旧 ADR)と `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の
  規範記述を、本 task で確定した新 contract(対象
  `config.json.*.tmp` のみ・sync 開始時に 1 回実行・`.bak` は
  verify が informational 報告)へ整合させる。
