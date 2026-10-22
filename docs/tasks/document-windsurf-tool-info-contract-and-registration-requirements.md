---
id: "TASK-034KxDvREdpF6P10oziJsy"
type: "task"
status: "done"
title: "Document windsurf tool_info contract and dual-registration trap in distribution guide"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/161"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/distribution.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
    - "docs/tasks/add-windsurf-range-deny-guidance.md"
    - "docs/tasks/add-hook-config-compat-degraded-findings.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md"
  references:
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/adr/defensive-compat-normalization-with-registration-surfacing.md"
    - "docs/guides/distribution.md"
  defers: []
  deferred-by: []
---

# Document windsurf tool_info contract and dual-registration trap in distribution guide

## Work

plan `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
(PLAN-034KxCQ2FWFt55qhBNp6sR)の T5。`docs/guides/distribution.md`
の Devin 登録節(現行 :110-207 付近、`hook-config sync`・lifecycle
sync・`read_config_from`・verify の説明がある区画)に、windsurf
`tool_info` contract と dual-registration trap の説明を追記する。
spec(SPEC-034KwuiFzl4WvVQKdjfSnM)R7 と design の Verify 経路の
文言(DESIGN-034KwO2y86CrXOmaoae1jS)をそのまま運び、文言の独自
拡大はしない。

追記範囲は 2 点:

### 1. `tool_info` contract の説明

compat payload の `tool_info` として受理する range/search 語彙:

- **明示 aliases**: `offset`/`line_offset`(開始行)・
  `limit`/`line_limit`/`max_lines`(行数)。1-based・limit は行数。
- **pair 形式**: `start_line`+`end_line`(1-based・inclusive、
  `end >= start` が必要で `limit = end-start+1`)。
- **array 形式**: `view_range:[start,end]`(1-based・inclusive、
  `[start,-1]` は「start 行〜EOF」sentinel)。
- **`end_line` 単独**: 1〜`end_line` 行。
- **`start_line` 単独**: `start_line` 行〜EOF。
- **search 語彙**: `pattern`(非空)+ `file_path` が欠落/空/非
  string/dir 解決の場合のみ Search 正規化。file 解決の `file_path`
  併記・空 `pattern` は FileRead として評価される。

受理優先順位は「明示 offset/limit aliases → `start_line`+
`end_line`/`view_range`/`end_line` 単独 → fallback」で deterministic
解決(有効な `ReadRange` までは shared contract として parity)。
無効・矛盾・非整数値は範囲を狭めず、full-file 評価またはより広い
window へ縮退する(fail-closed)。

### 2. dual-registration trap + registration の説明

- **native registration が前提であること**: Devin 利用時は
  `cheaper hook-config sync`(または既存 lifecycle sync)で native
  Devin registration を行う前提を明示する。
- **dual-registration trap**: Devin が `.windsurf/hooks.json`
  legacy registration も import して発火し続ける場合、compat firing
  と native firing は dedup されない — verdict fingerprint は
  canonical request(`range` を含む)を含み、range を落とす compat
  firing は別 fingerprint のため重複発火となる。canonical request
  も異なれば dedup の対象でなく、deny が verdict union で勝つため
  bounded native firing が通っても全文 compat firing が残れば deny
  が立つ。
- **remediation**: Devin 利用環境では legacy `.windsurf/hooks.json`
  の cheaper 登録を除去するか、`read_config_from` で windsurf
  import を無効化する。実 Windsurf も使う環境では登録を残しつつ
  import を無効化することで Windsurf 側には影響しない旨を併記
  する(`read_config_from` の抑止 semantics は T4 の finding と同じ
  説明形に揃える)。
- **verify 経路**: `cheaper hook-config verify` が compat-degraded
  (native 登録未確認 + windsurf conflict + import 有効)と
  windsurf-conflict(native 登録済 + windsurf conflict + import
  有効)を informational finding として報告し、windsurf FileRead
  の範囲無し deny reason が `cheaper hook-config verify` への誘導を
  含む(T3 の guidance)ことを説明する。

新規 section 化ではなく、既存の Devin 登録説明節の中に追記する。
作成済み docs・先例ドキュメントはそのまま残し、本 task は
`distribution.md` の 1 ファイルのみを変更する。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
  (PLAN-034KxCQ2FWFt55qhBNp6sR — T5)
- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS — §`docs/guides/distribution.md`・
  verify 文言・dedup 説明)
- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM — R7)
- `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
  (TSPEC-034KxD0IRkMph8hSZbhTRx — docs 更新・verify 経路・
  dual-registration 説明の Rules)

## Dependencies

- `docs/tasks/add-windsurf-range-deny-guidance.md`(T3) — guidance
  文言を説明に引用するため。
- `docs/tasks/add-hook-config-compat-degraded-findings.md`(T4) —
  finding 名・発火条件・`read_config_from` 抑止 semantics を説明に
  引用するため。

## Affected Files

- `docs/guides/distribution.md`(modify — Devin 登録節への追記のみ)

## Done When

- [x] `distribution.md` の Devin 登録節に `tool_info` で受理する
      range/search 語彙(明示 aliases・pair・array・end_line/
      start_line 単独・gated `pattern`)と受理優先順位が記載される
- [x] dual-registration trap(fingerprint が `range` を含むため
      dedup 不成立・deny が union で勝つ)と remediation(legacy
      除去または `read_config_from` で windsurf import 無効化)が
      記載される
- [x] `hook-config verify` の compat-degraded・windsurf-conflict
      finding 名と発火条件・informational severity・windsurf FileRead
      範囲無し deny reason の `verify` 誘導が記載される
- [x] 文言は spec R7・design の verify 文言と整合し、独自拡大が
      ない
- [x] 実装コードへの変更が無い(docs のみ)

## Out of Scope

- 実装コードの変更(T1-T4 で済み)
- 新規 section 追加・他 docs ファイルの変更
- `sync`/`remove`/`verify` の CLI contract 変更(不変)

## Verification

- [x] `mise exec -- node scripts/check-doc-ids.mjs` exits 0(front
      matter 壊れ無し) — 実測: `doc id check OK: 1109 markdown files
      scanned`
- [x] `node scripts/audit_docs.js --json docs/guides/distribution.md`
      — 本 repo に `scripts/audit_docs.js` は存在しない(`scripts/` 配下の
      audit 系 script 無し、biome も `.md` を対象外)。代替として
      `check-doc-ids.mjs` の全 scan で error 0 を確認し、diff が
      `distribution.md` の追記のみであることを `git diff` で review 済み
