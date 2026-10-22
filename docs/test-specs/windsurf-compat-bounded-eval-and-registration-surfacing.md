---
id: "TSPEC-034KxD0IRkMph8hSZbhTRx"
type: "test-spec"
status: "draft"
title: "Windsurf-compat bounded evaluation coverage and registration surfacing (issue #161)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/161"
  changes:
    added: []
    modified: []
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
  related:
    - "docs/plans/windsurf-compat-range-normalization-and-registration-surfacing.md"
    - "docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
    - "docs/adr/defensive-compat-normalization-with-registration-surfacing.md"
  derived-by: []
  verifies:
    - "docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "docs/designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  defers: []
  deferred-by: []
---

# Windsurf-compat bounded evaluation coverage and registration surfacing (issue #161)

## Purpose

issue #161(#107 再発)で確定した 2 つの cheaper 側 gap —
windsurf adapter の受理面が共有 `tool_use_normalize` より狭いことと、
compat-degraded な登録状態が失敗時点・検出時点に surface されない
こと — に対し、SPEC-034KwuiFzl4WvVQKdjfSnM R1-R8 と
DESIGN-034KwO2y86CrXOmaoae1jS の Option B 実装 contract を pin
する。

保つべき意図は 3 つ。(1) **fail-closed の維持**: 防御的に広げた
受理面でも、無効・曖昧・矛盾する範囲表現は `None`(全文評価)または
より広い window へ倒れ、tool が返す範囲より狭い評価で allow する
過少評価へは決して縮退しない。(2) **compat emitter の非対称を
permanent に拾う**: Devin 側 compat 変換(cheaper 制御外)が今後
どの alias・形式で range/grep field を送っても、共有 normalize と
同じ受理面で救済できる。(3) **縮退は発見可能であること**: bounded
eval が発動しない登録状態は、deny reason・`hook-config verify`
finding・docs の 3 面から remediation へ誘導される — consumer が
「なぜ効かないか」を自力診断できる。

退役条件: Devin CLI の compat 変換が range/search field を安定的に
転送するようになり wire 観測で確定した場合(受理面の Rule は共有
normalize parity として価値を残すが、compat 固有の guidance/
finding は再検討)、あるいは windsurf hook 経路自体が廃止された
場合。

## Feature

windsurf `pre_read_code` compat firing の normalize(range alias/
pair/array・gated `pattern`→`Search`)と、`FileRead{range:None}`
Deny への remediation 付記、`hook-config verify` の
compat-degraded/`windsurf-conflict` informational finding、
`legacy-conflict` の dedup 文言訂正、`docs/guides/distribution.md`
の tool_info contract/dual-registration trap 文書化。

## Rules

- Rule: `tool_info` の offset 系 `offset`/`line_offset`/`start_line`
  と limit 系 `limit`/`line_limit`/`max_lines` は共有
  `tool_use_normalize` と同一 key 集合で受理され、`ReadRange`
  へ写像される(既存 `normalize_read_range` 規約: `offset` 欠落
  +`limit` 有りは `offset=1`、`offset==1 && limit==None` は全文評価
  へ畳む)。
- Rule: `start_line`+`end_line`(両方 `json_line_number` 受理)と
  `view_range` `[start,end]` は 1-based・inclusive 区間として
  `ReadRange{offset:start, limit:end-start+1}` へ写像される。
  `end_line` 単独は `{offset:1, limit:end_line}`、`start_line`
  単独は Step C fallback で `{offset:s, limit:None}`。
- Rule: 受理優先順位は「明示 offset/limit key が 1 つでも存在 →
  Step A `normalize_read_range` のみ → pair/array/end_line を
  参照しない」「`start_line`+`end_line` complete pair →
  `view_range` → `end_line` 単独 → Step C fallback」。混在語彙
  payload は deterministic に一方の語彙を採用する(過少評価不変
  条件の対象は単一語彙入力に限定)。
- Rule: complete pair(`end_line < start_line`)・complete
  `view_range`(`end < start`、ただし `end == -1` 以外)の矛盾は
  `windsurf_read_range` helper 全体が即座に `None` を返す
  short-circuit であり、後続の `view_range`/`end_line` 単独判定・
  Step C fallback へは進まない — `{start_line:500,end_line:20}`
  が lone `start_line` 規約で `{500,None}`(500 行〜EOF)へ
  縮退しない。
- Rule: `view_range` の `end == -1` は "to EOF" sentinel として
  `{offset:start, limit:None}` へ写像する。`end < -1`・`end==0`・
  `end` 非整数・`start` 側の 0/負値/非整数は complete array 不成立
  (incomplete)として次の判定へ進む。
- Rule: **[実装 note — validator 指摘]** `normalize_view_range`
  (旧 `copilot_view_range` の共有化)は `Option` では「有効」
  「矛盾(helper 全体 `None` short-circuit)」「不完全(次判定へ
  fall-through)」を区別できないため、tri-state または同等の 3 値
  表現を返すこと。windsurf caller は Valid→`Some(range)`・
  Contradiction→helper 全体 `None`・Incomplete→`end_line`/Step C
  へ進む、として呼ぶ。copilot caller は non-Valid を全て `None`
  へ写像する plain `Option` として呼ぶ(従来意味論と同値)。
- Rule: `tool_info.pattern` が**非空 string**(`as_str` 成功かつ
  `!is_empty()`)かつ、`file_path` が (i) 欠落・空文字・非 string
  または (ii) `cwd` 相対 resolve 先が directory の場合に限り
  `RoutingRequest::Search` へ正規化する。`pattern` が非 string・
  空 string、または `file_path` が file を指す場合は FileRead 経路
  — 空 `pattern` の無条件 `Allow`(eval 層規約)による fail-open
  と、file 内容 ⊆ matched lines への過少評価を normalize 段で塞ぐ。
- Rule: `pattern` gate の dir 判定は
  `nodepath::resolve(&cwd, file_path)` → `std::fs::metadata()`
  `.is_dir()` で行い、normalize 層の fs access は `pattern` 非空
  かつ `file_path` 非空のときの 1 回に限定される。metadata 失敗・
  file 解決は directory 非該当として FileRead 判定へ進む。
- Rule: `normalize_search` の `path` 解決は `path` key 存在時は
  従来どおり `path` 優先(非 string は `"."`)、`path` 欠落時のみ
  `file_path`(非空 string)を search root として使い、両者欠落は
  `"."`。この拡張は共有 `tool_use_normalize`(devin/pretooluse
  grep)にも効くが、`path` 優先で既存 contract は変わらない。
- Rule: windsurf target で `FileRead{range:None}` が `Deny` の
  場合のみ、deny reason 末尾へ `WINDSURF_RANGE_DROP_GUIDANCE`
  1 行を追記する(条件付き表現 — compat 登録経由なら range field
  が転送されていない可能性と `cheaper hook-config verify` 誘導)。
  `range:Some` deny・Allow・`Search`/`Shell` verdict・他 target
  (devin/pretooluse/copilot/cursor/codex)の reason は不変。
- Rule: guidance 付記は `handle_hook` の windsurf dispatch arm で
  verdict render 直前に行い、eval 層(`routing_policy`)・dedup
  verdict record(`eval_cache`)には mutate 前の生 verdict が
  persist される — cache 経由で serve される verdict 文言に
  guidance は混入しない。hook firing 経路は config/登録状態を
  読まない。
- Rule: `hook-config verify` は `own_state` が `"absent"`/`"missing"`
  かつ `.windsurf/hooks.json` suffix(separator `/` normalize 後
  の大小文字区別比較)の legacy conflict があり、かつ loaded
  config の `read_config_from.windsurf` が明示 JSON `false` で
  ない場合、`compat-degraded:` informational finding を emit
  する。`loaded` が `None`(config 不在・読取不能)は「無効化
  なし」扱い。severity は informational — `blocker`/`clean`/exit
  code(absent/missing 時は既存 blocker で 2)に影響しない。
- Rule: `own_state` が `"absent"`/`"missing"` 以外
  (`present`/`drifted`/`modified`/`duplicate`)で同上の windsurf
  conflict + 非無効化条件を満たす場合、`windsurf-conflict:`
  informational finding を emit する — native 登録を足しても
  legacy が残る限り dual-registration trap(fingerprint が
  `range` を含むため range-diverged firing は dedup されず
  verdict union で compat deny が勝つ)が存続することを報告する。
  clean 時の exit は 0 のまま。
- Rule: `read_config_from` key 欠落・非 object・`windsurf` が
  `false` 以外の値は全て「無効化されていない」として扱い、
  `false` のみを抑止条件とする保守判定 — shape 不確定時に
  finding を黙らせる方向へは倒さない。windsurf conflict が無い
  (`.claude/settings.json` 等のみの)場合は両 finding とも emit
  しない。
- Rule: `legacy-conflict:` の文言は「duplicate firings are
  deduplicated at eval time for identical requests」を「dedup は
  同一 canonical request(fingerprint 一致)に限り発動し、range
  field を落とす compat 経由 firing は dedup 対象外」と訂正する
  — range を落とす compat firing に対して現行文言は偽。
  発火条件・informational 規約・exit code は不変。
- Rule: **[validator pin — copilot 回帰面]** 共有
  `normalize_view_range` は copilot `view` 経路にも効くため、
  copilot `view_range` の unit test を必須とする:
  `view_range:[s,-1]`(s>=1)は `ReadRange{offset:s,limit:None}`
  へ写像する(新規受理 — 従来 `json_line_number` 不受理で
  `None` だった挙動が `-1` sentinel 規約で変わる、design 受理済
  の差分)。`view_range:[s,0]` は `end` が `json_line_number`
  不受理のため incomplete → `None`(全文評価縮退、従来どおり)。
  `view_range:[s,e]`(1<=e<s)は copilot で `None`(従来の矛盾
  扱いと同値)。
- Rule: hook firing 経路の新規 fs/config access は `pattern` gate
  の metadata 1 回に限定され、verify finding 側の
  `read_config_from` 判定は verify の既存 `loaded` config から
  読む(firing 毎の config read・登録状態を根拠にした allow
  defer は存在しない — fail-closed)。
- Rule: 既存の native 経路(devin/pretooluse/windsurf の
  offset/limit bounded 動作、invalid range の全文縮退、Search・
  Shell 評価、dedup fingerprint 契約)は一切回帰させない。

## Examples

- Example: `tool_info{file_path, start_line:1, end_line:20}` で
  threshold 超過 file の 20 行 window が bounded 評価され allow
  になる(現行: `start_line`/`end_line` 不達で全文 deny)。
  `view_range:[1,20]`・`offset:1,limit:20` でも同じ `ReadRange`
  に写像される。
- Example: `tool_info{file_path, start_line:5, limit:20}` は Step A
  (explicit `limit` key 存在)で `{5,20}`、`{file_path,
  start_line:5}` 単独は Step C fallback で `{5,None}`(5 行〜EOF)、
  `{file_path, end_line:20}` は `{1,20}` へ写像される — 同一
  `start_line` が文脈で 3 通りの意味を持つ documented precedence。
- Example: `{file_path, start_line:500, end_line:20}` は矛盾 pair
  → helper 全体 `None` → 全文評価で deny。`{500,None}`(500 行〜
  EOF)へ縮退しないことが fail-closed の実証。
- Example: `tool_info{file_path, pattern:"稀", max_results:5}` で
  `file_path` が dir 解決(または欠落)なら `Search` 評価へ到達し
  実体化結果が threshold 内なら allow。`file_path` が file を指す
  併記・`pattern:""` 併記は FileRead 全文評価を維持する。
- Example: `tool_info{file_path}`(範囲無し)の Deny stderr が
  `cheaper hook-config verify` 誘導行を含み、devin/pretooluse の
  同条件 deny には誘導行が載らない。
- Example: `.devin/config.json` 不在 + `.windsurf/hooks.json`
  登録ありの fixture で `verify` が `compat-degraded:` finding +
  exit 2 を返し、own entry present + 同 conflict では
  `windsurf-conflict:` finding + exit 0・`clean:true` を返す。
- Example: missing+conflict+`read_config_from:{windsurf:false}`
  の config では両 finding が emit されず(実 Windsurf 専用登録
  として `legacy-conflict:` のみ残る)、`.claude/settings.json`
  のみ conflict + absent でも両 finding が emit されない。
- Example: copilot `view` `toolArgs{path, view_range:[10,-1]}`
  が `{offset:10,limit:None}` へ写像され(10 行〜EOF の bounded
  eval)、`view_range:[10,0]` が `None`(全文評価)に縮退する。

## Guarantees

- 実装者が design を再解釈しなくても、Rules の受理順(Step A →
  pair/array/end_line → Step C)・矛盾 short-circuit・gate 2 条件・
  finding 発火条件・文言規約が `targets.rs`/`hook_config.rs` の
  unit+integration test で機械的に pin される。
- 新規受理経路のいずれでも、曖昧・無効・矛盾する範囲表現は
  `range:None`(全文評価)またはより広い window へ倒れる —
  単一語彙入力では過少評価に縮退する経路が存在しない。
- deny reason への追記は additive のみで、`range:Some` deny・
  Allow・`Search`/`Shell` verdict・他 target・dedup verdict
  record の文言は変わらない。
- verify の `result.findings` への追加は informational 文字列
  2 件のみで、`clean`・`remediation`・exit code 規約・
  `sync`/`remove`/`--event-stdin` の動作は不変。
- `docs/guides/distribution.md` に windsurf `tool_info` field
  contract・bounded eval の登録要件(native entry + 競合 legacy
  除去または import 無効化)・compat 経路の限界(grep metadata
  不達・`exec` は shell eval policy)が記載される。

## Non-goals

- Devin CLI 本体の compat 変換(native→windsurf payload 翻訳)の
  正しさ — cheaper 制御外であり検証しない。
- 混在語彙 payload(offset/limit 系 + pair/array 同時存在)が
  emitter の「意図範囲」と一致すること — 意図は識別不能であり、
  documented precedence による deterministic 解決のみを検証する。
- bounded shell read 評価(`head`/`tail -n`・pipe filter の
  bounding 解釈)— compat 固有でなく別 feature(defer 済み)。
- Devin 3000.11.92 の compat wire shape 実測(拡張 alias が実際に
  送られるか)— consumer 側 probe、本 spec では防御的受理面と
  parity のみを検証する。
- `read_config_from` の key shape 一次情報・Devin が実際に
  `.windsurf/hooks.json` を consume するかの確定 — finding は
  conditional 表現の informational であり真偽を保証しない。
- user scope config・親 dir・大小文字差(`.Windsurf/` 等)の
  windsurf 登録検出 — `KNOWN_LEGACY_FILES` 直下限定の既知限界を
  継承する。
- 検証コマンド・実行結果の記録 — task の `## Verification` と
  impl record の責務。

## Risk

- contradiction short-circuit が Step C へ漏れると、`{s,e}`(e<s)
  の矛盾 emitter 意図より狭い `{s,None}` 評価で allow し得る —
  bounded eval の fail-closed 前提を静かに破壊する最大の単点。
- `pattern` gate を緩めすぎると(非空チェック・dir 判定の欠落)、
  read payload が空 `pattern` の無条件 `Allow` または matched-lines
  ⊆ 全文の過少評価で素通しする。
- copilot `view_range` の共有化差分(`-1` sentinel)を pin しない
  と、将来の helper 変更が copilot 経路の意味論を無テストで変える
  — 現行は copilot `view_range` の test がゼロ。
- `compat-degraded`/`windsurf-conflict` を blocker にすると
  `post-install`/`post-update` lifecycle の verify code 透過
  (conflict 残余を exit 0 で返す既存 contract)を破壊し、APM
  install を落とす。
- `legacy-conflict` の dedup 文言を訂正しないと、consumer が
  「共存は安全」のまま native 登録追加のみで対処を終え、compat
  deny が残る remediation loop の false-green が再発する。
- guidance を firing 経路の登録判定で出し分ける設計にすると、
  firing 毎の config read I/O と registered≠fired の fail-open
  (ADR Option C 棄却理由)を持ち込む。
