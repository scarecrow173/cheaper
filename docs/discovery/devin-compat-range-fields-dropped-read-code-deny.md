---
id: "DISC-t3TdC7pUJUGdYgfkpkmhMU"
type: "discovery"
status: "resolved"
title: "Devin compat 経由の read/grep/exec が範囲指定を無視して全文 token 数で拒否される（#161, #107 再発）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/161"
    - "https://github.com/scarecrow173/cheaper/issues/107"
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
    - "../discovery/bounded-hook-read-request-contract.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
    - "../adr/defensive-compat-normalization-with-registration-surfacing.md"
    - "../designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../specs/tool-operator-built-in-worker.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../guides/distribution.md"
  defers: []
  deferred-by: []
metadata:
  discovery:
    question: "issue #161（#107 再発）: Devin windsurf-compat 経由の範囲指定 read・単一 file grep・shell 経由読み取りが全文 token 数で拒否される。発火経路と metadata 消失地点、cheaper 側の修正境界を確定する"
    scope: "crates/cheaper-cli/src/hook/{targets,routing_policy,eval_cache}.rs と hook_config.rs の実読、debug binary による合成 payload 再現、#107 の wire 観測記録との突合、登録 lifecycle（hook-config/legacy conflict/dedup）の coverage 評価"
    outcome: "確定 — compat payload は file_path のみ（#107 wire 観測、3000.11.92 は未確認）。cheaper 側の修正可能な gap は (a) windsurf adapter の範囲 field coverage が共有 normalize より狭いこと、(b) bounded eval 到達に必要な登録状態が失敗時点で surface されないこと。exec deny は metadata 消失ではなく shell eval policy の挙動（native でも同 verdict）"
---

# Devin compat 経由の read/grep/exec が範囲指定を無視して全文 token 数で拒否される

## Exploration Goal

issue #161（#107 再発）。code-knowledge（Windows・Devin CLI 3000.11.92）で、
threshold 超過 file への範囲指定 read・単一 file grep・shell 経由読み取りが
「ファイル全体の token 数」で拒否される。発火経路は `.windsurf/hooks.json`
の legacy 登録（`pre-read-code --target windsurf`）であり、`.devin/config.json`
の native 登録が無い（`hook-config verify` → `ownEntry: absent`）。

本 discovery の目的は、#107 で導入済みの bounded 評価が現行環境で発動しない
機序をコード・再現・#107 の wire 観測記録で確定し、cheaper 側の修正境界を
選ぶこと。

Entry Decision: A-3 (Combined Discovery) — adapter normalize 実読、合成
payload 再現、#107 の凍結 wire 証拠、登録 lifecycle 仕様の突合が必要。

## Key Issues and Assumptions

### 現行 windsurf normalize の coverage（confirmed: 実読）

`crates/cheaper-cli/src/hook/targets.rs`:

- `windsurf_normalize` (:155-189) — `pre_read_code` は
  `tool_info.file_path` を必須とし、`ReadRange` を
  `normalize_read_range(t, &["offset","line_offset"], &["limit","line_limit"])`
  (:173-175) で抽出する。**`start_line`/`end_line`/`view_range`/`max_lines`
  は一切読まない**。`pre_run_command` は `tool_info.command_line` → Shell。
- `tool_info` から `RoutingRequest::Search` への経路は存在しない —
  `pre_read_code` は常に `FileRead` になるため、`pattern` が届いても
  grep を bounded 評価できない。
- 対比として共有 `tool_use_normalize` (:456-495、pretooluse/devin/codex
  共通) は offset 系に `offset`/`line_offset`/`start_line`、limit 系に
  `limit`/`line_limit`/`max_lines` を読み (:479-481)、`grep` →
  `normalize_search` → `Search`、shell tools → `Shell` へ写像する。
  **windsurf adapter は範囲 field alias の coverage が共有 normalize より
  狭い非対称**にある。
- copilot `view` は `view_range` array を `copilot_view_range` (:419-433)
  で `[startLine, endLine]`（1-based・inclusive）→
  `ReadRange{offset: start, limit: end-start+1}` へ写像する実績がある。

### 合成再現（confirmed: `target/debug/cheaper.exe` で payload 駆動）

threshold 超過 fixture（2000 行・35999 tokens）に対し、現行 binary で確認:

| 経路 | payload（要点） | 結果 |
|---|---|---|
| windsurf `pre_read_code` | `tool_info{file_path, offset:1, limit:20}` | exit 0 allow（bounded eval 正常） |
| windsurf `pre_read_code` | `tool_info{file_path, start_line:1, end_line:20}` | exit 2 deny — 全文 35999 tokens |
| windsurf `pre_read_code` | `tool_info{file_path, view_range:[1,20]}` | exit 2 deny — 同上 |
| windsurf `pre_read_code` | `tool_info{file_path}`（#107 観測 shape） | exit 2 deny — 同上 |
| windsurf `pre_run_command` | `tool_info{command_line:"cat big.txt \| findstr ..."}` | `blocked shell command` deny |
| devin `PreToolUse` read | `tool_input{file_path, offset:1, limit:20}` | exit 0 allow |
| devin `PreToolUse` grep | `tool_input{path:big.txt, pattern:稀, max_results:5}` | exit 0 allow（結果実体化で評価） |
| devin `PreToolUse` exec | `tool_input{command:"head -n 5 big.txt"}` | `{"decision":"block"}` deny — `cat`/`head` 等の read candidate は常に全文評価 |

`evaluate` の分岐（`routing_policy.rs:1548-1595`）: `FileRead` は
`range.is_some()` → `evaluate_ranged_read`（window のみ実体化）、
`range:None` → `evaluate_file_read`（全文 tokenize）へ。`Shell` は
`evaluate_read_candidates` (:1530-1543) で各 read candidate を
`evaluate_file_read`（常に全文）へ通す — `head -n 5` や `cat | findstr`
の下流 filter・行数 option は bounding として考慮されない。

### Devin compat が実際に送る field（#107 wire 観測 + 残 open question）

- **confirmed（#107 fixture、Devin 3000.11.3）**:
  `bounded-hook-read-request-contract.md`（DISC-034FRguX1hjhDZ7ZMEBnbm）の
  sanitized wire record で、native `read{file_path, offset:334, limit:81}`・
  `grep{path, pattern, max_results:1}` が windsurf-compat `pre_read_code` へ
  翻訳される際、`tool_info` は **`{"file_path"}` のみ**（root・tool_info
  とも `otherKeyCount:0`）と実測済み。`start_line`/`end_line`/
  `view_range`/`pattern` を含む **いかなる追加 field も送られない**。
  metadata 消失地点は Devin 側 native→compat 変換（cheaper adapter より上流）。
- **open（#161 環境）**: Devin 3000.11.92 の compat wire shape は未取得。
  同一症状の再現は「依然 file_path のみ」と整合するが、新版が別名で範囲
  field を送る可能性は排除できない — これが防御的正規化（Option A）の
  設計根拠になる。
- **confirmed**: 実 Windsurf の `pre_read_code` payload も同一 key 集合
  （`{agent_action_name, tool_info:{file_path}, trajectory_id,
  execution_id, timestamp}`、`docs/impl/exp/apm-windsurf-consumer-e2e.jsonl`
  seq 12 観測）を持つため、payload shape から Devin compat firing と実
  Windsurf firing を区別する署名は存在しない。

### 発火経路と登録 gap（confirmed）

- #161 env の発火経路は `.windsurf/hooks.json` legacy 登録 → Devin
  windsurf-compat（issue 本文で stderr + exit 2 の payload 再現一致を確認
  済み）。`.devin/config.json` が無く `hook-config verify` は
  `ownEntry: absent` を返すが、**consumer が verify を呼ばない限りこの
  縮退状態はどこにも surface されない**。
- `hook_config.rs`: `run_verify` (:591-752) は config 不在時に
  `absent-config` finding + `remediation` + exit 2 を返す検出機構を既に
  持つ。`legacy_conflicts` (:362-396) は `.windsurf/hooks.json` 等の
  cheaper 登録を `legacyConflicts` として報告する。

### dual-registration trap（confirmed: fingerprint 読解 + #107 fixture 実測）

仮に consumer が `hook-config sync` で `.devin/config.json` へ native 登録を
足しても、`.windsurf/hooks.json` の cheaper 登録が残る限り症状は解消しない:

- Devin の `read_config_from` import により、同一 tool call に native
  `PreToolUse` firing と compat `pre_read_code` firing が併発する
  （#107 fixture で dual `pre_tool` firing を実測、deny 側が union で勝つ）。
- compat firing は `FileRead{range:None}`、native firing は
  `FileRead{range:Some}` を生成する。eval_cache の fingerprint は
  `FileRead` の `range` を含む（`eval_cache.rs:436-437`。
  `source_tool`/`target` は含めない :295-297）ため、両 request は
  fingerprint 非一致 → **dedup（issue #127）は発動せず**、compat の
  全文 deny が native の bounded allow と併存して tool call を block する。
- 「共存は安全」（ADR-034IKJq6tKWWAep0Q0ONzK の gate demotion）の根拠は
  「fingerprint が一致する同一 canonical request の dedup」であり、
  range を落とす compat firing とは request が diverge するため **bounded
  read に限り共存は成立しない**。つまり bounded eval を有効にするには
  native 登録に加えて、競合する legacy windsurf 登録の除去または
  `read_config_from` での windsurf import 無効化が必須。
- legacy file の除去・`read_config_from` の変更は consumer 側設定境界で
  あり、cheaper は own-entry-only（`.devin/config.json` の
  `hooks.PreToolUse` のみ管理、own entry 以外は保全）の所有境界上、
  自動では収束できない（ADR-034H2WBNtC4We8dyC5lQZJ）。

### exec の分類（confirmed: mechanism 解読 + 再現）

`exec`（compat `pre_run_command` 経由と推定 — deny message が
`evaluate_read_candidates` の `blocked shell command` prefix と一致）の
`cat file | findstr` 拒否は、**metadata 消失ではなく shell eval policy の
挙動**である:

- `command_line` は compat でも完全に到達する（#107 観測の shell 系
  tool_info contract と一致）。read candidate 抽出
  （`extract_read_candidates`）は `cat`/`head`/`tail`/`less`/`more` の
  file operand を拾い、各 candidate を `evaluate_file_read`（常に全文
  token 評価）へ通す。pipe 下流の filter・`head -n`/`tail -n` の行数
  option は bounding として解釈されない。
- 同じ command を native `--target devin`（`exec`/`tool_input.command`）
  へ送っても同一 deny を確認 — **compat 固有の問題ではなく、native
  登録を足しても exec 症状は変わらない**。bounded shell read（`head`/
  `tail -n`、pipe filter）の評価は別 feature 相当で、本 bug の修正境界
  とは分けて扱う。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. windsurf adapter の normalize coverage 拡張 — `start_line`/`end_line`/`view_range`/`max_lines` を `ReadRange` へ、`tool_info` の `pattern` 系を `Search` へ正規化 | 範囲 field が届く経路（新版 Devin compat・実 Windsurf・その他 compat emitter）を全て救済。共有 `tool_use_normalize` との非対称を解消。grep も `pattern` が届けば bounded 評価可能。追加は normalize のみで eval/policy 無変更 | #107 観測 shape（`file_path` のみ）では無効 — 主流ケースの症状は単独では解消しない。wire shape 未確認部分への防御的実装 | yes（部分採用） |
| B. A + remediation surfacing — windsurf FileRead deny（range 無し）へ登録案内を付記、`verify` が compat-degraded 状態を finding、contract を文書化 | field 未到達の主流ケースで「なぜ deny か・どう直すか」を失敗時点に surface できる。dual-registration trap の案内も可能。cheaper の所有境界（検出・案内・own entry 管理）内で完結 | deny reason 文字列が変わる（test 追随必要）。verify finding 追加は exit code 規約に影響しないが出力 contract の追加 | yes（採用） |
| C. compat firing が `.devin/config.json` の own entry 登録済みを検出したら allow へ defer し native eval へ委譲 | 混在 consumer（Windsurf 用に legacy 登録を残したい環境）で consumer 操作なしに bounded eval へ一本化できる唯一の cheaper 側経路 | **registered ≠ fired** — Devin が native hook を実際に呼ぶ保証が無い（config 未反映・version skew・matcher 不適合）場合、defer した compat firing は無防備な allow となり fail-open の穴を開ける。firing 毎の config read も必要 | no |
| D. bounded shell read eval（`head`/`tail -n`・pipe filter の bounding 解釈） | exec 症状の方向的解消 | 本 bug の compat 機序とは無関係（native 経路でも同 verdict）。option parse → 範囲 semantics の別 feature 相当 | defer |
| E. status quo + docs のみ | 実装最小 | 範囲 field が届くケースすら救済せず、検出 gap も残る | no |

## Tentative Conclusions and Hypotheses

- **confirmed（root cause）**: 症状の主因は Devin compat が `tool_info` に
  `file_path` 以外を載せないこと（#107 wire 観測で確定、3000.11.92 は
  open）＋ native 登録不在による compat 経路への縮退。metadata 消失地点は
  cheaper 制御外のため、**adapter 正規化だけでは主流ケースを直せない**。
- **confirmed**: cheaper 側に実在する gap は 2 点 — (a) windsurf adapter の
  範囲 field alias coverage が共有 normalize より狭い（`start_line`/
  `end_line`/`view_range`/`max_lines` 不達、`pattern`→`Search` 経路無し）、
  (b) compat-degraded 状態が失敗時点・検出時点で surface されない
  （verify の absent-config finding は存在するが呼ばれない、deny reason は
  bulk-read 案内のみで登録案内を持たない）。
- **confirmed**: `hook-config sync` 単独では症状は消えない — 競合 legacy
  登録が残る限り compat firing の deny が bounded eval を無効化する
  （dual-registration trap）。consumer 側で native 登録 + legacy 除去/
  import 無効化の両方が必要。
- **confirmed**: exec deny は compat 問題ではなく shell eval policy
  （read candidate は常に全文評価）で、native 経路でも同 verdict。
  本 scope では分離し bounded shell eval として defer する。
- **決定（推奨）**: Option B — windsurf normalize の防御的 coverage 拡張
  （Option A）+ deny 時 remediation surfacing + `hook-config verify` の
  compat-degraded finding + contract 文書化。Option C（defer-on-registered）
  は fail-open 穴のため棄却、Option D は別 feature として defer。

## Open Questions

- Devin 3000.11.92 の compat `pre_read_code` `tool_info` shape —
  `start_line`/`end_line`/`view_range`/`pattern` を送るか（#107 の
  3000.11.3 では `file_path` のみ・`otherKeyCount:0` を実測）。取得には
  consumer 側 wire probe が必要で、#107 で確立した sanitized probe 手法を
  再利用できる。Option A はこの不確定性をカバーする防御であり、観測で
  「送らない」が確定しても無害。
- Devin が compat（windsurf 系）hook process に `DEVIN_PROJECT_DIR` 等の
  env marker を立てるか（native hook では cwd fallback として既知）。
  compat firing を確定的に識別できるなら deny 案内の対象絞り込みに使える
  が、現時点では payload/env ともに確定 signal が無く、B は deny の種別
  （FileRead・range 無し）だけで判定する設計とした。
- compat `pre_read_code` firing と native `PreToolUse` firing が併発した
  場合の Devin 側 verdict 合成順序・失敗 semantics（union で deny 優先は
  #107 fixture の「observer pass + legacy deny → block」実測と整合するが、
  native bounded allow + compat deny の組合せは未実測）。
- `.claude/settings.json` の cheaper pretooluse 登録が Devin 配下で
  `read`（小文字 tool_name）に発火するか — Devin 側 matcher の大小文字
  semantics 未確認（発火しても compat deny が union で勝つため結論不変）。

## Promotion Candidates

- [x] spec-doc needed:
  `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  （SPEC-034KwuiFzl4WvVQKdjfSnM）
- [x] adr-doc needed:
  `docs/adr/defensive-compat-normalization-with-registration-surfacing.md`
  （ADR-034KwuE0Xnp3O6EUXacFG8）

## Sources and Relations

- Issue #161・#107（一次報告）。
- `docs/discovery/bounded-hook-read-request-contract.md`
  （DISC-034FRguX1hjhDZ7ZMEBnbm）— #107 の wire 観測・消失地点確定・
  dual firing 実測の一次記録。本 discovery の compat payload shape 証拠は
  この凍結記録に依拠する（cross-chain のため `related`/`references` で参照、
  lineage は張らない）。
- `crates/cheaper-cli/src/hook/targets.rs`（adapter normalize）、
  `routing_policy.rs`（eval 分岐・shell read candidate）、
  `eval_cache.rs`（fingerprint 構成）、`hook_config.rs`（verify/sync/
  legacy conflict 検出）。
- #107 系 spec/ADR/design は `references` 参照（bounded eval・
  hook-config lifecycle・conflict gate demotion・dedup の各決定）。

## Seed（2026-10-23 briefing 前の最小記録 — 履歴として保持）

Issue #161（#107 再発）。code-knowledge（Windows・Devin CLI）で、
threshold 超過ファイルへの範囲指定 read・単一ファイル grep・shell 経由
読み取りが、いずれも「ファイル全体の token 数」で拒否される。

### 既知の事実（issue 本文より — briefing で検証済み）

- 発火経路は `.windsurf/hooks.json` の legacy 登録（`pre-read-code`
  `--target windsurf`）— `.devin/config.json` が無い環境では native
  `--target devin` 経路が使われず windsurf 互換変換に縮退する
- 各 target の bounded 評価自体は正常（windsurf/devin/pretooluse で
  `limit`/`offset` 付き入力は allow、無しは全文 deny — 直接検証済み）
- 推定原因: Devin の windsurf 互換変換が `tool_info` に adapter が読む
  範囲 field（`offset`/`line_offset`/`limit`/`line_limit`）を載せていない、
  または別名（`start_line`/`end_line`/`view_range`）で送っている
- 現行の windsurf 正規化は `offset`/`line_offset`/`limit`/`line_limit`
  のみを読み `start_line`/`end_line`/`view_range` は無視する
  （`crates/cheaper-cli/src/hook/targets.rs`）

### 候補対応（issue 本文より — Alternatives で評価済み）

1. windsurf adapter が `start_line`/`end_line`/`view_range` も
   `ReadRange` へ正規化する防御
2. Devin compat が実際に `tool_info` へ送る field 集合を調査し contract
   として文書化
3. native `hook-config sync` 未登録環境での縮退経路の扱い
   （legacy 登録の検出・案内）
