---
id: "TASK-034IQdTig12S00T3Bnz5UQ"
type: "task"
status: "todo"
title: "Amend historical docs and cheaper-usage skill for orphan sweep boundary"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
  changes:
    added: []
    modified:
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
    - "docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  implemented-by: []
  depends-on:
    - "docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  defers: []
  deferred-by: []
---

# Amend historical docs and cheaper-usage skill for orphan sweep boundary

## Work

plan の T2: `.bak` sweep を現行挙動として記述する歴史的 doc 3 file・4 箇所を
spec R7 の新 contract(対象 `config.json.*.tmp` のみ・sweep は sync 開始時に
1 回実行 — 失敗時の別途 sweep なし)へ amend し、cheaper-usage SKILL.md の
verify 説明を 1 行 clarify する。各訂正文は design の verbatim before→after
をそのまま適用し、修正文の直後へ dated amend-note を付す(drift-note 併記では
なく文そのものの in-place 訂正 — spec R7 の明示方式)。

amend-note の定型文(design Documentation Design より、修正文直後の
blockquote):

```text
> Amend（2026-10-20 — SPEC-034IOh5OATJ4AcprA5u47p / issue #137）:
> orphan sweep の対象を `config.json.*.tmp`（cheaper 生成 temp）に
> 限定し、実行時点を「sync 開始時の 1 回（失敗時の別途 sweep
> なし）」へ訂正 — 旧記述の `.bak` sweep と「起動時 + 失敗時」
> timing は実装と齟齬していた。
```

対象文の特定は行番号ではなく before 引用文の verbatim 一致で行う(design が
±2 行程度の drift を許容する旨を明記 — 本 plan 作成時点の実測では spec
:275-278・design :152-154・:267-269・ADR :149-151)。

### spec(`docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`)

- [ ] R17 内(:275-278)— before: 「…`hook-config` の書き込みは temp file
      + atomic replace（`fs::rename` は dest 存在時に置換）で、capability
      確認との間の外部変更検出・`config.json.*.tmp`/`.bak` の orphan
      sweep を持つ。」→ after: 「…`config.json.*.tmp` のみの orphan
      sweep（sync 開始時に 1 回 — 失敗時の別途 sweep は行わず、rename
      失敗時の temp 除去は atomic write 自身が行う）を持つ。」
      + amend-note(requirement R17 の一部という規範的記述のため文そのもの
      を訂正)

### design(`docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`)— 2 箇所

- [ ] :152-154(`sync` bullet 内)— before: 「…temp+rename で atomic
      replace、`config.json.*.tmp`/`.bak` の orphan sweep を起動時と
      失敗時に行う。」→ after: 「…temp+rename で atomic replace、
      `config.json.*.tmp` のみの orphan sweep を sync 開始時に 1 回行う
      （失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は atomic
      write 自身が行う）。」+ amend-note
- [ ] :267-269(Non-Functional Constraints)— before: 「…`hook-config`
      の書き込みは temp+rename で atomic replace、外部変更を capability
      確認との間で検出して中断し、`config.json.*.tmp`/`.bak` の orphan
      を sweep する。」→ after: 「…`config.json.*.tmp` のみの orphan を
      sync 開始時に 1 回 sweep する（失敗時の別途 sweep は行わず、rename
      失敗時の temp 除去は atomic write 自身が行う）。」+ amend-note

### ADR(`docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`)

- [ ] :149-151 — before: 「…temp+rename の atomic replace（`fs::rename`
      は dest 存在時に置換）、capability 確認との間の外部変更検出、
      `config.json.*.tmp`/`.bak` の orphan sweep（起動時 + 失敗時）、
      無関係 entry の保持、…」→ after: 「…`config.json.*.tmp` のみの
      orphan sweep（sync 開始時に 1 回 — 失敗時の別途 sweep は行わず、
      rename 失敗時の temp 除去は atomic write 自身が行う）、無関係
      entry の保持、…」+ amend-note(旧 ADR の status・core 決定・
      位置付けは変更しない — ADR-034IOqXM5YpqVgFZKatIXN :359-371 の
      記述どおり `.bak` sweep 記述文の訂正のみ)

### skill(`packages/apm/.apm/skills/cheaper-usage/SKILL.md`)

配布 source のみを更新 — repo 内 `.agents/skills/cheaper-usage/SKILL.md` は
deploy 写しで対象外。

- [ ] :160-161 — verify の説明 comment「…orphan temp files」に、`.bak`
      orphan は verify が報告するが sync が自動除去しない旨を 1 行追記
      (explainability の wording 変更のみ — 挙動変更なし)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md
  (T2 節)
- docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md
  (Documentation Design — verbatim before→after と amend-note 定型文)
- docs/specs/hook-config-orphan-sweep-ownership-boundary.md (R7 — amend
  対象・方式・付記内容)

## Dependencies

- `docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md` (T1)
  — 本 task が amend するのは「現行挙動」を定める規範的記述であり、T1 の
  code+test が着地して新 contract が実装上の現行になるまで docs を先に
  書き換えると規範記述と実装が乖離する。drafting は design の確定 before→
  after をそのまま使うため T1 完了前でも可能だが、merge・最終照合は T1
  着地後に行う。

## Affected Files

- modify: `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
- modify: `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
- modify: `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
- modify: `packages/apm/.apm/skills/cheaper-usage/SKILL.md`

Out of scope(本 task では触らない): `hook_config.rs`・Rust integration
test(T1)、別 chain の時点記録
(`docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md:258-262`・
`docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md:268` —
historical change record であり amend 対象外、R7 が明示)、repo 内
`.agents/skills/cheaper-usage/SKILL.md`(deploy 写し)。

## Done When

- [ ] 4 箇所全てが「対象 `config.json.*.tmp` のみ・sync 開始時に 1 回
      (失敗時 sweep なし — rename 失敗時の temp 除去は atomic write
      自身)」へ in-place 訂正され、各箇所の直後に dated amend-note
      (SPEC-034IOh5OATJ4AcprA5u47p / issue #137 への参照付き)がある
- [ ] 旧記述(`.bak` sweep・「起動時 + 失敗時」timing)が対象 file に
      残っていない
- [ ] cheaper-usage SKILL.md の verify 説明に `.bak` orphan が sync で
      自動除去されない旨が追記されている

## Verification

- [ ] `grep -n "config.json.\*.tmp.\/.bak\|orphan sweep（起動時" docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md` が 0 件(旧 sweep 記述の残存なし)
- [ ] `grep -n "Amend（2026-10-20" docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md` が 4 件(各訂正箇所に dated note)
- [ ] `grep -n "not.*auto-removed\|never auto-removed\|自動除去" packages/apm/.apm/skills/cheaper-usage/SKILL.md` が 1 件以上
- [ ] `mise run check` が exit 0(canonical pipeline 全体)
