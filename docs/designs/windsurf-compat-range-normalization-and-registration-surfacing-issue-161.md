---
id: "DESIGN-034KwO2y86CrXOmaoae1jS"
type: "design"
status: "approved"
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
        path: "crates/cheaper-cli/tests/regression/expected/cli-hook-windsurf-deny.json"
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
- `pre_read_code` `tool_info` の `pattern`（非空 string）検出による
  `RoutingRequest::Search` 正規化 — `file_path` が file を指す
  payload では FileRead 評価を優先する gate 付き（後述）。
  `normalize_search` の `path` 解決へ `file_path` fallback を追加。
- `handle_hook` の windsurf arm で `FileRead{range:None}` Deny へ
  remediation 付記（eval 層・dedup record は不変）。
- `hook_config.rs` `run_verify` の compat-degraded finding 追加・
  `windsurf-conflict` finding（present+conflict の trap 残存）追加・
  `legacy-conflict` の dedup 文言訂正。
- `docs/guides/distribution.md` の Devin native hook 登録節への
  `tool_info` field contract・dual-registration trap・compat 限界の
  記載。

### Out of Scope

- eval 本体の変更 — `evaluate`・`evaluate_file_read`・
  `evaluate_ranged_read`・`evaluate_search`・`evaluate_read_candidates`
  （`routing_policy.rs:537-709,970-1032,1530-1595`）、`DEFAULT_THRESHOLD`、
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
  gate、orphan sweep、`absent-config` 等の既存 finding 文言
  （`legacy-conflict` の dedup 文言訂正のみ例外として行う —
  「compat-degraded / windsurf-conflict verify finding」節参照）。
- AGENTS.md（repo-level 規約の変更は発生しない — hook contract は
  guides/designs の責務）。

## Components and Boundaries

### `hook::targets` — `windsurf_normalize` の `pre_read_code` 拡張

`targets.rs:162-177` の `pre_read_code` 分岐を次の順序で評価する:

1. **`pattern` → `Search`（kind 判定を最優先、ただし 2 条件の gate 付き）**
   `tool_info` object が `pattern` を **非空 string**（`as_str` 成功
   かつ `!is_empty()`）として持ち、かつ `file_path` が
   (i) 欠落・空文字・非 string、または (ii) `cwd` 相対で解決した先が
   **directory** である場合に限り、`normalize_search(tool_info, cwd,
   Some("windsurf:pre_read_code"))` で `RoutingRequest::Search` を返す。
   この gate は spec R3 の不変条件（無効・曖昧な入力が過少評価へ
   縮退しない）を `pattern` 経路にも適用するもので、normalize 段で
   次の 2 経路を塞ぐ:
   - **空 `pattern` の fail-open** — `evaluate` は
     `pattern.is_empty()` を無条件 `Allow` とする
     （`routing_policy.rs:1575-1577`）。`pattern` を string として
     持つだけの read payload を無条件に Search へ倒すと、実 read が
     未評価のまま allow される。
   - **file を指す `file_path` との併記による過少評価** —
     `evaluate_search`（:970-1032）の file 走査は一致行 ⊆ file 内容の
     実体化であり、`file_path` が file を指す payload（tool 実挙動が
     全文 read の場合を含む）を Search へ倒すと threshold 比較を
     すり抜ける。file を指す `file_path` は「全文 read が最悪ケース」
     として FileRead eval を優先する。
   directory 判定は `nodepath::resolve(&cwd, file_path)` の結果への
   `std::fs::metadata().is_dir()` で行う — `resolve_target`
   （`routing_policy.rs:316-350`）と同一の「cwd 相対 resolve →
   metadata 分類」規約に揃えるが、normalize 層では workspace
   containment は見ない（`OutOfScope` の allow 判定は eval 側の
   責務）。metadata 失敗・file 解決は directory 非該当として
   FileRead 判定へ進む。この fs 参照は `pattern` 非空かつ
   `file_path` 非空の場合にのみ走り、normalize 層への fs アクセス
   追加はその 1 回に限定される。
   `pattern` が非 string・空 string の場合はこの経路を取らず
   FileRead 判定へ進む（`normalize_search` の `as_str` gate と同一
   条件でガードし、None 返しの分岐を残さない）。
   この gate は spec R4 の字句（`pattern` を string として持てば
   `Search`）の発火条件を絞る design 側の修正であり、R3（無効・
   曖昧な入力が過少評価へ縮退しない）を優先する — `pattern` の
   存在だけでは payload が search 意図とは確定しないため、file を
   指す `file_path` 併記時はより広い FileRead 評価を採る方向で
   両者を整合させる。
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
  - `start_line`・`end_line` が**共に** `json_line_number` で受理できる
    整数なら complete pair として確定し、`end_line < start_line` なら
    **helper 全体が即座に `None` を返す**（`copilot_view_range`
    :419-433 と同じ矛盾扱い）。後続の `view_range`/`end_line` 単独
    判定にも Step C fallback にも進まない — 例えば
    `{start_line:500, end_line:20}` を Step C の lone `start_line`
    規約で `{offset:500, limit:None}`（500 行〜EOF）へ倒すと
    emitter の意図範囲より狭い過少評価になり得るため、矛盾 pair は
    全文評価へ倒す。`end_line >= start_line` なら
    `ReadRange{offset: start_line, limit: end_line - start_line + 1}`
    （1-based・inclusive）。
  - `start_line`+`end_line` が complete pair を構成しない（片方の
    欠落・受理不能）場合のみ `view_range` を評価する。`view_range`
    が array `[start, end]` で `start` が `json_line_number` 受理、
    `end` も `json_line_number` 受理なら同規約で
    `ReadRange{offset: start, limit: end-start+1}`、`end < start`
    なら同上の理由で **helper 全体が `None`**。例外として
    `end == -1` は `[start, -1]` を
    `ReadRange{offset: start, limit: None}`（start 行〜EOF）へ写像
    する — `view_range` 系で `-1` は "to EOF" sentinel の慣例値で
    あり、`limit:None` は lone `start_line` → `{s,None}` と同じ
    「起点以降すべて」を意味し過少評価にならない（`start` 未満の行を
    落とすが、"from start" 読みの tool 実出力もその行を含まない）。
    `end < -1` や `start` 側の 0/負値/非整数は `json_line_number`
    不受理のまま（complete array 不成立 → 次の判定へ進む）。
    実装は `copilot_view_range` を共用 helper（`normalize_view_range`
    へ rename）として copilot 経路と共有する — 同一規約の変換処理を
    一箇所へ集約する。`-1` sentinel 受理は共有化に伴い copilot
    `view` にも効く（copilot wire では `-1` 未観測だが、同一 helper
    の規約として許容する — `to EOF` 解釈が copilot にも通用する）。
  - `end_line` 単独（`start_line` 欠落・無効値）で受理できる整数なら
    `ReadRange{offset: 1, limit: end_line}`。
  - pair 判定順は `start_line`+`end_line` → `view_range` → `end_line`
    単独。`start_line` が無効で `end_line` が有効なら `end_line` 単独
    として `{offset:1, limit:end_line}`（より広い評価=安全側）へ
    縮退する。
- **Step C — fallback**: 上記いずれも不成立なら（complete pair/array
  が一度も確定しなかった場合に限り — 矛盾による short-circuit
  `None` はここへ落ちない）`normalize_read_range`（Step A と同一
  key 集合）を呼ぶ — `start_line` 単独で `{offset:s, limit:None}`
  （s 行〜EOF）を受理する経路。`None` は全文評価縮退を意味する。

いかなる経路でも**過少評価に縮退しない**: 無効・矛盾・非整数値は
`None`（全文評価）またはより広い window へ倒す。ただしこの不変
条件の厳密な対象は**単一語彙の入力**に限る — `offset`/`limit` 系
alias と pair/array 形式が混在する payload は上記 precedence で
deterministic に一方の語彙を採用するため、採用されなかった語彙の
解釈が表す範囲より狭い評価になり得る（例: `{offset:10, limit:5,
view_range:[1,1000]}` は `{10,5}` に確定する）。混在 emitter の意図は
識別不能であり、precedence による deterministic 解決自体が仕様 —
invariant が保証するのは「同一語彙内の無効・矛盾値が `None`/より
広い window 以外へ縮退しないこと」と、「`pattern` gate により file
を指す payload が Search の matched-lines 評価へ過少に倒れない
こと」に限定される。

#### `normalize_search` の `path` fallback（共有 normalize の拡張）

`targets.rs:297-348` の `path` 解決を、`path` key 欠落時に
`file_path`（非空 string）を search root として使う規約へ拡張する。
`path` が存在する場合は従来どおり `path` が優先（非 string は `"."`）。
`file_path` が空文字・非 string・欠落なら `"."`。この拡張は共有
`tool_use_normalize`（devin/pretooluse の `grep`）にも効くが、両者の
grep contract は `path` であり `file_path` を送る wire は未観測。
なお fallback による search root の縮小（`"."` の workspace 全体より
狭い root）は **`file_path` が ambient context（tool の意図する検索
root ではなく付随して載る file 参照）の場合に unsafe 方向** —
実返却範囲より狭い評価になり得るため、無条件に「安全側」とは
言えない。windsurf 側では前記 gate が `pattern` + file を指す
`file_path` の併記を FileRead 評価へ優先させるため、この fallback
が効くのは `file_path` が dir・存在しない・欠落の場合に限定される
（dir は検索 root として正当な意味を持つ）。devin/pretooluse の
grep で `file_path` が届く未観測 wire では、file scope 検索としての
意味が通る一方、ambient context であった場合の過少評価は残余
リスクとして残る（Risks 参照）。

`pre_run_command`（`tool_info.command_line` → `Shell`）は一切変更
しない。

### `hook::targets` — windsurf FileRead deny への remediation 付記

付記は `handle_hook` の windsurf dispatch arm（`targets.rs:780-786`）
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

### `hook::hook_config` — compat-degraded / windsurf-conflict verify finding

`run_verify`（`hook_config.rs:591-752`）に、compat 縮退状態を説明する
informational finding を 2 種追加する。`ownEntry` の状態
（`own_state`、:625-670）と `legacy_conflicts` 結果（:673-685）の
両方が確定した後（`legacyConflicts` insert 直後）に評価する。

**windsurf 系登録の識別**: `conflicts` の path（separator を `/` へ
normalize して比較）が `.windsurf/hooks.json` で終わるものを windsurf
経路とみなす（`KNOWN_LEGACY_FILES` :42-49 の中で
`.windsurf/hooks.json` のみが windsurf 経路であることに依拠 —
`legacy_conflicts` が path 文字列のみを返す現契約を保ち、構造化への
変更は行わない）。

**import 無効化の検出**: loaded config（`loaded` が `Some` の場合のみ
判定可能）の `root.read_config_from.windsurf` が明示の JSON `false`
なら、windsurf import が無効化されており `.windsurf/hooks.json` の
登録は Devin へ発火しない — degradation を主張する根拠（Devin が
当該登録を consume し得る）が消えるため、両 finding とも emit しない
（実 Windsurf 専用の登録であり、既存 `legacy-conflict:` finding が
引き続き residual を報告する）。`read_config_from` key 欠落・非
object・`windsurf` が `false` 以外の値は全て「無効化されていない」
として扱う — key shape が一次情報未確定のため `false` のみを抑止
条件とする保守判定で、shape 不確定時に finding を黙らせる方向へは
倒さない。`loaded` が `None`（config 不在・読取不能）では判定不能
として同様に「無効化なし」扱いとする。

**finding 1 — `compat-degraded`（informational）**:
`own_state` が `"absent"` または `"missing"`、かつ windsurf conflict
があり、かつ import が無効化されていない場合に emit:

```text
compat-degraded: a legacy `.windsurf/hooks.json` cheaper registration can
still fire into Devin while no native Devin registration could be
confirmed — bounded evaluation does not engage through the compat path
because range/search fields may be dropped before reaching the hook (if
this workspace uses Devin CLI: run `cheaper hook-config sync` to register
the native entry, then remove the legacy `.windsurf/hooks.json`
registration or disable the windsurf import via `read_config_from`; if
the file serves a real Windsurf install it is the legitimate native
registration for it — do not remove it on that account)
```

文言上の hedge を 2 点固定する:

- `own_state == "absent"` は config 不在だけでなく読取不能
  （`config-unreadable:` blocker finding と併存するケース）も含む
  ため、「native registration が存在しない」ではなく「確認できない」
  （could not be confirmed）と表現する。
- `.windsurf/hooks.json` は実 Windsurf consumer にとって
  `packages/apm-hook-windsurf` が配布する正規の native 登録であり、
  remediation は Devin 利用を条件とする表現 + 非 Devin 環境では
  削除しないことを明記する（実 Windsurf 単独利用では本 finding の
  前提自体が成り立たない可能性がある点を conditional wording で
  吸収する — import 未無効の判定が取れても Devin の利用有無は
  verify からは判別不能なため、文言側で実害を限定する）。

informational とし `blocker`/`clean`/exit code に影響しない —
absent/missing 状態では既存 `absent-config:`/`missing:` finding が
既に blocker で exit 2 のため、本 finding は説明追加のみ。

**finding 2 — `windsurf-conflict`（informational）**:
`own_state` が `"absent"`/`"missing"` 以外（native entry が存在する
状態 = `present`/`drifted`/`modified`/`duplicate`）、かつ windsurf
conflict、かつ import 無効化でない場合に emit:

```text
windsurf-conflict: the legacy `.windsurf/hooks.json` cheaper registration
still fires alongside the native entry — compat firings are not
deduplicated against native firings because the request fingerprint
includes `range`, so a range-dropping compat deny still wins while the
windsurf import is active (remove the legacy `.windsurf/hooks.json`
registration or disable the windsurf import via `read_config_from`; keep
it only if it serves a real Windsurf install)
```

これは remediation loop の false-green を塞ぐもの —
`hook-config sync` で native 登録を足しても legacy を残す限り
`compat-degraded` は消えるが dual-registration trap
（`eval_cache.rs:436-438` の fingerprint が `range` を含むため、
range 無し compat firing と range 有り native firing は dedup
されず、verdict union で compat deny が勝つ）は存続する。本 finding
が trap の存続を報告し続ける。severity は informational とする —
dual-registration trap は登録状態の破損ではなく bounded eval の
未発動条件であり、blocker にすると `post-install`/`post-update`
lifecycle の verify code 透過（conflict 残余を exit 0 で返す既存
contract、`legacy-conflict` の informational 規約）を破壊する。
R6 の字句（absent/missing 時の finding）を超える追加だが、exit
code 規約を変えない informational finding であり design 側の
拡張として採用する。

**`legacy-conflict` 文言の訂正**（実装 scope に含める）:
`hook_config.rs:674-681` の「duplicate firings are deduplicated at
eval time for identical requests」は、range を落とす compat firing
に対しては偽 — fingerprint が `range` を含むため range-diverged した
firing は dedup されない。文言を「dedup は同一 canonical request
（fingerprint が一致する firing）に限り発動し、range field を落とす
compat 経由の firing は dedup 対象外」方向へ訂正し、dual-
registration trap の説明は `windsurf-conflict` finding と
distribution.md 側へ集約する（発火条件・informational 規約は不変）。

### 文書化 — `docs/guides/distribution.md`

「Devin CLI の native hook 登録（`cheaper hook-config`）」節
（:110-207、bounded eval 段落 :146-149 の直後）に以下を追記する:

- **windsurf `tool_info` field contract** — cheaper が `pre_read_code`
  の `tool_info` から読む field 集合: `file_path`（FileRead 経路では
  必須、`pattern` が非空の Search 経路では dir 解決時に search root
  として使う）、range 系
  `offset`/`line_offset`/`start_line`・`limit`/`line_limit`/`max_lines`・
  `start_line`+`end_line` pair・`view_range` array・`end_line` 単独、
  `pattern`（非空かつ `file_path` が file 非指摘のとき `Search`
  評価）、`path`/`max_results`/`context`/
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

`verify` の `result.findings` に `compat-degraded:`・
`windsurf-conflict:` が加わり、`legacy-conflict:` の dedup 文言が
訂正される出力 contract の変更も同節で言及する。

## Data and Control Flow

### pre_read_code 正規化（拡張後）

```
tool_info object?
  ├─ pattern が非空 string かつ
  │   file_path が欠落/空/非 string または dir 解決
  │    → normalize_search → RoutingRequest::Search
  │    （path = tool_info.path ?? file_path(非空) ?? "."）
  └─ file_path 非空 string → RoutingRequest::FileRead
       range = windsurf_read_range(tool_info):
         ├─ explicit key（offset|line_offset|limit|line_limit|max_lines）
         │    → normalize_read_range(extended aliases)
         ├─ start_line+end_line 有効 → {s, e-s+1} / 矛盾 → helper 全体 None
         ├─ view_range [s,e] 有効 → {s, e-s+1} / e=-1 → {s,None}
         │    / 矛盾 → helper 全体 None
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
  windsurf_conflict = conflicts ∋ *.windsurf/hooks.json
  import_disabled = loaded?.read_config_from.windsurf == false
  windsurf_conflict && !import_disabled のとき:
    own_state ∈ {absent, missing}
      → findings += "compat-degraded: ..."（informational、exit 不変）
    own_state ∉ {absent, missing}
      → findings += "windsurf-conflict: ..."（informational、exit 不変）
```

## Non-Functional Constraints

- **fail-closed 維持**: すべての新規受理経路で曖昧・無効・矛盾する
  範囲表現は `range: None`（全文評価）またはより広い window へ倒す。
  過少評価（tool が返す範囲より狭い評価で allow）に繋がる経路は
  作らない — ただしこの保証は単一語彙の入力に対するもので、混在
  語彙 payload は documented precedence による deterministic 解決と
  なる（`windsurf_read_range` 節末尾の限定を参照）。
- **eval/dedup 層への波及ゼロ**: `routing_policy.rs`・`eval_cache.rs`
  の変更は無い。fingerprint・verdict record・threshold・cap の wire
  contract は不変。
- **deny reason の additive 変更のみ**: guidance は既存 reason 末尾へ
  1 行追加し、deny 本体の文言は変更しない。prefix/contains 系 assert
  （`starts_with`・tokens/bulk-read 含有）はそのまま追従するが、
  末尾追加で破損する検証が 2 箇所あり更新対象とする:
  `tests/integration/hook.rs:300` の
  `deny.stderr.ends_with("workspace-relative path: big.txt\n")`
  （guidance 行が末尾へ付くため `ends_with` は成立しなくなる →
  `contains` 系へ変更）、および `tests/regression/expected/
  cli-hook-windsurf-deny.json` の byte-exact stderr snapshot
  （case 定義 `tests/regression/cases/cli-hook-windsurf-deny.json`
  が `compareStderr:true`・`mode:"raw"` で stderr を pin しており、
  `CHEAPER_REGRESSION_CAPTURE=1` で再 capture する）。
- **verify 出力は findings の追加が中心**: `compat-degraded:`・
  `windsurf-conflict:` の 2 件が既存の文字列 array に加わるだけで、
  `clean`・`remediation`・exit code 規約（finding 種別 → code）は
  不変。両 finding は informational で blocker にしない。例外として
  `legacy-conflict:` の dedup 文言は誤りのため文言自体を訂正する
  （発火条件・exit code への影響は不変）。
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
  通り固定する。混在ケースは単一語彙の「より広い評価」ではなく
  documented precedence による deterministic 解決であり、採用側以外の
  語彙が表す範囲より狭い評価になり得る点は受理済みの仕様として
  `windsurf_read_range` 節末尾に明記した。
- `pattern`→`Search` は `pre_read_code` に grep metadata を載せる
  compat emitter が存在する前提の防御 — 現行 Devin（`file_path` のみ）
  では発火しない dead path になり得るが、共有 normalize との parity
  自体が目的。gate（非空 `pattern` + `file_path` が file 非指摘）
  により、read payload に `pattern` key が誤って混ざった場合でも
  `file_path` が file を指す限り FileRead 全文評価へ優先されるため
  過少評価にはならない。残余: gate の dir 判定は normalize 時点の
  metadata 1 回のみで、eval 走査までに dir→file へ変わる TOCTOU
  race では Search が file 走査（一致行 ⊆ 内容）へ倒れ得る —
  window は firing〜scan の間の狭い区間で、発生しても scan cap・
  pattern 評価は走るため完全な未評価にはならない残余として受理する。
- guidance 行が実 Windsurf の range 無し deny にも載る（条件付き表現
  で軽微）。compat firing の確定的識別（env marker 等）は wire/env
  観測が取れ次第の絞り込み改善として残す。
- `compat-degraded`/`windsurf-conflict` finding は windsurf conflict と
  import 状態から compat 経路の可能性を推論する間接検出 — Devin が
  実際に `.windsurf/hooks.json` を読むか（version skew・import 機構の
  実装差）は一次情報未確定のため、文言は conditional 表現とし
  informational に留める。`read_config_from.windsurf == false` の明示
  無効化がある場合は emit しない。
- `file_path`→`path` fallback が共有 `normalize_search` を通るため
  devin/pretooluse の grep normalize にも効く — `path` 優先で既存
  contract は壊さないが、root 縮小は ambient context としての
  `file_path` に対して unsafe 方向であり無条件の「安全側」ではない
  （前述の通り）。windsurf 側は gate で限定済み、devin/pretooluse 側は
  `file_path` を送る wire 未観測の防御 residual として残す。

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
- **windsurf conflict 検出の限界** | `legacy_conflicts` は
  `KNOWN_LEGACY_FILES` を project root（`root_dir`）直下へ literal
  join して走査するため、`.windsurf/hooks.json` の suffix 比較は大小
  文字を区別し（`.Windsurf/`・`HOOKS.JSON` 等の大小文字差のある
  path は検出しない）、かつ project root 直下のみが対象（user scope
  config・親 dir・nested dir の同名 file は走査対象外）。両 finding
  もこの限界を継承する。既知の制限として記録し、検出範囲の拡大が
  必要になった時点で扱う。

## Implementation Plan

1. `targets.rs`: `normalize_view_range` へ rename（copilot 共用化、
   `-1` sentinel 規約含む）、`windsurf_read_range` helper 追加、
   `windsurf_normalize` の `pre_read_code` 分岐で gate 付き
   `pattern`→`Search`（非空 `pattern` かつ `file_path` 欠落/空/
   dir 解決）+ range helper 接続、`normalize_search` の
   `file_path`→`path` fallback。
2. `targets.rs`: `WINDSURF_RANGE_DROP_GUIDANCE` 定数 +
   `windsurf_range_deny_guidance` + windsurf dispatch arm 接続。
3. `hook_config.rs`: `run_verify` の compat-degraded +
   `windsurf-conflict` finding 追加（`read_config_from.windsurf`
   抑止判定含む）、`legacy-conflict` の dedup 文言訂正
   （`hook_config.rs:674-681`）。
4. `tests/integration/hook.rs`（windsurf fixture 拡張 +
   `ends_with` assert の `contains` 系への修正）+
   `hook_config.rs`（finding fixture）+ `targets.rs` unit tests +
   `tests/regression/expected/cli-hook-windsurf-deny.json` の
   `CHEAPER_REGRESSION_CAPTURE=1` 再 capture。
5. `docs/guides/distribution.md` 追記。

## Verification Notes

- [ ] `targets.rs` unit test で `windsurf_read_range` の
  alias/pair/array/end_line/explicit/fallback 経路・矛盾 pair の
  helper 全体 `None` 縮退（`{start_line:500, end_line:20}` が
  `{500,None}` にならないこと）・`view_range` `-1` sentinel 写像、
  `windsurf_normalize` の `pattern`→`Search` 写像と gate（空
  `pattern`・`file_path`→file 併記で FileRead 優先、dir/欠落で
  Search）を table-driven に検証。`mod tests` へ追加する —
  `windsurf_normalize` と range/search 系 helper は現行 unit test を
  持たない（`mod tests` の既存 coverage は render 系と
  `cursor_normalize` の nested `tool_input` 判定のみ）ため、判定表が
  増える本変更では最低層で pin する。
- [ ] `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`
  — `write_big_lines` fixture で `start_line`/`end_line`/`view_range`/
  `max_lines` 各 payload が bounded allow、`pattern`+`file_path` 欠落
  （または dir）payload が Search 経路で allow/deny、`pattern`+
  `file_path`（file 解決）併記・空 `pattern` 併記 payload が FileRead
  評価を維持すること、`file_path` のみ payload の deny stderr に
  `hook-config verify` 誘導、devin/pretooluse deny には誘導行が
  載らないことを検証。`hook.rs:300` の
  `deny.stderr.ends_with("workspace-relative path: big.txt\n")` は
  guidance 末尾追加で破損するため `contains` 系 assert へ修正する。
- [ ] `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::`
  — absent+`.windsurf` conflict → compat-degraded finding+exit 2、
  missing（空 config）+conflict → 同 finding、missing+conflict+
  `read_config_from.windsurf=false` → 両 finding 無し、
  present+conflict → `windsurf-conflict` finding（exit 0・
  `clean:true`）、`.claude` のみ conflict+absent → 両 finding 無し、
  `legacy-conflict` の訂正後文言を検証。
- [ ] `CHEAPER_REGRESSION_CAPTURE=1` で
  `tests/regression/expected/cli-hook-windsurf-deny.json` を再 capture
  し diff を review（windsurf deny stderr は `mode:"raw"` +
  `compareStderr:true` で byte-exact pin されており、guidance 行追加で
  破損する — `cargo test -p cheaper-cli --test regression`）。
- [ ] 既存 `hook::`（windsurf deny 文言・bounded/縮退・search/shell、
  dedup）・`hook_config::`・`hook_eval_dedup::` が非回帰。
- [ ] `mise run check` が pass。
