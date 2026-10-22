---
id: "DESIGN-034KwO2y86CrXOmaoae1jS"
type: "design"
status: "draft"
title: "Windsurf-compat range normalization and registration surfacing (issue #161)"
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
        path: "docs/guides/distribution.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "../adr/defensive-compat-normalization-with-registration-surfacing.md"
    - "../discovery/devin-compat-range-fields-dropped-read-code-deny.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-cli/tests/integration/hook.rs"
    - "../../crates/cheaper-cli/tests/integration/hook_config.rs"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "../discovery/bounded-hook-read-request-contract.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../guides/distribution.md"
  defers: []
  deferred-by: []
---

# Windsurf-compat range normalization and registration surfacing (issue #161)

## Context

issue #161（#107 再発）。Devin CLI 環境で `.devin/config.json` の native
登録が無いまま `.windsurf/hooks.json` の legacy 登録（`pre-read-code`
`--target windsurf`）経由で発火すると、Devin の windsurf-compat 変換が
`tool_info` に `file_path` 以外を載せないため（#107 wire 観測、
DISC-034FRguX1hjhDZ7ZMEBnbm で確定。3000.11.92 は未観測）、範囲指定
read・grep が `FileRead{range:None}` に縮退し全文 token 数で deny
される。metadata 消失地点は Devin 側 compat 変換で cheaper 制御外。

DISC-t3TdC7pUJUGdYgfkpkmhMU が確定した cheaper 側の gap は 2 点:

1. `windsurf_normalize`（`crates/cheaper-cli/src/hook/targets.rs:155-189`）
   の `ReadRange` alias coverage が共有 `tool_use_normalize`（:456-495）
   より狭い — `start_line`/`end_line`/`view_range`/`max_lines` 不達、
   `pattern`→`Search` 経路無し。
2. compat-degraded な登録状態（native own entry 不在 + windsurf 系
   legacy 登録残存）が deny 時点・`hook-config verify` 出力のどこにも
   surface されない。

本 design は SPEC-034KwuiFzl4WvVQKdjfSnM の要求を
ADR-034KwuE0Xnp3O6EUXacFG8 で選択された Option B（防御的正規化 +
registration surfacing）として実装する。eval 層（`routing_policy` の
評価・threshold・dedup fingerprint）は #107 で正常確認済みのため
一切変更しない。

## Scope

### In Scope

- `windsurf_normalize` の `pre_read_code` 経路の `ReadRange` 抽出
  coverage 拡張（`targets.rs` のみ変更）。
- `pre_read_code` `tool_info` の `pattern`（string）検出による
  `RoutingRequest::Search` 正規化。`normalize_search` の `path`
  解決へ `file_path` fallback を追加。
- `handle_hook` の windsurf arm で `FileRead{range:None}` Deny へ
  remediation 付記（eval 層・dedup record は不変）。
- `hook_config.rs` `run_verify` の compat-degraded finding 追加。
- `docs/guides/distribution.md` の Devin native hook 登録節への
  `tool_info` field contract・dual-registration trap・compat 限界の
  記載。

### Out of Scope

- eval 本体の変更 — `evaluate`・`evaluate_file_read`・
  `evaluate_ranged_read`・`evaluate_search`・`evaluate_read_candidates`
  （`routing_policy.rs:537-709,1530-1595`）、`DEFAULT_THRESHOLD`、
  byte fast path、`count_max_bytes` cap、`SEARCH_SCAN_MAX_FILES`。
- dedup fingerprint 契約（`eval_cache.rs:294-464`）— `range` を含む
  `VariantInputs`・`source_tool`/`target` を含めない規則は不変。
- bounded shell read 評価（`head`/`tail -n`・pipe filter の bounding
  解釈）— compat 固有でなく native 経路でも同 verdict であることが
  discovery で確定済み。別 feature として defer。
- Devin CLI 本体の compat 変換修正、Devin 3000.11.92 の wire shape
  実測（consumer 側 probe が必要）。
- legacy 登録 file（`.windsurf/hooks.json` 等）の自動除去・
  `read_config_from` の自動変更 — own-entry-only 所有境界
  （ADR-034H2WBNtC4We8dyC5lQZJ）を維持。
- 登録済みを根拠にした allow defer（ADR Option C — fail-open のため
  棄却済み）。
- `sync`/`remove`/`--event-stdin` の動作、own-group 検証、capability
  gate、orphan sweep、`legacy-conflict`/`absent-config` 等の既存
  finding 文言。
- AGENTS.md（repo-level 規約の変更は発生しない — hook contract は
  guides/designs の責務）。

## Components and Boundaries

### `hook::targets` — `windsurf_normalize` の `pre_read_code` 拡張

`targets.rs:162-176` の `pre_read_code` 分岐を次の順序で評価する:

1. **`pattern` → `Search`（kind 判定を最優先）**
   `tool_info` object が `pattern` を **string**（`as_str` 成功、空文字
   可）として持つ場合、`normalize_search(tool_info, cwd,
   Some("windsurf:pre_read_code"))` で `RoutingRequest::Search` を返す。
   `file_path` はこの経路では必須としない — `path` 解決は
   `normalize_search` 側の規約（下記 fallback 拡張含む）に従う。
   `pattern` が非 string ならこの経路は取らず FileRead 判定へ進む
   （`normalize_search` の `as_str` gate と同一条件でガードし、
   None 返しの分岐を残さない）。
2. **`file_path` → `FileRead`**
   現行どおり非空 `file_path` string を必須とし、range を新ヘルパー
   `windsurf_read_range(tool_info)` で抽出する。

#### `windsurf_read_range(tool_info) -> Option<ReadRange>`（新規 helper）

受理規則を次の優先順位で固定する（spec 「明示 offset/limit aliases
→ pair/array 形式 → `end_line` 単独」）:

- **Step A — 明示 alias 経路**: `offset`/`line_offset`/`limit`/
  `line_limit`/`max_lines` のいずれかの key が `tool_info` に存在する
  場合、`normalize_read_range(tool_info, &["offset","line_offset",
  "start_line"], &["limit","line_limit","max_lines"])` をそのまま返す。
  `start_line` は offset 系 alias としてここで受理される（共有
  `tool_use_normalize` :477-481 と同一 key 集合）。explicit key が
  1 つでもあれば pair/array/end_line 形式は参照しない（曖昧な混在
  payload は明示語彙側へ倒し、`offset==1 && limit==None` の畳み込みや
  無効値の `None` 縮退=全文評価は既存 `normalize_read_range` 規約の
  まま）。
- **Step B — inclusive pair/array 形式**（Step A の key が 1 つも
  無い場合のみ評価）:
  - `start_line`・`end_line` が共に `json_line_number` で受理できる
    整数なら `end_line < start_line` → `None`（矛盾 range は全文評価へ
    倒す、`copilot_view_range` :419-433 と同じ扱い）、それ以外は
    `ReadRange{offset: start_line, limit: end_line - start_line + 1}`
    （1-based・inclusive）。
  - `view_range` が array `[start, end]` で両要素が `json_line_number`
    受理なら同規約で `ReadRange{offset: start, limit: end-start+1}`、
    `end < start` → `None`。実装は `copilot_view_range` を共用 helper
    （`normalize_view_range` へ rename）として copilot 経路と共有する —
    同一規約の変換処理を一箇所へ集約する。
  - `end_line` 単独（`start_line` 欠落・無効値）で受理できる整数なら
    `ReadRange{offset: 1, limit: end_line}`。
  - pair 判定順は `start_line`+`end_line` → `view_range` → `end_line`
    単独。`start_line` が無効で `end_line` が有効なら `end_line` 単独
    として `{offset:1, limit:end_line}`（より広い評価=安全側）へ
    縮退する。
- **Step C — fallback**: 上記いずれも不成立なら
  `normalize_read_range`（Step A と同一 key 集合）を呼ぶ —
  `start_line` 単独で `{offset:s, limit:None}`（s 行〜EOF）を受理する
  経路。`None` は全文評価縮退を意味する。

いかなる経路でも**過少評価に縮退しない**: 無効・矛盾・非整数値は
`None`（全文評価）またはより広い window へ倒す。

#### `normalize_search` の `path` fallback（共有 normalize の拡張）

`targets.rs:297-348` の `path` 解決を、`path` key 欠落時に
`file_path`（非空 string）を search root として使う規約へ拡張する。
`path` が存在する場合は従来どおり `path` が優先（非 string は `"."`）。
`file_path` が空文字・非 string・欠落なら `"."`。この拡張は共有
`tool_use_normalize`（devin/pretooluse の `grep`）にも効くが、両者の
grep contract は `path` であり `file_path` を送る wire は未観測 —
仮に届いた場合も search root が狭まる方向（workspace 全体より狭い
bounded 評価）であり安全側。

`pre_run_command`（`tool_info.command_line` → `Shell`）は一切変更
しない。

### `hook::targets` — windsurf FileRead deny への remediation 付記

付記は `handle_hook` の windsurf dispatch arm（`targets.rs:780-785`）
で行う — target 情報を持つのはこの層であり、eval 層
（`routing_policy`）や dedup 層に target 判定を持ち込まない
（ADR の「実装時に最小差分で決める」の決定を dispatch 側付記で
固定）。windsurf arm 内で `evaluate_with_routes` 後に:

```rust
let (verdict, _supply) = evaluate_with_routes(&request, deps).await;
let verdict = windsurf_range_deny_guidance(&request, verdict);
(verdict_name(&verdict), false, windsurf_render(verdict))
```

新規 helper `windsurf_range_deny_guidance(request, verdict)`:

```rust
match (request, verdict) {
    (RoutingRequest::FileRead { range: None, .. }, Verdict::Deny { reason }) =>
        Verdict::Deny { reason: format!("{reason}\n{WINDSURF_RANGE_DROP_GUIDANCE}") },
    (_, v) => v,
}
```

付記は `request` が `FileRead{range:None}` で verdict が `Deny` の
場合のみ — `range:Some` の deny（ranged window 超過）、`Search`/
`Shell` の deny、Allow、Rewrite fallback（FileRead request は route
arm を通らないため Rewrite にならない）、他 target では従来どおり
の reason。

文言（定数 `WINDSURF_RANGE_DROP_GUIDANCE`、reason 文言規約に合わせ
英文・1 行追加）:

```text
If this firing reached the hook through a compatibility registration
(e.g. a legacy `.windsurf/hooks.json` entry under Devin CLI), range
fields may not have been forwarded — run `cheaper hook-config verify`
to inspect the registration state.
```

実 Windsurf 環境の `range:None` deny にも同じ行が載るが、条件付き
表現（"If this firing reached the hook through a compatibility
registration"）で実害は軽微（ADR Consequences 受理済み）。

dedup verdict record（`eval_cache`）には mutate 前の生 verdict が
persist される — guidance は render 直前の firing 毎付記であり、
cache 経由で serve される verdict 文言に混入しない。

### `hook::hook_config` — compat-degraded verify finding

`run_verify`（`hook_config.rs:591-752`）に、compat-degraded 状態を
説明する informational finding を追加する。`ownEntry` の状態
（`own_state`、:625-670）と `legacy_conflicts` 結果（:673-685）の
両方が確定した後（`legacyConflicts` insert 直後）に評価する。

**条件**: `own_state` が `"absent"` または `"missing"`、かつ
`conflicts` の path（separator を `/` へ normalize して比較）が
`.windsurf/hooks.json` で終わるものが存在する。windsurf 系登録の
識別は `KNOWN_LEGACY_FILES`（:42-49）の中で `.windsurf/hooks.json`
のみが windsurf 経路であることに依拠する — `legacy_conflicts` が
path 文字列のみを返す現契約を保ち、構造化への変更は行わない。

**finding 文言**（informational — `blocker`/`clean`/exit code に
影響しない。absent/missing 状態では既存 `absent-config:`/`missing:`
finding が既に blocker で exit 2 のため、本 finding は説明追加のみ）:

```text
compat-degraded: a legacy `.windsurf/hooks.json` cheaper registration can
still fire while no native Devin registration exists — bounded evaluation
does not engage through the compat path because range/search fields may be
dropped before reaching the hook (run `cheaper hook-config sync` to register
the native entry, then remove the legacy `.windsurf/hooks.json` registration
or disable the windsurf import via `read_config_from`)
```

remediation に「native 登録 **だけ** では不十分」を明記する —
dual-registration trap（`eval_cache.rs:436-437` の fingerprint が
`range` を含むため、range 無し compat firing と range 有り native
firing は dedup されず、verdict union で compat deny が勝つ）により、
legacy 除去または windsurf import 無効化まで bounded eval は発動
しない。`own_state` が `present`/`drifted`/`modified`/`duplicate` で
windsurf conflict が残るケース（trap 自体は存続）は spec R6 の境界
により本 finding の対象外 — 検出は既存 `legacy-conflict:` finding が
担い、trap の説明責務は distribution.md 側へ置く。

### 文書化 — `docs/guides/distribution.md`

「Devin CLI の native hook 登録（`cheaper hook-config`）」節
（:110-207、bounded eval 段落 :146-149 の直後）に以下を追記する:

- **windsurf `tool_info` field contract** — cheaper が `pre_read_code`
  の `tool_info` から読む field 集合: `file_path`（必須）、range 系
  `offset`/`line_offset`/`start_line`・`limit`/`line_limit`/`max_lines`・
  `start_line`+`end_line` pair・`view_range` array・`end_line` 単独、
  `pattern`（検出時は `Search` 評価）、`path`/`max_results`/`context`/
  `glob`/`output_mode`/`case_insensitive`（`Search` 写像）、および
  `pre_run_command` の `command_line`。
- **compat 経路の限界** — Devin の native→compat 翻訳では `tool_info`
  が `file_path` のみになり得る（#107 wire 観測、
  DISC-034FRguX1hjhDZ7ZMEBnbm）。範囲 field が届く compat emitter は
  拡張した受理面で bounded eval へ到達するが、`file_path` のみの
  firing は全文 deny で `hook-config verify` 誘導行が付く。
- **dual-registration trap** — native 登録を足しても windsurf 系
  legacy 登録が残る限り compat firing と native firing が併発し、
  fingerprint が `range` を含むため diverge した request は dedup
  されない — deny 側が union で勝つため bounded eval 有効化には
  legacy 登録の除去または `read_config_from` での windsurf import
  無効化が必須。`legacy-conflict:` finding の「共存は安全」は同一
  canonical request の dedup 前提であり、range を落とす compat
  firing には適用されない点を明記する（ADR-034IKJq6tKWWAep0Q0ONzK
  の demotion 根拠との整合）。
- **shell 経由読み取りの deny** — `exec`/`pre_run_command` の read
  candidate 評価は常に全文 token 評価（`evaluate_read_candidates`、
  `routing_policy.rs:1530-1543`）であり compat 固有の症状ではない。

`verify` の `result.findings` に `compat-degraded:` が加わる出力
contract の変更も同節で言及する。

## Data and Control Flow

### pre_read_code 正規化（拡張後）

```
tool_info object?
  ├─ pattern が string → normalize_search → RoutingRequest::Search
  │    （path = tool_info.path ?? file_path(非空) ?? "."）
  └─ file_path 非空 string → RoutingRequest::FileRead
       range = windsurf_read_range(tool_info):
         ├─ explicit key（offset|line_offset|limit|line_limit|max_lines）
         │    → normalize_read_range(extended aliases)
         ├─ start_line+end_line 有効 → {s, e-s+1} / 矛盾 → None
         ├─ view_range [s,e] 有効 → {s, e-s+1} / 矛盾 → None
         ├─ end_line 単独 有効 → {1, e}
         └─ fallback → normalize_read_range（lone start_line → {s,None}）
```

### deny guidance

```
handle_hook → Windsurf arm
  windsurf_normalize → FileRead{range:None}
  evaluate_with_routes → Deny{reason}
  windsurf_range_deny_guidance → reason += "\n" + guidance
  windsurf_render → stderr + exit 2
```

### verify finding

```
run_verify
  load_config → own_state ∈ {absent, missing, present, drifted, ...}
  legacy_conflicts → conflicts[]
  own_state ∈ {absent, missing} && conflicts ∋ *.windsurf/hooks.json
    → findings += "compat-degraded: ..."（informational、exit 不変）
```

## Non-Functional Constraints

- **fail-closed 維持**: すべての新規受理経路で曖昧・無効・矛盾する
  範囲表現は `range: None`（全文評価）またはより広い window へ倒す。
  過少評価（tool が返す範囲より狭い評価で allow）に繋がる経路は
  作らない。
- **eval/dedup 層への波及ゼロ**: `routing_policy.rs`・`eval_cache.rs`
  の変更は無い。fingerprint・verdict record・threshold・cap の wire
  contract は不変。
- **deny reason の additive 変更のみ**: guidance は既存 reason 末尾へ
  1 行追加。reason 文字列を assert する既存 test（`hook.rs` の
  windsurf deny 文言・`evaluate_typed_entry_matches_ts_verdicts` 等）
  は prefix/contains 系で追従可能 — deny 本体の文言は変更しない。
- **verify 出力の additive 変更のみ**: findings は既存の文字列 array
  に 1 件追加されるだけで、`clean`・`remediation`・exit code 規約
  （finding 種別 → code）は不変。新 finding は informational で
  blocker にしない。
- **hook firing 経路で config を読まない**: deny guidance は登録状態
  を firing 毎に判定しない（firing 経路の I/O 追加・registered ≠
  fired の fail-open を避ける）。検出は verify、案内は条件付き文言
  で担う。
- 新規 runtime 依存は入れない。

## Risks and Trade-offs

- `start_line` が offset alias と `start_line`+`end_line` pair の両方の
  意味を持つ（`{start_line:5, end_line:20}` は `{5,16}`、`{start_line:5}`
  単独は `{5,None}`、 `{start_line:5, limit:20}` は `{5,20}`）。
  explicit key 存在チェックで pair 判定を gate する優先順位を上記の
  通り固定し、混在ケースはすべて「より広い評価」側へ倒す。
- `pattern`→`Search` は `pre_read_code` に grep metadata を載せる
  compat emitter が存在する前提の防御 — 現行 Devin（`file_path` のみ）
  では発火しない dead path になり得るが、共有 normalize との parity
  自体が目的で、誤検出（read payload に `pattern` key）の実害は
  `Search` の bounded eval が走るだけで過少評価ではない。
- guidance 行が実 Windsurf の range 無し deny にも載る（条件付き表現
  で軽微）。compat firing の確定的識別（env marker 等）は wire/env
  観測が取れ次第の絞り込み改善として残す。
- `compat-degraded` finding は absent/missing 限定 — present+conflict
  の trap 残存は文書カバレッジで補う（R6 境界を広げると verify の
  finding 意味論が変わるため拡大しない）。
- `file_path`→`path` fallback が共有 `normalize_search` を通るため
  devin/pretooluse の grep normalize にも効く — `path` 優先で既存
  contract を壊さず、`file_path` のみ届く payload の search root が
  `"."` から当該 file/dir へ狭まる（安全側）。

## Deferred Design Concerns

- **bounded shell read 評価** | `head`/`tail -n`・pipe 下流 filter を
  bounding として解釈する shell eval 拡張。compat 機序とは無関係
  （native 経路でも同 verdict）で別 feature 相当。再開契機: shell
  read bounding の需要報告。無視した場合のリスク: `exec` 経由の大
  file 読み取りが引き続き全文 deny になる。
- **compat firing の環境識別** | `DEVIN_PROJECT_DIR` 等の env marker
  が compat firing にも立つか未確認 — 確定すれば deny 案内を Devin
  context に絞り込める。再開契機: wire/env 観測の取得。
- **Devin 3000.11.92 wire shape 実測** | 拡張した alias が実際に
  送られるかの確認。再開契機: consumer 側 wire probe 実施。観測で
  「送らない」が確定しても共有 normalize parity 自体に価値がある
  ため許容。
- **present+conflict 時の compat-degraded surfacing** | own entry
  登録済みでも windsurf legacy 残存で trap が続く状態を verify が
  どう説明するか（`legacy-conflict` 文言の bounded-eval 限定化等）。
  R6 境界の見直しが必要になった時点。

## Implementation Plan

1. `targets.rs`: `normalize_view_range` へ rename（copilot 共用化）、
   `windsurf_read_range` helper 追加、`windsurf_normalize` の
   `pre_read_code` 分岐で `pattern`→`Search` + range helper 接続、
   `normalize_search` の `file_path`→`path` fallback。
2. `targets.rs`: `WINDSURF_RANGE_DROP_GUIDANCE` 定数 +
   `windsurf_range_deny_guidance` + windsurf dispatch arm 接続。
3. `hook_config.rs`: `run_verify` の compat-degraded finding 追加。
4. `tests/integration/hook.rs`（windsurf fixture 拡張）+
   `hook_config.rs`（finding fixture）+ `targets.rs` unit tests。
5. `docs/guides/distribution.md` 追記。

## Verification Notes

- [ ] `targets.rs` unit test で `windsurf_read_range` の
  alias/pair/array/end_line/explicit/fallback 経路と
  `windsurf_normalize` の `pattern`→`Search` 写像を table-driven に
  検証（`mod tests` へ追加 — normalize は現在 unit coverage 外だが
  本変更で判定表が増えるため最低層で pin）。
- [ ] `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`
  — `write_big_lines` fixture で `start_line`/`end_line`/`view_range`/
  `max_lines` 各 payload が bounded allow、`pattern` payload が Search
  経路で allow/deny、`file_path` のみ payload の deny stderr に
  `hook-config verify` 誘導、devin/pretooluse deny には誘導行が
  載らないことを検証。
- [ ] `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::`
  — absent+`.windsurf` conflict → compat-degraded finding+exit 2、
  missing（空 config）+conflict → 同 finding、present+conflict →
  finding 無し、`.claude` のみ conflict+absent → finding 無しを検証。
- [ ] 既存 `hook::`（windsurf deny 文言・bounded/縮退・search/shell、
  dedup）・`hook_config::`・`hook_eval_dedup::` が非回帰。
- [ ] `mise run check` が pass。
