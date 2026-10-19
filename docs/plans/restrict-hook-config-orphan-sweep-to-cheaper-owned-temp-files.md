---
id: "PLAN-034IQ9W3TwoNQIp08Cn0SS"
type: "plan"
status: "approved"
title: "Restrict hook-config orphan sweep to cheaper-owned temp files"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
    - "https://github.com/scarecrow173/cheaper/issues/128"
    - "https://github.com/scarecrow173/cheaper/issues/107"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
      - type: "file"
        path: "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
      - type: "file"
        path: "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
  implemented-by:
    - "docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md"
    - "docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-orphan-sweep-ownership-contract.md"
  references:
    - "docs/discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "scripts/test/devin-hooks-install.test.mjs"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
  defers: []
  deferred-by: []
---

# Restrict hook-config orphan sweep to cheaper-owned temp files Implementation Plan

## Goal

`cheaper hook-config sync` の orphan sweep 削除対象を、cheaper 自身の atomic
write lifecycle が実際に生成しうる `config.json.*.tmp` のみへ限定し、
`.bak`(producer extinct のため全て foreign file)を削除 domain から完全に
外す(issue #137 — `config.json.bak` 等の user backup が同意なく sync・APM
lifecycle で削除される所有境界違反の解消)。verify の `find_orphans` は走査
条件を維持して `.bak` を informational orphan として報告し続けるが、
`orphan:` finding 文言を `.tmp`(atomic write 残留 — 次回 sync で自動回収)/
`.bak`(cheaper 管理外 — 削除判断は user)で区分し一律 "safe to delete" を
廃止する。併せて `.bak` sweep を現行挙動として記述する歴史的 doc 3 箇所を
R7 contract(対象 `config.json.*.tmp` のみ・sweep は sync 開始時に 1 回実行、
失敗時の別途 sweep なし)へ amend し、cheaper-usage SKILL.md の verify 説明を
1 行 clarify する。

## Scope

単一 implementation stream — `hook_config.rs` 内の is_orphan predicate
narrowing + doc comment 訂正 + finding 文言区分と、その contract test・docs
追従。critical path と release intent が同一(1 PR で ship する挙動変更)のため
分割しない。

In scope:

- `crates/cheaper-cli/src/hook_config.rs` — `sweep_orphans` の is_orphan
  predicate(:496-497)から `.bak` suffix 条件を除去し `{name}.` prefix +
  `.tmp` suffix のみにする変更、stale な doc comment(:480-481)の 2 箇所訂正
  (対象記述を `config.json.*.tmp` 相当へ・戻り値説明を `orphansRemoved`
  field 用へ)、`run_verify` の `orphan:` finding 文言(:711-713)を
  `.tmp`/`.bak` で区分、および直上 inline comment(`// orphan temp/bak。`
  :707)の更新。
- `crates/cheaper-cli/tests/integration/hook_config.rs` — design Test Design
  の matrix どおり 6 本の新規 test + sibling file 作成 helper(現状 orphan
  coverage はゼロ)。
- 歴史的 doc 3 箇所の amend(R7 — verbatim 一致で特定し、design の
  before→after 文をそのまま適用 + dated amend-note):
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  (requirement R17 内 :275-278)、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  (:152-154・:267-269 の 2 箇所)、
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  (:149-151)。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`(:160-161)— verify の
  説明 comment「orphan temp files」へ `.bak` orphan は verify が報告するが
  sync が自動除去しない旨の 1 行 clarify(挙動変更なし)。

Out of scope(spec / design と同じ境界):

- `.bak` 削除を制御する opt-in flag・設定・env var(Option C —
  `--event-stdin` に flag 伝達路が無く dead weight)。
- `.tmp` sweep 挙動自体の変更 — 呼出位置(`run_sync` upsert 前 :763)、
  「毎回実行」規約、`{name}.` prefix 条件は不変(R2)。
- `find_orphans`(:506-525、glob :520)の走査条件 narrowing — verify の
  `.bak` informational 報告は非破壊の可視性として維持(R5)。
- `remove` command の scope 変更、`orphansRemoved`/`orphans` field の
  rename・shape 変更、findings への severity field 等新構造(YAGNI・
  表現ドリフト回避)。
- `config.json.` prefix + `.tmp` suffix に一致する外部 file への追加防護
  (`config.json.<pid>.tmp` strict narrowing は deferred concern)。
- issue #128 系 legacy conflict gate・dedup 機構への変更(cross-chain —
  `related` 参照のみ)。
- 別 chain の時点記録
  (`docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md:258-262`・
  `docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md:268`)の
  `.bak` sweep 言及 — historical change record であり amend 対象外。
- `write_config_atomic` の temp name comment(:565-571)— `.bak` 言及が無く
  narrowing 後も正確なまま残るため訂正不要(design で確認済み)。

## Upstream Documents

- `docs/specs/hook-config-orphan-sweep-ownership-boundary.md`
  (SPEC-034IOh5OATJ4AcprA5u47p, approved) — R1-R7 要件・Acceptance
  Criteria。拘束力のある contract。
- `docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (ADR-034IOqXM5YpqVgFZKatIXN, accepted) — Option 1「削除対象を
  `config.json.*.tmp` のみに限定」の構造決定(Option 2-4 却下の根拠を
  含む)。

## Design Inputs

- `docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (DESIGN-034IPoKswcH3LaUZdnb0XT, approved) — touch point 単位の実装
  青写真・Wording Contract(finding 文言の semantic contract と具体案)・
  R7 amend の verbatim before→after・Test Design matrix。本 plan は
  design を implementation order へ転記したもので独自判断を足さない。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-cli/src/hook_config.rs` | modify | `sweep_orphans` predicate narrowing(:496-497)+ doc comment 訂正(:480-481)、`run_verify` の `orphan:` finding 文言区分(:708-719)+ inline comment 更新(:707) | signature・戻り値 `Vec<String>`・呼出位置・`find_orphans`・`run_sync`・`run_event_stdin`・`write_config_atomic` は不変 |
| `crates/cheaper-cli/tests/integration/hook_config.rs` | modify | sibling orphan file 作成 helper + 6 本の新規 contract test | `fixture_with_cheaper_on_path` PATH shim・`config_path(&fx)`・`fx.run`・`fx.stdin.data` 差替え pattern(:334)を再利用 |
| `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md` | modify | R17 内の `.bak` sweep 記述(:275-278)を新 contract へ in-place 訂正 + dated amend-note | verbatim 一致で特定(design の before 文) |
| `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md` | modify | 同じく 2 箇所(:152-154・:267-269)を訂正 + 各箇所へ amend-note | 同上 |
| `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md` | modify | :149-151 の `.bak` sweep + 「起動時 + 失敗時」記述を訂正 + amend-note | 同上 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | modify | verify 説明(:160-161)に `.bak` orphan は報告のみ・自動除去されない旨を追記 | 配布 source のみ — repo 内 `.agents/` 写しは対象外 |

## Implementation Sequence

task slice は T1 → T2 の直列(T2 の doc amend が記述するのは T1 で着地する
新 contract — 文言は design で確定済みだが、歴史的 spec/ADR/design を
「現行挙動」の訂正として先に書き換えると、code 未反映の間に規範記述と実装が
乖離する窓ができるため、code+test 着地後に docs を追従させる)。

### T1 — `hook_config.rs` narrowing + finding 区分 + Rust integration tests

- [ ] `sweep_orphans`(:482-503)の is_orphan predicate(:496-497)から
      `.bak` suffix 条件を除去し `fname.starts_with(&tmp_prefix) &&
      fname.ends_with(".tmp")` へ narrow する — 所有境界 comment を併記
      (signature・`removed` push 条件 :498-499・`read_dir` 失敗時 empty
      :491-493 は不変)
- [ ] `sweep_orphans` doc comment(:480-481)を訂正 — 対象記述を
      `{name}.` prefix + `.tmp` suffix(`config.json.*.tmp` 相当)へ、
      戻り値説明を「sync result の `orphansRemoved` field 用(verify が
      使う走査は `find_orphans`)」へ
- [ ] `run_verify`(:708-719)の `orphan:` finding を `ends_with(".tmp")`
      で区分 — `.tmp` は sync 自動回収を示す文言(design 案:
      `orphan: {o} (leftover atomic-write temp; auto-removed by the next
      `hook-config sync`)`)、`.bak` は cheaper 管理外 + 手動削除判断の
      文言(design 案: `orphan: {o} (not managed by cheaper; remove
      manually if unwanted)`)。"safe to delete" は廃止。`orphan:`
      prefix・findings の string array 形式・informational 扱い・
      `orphans` field populate(:716-719)は不変。文言細部は spec R6 の
      semantic contract を外れない範囲で調整可
- [ ] 直上の inline comment(`// orphan temp/bak。` :707)を区分後
      contract に整合する内容へ更新
- [ ] `find_orphans`(:506-525)・`run_sync`(:763・:774-784)・
      `run_event_stdin`(:1013-1019)・`write_config_atomic`(:565-571)
      はコード変更なし — 新 contract は predicate 経由で継承されることを
      確認のみ
- [ ] `tests/integration/hook_config.rs` に sibling file 作成 helper
      (`write_orphan_file(fx, name)` 相当 — `.devin/` dir 作成 + 任意名
      file 書込み)を追加
- [ ] 新規 `hook_config_sync_sweeps_tmp_orphan` —
      `config.json.12345.tmp` を配置 → `sync` → file 削除・
      `result.orphansRemoved` に `.tmp` 名・exit 0
- [ ] 新規 `hook_config_sync_preserves_bak_files` —
      `config.json.bak` + `config.json.manual.bak` を配置 → `sync` →
      両 file 残存・`orphansRemoved` が `.bak` 名を一切含まない
      (orphan 無しなら key 不在 — 出現条件 :774-784)
- [ ] 新規 `hook_config_sync_sweeps_only_tmp_among_mixed` —
      `config.json.<x>.tmp` + `config.json.bak` 混在 → `sync` → `.tmp`
      のみ削除・`.bak` 残存・`orphansRemoved` が `.tmp` 名のみ
- [ ] 新規 `hook_config_verify_reports_bak_orphan_informational` —
      converged config(own entry 登録済み)前提で `config.json.bak` →
      `verify` → `result.orphans` + `orphan:` finding に file 名・
      file 非削除・finding が "safe to delete" を含まず
      "not managed by cheaper" を含む・`clean`/exit code は blocker
      有無のみで決まる(informational のまま — config 不在では
      `absent-config` blocker が `clean:false` にするため前提必須)
- [ ] 新規 `hook_config_verify_tmp_orphan_wording_and_readonly` —
      `config.json.<x>.tmp` → `verify` → `orphan:` finding が sync
      自動回収を示す文言(`auto-removed` + `sync` 等)・verify 実行後も
      `.tmp` file が残存(read-only 契約の pin — R5)
- [ ] 新規 `hook_config_event_stdin_preserves_bak` — `config.json.bak`
      配置 → `{"event":"post-install","packages":["cheaper"]}` と
      `{"event":"post-update","packages":["cheaper"]}` を同 fixture で
      連続実行(`fx.stdin.data` 差替え — :334 pattern)→ 両実行とも
      `.bak` 残存・`result.sync.orphansRemoved` に `.bak` 名を含まない
      (同一 arm :1013-1019 の contract 継承を pin)
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      hook_config::` が exit 0、`cargo fmt --check`・clippy が pass

### T2 — 歴史的 doc amend + SKILL.md clarify(depends T1)

- [ ] `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
      — R17 内(:275-278、verbatim 一致で特定)の
      「`config.json.*.tmp`/`.bak` の orphan sweep を持つ」を design の
      after 文(「`config.json.*.tmp` のみの orphan sweep(sync 開始時に
      1 回 — 失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は
      atomic write 自身が行う)を持つ」)へ in-place 訂正 + amend-note
- [ ] `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
      — :152-154(`sync` bullet 内)と :267-269(Non-Functional
      Constraints)の 2 箇所をそれぞれ design の after 文へ訂正 + 各箇所へ
      amend-note
- [ ] `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
      — :149-151 の「`config.json.*.tmp`/`.bak` の orphan sweep(起動時 +
      失敗時)」を design の after 文へ訂正 + amend-note
- [ ] 各 amend-note は修正文直後の blockquote で
      `> Amend（2026-10-20 — SPEC-034IOh5OATJ4AcprA5u47p / issue #137）: …`
      形式とし、「`.bak` sweep と『起動時 + 失敗時』timing は実装と
      齟齬していた」旨を記す(design の定型文)
- [ ] `packages/apm/.apm/skills/cheaper-usage/SKILL.md`(:160-161)—
      verify 説明 comment の "orphan temp files" に `.bak` orphan は
      verify が報告するが sync が自動除去しない旨を 1 行追記
- [ ] `mise run check` が exit 0(biome・`cargo fmt --check`・clippy・
      version/doc-id・`cargo test --workspace`・`node --test`・
      lockfile/payload check — canonical pipeline 全体)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 hook_config.rs narrowing + finding 区分 + Rust tests | — | T2 |
| T2 docs amend + SKILL.md clarify | T1 | — |

critical path は T1 → T2 の直列で並行 stream は無い — file 集合は
disjoint(Rust code+test vs docs 4 file)だが、T2 が amend する規範記述の
対象は T1 で着地する挙動であり、先に docs を書き換えると「現行挙動」の記述と
実装が乖離するため `depends-on` で直列化する。drafting は design の確定文言を
そのまま使うため T1 完了前でも可能だが、merge・verification は T1 着地後。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | `.bak` orphan の verify 報告が informational のまま exit code に現れず、sync との報告乖離(verify は `.bak` を報告し続けるが sync は除去しない)が user を混乱させる | med | low | 意図的かつ永続する trade-off(ADR Consequences)。新 finding 文言「not managed by cheaper; remove manually if unwanted」が乖離の理由そのものを説明する役割を担い、SKILL.md clarify で explainer を補強 |
| 2 | 非 pid 命名の外部 `.tmp`( `config.json.tmp`・`config.json.backup.tmp` 等)は post-fix でも sweep されうる残存リスク | low | low | spec/design が受容を決めた residual — glob は prefix + `.tmp` suffix のみ要求し pid 成分は条件外。strict `<pid>.tmp` narrowing は deferred concern として記録済みで本変更に含めない |
| 3 | finding 文言変更が findings を機械消費する consumer に見える差分になる | low | low | findings は人間向け message で shape は不変(`orphan:` prefix・string array)。severity 構造化は YAGNI(deferred) |
| 4 | `remove_file` 失敗・`read_dir` 失敗時の挙動回帰 | low | low | 現行構造を一切変更しない(`is_ok()` gate :498-499・empty 返却 :491-493)— narrowing は suffix check 1 項の除去のみで cost 減方向 |
| 5 | amend 対象行の line drift — design の行番号 citation が実 file とずれる | med | low | 設計が明示する対処どおり verbatim 一致で特定する。本 plan 作成時点で全 4 箇所の before 文を verbatim 確認済み(:275-278・:152-154・:267-269・:149-151 — design 記載より ±数行 drift している) |
| 6 | 外部依存 availability — 新規 crate dep・env var・flag 無し。外部状態は fixture tempdir のみ | low | low | Rust test は `CARGO_BIN_EXE` PATH shim で隔離。`--event-stdin` test も同一 in-process 経路 |

### Rollback

- **Trigger**: narrowing 後の sync/verify/lifecycle 収束に障害(`.tmp`
  orphan が削除されない・`.bak` が依然削除される・finding が出ない・
  event-stdin 経路で異なる挙動になる等)。
- **Procedure**: revert unit は T1/T2 の commit — `git revert` で
  `hook_config.rs` の predicate・文言・comment と test・docs amend を戻す。
  schema 変更・migration・共有 state は無く、削除「しない」方向の変更のため
  revert で完全に戻せる不可逆操作は存在しない。
- **Verification**: revert 後に `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration hook_config::` が pass すること。

## Task Breakdown

task doc へ分割する候補(2 task — file 集合が disjoint で review 単位を
分けるが、実行は T1 → T2 直列):

- T1: `sweep_orphans` narrowing + doc comment 訂正 + `run_verify` finding
  区分 + inline comment 更新 + Rust integration test 6 本
  — `docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md`
- T2: 歴史的 doc 3 file・4 箇所の R7 amend + cheaper-usage SKILL.md
  clarify
  — `docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| AC: `config.json.<pid>.tmp` 相当 orphan が sync で削除され `orphansRemoved` に載る(R1-R3) | `hook_config_sync_sweeps_tmp_orphan` | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` |
| AC: `config.json.bak`・`config.json.manual.bak` が sync で削除されず `orphansRemoved` に `.bak` 名を含まない(R1/R3) | `hook_config_sync_preserves_bak_files` | 同上 |
| AC: `.tmp`+`.bak` 混在で `.tmp` のみ削除・`orphansRemoved` が `.tmp` 名のみ(R1-R3) | `hook_config_sync_sweeps_only_tmp_among_mixed` | 同上 |
| AC: `.bak` が verify の `orphans` field + `orphan:` finding に報告・file 非削除(R5) | `hook_config_verify_reports_bak_orphan_informational` | 同上 |
| AC: `.bak` finding が "safe to delete" を含まず cheaper 管理外を伝え、`.tmp` finding が sync 自動回収を示す(R6) | 同上 + `hook_config_verify_tmp_orphan_wording_and_readonly`(read-only pin 兼) | 同上 |
| AC: `--event-stdin` `post-install`/`post-update` 両経路で `.bak` 非削除(R4) | `hook_config_event_stdin_preserves_bak` | 同上 |
| AC: R7 — 歴史的 doc 3 箇所が「`config.json.*.tmp` のみ・sync 開始時に 1 回(失敗時 sweep なし)」へ amend され dated 参照を伴う | review check — 各 file で旧文(`.bak` sweep・「起動時 + 失敗時」)が残らず amend-note が付いていること | `grep -n "bak.*orphan sweep\|orphan sweep" docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md` |
| 全体 | biome・`cargo fmt --check`・clippy・version/doc-id・`cargo test --workspace`・`node --test`・lockfile/payload check | `mise run check` |

## Open Questions / Gaps

- design が文言・predicate・test 構成・amend before→after を確定済みのため、
  実装上の不明点は無い。finding 文言の細部(言い回し)のみ R6 semantic
  contract を外れない範囲で実装時に調整可。
- 記録済みの意図的変更(implementation 時に驚かないための明示):
  - `.bak` file は `find_orphans` の報告対象に残るが sweep 対象からは
    完全に外れる — verify ↔ sync の報告乖離は永続する設計(ADR
    Consequences で受容済み)。
  - `config.json.bak.tmp` のような複合 suffix は `.tmp` で終わるため
    sweep 対象 — 命名上 temp 形なので新 contract では削除が妥当。
  - `.bak` の大文字化(`config.json.BAK`)は `ends_with` が case
    sensitive で従来どおり無視(変化なし)。
  - `config.json.12345.tmp` 等の非 pid `.tmp` 名は sweep 対象のまま —
    strict narrowing は deferred concern。
- `scripts/test/devin-hooks-install.test.mjs` への追加 assertion は不要と
  判断 — wrapper 経路は `hook-config` への thin 委譲であり lifecycle 経路の
  `.bak` 保全は `--event-stdin` test(T1)が同一 arm(:1013-1019)を pin
  する。mjs 側に orphan 系 assertion は現状存在しない(design Test Design
  の対象外)。

## Self-Review

- [x] Every spec requirement maps to a task.(R1-R4・R7・全 AC →
      Verification matrix と T1/T2 checklist を対応付け済み。R5-R6 は
      verify 側 test が固定)
- [x] No placeholders remain.(文言は design Wording Contract の確定
      文言・before→after を転記)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation — T1 は
  code+test の self-contained slice、T2 は docs の追従 slice で、各 slice
  が review checkpoint として独立する。
- Independent work streams: なし(T1 → T2 直列。file 集合は disjoint だが
  T2 の amend が記述する規範記述は T1 の着地が前提)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in
  the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and
  verification matrix.

## Review Handoff

- 前提: upstream 3 doc(spec approved・design approved・ADR accepted)が
  predicate・文言・test matrix・amend before→after まで確定済み。plan は
  design を implementation order へ転記したもので独自判断を足していない。
- 重点確認: (a) `sweep_orphans` の narrowing が suffix 条件の除去のみで
  `find_orphans` 側 glob(:520)を締めていないこと(R5 維持)、(b)
  `orphan:` finding 区分で `.bak` に "safe to delete" が残っていないこと
  (R6)、(c) amend-note が各訂正箇所の直後に dated 形式で付いていること
  (R7 — provenance を消さない)、(d) `--event-stdin` test が
  `post-install` AND `post-update` の両方を同一 fixture で pin していること
  (R4 — 同一 arm の contract 継承)。
