---
id: "IMPL-034IQI6Q5wQd1Ql4s3At3r"
type: "impl"
status: "completed"
title: "Amend orphan sweep ownership docs and usage skill"
created: "2026-10-20"
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
      - type: "file"
        path: "docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md"
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
  derives-from:
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "docs/impl/ir/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Amend orphan sweep ownership docs and usage skill

## Summary

plan の T2: `.bak` sweep を現行挙動として記述していた規範的 doc
3 file・4 箇所を、spec R7 の新 contract（削除対象は cheaper 生成の
`config.json.*.tmp` のみ・sweep は sync 開始時に 1 回実行 — 失敗時の
別途 sweep はなく、rename 失敗時の temp 除去は `write_config_atomic`
自身が担う）へ in-place 訂正し、各訂正箇所の直後に dated amend-note
（SPEC-034IOh5OATJ4AcprA5u47p / issue #137）を付した。併せて
`packages/apm/.apm/skills/cheaper-usage/SKILL.md` の verify 説明
comment に「`.bak` orphan は報告されるが sync が自動除去しない」旨を
1 行追記した（explainability wording のみ — 挙動変更なし）。
全ての訂正文・amend-note は design Documentation Design の確定
before→after・定型文を verbatim で適用（独自言い換え無し）。

## Context

- task: `docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md`
  (TASK-034IQdTig12S00T3Bnz5UQ)
- plan: `docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (T2 — docs amend + SKILL.md clarify slice)
- spec: `docs/specs/hook-config-orphan-sweep-ownership-boundary.md`
  (SPEC-034IOh5OATJ4AcprA5u47p — R7 が amend 対象・in-place 訂正方式・
  付記内容を規定)
- design: `docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`
  (DESIGN-034IPoKswcH3LaUZdnb0XT — Documentation Design の verbatim
  before→after と amend-note 定型文)
- 前提: T1 (`docs/tasks/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md`、
  IR `docs/impl/ir/restrict-orphan-sweep-to-tmp-and-split-verify-findings.md`)
  の code+test が着地済み — amend が記述する規範記述は T1 の新 contract
  が現行挙動であることが前提（plan の直列化どおり T1 着地後に実施）。

## Implementation

### Changes

- `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  （requirement R17 内 — 修正後 :274-287）
  - 「`config.json.*.tmp`/`.bak` の orphan sweep を持つ」を
    「`config.json.*.tmp` のみの orphan sweep（sync 開始時に 1 回 —
    失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は atomic
    write 自身が行う）を持つ」へ文そのものを訂正し、直後に amend-note
    blockquote（R17 list item 内の 2-space indent `>`）を付した。
    front matter `updated` を 2026-10-20 へ。
- `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  — 2 箇所を訂正（各箇所へ amend-note、`updated` 同様に bump）:
  - `sync` bullet 内（修正後 :153-162）— 「`config.json.*.tmp`/`.bak`
    の orphan sweep を起動時と失敗時に行う」を「`config.json.*.tmp`
    のみの orphan sweep を sync 開始時に 1 回行う（失敗時の別途
    sweep は行わず、rename 失敗時の temp 除去は atomic write 自身が
    行う）」へ訂正 + list item 内 amend-note。
  - Non-Functional Constraints bullet（修正後 :275-285）—
    「`config.json.*.tmp`/`.bak` の orphan を sweep する」を
    「`config.json.*.tmp` のみの orphan を sync 開始時に 1 回 sweep
    する（失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は
    atomic write 自身が行う）」へ訂正 + list item 内 amend-note。
- `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  （修正後 :151-153・note :163-167、`updated` 同様に bump）
  - 「`config.json.*.tmp`/`.bak` の orphan sweep（起動時 + 失敗時）」を
    「`config.json.*.tmp` のみの orphan sweep（sync 開始時に 1 回 —
    失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は atomic
    write 自身が行う）」へ訂正。修正句が paragraph 途中のため
    amend-note は段落の直後へ column-0 blockquote で付した。
    旧 ADR の status・core 決定・位置付けは不変（ADR-034IOqXM5YpqVgFZKatIXN
    :359-371 の記述どおり `.bak` sweep 記述文の訂正のみ）。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（:160-162）
  - `hook-config verify` の説明 comment「…orphan temp files」の次行へ
    `# (.bak orphans are reported but never auto-removed)` を 1 行追記。
    配布 source のみ — repo 内 `.agents/skills/cheaper-usage/SKILL.md`
    （deploy 写し）は対象外として未変更。
- `docs/tasks/amend-orphan-sweep-ownership-docs-and-usage-skill.md`
  — status `done`、checklist 全項目 checked。
- `docs/impl/ir/README.md` — 本 IR の index row（`new_impl_record.js`
  が作成時に自動追加）。

### Decisions During Implementation

- amend-note の配置: 修正文が bullet 内にある 3 箇所（spec R17・design
  の `sync` bullet・Non-Functional Constraints bullet）は `>` を list
  item の content indent（2 space）に揃えて item 内 blockquote とした —
  column-0 の blockquote を挟むと `- sync:` と `- verify:` の間で list
  が分断されるため（修正文への結合を保つ「直後」配置）。ADR は修正句が
  paragraph 途中にあり文内へ blockquote を挿入できないため、paragraph
  直後へ置いた。
- 対象文の特定は design の before 引用文の verbatim 一致で行った
  （行番号 citation は ±2 行程度 drift しうる旨の design 明示どおり。
  実測では spec :277-279・design :153-155/:268-270・ADR :150-152 に
  あり全て verbatim 一致）。
- 文言は design の after 文・amend-note 定型文を verbatim で使用。
  3 doc の `updated` bump は amend 付記の repo 先例
  （`docs/specs/unified-version-source-of-truth.md` の superseded note
  運用）に倣った最小の metadata 更新で、他の front matter・status・
  他 content には触れていない。

## Related Experiments

なし（既知の実装 task のため Experiment Log は作成していない）。

## Validation

- `grep -n "config.json.\*.tmp.\/.bak\|orphan sweep（起動時" <3 files>`
  — 0 件（旧 sweep 記述の残存なし）。
- `grep -n "Amend（2026-10-20" <3 files>` — 4 件（spec 1・design 2・
  ADR 1、各訂正箇所の直後に dated note）。
- `grep -n "not.*auto-removed\|never auto-removed\|自動除去"
  packages/apm/.apm/skills/cheaper-usage/SKILL.md` — 1 件。
- 3 file の残存 `.bak` 言及は front matter の discovery doc reference
  と amend-note 内の旧記述引用（「旧記述の `.bak` sweep と『起動時 +
  失敗時』timing は実装と齟齬していた」）のみ — 規範記述としての
  stale claim は残っていない。
- `node .agents/skills/impl-doc/scripts/audit_impl_record.js --json`
  — 本 IR を含め findings 0 件。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type
  spec|design|adr --json` — 対象 3 file の finding は amend 前
  baseline と同一（spec 0 件・design 0 件・ADR は pre-existing の
  `inconsistent-reciprocal-relation` warning 1 件 — 新規
  error/blocking finding なし）。
- `mise run check` — exit 0（canonical pipeline 全体）。

## Risks

- amend-note が `.bak`・「起動時 + 失敗時」の文字列自体を含むため、
  旧記述の残存確認 grep は note の meta 引用を弾く必要がある — task
  の verification pattern（`config.json.*.tmp`/`.bak` pairing と
  `orphan sweep（起動時`）は note に一致しない設計になっている。
- verify ↔ sync の報告乖離（verify は `.bak` orphan を報告し続けるが
  sync は除去しない）は意図的かつ永続する trade-off — SKILL.md の追記
  と T1 の finding 文言区分が乖離の理由を説明する役割を担う。

## Follow-ups

なし — plan の両 task（T1・T2）がこれで完了し、issue #137 の
code+docs contract は spec R1-R7 どおり着地した。
