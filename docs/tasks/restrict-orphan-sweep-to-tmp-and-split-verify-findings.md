---
id: "TASK-034IQdP0etbaZJqUzaP6sl"
type: "task"
status: "done"
title: "Restrict orphan sweep to .tmp and split verify orphan findings"
created: "2026-10-19"
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
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
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
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-orphan-sweep-ownership-contract.md"
  references:
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
  defers: []
  deferred-by: []
---

# Restrict orphan sweep to .tmp and split verify orphan findings

## Work

plan の T1: `crates/cheaper-cli/src/hook_config.rs` の `sweep_orphans` を
`.tmp` のみの削除へ narrow し、`run_verify` の `orphan:` finding 文言を
`.tmp`/`.bak` で区分、`tests/integration/hook_config.rs` に orphan
contract test 6 本を追加する。文言は design の Wording Contract の具体案を
そのまま使う(spec R6 の semantic contract を外れない範囲でのみ調整可)。

### Code 変更(`crates/cheaper-cli/src/hook_config.rs`)

- [x] `sweep_orphans`(:482-503)の is_orphan predicate(:496-497)を
      `(fname.ends_with(".tmp") || fname.ends_with(".bak"))` から
      `fname.ends_with(".tmp")` へ narrow — `.bak` は cheaper が生成
      しない(所有境界外)旨の comment を併記。signature・戻り値
      `Vec<String>`・`removed` push 条件(`remove_file` 成功時のみ
      :498-499)・`read_dir` 失敗時 empty(:491-493)は不変
- [x] `sweep_orphans` doc comment(:480-481)を 2 箇所訂正 —
      (a) 対象記述を `{name}.` prefix + `.tmp` suffix(`config.json.*.tmp`
      相当)へ(現行の `config.json.<pid>.tmp`/`*.bak` 列挙は stale:
      glob は pid 成分を要求せず `.bak` は削除しない)、
      (b) 戻り値説明を「verify の finding 用」から「sync result の
      `orphansRemoved` field(:774-784)用 — verify が使う走査は
      `find_orphans`」へ
- [x] `run_verify`(:708-719)の `orphan:` finding を
      `o.ends_with(".tmp")` で区分 — `.tmp` 側:
      `orphan: {o} (leftover atomic-write temp; auto-removed by the next \`hook-config sync\`)`、
      else 側(`.bak`):
      `orphan: {o} (not managed by cheaper; remove manually if unwanted)`。
      `orphan:` prefix・findings の string array 形式・informational
      扱い(`clean` flag に影響しない)・`orphans` field populate
      (:716-719)は不変。"safe to delete" は `.bak` に対し廃止
- [x] 直上の inline comment(`// orphan temp/bak。` :707)を区分後
      contract に整合する内容へ更新(`.tmp` は自身の atomic write 残留で
      次回 sync が自動回収、`.bak` は cheaper 管理外で削除判断は user)
- [x] `find_orphans`(:506-525、glob :520)・`run_sync`(:763・:774-784)・
      `run_event_stdin`(:1013-1019)・`write_config_atomic`(:565-571)
      はコード変更なし — 新 contract は predicate 経由で自動継承。
      確認のみ

### Test 変更(`crates/cheaper-cli/tests/integration/hook_config.rs`)

`fixture_with_cheaper_on_path` PATH shim・`config_path(&fx)`・
`fx.run(&[...])`・`fx.stdin.data` 差替え(:334 pattern)を再利用。

- [x] helper `write_orphan_file(fx, name)` 相当を追加 — `.devin/` dir 作成 +
      任意名 file 書込み
- [x] 新規 `hook_config_sync_sweeps_tmp_orphan` —
      `config.json.12345.tmp` 配置 → `sync` → `code == 0`・file 削除済み・
      `result.orphansRemoved` に `.tmp` 名が載る
- [x] 新規 `hook_config_sync_preserves_bak_files` —
      `config.json.bak` + `config.json.manual.bak` 配置 → `sync` →
      `code == 0`・両 file 残存・`result.orphansRemoved` が `.bak` 名を
      一切含まない(orphan 無しなら key 不在 — 出現条件 :774-784)
- [x] 新規 `hook_config_sync_sweeps_only_tmp_among_mixed` —
      `config.json.<x>.tmp` + `config.json.bak` 混在 → `sync` → `.tmp`
      のみ削除・`.bak` 残存・`orphansRemoved` が `.tmp` 名のみ含む
- [x] 新規 `hook_config_verify_reports_bak_orphan_informational` —
      先に converged config を用意(`write_config_with_groups` で own
      entry 登録済み — config 不在だと `absent-config` blocker で
      `clean:false`・exit 2 になるため前提必須)→ `config.json.bak`
      配置 → `verify` → `result.orphans` + `orphan:` finding に file 名・
      file 非削除・finding が "safe to delete" を含まず
      "not managed by cheaper" を含む・`clean`/exit code は blocker
      有無のみで決まる(informational — `.bak` orphan 単独では
      `clean:true`・exit 0)
- [x] 新規 `hook_config_verify_tmp_orphan_wording_and_readonly` —
      `config.json.<x>.tmp` 配置 → `verify` → `orphan:` finding が sync
      自動回収を示す文言(`auto-removed` + `sync` を含む)・verify 実行後も
      `.tmp` file が残存(read-only 契約の pin — R5)
- [x] 新規 `hook_config_event_stdin_preserves_bak` — `config.json.bak`
      配置 → `{"event":"post-install","packages":["cheaper"]}` と
      `{"event":"post-update","packages":["cheaper"]}` を同 fixture で
      連続実行(`fx.stdin.data` 差替え)→ 両実行とも `code == 0`・`.bak`
      残存・`result.sync.orphansRemoved` に `.bak` 名を含まない(同一
      arm :1013-1019 の contract 継承 — R4)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md
  (T1 節)
- docs/test-specs/hook-config-orphan-sweep-ownership-contract.md (test
  contract — Rules/Examples と test case 対応)
- docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md
  (Detailed Design・Wording Contract・Test Design)
- docs/specs/hook-config-orphan-sweep-ownership-boundary.md (R1-R6)

## Dependencies

- 前提 task なし(plan の先頭 slice)。T2
  (`amend-orphan-sweep-ownership-docs-and-usage-skill.md`)は本 task の着地が
  前提 — docs amend が記述する規範記述は本 task の新 contract が現行挙動に
  なってからでないと実装と乖離する。

## Affected Files

- modify: `crates/cheaper-cli/src/hook_config.rs`
- modify: `crates/cheaper-cli/tests/integration/hook_config.rs`

Out of scope(本 task では触らない): 歴史的 doc 3 file・
`packages/apm/.apm/skills/cheaper-usage/SKILL.md`(T2)、`find_orphans`
glob・`run_sync`/`run_event_stdin`/`write_config_atomic` のコード、
`scripts/test/devin-hooks-install.test.mjs`(wrapper は thin 委譲 —
lifecycle 経路の `.bak` 保全は `--event-stdin` test が pin する)。

## Done When

- [x] is_orphan predicate が `.tmp` のみを削除対象とし、doc comment が
      `config.json.*.tmp` 相当・`orphansRemoved` field 用の 2 点へ訂正
      されている
- [x] `orphan:` finding が `.tmp`(sync 自動回収)/`.bak`(cheaper
      管理外・手動削除判断)で区分され、"safe to delete" が残っていない
- [x] 6 本の新規 test が上記 contract を pin し、既存 `hook_config::`
      test 群を含む integration target に回帰が無い

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` が exit 0
- [x] `mise exec -- cargo fmt --check` が exit 0
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` が exit 0
- [x] `grep -n "safe to delete" crates/cheaper-cli/src/hook_config.rs` が 0 件
