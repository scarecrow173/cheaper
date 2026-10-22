---
id: "IMPL-034KyCatPzZz2THKRqjLIe"
type: "impl"
status: "completed"
title: "Windsurf-compat range normalization and registration surfacing issue 161"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
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
    - "docs/tasks/extend-windsurf-read-range-normalization.md"
    - "docs/tasks/add-windsurf-gated-pattern-search-normalization.md"
    - "docs/tasks/add-windsurf-range-deny-guidance.md"
    - "docs/tasks/add-hook-config-compat-degraded-findings.md"
    - "docs/tasks/document-windsurf-tool-info-contract-and-registration-requirements.md"
  implemented-by: []
  depends-on:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/windsurf-compat-bounded-eval-and-registration-surfacing.md"
  references:
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/adr/defensive-compat-normalization-with-registration-surfacing.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Windsurf-compat range normalization and registration surfacing issue 161

## Summary

issue #161(#107 再発)— Devin compat(windsurf 経由)firing で `tool_info`
の range/search 語彙が届かず `FileRead{range:None}` の全文評価へ縮退
して deny される問題へ、ADR-034KwuE0Xnp3O6EUXacFG8 で選択された
Option B(防御的正規化 + registration surfacing)を 5 task で実装した。
eval 層(`routing_policy` の評価・threshold・dedup fingerprint)は
#107 で正常確認済みのため一切変更していない。

1. `targets.rs`(T1): `copilot_view_range` を tri-state 返却の共有
   `normalize_view_range` へ拡張し、`windsurf_read_range` helper を
   追加 — 共有 `tool_use_normalize` との `ReadRange` 受理 parity
   (offset/line_offset・limit/line_limit/max_lines alias、`start_line`+
   `end_line` pair、`view_range` array、`end_line` 単独、Step C
   fallback)を達成。
2. `targets.rs`(T2): 非空 `pattern` + `file_path` 欠落/空/dir 解決で
   `RoutingRequest::Search` へ写像する gate 付き `pattern`→`Search`
   正規化、および `normalize_search` の `path` 解決への `file_path`
   fallback を追加。
3. `targets.rs`(T3): windsurf `FileRead{range:None}` Deny に
   `WINDSURF_RANGE_DROP_GUIDANCE`(compat 範囲不達の可能性 +
   `hook-config verify` 誘導)を dispatch arm で post-eval 付記。
   regression snapshot `cli-hook-windsurf-deny.json` を再 capture。
4. `hook_config.rs`(T4): `run_verify` に informational の
   `compat-degraded`(own_entry absent/missing + windsurf legacy
   conflict)・`windsurf-conflict`(own_entry present 系 + conflict
   残存 = dual-registration trap) finding を追加し、
   `legacy-conflict` の dedup 文言を「同一 canonical request
   (fingerprint 一致)限定・range-dropping compat firing は対象外」へ
   訂正。
5. `distribution.md`(T5): windsurf `tool_info` field contract・受理
   語彙と precedence・fail-closed 縮退・dual-registration trap・
   remediation・compat 経路の限界を記載。

## Context

TASK-034KxDc5lyn6aDJbljcI3N(T1)・TASK-034KxDgCFTiXTthZajUTX1(T2)・
TASK-034KxDm7X0zR41q82TJxVY(T3)・TASK-034KxDr0eOqS1QjXxMazEk(T4)・
TASK-034KxDvREdpF6P10oziJsy(T5)。PLAN-034KxCQ2FWFt55qhBNp6sR /
DESIGN-034KwO2y86CrXOmaoae1jS / SPEC-034KwuiFzl4WvVQKdjfSnM /
ADR-034KwuE0Xnp3O6EUXacFG8 / TSPEC-034KxD0IRkMph8hSZbhTRx に従う。
原因は DISC-t3TdC7pUJUGdYgfkpkmhMU で確定済み — Devin 側 compat
変換が `tool_info` に `file_path` 以外を載せないため(cheaper 制御外)、
cheaper 側の gap である windsurf normalize の受理面狭小と
compat-degraded 登録状態の不可視を修正した。

## Implementation

### Changes

- `crates/cheaper-cli/src/hook/targets.rs`:
  - `copilot_view_range` → `normalize_view_range` rename + tri-state
    (`Valid`/`Contradiction`/`Incomplete`)化。copilot `view` 呼出は
    non-Valid を plain `None` 写像し従来意味論を維持(`[s,-1]` EOF
    sentinel は共有化差分として design 受理済)。
  - `windsurf_read_range(tool_info)` 新規 helper — Step A(明示
    alias key 存在時は `normalize_read_range` へ直接委譲)/ Step B
    (complete `start_line`+`end_line` pair → `{s,e-s+1}`、`view_range`
    → tri-state 評価、`end_line` 単独 → `{1,e}`)/ Step C(fallback —
    `start_line` 単独で `{s,None}`)の受理順。矛盾 pair/array は
    helper 全体 `None` で short-circuit し Step C へ落ちない。
  - `windsurf_normalize` の `pre_read_code` 分岐で `pattern` gate
    (非空 string の `pattern` があり `file_path` が欠落/空/dir 解決
    の場合のみ `RoutingRequest::Search`、file 解決・metadata 失敗時は
    FileRead 全文評価を維持 — fs access は gated 経路の metadata
    call 1 回に限定)と `windsurf_read_range` 接続。
  - `normalize_search` の `path` 解決へ非空 `file_path` fallback を
    追加(`path` key 存在時は従来どおり、`.` default 不変)。
  - `WINDSURF_RANGE_DROP_GUIDANCE` 定数 + windsurf dispatch arm で
    `FileRead{range:None}` Deny のみへ remediation 付記(dedup
    verdict record は raw verdict を保持)。
  - `mod tests` へ table-driven unit test を追加(windsurf normalize
    と range/search 系 helper は従来 unit test を持たなかった)。
- `crates/cheaper-cli/src/hook_config.rs`: `run_verify` の
  `legacyConflicts` insert 直後に windsurf 系 conflict 識別
  (`.windsurf/hooks.json` suffix 比較、separator `/` normalize)+
  `read_config_from.windsurf == false` 抑止判定を追加し、
  `compat-degraded`/`windsurf-conflict` finding を emit。両者とも
  informational — blocker/clean/exit code 不変。`legacy-conflict`
  の dedup 文言を訂正。
- `crates/cheaper-cli/tests/integration/hook.rs`: windsurf ranged
  fixture(`write_big_lines`)・pattern gate fixture・deny guidance
  fixture を追加。`deny.stderr.ends_with(...)` assert を `contains`
  系へ修正(guidance 末尾追加に伴う)。
- `crates/cheaper-cli/tests/integration/hook_config.rs`: finding
  fixture 5 test を追加。
- `crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json`:
  `CHEAPER_REGRESSION_CAPTURE=1` で再 capture(guidance 1 行追加の
  み)。
- `docs/guides/distribution.md`: Devin native hook 登録節へ
  `tool_info` field contract・受理語彙/precedence・dual-registration
  trap・compat 限界・remediation を追記(+61 行)。

### Decisions During Implementation

- tri-state `normalize_view_range` の共有化 — `Option` のままでは
  「矛盾 = helper 全体 `None`」と「不完全 = 次判定へ fall-through」
  を区別できないため(TSPEC 実装 note・design §`windsurf_read_range`
  Step B)。copilot 側は従来意味論との同値性を保つ plain `Option`
  扱いとした。
- `pattern` gate の dir/file 判定は `file_path` の metadata 解決に
  限定し、file 解決時は FileRead 評価を優先 — 範囲語彙と grep
  payload の混在で過少評価へ倒さない fail-closed 設計(design 固定)。
- finding は informational 限定 — blocker にすると `post-install`/
  `post-update` lifecycle の verify code 透過(conflict 残余を
  exit 0 で返す既存 contract・`legacy-conflict` informational 規約)
  を破壊するため。
- 抑止判定は `read_config_from.windsurf` が明示 JSON `false` の
  場合のみ — key 欠落・非 object・`false` 以外の値・config 不在は
  全て「無効化なし」の保守判定。
- regression 再 capture で同時発生した `cli-usage-empty`/
  `cli-usage-seeded` の field order churn は本 task 範囲外として
  revert 済み(T3 task の Verification 記録)。

## Related Experiments

### Adopted

なし(known-solution task。design が受理規則・判定表・finding 文言
まで確定済みのため Experiment Log は未作成)。

### Rejected

なし。

## Validation

- `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`:
  実測 29 passed / 0 failed(T1 で `windsurf_ranged_read_vocabularies_bounded_eval`
  — bounded allow 9 系・全文縮退 deny 8 系、T2 で
  `windsurf_pattern_gate_routes_search_preserves_fileread` — Search
  deny 2 系・allow 3 系・FileRead 維持 3 系、T3 で
  `windsurf_range_deny_guidance_conditions_and_dedup` を追加)
- `cargo test ... --test integration hook_config::`: 実測 26 passed /
  0 failed(新規 5 test: `hook_config_verify_compat_degraded_findings`・
  `hook_config_verify_windsurf_conflict_finding_informational`・
  `hook_config_verify_read_config_from_suppression`・
  `hook_config_verify_non_windsurf_conflict_emits_neither`・
  `hook_config_verify_legacy_conflict_wording_corrected`)
- `cargo test ... --lib`: 実測 95 passed / 0 failed(新規
  `windsurf_read_range_table`・`copilot_view_range_maps_non_valid_to_none`・
  `windsurf_normalize_maps_tool_info_range_vocabularies`・
  `windsurf_normalize_pattern_gate_routes_search_or_fileread`・
  `normalize_search_path_fallback_order`)
- TSPEC pin 検証: copilot `view_range` の `-1` sentinel・`0`・矛盾値
  の non-Valid→`None` 写像、空 `pattern` の fail-closed(FileRead
  維持)、矛盾 pair/array の helper 全体 `None` short-circuit
  (`{start_line:500,end_line:20}` が `{500,None}` にならない)、
  `read_config_from` suppression — 全て unit + integration test で
  pin 済み
- regression: `CHEAPER_REGRESSION_CAPTURE=1` 再 capture 後の
  `regression_golden` 再実行で pass(diff は windsurf deny への
  guidance 1 行のみ)
- `node scripts/check-doc-ids.mjs`: pass(1109 files scanned、T5 実測)
- `mise run check`: 全 step exit 0(biome・`cargo fmt --check`・
  clippy・version/doc-id check・`cargo test --workspace` 全 target
  0 failed・`node --test`・lockfile/apm-payload check)

証跡 commit: `768305a`(discovery resolved)→ `a8b68c9`(spec)→
`bad68b0`(ADR)→ `44b97e4`(design)→ `314668f`(design review
P1/P2 反映)→ `c616dae`(spec/ADR/design 承認)→ `5531a2a`(plan)→
`417f3d8`(test-spec)→ `5a775d7`(task breakdown)→ `f2b3f74`(T1)→
`8d6d9b4`(T2)→ `a9b195a`(T3)→ `1ab09ff`(T4)→ `18ba2db`(T5)。

## Risks

- 実 Devin compat wire での確認は未実施(3000.11.92 未観測 —
  consumer 側 probe が必要)。拡張 alias が実際に送られない場合は
  dead code 化し得るが、共有 normalize との parity 自体に価値が
  あるため spec/design で許容済み。
- findings・deny guidance は実 Windsurf install の正当な
  `.windsurf/hooks.json` 登録にも emit され得る — 文言は
  conditional 表現 + 「実 Windsurf install では削除しない」hedge
  付きで実害は軽微(design の環境識別未確認に起因)。
- windsurf conflict 検出は `KNOWN_LEGACY_FILES` の project root
  直下 literal join + 大小文字区別 suffix 比較の限界を継承する
  (design Deferred Design Concerns で記録済み)。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #161)由来の未分類・未処理の follow-up はない。下記は
全て spec「Deferred Design Concerns」・design 同節・test-spec
「Non-goals」・各 task「Out of Scope」で scope 外確定・記録済みの
defer/non-goal であり、隣接 verification は chain 外の issue
(#162 等)で追跡済みのため新規 chain は起こさない。

- **bounded shell read 評価**(`cat|head|tail`・`tail -n`・pipe 下流
  filter の bounding 解釈)— compat 固有でなく native 経路でも同
  verdict で別 feature 相当。再開契機: shell read bounding の需要
  報告。
- **Devin 3000.11.92 の compat wire shape 実測** — 拡張 alias が
  実際に送られるかの consumer 側 wire probe。cheaper 制御外のため
  本 chain の作業ではない。
- **`.windsurf/hooks.json` suffix の大小文字非区別化・user scope/
  親 dir の legacy scan** — `KNOWN_LEGACY_FILES` project root 直下
  限定の既知検出限界(design Deferred Design Concerns「windsurf
  conflict 検出の限界」)。検出範囲拡大が必要になった時点で扱う。
- **`read_config_from` 抑止判定が `config.local.json` を参照しない
  点** — verify は main config の `loaded` のみを判定に使う。同上の
  検出 scope 限界系で、scope 外確定。
- **compat firing の環境識別**(`DEVIN_PROJECT_DIR` 等 env marker が
  compat firing にも立つか)— 確定すれば deny 案内を Devin context
  に絞れる。再開契機: wire/env 観測の取得。
