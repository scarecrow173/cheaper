---
id: "TASK-034KxDgCFTiXTthZajUTX1"
type: "task"
status: "done"
title: "Add gated pattern-to-Search normalization to windsurf pre_read_code"
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
    - "docs/tasks/extend-windsurf-read-range-normalization.md"
  blocks:
    - "docs/tasks/add-windsurf-range-deny-guidance.md"
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
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
  defers: []
  deferred-by: []
---

# Add gated pattern-to-Search normalization to windsurf pre_read_code

## Work

plan `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
(PLAN-034KxCQ2FWFt55qhBNp6sR)の T2。`windsurf_normalize` の
`pre_read_code` 分岐に gate 付き `pattern`→`Search` 正規化を追加し、
共有 `normalize_search` に `file_path`→`path` fallback を入れる。
design(DESIGN-034KwO2y86CrXOmaoae1jS)「§`hook::targets` —
`windsurf_normalize` の `pre_read_code` 拡張」step 1 と
「§`normalize_search` の `path` fallback」をそのまま実装する。

### 1. `pattern`→`Search`(kind 判定を最優先、2 条件 gate 付き)

`pre_read_code` 分岐(T1 後の `targets.rs:162` 付近)で、`tool_info`
object が `pattern` を**非空 string**(`as_str` 成功かつ
`!is_empty()`)として持ち、かつ `file_path` が次のいずれかの場合に
限り、`normalize_search(tool_info, cwd,
Some("windsurf:pre_read_code"))` で `RoutingRequest::Search` を返す:

- (i) `file_path` が欠落・空文字・非 string、または
- (ii) `nodepath::resolve(&cwd, file_path)` の結果への
  `std::fs::metadata().is_dir()` が true(directory 解決)。

`resolve_target`(`routing_policy.rs:316-350`)と同一の「cwd 相対
resolve → metadata 分類」規約に揃えるが、normalize 層では
workspace containment は見ない(`OutOfScope` の allow 判定は eval
側の責務)。metadata 失敗・file 解決は directory 非該当として
FileRead 判定へ進む。この fs 参照は `pattern` 非空かつ
`file_path` 非空の場合にのみ走り、normalize 層への fs アクセス
追加はその 1 回に限定する。

`pattern` が非 string・空 string の場合はこの経路を取らず
FileRead 判定へ進む(`normalize_search` の `as_str` gate と同一
条件でガードし、None 返しの分岐を残さない)。`file_path` が file
を指す `pattern` 併記 payload も FileRead 経路 — 「全文 read が
最悪ケース」として FileRead eval を優先する。

この gate は spec R4 の字句(`pattern` を string として持てば
`Search`)の発火条件を絞る design 側の修正であり、R3(無効・曖昧
な入力が過少評価へ縮退しない)を優先する — 空 `pattern` の無条件
`Allow`(`routing_policy.rs:1575-1577`)による fail-open と、
`evaluate_search`(:970-1032)の一致行 ⊆ file 内容実体化による
過少評価を normalize 段で塞ぐ。

### 2. `normalize_search` の `path` fallback

`targets.rs:297-348` の `path` 解決を、`path` key 欠落時に
`file_path`(非空 string)を search root として使う規約へ拡張する:

- `path` key が存在 → 従来どおり `path` 優先(非 string は `"."`)。
- `path` 欠落 かつ `file_path` が非空 string → `file_path`。
- 両者欠落・`file_path` 空文字/非 string → `"."`。

この拡張は共有 `tool_use_normalize`(devin/pretooluse の `grep`)
にも効くが、両者の grep contract は `path` であり `file_path` を
送る wire は未観測。fallback による root 縮小(`"."` より狭い
root)は `file_path` が ambient context の場合に unsafe 方向の
残余 risk として残る(design §Risks — windsurf 側は gate で限定
済み)。

### 3. unit test(`targets.rs` `mod tests`)

- `windsurf_normalize` の `pattern` gate 全分岐:
  - `pattern` 非空 + `file_path` 欠落 → `Search` request
    (`source_tool:"windsurf:pre_read_code"`)。
  - `pattern` 非空 + `file_path` 空文字/非 string → `Search`。
  - `pattern` 非空 + `file_path` が dir 解決(tempdir fixture)→
    `Search`、search root が `file_path`。
  - `pattern` 非空 + `file_path` が file 解決 → `FileRead`
    (gate が FileRead 優先)。
  - `pattern:""` / `pattern` 非 string → `FileRead`(Search へ
    倒れない)。
- `normalize_search` `path` fallback: `path` 有り → `path` 優先、
  `path` 非 string → `"."`、`path` 欠落 + `file_path` 非空 →
  `file_path`、両者欠落 → `"."`。

### 4. integration test(`tests/integration/hook.rs`)

- `tool_info{file_path, pattern:"稀" or "lorem", max_results:5}`
  で `file_path` が dir 解決(または欠落)の payload → `Search`
  評価へ到達し、実体化結果が threshold 内なら allow・超過なら
  deny(現行は FileRead 全文 deny)。
- `pattern` + `file_path`(file 解決)併記 → FileRead 評価を
  維持(全文 deny)。
- `pattern:""` 併記 → FileRead 評価を維持(空 pattern の
  fail-open しない)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md`
  (PLAN-034KxCQ2FWFt55qhBNp6sR — T2)
- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS — 「§`hook::targets`」step 1・
  「§`normalize_search` の `path` fallback」が AUTHORITATIVE)
- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM — R3・R4)
- `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
  (TSPEC-034KxD0IRkMph8hSZbhTRx — gate・fallback・fs access 限定
  の Rules)

## Dependencies

- `docs/tasks/extend-windsurf-read-range-normalization.md`(T1) —
  同一 `windsurf_normalize` `pre_read_code` 分岐の構造上、kind 判定
  最優先の `pattern` gate を T1 の FileRead/range 接続後に挿入する。

## Affected Files

- `crates/cheaper-cli/src/hook/targets.rs`(modify —
  `windsurf_normalize` `pre_read_code` 分岐・`normalize_search`・
  `mod tests`)
- `crates/cheaper-cli/tests/integration/hook.rs`(modify — pattern
  gate fixture 追加)

## Done When

- [x] `pattern` 非空 string かつ `file_path` が欠落/空/非 string/
      dir 解決の payload が `RoutingRequest::Search` へ正規化される
- [x] `pattern` 非空かつ `file_path` が file 解決の payload が
      FileRead 評価を維持する(過少評価しない)
- [x] `pattern:""`・非 string の payload が FileRead 判定へ進む
      (空 `pattern` の fail-open しない)
- [x] `normalize_search` の `path` 解決が `path` 優先・
      `file_path` fallback・`"."` 既定の規約で動く
- [x] normalize 層の fs access が `pattern` 非空かつ `file_path`
      非空時の metadata 1 回に限定される
- [x] deny guidance(T3)が未実装 — 本 task の normalize 変更のみ

## Out of Scope

- `windsurf_read_range` 本体(T1 で済み)
- `WINDSURF_RANGE_DROP_GUIDANCE`・`windsurf_range_deny_guidance`・
  `hook.rs:300` 修正・regression 再 capture(T3)
- `hook_config.rs` の変更(T4)
- `routing_policy.rs` の eval 本体変更(不変)

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib` exits 0(gate 全分岐の unit test 含む) — 実測: 95 passed / 0 failed(新規 2 test: `windsurf_normalize_pattern_gate_routes_search_or_fileread`・`normalize_search_path_fallback_order`)
- [x] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::` exits 0(pattern gate fixture + 既存 search/shell 非回帰) — 実測: 28 passed / 0 failed(新規 `windsurf_pattern_gate_routes_search_preserves_fileread` — Search deny 2 系・allow 3 系・FileRead 維持 3 系)
- [x] `mise exec -- cargo clippy -p cheaper-cli --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` exits 0
