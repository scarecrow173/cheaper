---
id: "PLAN-034KxCQ2FWFt55qhBNp6sR"
type: "plan"
status: "approved"
title: "Windsurf-compat range normalization and registration surfacing (issue #161)"
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
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json"
      - type: "file"
        path: "docs/guides/distribution.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
  implemented-by:
    - "docs/tasks/extend-windsurf-read-range-normalization.md"
    - "docs/tasks/add-windsurf-gated-pattern-search-normalization.md"
    - "docs/tasks/add-windsurf-range-deny-guidance.md"
    - "docs/tasks/add-hook-config-compat-degraded-findings.md"
    - "docs/tasks/document-windsurf-tool-info-contract-and-registration-requirements.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
    - "docs/adr/defensive-compat-normalization-with-registration-surfacing.md"
  derived-by:
    - "docs/tasks/extend-windsurf-read-range-normalization.md"
    - "docs/tasks/add-windsurf-gated-pattern-search-normalization.md"
    - "docs/tasks/add-windsurf-range-deny-guidance.md"
    - "docs/tasks/add-hook-config-compat-degraded-findings.md"
    - "docs/tasks/document-windsurf-tool-info-contract-and-registration-requirements.md"
  verifies: []
  verified-by:
    - "docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md"
  references:
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "docs/adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/guides/distribution.md"
  defers: []
  deferred-by: []
---

# Windsurf-compat range normalization and registration surfacing (issue #161) Implementation Plan

## Goal

DESIGN-034KwO2y86CrXOmaoae1jS の Option B(防御的正規化 +
registration surfacing、ADR-034KwuE0Xnp3O6EUXacFG8)を実装する —
windsurf-compat `pre_read_code` 経路の `ReadRange`/`Search`
受理面を共有 `tool_use_normalize` と parity に拡張し、compat firing
由来の `FileRead{range:None}` Deny へ `hook-config verify` 誘導を
付記、`hook-config verify` に compat-degraded / windsurf-conflict
finding を追加、`legacy-conflict` の dedup 文言を訂正し、
`docs/guides/distribution.md` へ tool_info contract・
dual-registration trap・compat 限界を文書化する。eval 層
(`routing_policy` の評価・threshold・dedup fingerprint)は
一切変更しない。

## Scope

単一の implementation stream。`targets.rs` の normalize/render 差分・
`hook_config.rs` の verify finding・test・docs は同じ release
意図(issue #161 の compat 経路救済 + 診断可能性)を共有し、T1→T2→T3
の同一ファイル直列部分と T4(別ファイル・並列可)、T5(全変更を
文書化する最終段)に分解する。

| In | Out |
|---|---|
| `windsurf_normalize` `pre_read_code` の `ReadRange` alias/pair/array/end_line 受理 + gated `pattern`→`Search` | `routing_policy.rs`・`eval_cache.rs`・threshold・dedup fingerprint(不変) |
| `normalize_view_range` 共有化(copilot `copilot_view_range` の rename + `-1` sentinel・tri-state) | copilot `view` の `[s,e]`(1<=e<s 以外)・他 target normalize の既存意味論 |
| `handle_hook` windsurf arm の `FileRead{range:None}` Deny への guidance 付記 | `range:Some` deny・Allow・Search/Shell verdict・他 target の reason(不変) |
| `hook_config.rs` `run_verify` の `compat-degraded:`/`windsurf-conflict:` informational finding + `legacy-conflict:` dedup 文言訂正 | `sync`/`remove`/`--event-stdin`・own-group 検証・capability gate・orphan sweep・他 finding 文言・exit code 規約(不変) |
| `hook.rs` 統合 test(windsurf ranged/pattern/guidance fixture・`ends_with`→`contains` 修正)・`hook_config.rs` finding fixture・`targets.rs` unit test・regression snapshot 再 capture | bounded shell read 評価・Devin compat 変換本体・wire shape 実測・legacy 登録の自動除去(design Out of Scope と同一境界) |
| `docs/guides/distribution.md` の Devin native hook 登録節への追記 | skill 文書・AGENTS.md(変更不要) |

## Upstream Documents

- `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  (SPEC-034KwuiFzl4WvVQKdjfSnM、approved — R1-R8・acceptance criteria)
- `docs/adr/defensive-compat-normalization-with-registration-surfacing.md`
  (ADR-034KwuE0Xnp3O6EUXacFG8、accepted — Option B・棄却 option の理由)
- `docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md`
  (DISC-t3TdC7pUJUGdYgfkpkmhMU — 発火経路・metadata 消失地点・
  dual-registration trap・exec 分類の確定根拠。`related` のみ、
  cross-chain lineage は張らない)

## Design Inputs

- `docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md`
  (DESIGN-034KwO2y86CrXOmaoae1jS、approved — AUTHORITATIVE な HOW:
  `windsurf_read_range` の Step A/B/C 受理順・矛盾 short-circuit・
  `[s,-1]` sentinel・`normalize_view_range` tri-state 共有化・
  `pattern` gate の 2 条件・`normalize_search` `file_path`→`path`
  fallback・`windsurf_range_deny_guidance` の dispatch 層付記・
  `WINDSURF_RANGE_DROP_GUIDANCE` 文言・verify 両 finding の発火
  条件/文言/severity・`read_config_from.windsurf` 抑止判定・
  `legacy-conflict` 文言訂正・文書化範囲・file manifest)

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-cli/src/hook/targets.rs` | `copilot_view_range`→`normalize_view_range` rename + tri-state 化 + `-1` sentinel、`windsurf_read_range` helper(Step A 明示 alias → Step B pair/array/end_line → Step C fallback)、`windsurf_normalize` `pre_read_code` 分岐の gate 付き `pattern`→`Search` + range helper 接続、`normalize_search` の `file_path`→`path` fallback、`WINDSURF_RANGE_DROP_GUIDANCE` 定数 + `windsurf_range_deny_guidance` + windsurf dispatch arm 接続、`mod tests` の unit test | T1(range parity)・T2(pattern gate + search fallback)・T3(guidance)が変更。`pre_run_command` 経路は不変 |
| `crates/cheaper-cli/src/hook_config.rs` | `run_verify`(:591-752)への `compat-degraded:`/`windsurf-conflict:` finding 追加(`legacyConflicts` insert 直後、`read_config_from.windsurf` 抑止判定)、`legacy-conflict:` dedup 文言訂正(:674-681) | T4 が変更。finding は両方 informational — `blocker`/`clean`/exit code・`remediation` 契約不変 |
| `crates/cheaper-cli/tests/integration/hook.rs` | windsurf ranged payload(`start_line`+`end_line`/`view_range`/`max_lines`)bounded allow、invalid range の全文縮退、pattern gate 各経路、deny stderr の guidance 含有/非含有、`hook.rs:300` `ends_with`→`contains` 修正 | T1・T2・T3 が変更。`write_big_lines` fixture を再利用 |
| `crates/cheaper-cli/tests/integration/hook_config.rs` | compat-degraded/windsurf-conflict finding fixture(absent/missing+conflict、import 無効化、present+conflict、`.claude` only conflict)、`legacy-conflict` 訂正後文言 | T4 が変更。`write_legacy_conflict` fixture を再利用 |
| `crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json` | `CHEAPER_REGRESSION_CAPTURE=1` で byte-exact stderr snapshot を再 capture(guidance 行追加で破損) | T3 が変更。case `cli-hook-windsurf-deny.json` は `mode:"raw"` + `compareStderr:true` |
| `docs/guides/distribution.md` | Devin native hook 登録節(:110-207、bounded eval 段落 :146-149 直後)へ tool_info field contract・compat 限界・dual-registration trap・shell eval 説明・verify 出力 contract 変更の追記 | T5 が変更 |

## Implementation Sequence

- [ ] TASK-034KxDc5lyn6aDJbljcI3N (T1): `normalize_view_range` への
      rename + tri-state(Valid/Contradiction/Incomplete)化 +
      `[s,-1]`→`{s,None}` sentinel、`windsurf_read_range` helper の
      Step A/B/C 受理順実装、`windsurf_normalize` FileRead 分岐への
      接続、`mod tests` の table-driven unit test(range helper 全
      経路 + copilot `view` の `[s,-1]`/`[s,0]` pin)
- [ ] TASK-034KxDgCFTiXTthZajUTX1 (T2): `windsurf_normalize`
      `pre_read_code` 分岐の kind 判定最優先 gate 付き
      `pattern`→`Search`(非空 `pattern` かつ `file_path` 欠落/空/
      非 string/dir 解決)、`normalize_search` の `file_path`→`path`
      fallback、unit test(gate 全分岐)
- [ ] TASK-034KxDm7X0zR41q82TJxVY (T3): `WINDSURF_RANGE_DROP_GUIDANCE`
      定数 + `windsurf_range_deny_guidance` + windsurf dispatch arm
      接続、`hook.rs` windsurf 統合 test(ranged/pattern/guidance
      fixture)+ `:300` `ends_with`→`contains` 修正、regression
      snapshot `CHEAPER_REGRESSION_CAPTURE=1` 再 capture
- [ ] TASK-034KxDr0eOqS1QjXxMazEk (T4): `run_verify` の
      `compat-degraded:`/`windsurf-conflict:` finding + windsurf
      conflict 識別(suffix 比較)+ `read_config_from.windsurf`
      抑止判定、`legacy-conflict:` dedup 文言訂正、`hook_config.rs`
      統合 test fixture
- [ ] TASK-034KxDvREdpF6P10oziJsy (T5): `docs/guides/distribution.md`
      追記 + `mise run check` で閉じる

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 range parity | — | T2 |
| T2 pattern gate + search fallback | T1 | T3 |
| T3 deny guidance + test/snapshot | T2 | T5 |
| T4 verify findings + fixtures | — | T5 |
| T5 docs + 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T3 → T5。T2 が T1 に依存するのは同一
`windsurf_normalize` `pre_read_code` 分岐の構造上(pattern kind 判定が
file_path→FileRead 分岐より先に評価される順序を T1 の helper 接続後に
挿入する)から。T3 は同一 `targets.rs` の dispatch arm + 共有 test
file を触るため T2 の直列後続。T4 は `hook_config.rs` 系のみで
T1-T3 とファイル非重複のため並列実行可能。T5 は全変更の文書化 +
`mise run check` で閉じる最終段。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 混在語彙 payload(`offset`/`limit` 系 + pair/array)が emitter 意図より狭い評価へ deterministic 解決される | low | med | precedence を design で固定済み(明示 alias → pair/array → end_line)。単一語彙の「無効→None/広い window」不変条件のみ保証し、混在の限定は test-spec の実装 note で明文化 |
| 2 | `pattern`→`Search` が file を指す `file_path` 併記 payload を過少評価(matched-lines ⊆ 全文) | med | high | 2 条件 gate(非空 `pattern` かつ `file_path` 欠落/空/非 string/dir 解決)で FileRead 評価を優先 — T2 の unit/integration test で pin |
| 3 | `pattern` gate の dir 判定が normalize 層に fs access を持ち込む | low | low | `pattern` 非空かつ `file_path` 非空の場合のみ `std::fs::metadata` 1 回に限定。eval 走査までの dir→file TOCTOU は design 受理済み残余 |
| 4 | guidance 行が実 Windsurf 環境の range 無し deny にも載る | high(必発) | low | 条件付き表現("If this firing reached the hook through a compatibility registration")で実害を限定 — ADR Consequences 受理済み。deny 文言 assert 2 箇所(`hook.rs:300`・regression snapshot)は T3 で追随 |
| 5 | `compat-degraded`/`windsurf-conflict` finding が Devin の windsurf import 実装差(version skew)で実態と食い違う | med | low | `read_config_from.windsurf == false` 明示無効化でのみ抑止し、文言は conditional 表現 + informational severity に留める(design 固定文言を転記) |
| 6 | `file_path`→`path` fallback が devin/pretooluse grep の ambient `file_path` を unsafe 方向(search root 縮小)へ倒す | low | med | `path` key 優先の既存 contract 維持 + windsurf 側は gate で file 指摘 `file_path` 併記を排除済み。未観測 wire の残余 risk として test-spec・doc に明記 |
| 7 | `normalize_view_range` 共有化が copilot `view` に `-1` sentinel を効かせ現行 `None` との差分を生む | high(必発) | low | design 受理済み(copilot wire で `-1` 未観測、`to EOF` 解釈は通用)。copilot `view_range` の unit test pin を T1 に含める |
| 8 | data-loss / breaking-change | — | — | deny reason は additive 追記のみ、verify finding は informational 追加のみ、登録 file 自動除去なし。既存 consumer config・hook 応答 schema は非互換なし |
| 9 | external dependency | — | — | 新規 crate 依存ゼロ(design 明記)。検証は in-process `run_cli` + fixture のみ |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step(migration・外部 format 固定)は無い — snapshot 再
capture も `CHEAPER_REGRESSION_CAPTURE=1` の再実行で再現可能。

## Task Breakdown

- T1 `docs/tasks/extend-windsurf-read-range-normalization.md`
  (`TASK-034KxDc5lyn6aDJbljcI3N`) — `normalize_view_range` tri-state
  共有化 + `windsurf_read_range` + FileRead 分岐接続 + unit test。
  verified by `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
- T2 `docs/tasks/add-windsurf-gated-pattern-search-normalization.md`
  (`TASK-034KxDgCFTiXTthZajUTX1`) — gate 付き `pattern`→`Search` +
  `normalize_search` `file_path` fallback + unit test。
  verified by `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
- T3 `docs/tasks/add-windsurf-range-deny-guidance.md`
  (`TASK-034KxDm7X0zR41q82TJxVY`) — guidance 定数/helper/dispatch
  接続 + windsurf 統合 test + `ends_with` 修正 + snapshot 再 capture。
  verified by `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
- T4 `docs/tasks/add-hook-config-compat-degraded-findings.md`
  (`TASK-034KxDr0eOqS1QjXxMazEk`) — `compat-degraded:`/
  `windsurf-conflict:` finding + `read_config_from` 抑止 +
  `legacy-conflict` 文言訂正 + 統合 test fixture。
  verified by `docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md`
- T5 `docs/tasks/document-windsurf-tool-info-contract-and-registration-requirements.md`
  (`TASK-034KxDvREdpF6P10oziJsy`) — tool_info contract・
  dual-registration trap・compat 限界・verify 出力 contract の
  文書化 + `mise run check` 最終検証

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --lib`
      で `targets.rs` unit test(range helper・pattern gate・copilot
      view_range pin)が pass(T1・T2)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`
      で windsurf の `start_line`/`end_line`/`view_range`/`max_lines`
      bounded allow・invalid 縮退・pattern gate・deny guidance 含有/
      非含有、および既存 devin/pretooluse bounded・search/shell・
      dedup の非回帰を確認(T1・T2・T3)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::`
      で compat-degraded/windsurf-conflict finding・import 無効化
      抑止・`legacy-conflict` 訂正文言・exit code 規約を確認(T4)
- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test regression`
      で再 capture 済み `cli-hook-windsurf-deny.json` を含む全
      snapshot が pass(T3)
- [ ] `mise run check` が pass(T5)
- [ ] `node scripts/check-doc-ids.mjs` が pass(全 task)
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --json`
      で新規 doc の error 0(全 task)

## Open Questions / Gaps

- Devin 3000.11.92 の compat wire shape は未観測 — defensive
  normalization が不確定性をカバーする design 前提であり、wire probe
  は consumer 側の別作業(design §Deferred Design Concerns)。
- compat firing の環境識別(`DEVIN_PROJECT_DIR` 等の env marker が
  compat firing にも立つか)は未確認 — 確定すれば guidance を Devin
  context に絞り込めるが、本 plan では条件付き文言で対応する。
- windsurf conflict 検出は `KNOWN_LEGACY_FILES` の project root 直下
  大小文字区別の suffix 比較に限定される(design §Deferred Design
  Concerns の継承限界 — user scope・親 dir・大小文字差 variant は
  検出しない)。
- `normalize_view_range` の tri-state 表現(enum/入れ子 Option 等の
  具体形)は T1 実装者の選択に委ねる — 契約(矛盾 = helper 全体
  short-circuit、不完全 = 次判定へ fall-through)のみ test-spec の
  assertion で固定する。

## Self-Review

- [x] Every spec requirement maps to a task.(R1-R3→T1、R4→T2、
      R5→T3、R6→T4、R7→T5、R8→T1-T4 の非回帰検証)
- [x] No placeholders remain.(未確定事項は Open Questions へ明示)
- [x] File paths, relations, status values, and terminology are
      consistent.(design manifest と同一 path 集合)

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation —
  T4(`hook_config.rs` 系)は T1-T3(`targets.rs` 系)とファイル
  非重複のため並列実行可能。
- Independent work streams: T1→T2→T3(critical path、同一ファイル
  直列)、T4(並列可)、T5(最終、T3+T4 完了後)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 焦点: `pattern` gate が「非空 `pattern` かつ `file_path` が file
  非指摘」の両条件でしか Search へ倒さないこと(空 pattern の
  fail-open・file 併記の過少評価を塞ぐ)、矛盾 pair/array が
  Step C fallback へ落ちず helper 全体 `None` で short-circuit
  すること(`{start_line:500,end_line:20}`→`{500,None}` になら
  ない)、`normalize_view_range` の tri-state で copilot が
  non-Valid を plain `None` に写像し続けること。
- 明示仮定: `compat-degraded`/`windsurf-conflict` は windsurf
  conflict + import 状態から compat 経路の可能性を推論する間接
  検出であり、Devin が実際に `.windsurf/hooks.json` を consume
  するかは一次情報未確定 — informational severity・conditional
  文言で限定する(design Risks)。
- 未解決依存: Devin 3000.11.92 wire shape 実測・env marker 確認は
  consumer 側 probe の別作業 — 本 plan の完了条件には含まない。
- 上流: SPEC-034KwuiFzl4WvVQKdjfSnM /
  ADR-034KwuE0Xnp3O6EUXacFG8 / DESIGN-034KwO2y86CrXOmaoae1jS。
