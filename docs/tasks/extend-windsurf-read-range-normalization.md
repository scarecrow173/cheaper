---
id: "TASK-034KxDc5lyn6aDJbljcI3N"
type: "task"
status: "done"
title: "Extend windsurf pre_read_code ReadRange normalization parity"
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
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  implemented-by: []
  depends-on:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  blocks:
    - "docs/tasks/add-windsurf-gated-pattern-search-normalization.md"
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
    - "crates/cheaper-cli/src/hook/targets.rs"
  defers: []
  deferred-by: []
---

# Extend windsurf pre_read_code ReadRange normalization parity

## Work

plan `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
(PLAN-034KxCQ2FWFt55qhBNp6sR)の T1。`targets.rs` の windsurf
`pre_read_code` 経路の `ReadRange` 抽出 coverage を共有
`tool_use_normalize`(`targets.rs:456-495`)と parity に拡張する。
design(DESIGN-034KwO2y86CrXOmaoae1jS)「§`windsurf_read_range`」の
受理規則をそのまま実装する — 再解釈・再導出しない。

### 1. `copilot_view_range` → `normalize_view_range` の共有化

`copilot_view_range`(`targets.rs:419-433`)を `normalize_view_range`
へ rename し、戻り値を `Option<ReadRange>` から **3 値表現**
(tri-state — `Valid(ReadRange)`/`Contradiction`/`Incomplete` を
区別できる enum または同等形)へ拡張する。`Option` のままでは
windsurf caller が「矛盾 = helper 全体 `None` short-circuit」と
「不完全 = 次判定へ fall-through」を区別できない(TSPEC の実装
note、design「§`windsurf_read_range`」Step B)。

変換規約(1-based・inclusive、`[startLine, endLine]` array):

- `start` が `json_line_number` 受理(`>=1` の整数)でない →
  Incomplete。
- `end == -1`(raw 値)→ `Valid(ReadRange{offset:start, limit:None})`
  — "to EOF" sentinel。共有化に伴い copilot `view` にも効く
  (design 受理済の差分)。
- `end` が `json_line_number` 受理かつ `end < start` →
  Contradiction。`end >= start` →
  `Valid(ReadRange{offset:start, limit:end-start+1})`。
- `end` が上記以外(`end < -1`、`end == 0`、非整数、欠落、
  array 非形成)→ Incomplete。

copilot 側の呼出し(`copilot_handle` の `view` 経路、:398)は
non-Valid を全て `None` へ写像する plain `Option` 扱いとし、従来
意味論と同値に保つ。

### 2. `windsurf_read_range(tool_info) -> Option<ReadRange>` 新規 helper

受理優先順位(design 固定 — spec「明示 offset/limit aliases →
pair/array 形式 → `end_line` 単独」):

- **Step A — 明示 alias 経路**: `offset`/`line_offset`/`limit`/
  `line_limit`/`max_lines` のいずれかの key が `tool_info` に存在
  する場合、`normalize_read_range(tool_info,
  &["offset","line_offset","start_line"],
  &["limit","line_limit","max_lines"])` をそのまま返す。explicit
  key が 1 つでもあれば pair/array/end_line 形式は参照しない
  (曖昧な混在 payload は明示語彙側へ deterministic 解決)。
- **Step B — inclusive pair/array 形式**(Step A の key が 1 つも
  無い場合のみ評価):
  - `start_line`+`end_line` が**共に** `json_line_number` 受理 →
    complete pair。`end_line < start_line` なら **helper 全体が
    即座に `None`**(矛盾 short-circuit — 後続の `view_range`・
    `end_line` 単独・Step C へ進まない)。`end_line >= start_line`
    なら `ReadRange{offset:start_line, limit:end_line-start_line+1}`。
  - complete pair を構成しない場合のみ `view_range` を
    `normalize_view_range` で評価: Valid → その `ReadRange`、
    Contradiction → **helper 全体 `None`**、Incomplete → 次へ。
  - `end_line` 単独(`start_line` 欠落・無効値)で `json_line_number`
    受理できる整数なら `ReadRange{offset:1, limit:end_line}`。
- **Step C — fallback**: 上記いずれも不成立(complete pair/array
  が一度も確定しなかった場合に限り — 矛盾 short-circuit `None` は
  ここへ落ちない)なら `normalize_read_range`(Step A と同一 key
  集合)を呼ぶ — `start_line` 単独で `{offset:s, limit:None}`
  (s 行〜EOF)を受理する経路。`None` は全文評価縮退を意味する。

いかなる経路でも**過少評価に縮退しない**: 単一語彙入力では
無効・矛盾・非整数値は `None`(全文評価)またはより広い window
へ倒す。混在語彙 payload は documented precedence による
deterministic 解決(不変条件の限定は design「§`windsurf_read_range`」
末尾と TSPEC Rule を参照)。

### 3. `windsurf_normalize` の FileRead 分岐接続

`windsurf_normalize`(`targets.rs:162-177`)の `pre_read_code` 分岐
で、`RoutingRequest::FileRead` の `range` を現行の
`normalize_read_range(t, &["offset","line_offset"],
&["limit","line_limit"])` から `windsurf_read_range(t)` へ差し替える。
`file_path` 必須条件・`source_tool`・`pre_run_command` 分岐
(:178-189)は不変。本 task では `pattern`→`Search` 分岐を
**入れない**(T2 の範囲)。

### 4. unit test(`targets.rs` `mod tests`)

`windsurf_normalize` と range/search 系 helper は現行 unit test を
持たない(design §Verification Notes)— 判定表が増える本変更では
最低層で pin する。`mod tests` へ table-driven test を追加:

- `windsurf_read_range`: offset/limit 各 alias(`offset`/
  `line_offset`/`limit`/`line_limit`/`max_lines`/`start_line` 単独・
  組合せ)、`start_line`+`end_line` pair→`{s,e-s+1}`、
  `view_range`→`{s,e-s+1}`、`end_line` 単独→`{1,e}`、
  `start_line` 単独→`{s,None}`(Step C)、`offset==1 && limit==None`
  →`None`、無効値(0・負値・非整数・非数)→`None`。
- 矛盾 short-circuit: `{start_line:500,end_line:20}` → helper 全体
  `None`(Step C の `{500,None}` に**ならない**)、
  `view_range:[5,2]` → `None`。
- `view_range:[s,-1]` → `{s,None}`、"to EOF" sentinel。
- 混在: `{offset:10,limit:5,view_range:[1,1000]}` → `{10,5}`
  (explicit key 優先で pair/array 不参照)。
- copilot pin: `copilot_handle` の `view` で `view_range:[10,-1]`
  → `range:Some({10,None})`、`view_range:[10,0]` → `range:None`
  (従来どおりの全文評価)、`view_range:[10,2]` → `range:None`。
- `windsurf_normalize` 経由: `pre_read_code` payload の
  `tool_info` 各形が期待 `ReadRange` を持つ `FileRead` request へ
  正規化される。

### 5. integration test(`tests/integration/hook.rs`)

`write_big_lines`(各行小・合計 threshold 超過)fixture で:

- `tool_info{file_path:"big_lines.txt", start_line:1, end_line:20}`
  → bounded allow(現行は全文 deny)。
- `tool_info{file_path:"big_lines.txt", view_range:[1,20]}` →
  bounded allow。
- `tool_info{file_path:"big_lines.txt", offset/limit}` 既存形・
  `start_line`+`limit`/`max_lines` 組合せ → bounded allow。
- 無効な `start_line`/`end_line`/`view_range` → 全文評価縮退で
  deny(過少評価しない)。

`windsurf_read` helper は `tool_info` をそのまま埋め込める形へ
拡張してよい(payload 構築の重複を避ける)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
  (PLAN-034KxCQ2FWFt55qhBNp6sR — T1)
- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS — 「§`windsurf_read_range`」・
  「§`hook::targets` — `windsurf_normalize` の `pre_read_code`
  拡張」が AUTHORITATIVE)
- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM — R1-R3)
- `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
  (TSPEC-034KxD0IRkMph8hSZbhTRx — 受理順・矛盾・tri-state・
  copilot pin の Rules)

## Dependencies

- なし(起点 task)。`normalize_read_range`・`json_line_number`・
  `pick_value` は既存のまま利用。

## Affected Files

- `crates/cheaper-cli/src/hook/targets.rs`(modify —
  `copilot_view_range` rename + tri-state 化・`windsurf_read_range`
  追加・`windsurf_normalize` FileRead 分岐・`copilot_handle` 呼出
  し・`mod tests`)
- `crates/cheaper-cli/tests/integration/hook.rs`(modify — windsurf
  ranged fixture 追加)

## Done When

- [x] `normalize_view_range` が 3 値(Valid/Contradiction/
      Incomplete)を返し、copilot 経路が従来どおり plain `None`
      写像で動く
- [x] `windsurf_read_range` が Step A/B/C の受理順で
      alias/pair/array/end_line/fallback を写像する
- [x] 矛盾 pair/array が helper 全体 `None` で short-circuit し
      Step C へ落ちない(`{start_line:500,end_line:20}`→`{500,None}`
      にならない)
- [x] `view_range:[s,-1]` が `{s,None}` へ写像される
- [x] windsurf `pre_read_code` の ranged payload が integration
      test で bounded allow・invalid が全文縮退 deny になる
- [x] copilot `view_range:[s,-1]`/`[s,0]` の unit test が pin
      されている
- [x] `pattern`→`Search` 分岐・deny guidance が未実装(T2/T3
      の範囲)

## Out of Scope

- `pattern`→`Search` gate・`normalize_search` の `file_path`
  fallback(T2)
- `WINDSURF_RANGE_DROP_GUIDANCE`・`windsurf_range_deny_guidance`・
  `hook.rs:300` 修正・regression 再 capture(T3)
- `hook_config.rs` の変更(T4)
- `pre_run_command` 経路・`routing_policy.rs`・`eval_cache.rs`
  (変更不要)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib` exits 0(`mod tests` の新規 unit test 含む全 pass) — 実測: 93 passed / 0 failed(新規 3 test: `windsurf_read_range_table`・`copilot_view_range_maps_non_valid_to_none`・`windsurf_normalize_maps_tool_info_range_vocabularies`)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0(windsurf ranged fixture + 既存 contract 非回帰) — 実測: 27 passed / 0 failed(新規 `windsurf_ranged_read_vocabularies_bounded_eval` — bounded allow 9 系・全文縮退 deny 8 系を loop assert)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0
