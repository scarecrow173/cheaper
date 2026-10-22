---
id: "SPEC-034KwuiFzl4WvVQKdjfSnM"
type: "spec"
status: "approved"
title: "Windsurf-compat bounded evaluation coverage and Devin registration surfacing (issue #161)"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
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
    - "../adr/defensive-compat-normalization-with-registration-surfacing.md"
    - "../discovery/bounded-hook-read-request-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/devin-compat-range-fields-dropped-read-code-deny.md"
  derived-by:
    - "../designs/windsurf-compat-range-normalization-and-registration-surfacing-issue-161.md"
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../guides/distribution.md"
  defers: []
  deferred-by: []
metadata:
  spec:
    audience: ["maintainer", "implementer"]
---

# Windsurf-compat bounded evaluation coverage and Devin registration surfacing (issue #161)

## What Should Be Built

issue #161（#107 再発）に対応し、windsurf-compat 経路の bounded 評価
coverage と、native 登録不在の縮退状態の surfacing を実装する。

1. **windsurf adapter の範囲・検索 field coverage 拡張**
   `crates/cheaper-cli/src/hook/targets.rs` の `windsurf_normalize` が
   `pre_read_code` の `tool_info` から抽出する `ReadRange` の alias を、
   共有 `tool_use_normalize` と parity させる。具体的には offset 系に
   `start_line` を、limit 系に `max_lines` を追加し、さらに
   `start_line`+`end_line` pair・`view_range` array（共に 1-based・
   inclusive）を `ReadRange` へ写像する。`tool_info` に `pattern`
   （string）がある payload は `RoutingRequest::Search` へ正規化する
   （compat emitter が grep 系 metadata を載せた場合に bounded 検索評価へ
   到達させる防御）。
2. **deny 時 remediation surfacing**
   windsurf target で `range:None` の `FileRead` が Deny となった場合、
   deny reason に「Devin CLI の compat firing では範囲 field が到達しない
   可能性」と `cheaper hook-config verify` への誘導を付記する。allow・
   range 有り deny・他 target では従来通りの reason とする。
3. **`hook-config verify` の compat-degraded finding**
   `ownEntry` が `absent`/`missing` で、かつ legacy conflicts に
   `.windsurf/hooks.json`（windsurf 系の cheaper hook 登録）を検出した
   場合、bounded 評価が compat 経路では到達不能である旨を説明する
   finding を追加する。exit code・既存 finding の規約は維持する。
4. **contract・運用の文書化**
   `docs/guides/distribution.md`（および必要に応じ skill 文書）に、
   (a) windsurf `tool_info` から cheaper が読む field 集合の contract、
   (b) compat 経路では grep metadata が到達しない限界、
   (c) bounded 評価の有効化には native 登録**に加えて**競合する legacy
   windsurf 登録の除去または `read_config_from` による windsurf import
   無効化が必要であること（dual-registration trap）、
   (d) `exec`/shell 経由読み取りの deny が shell eval policy 由来である
   こと、を明記する。

## Why Now

- code-knowledge（Devin CLI 3000.11.92）で `.devin/config.json` 不在の
  まま legacy `.windsurf/hooks.json` 登録へ縮退しており、範囲指定
  read・grep・exec 経由読み取りが全文 token 数で拒否されている
  （issue #161、#107 再発）。
- #107 の wire 観測（DISC-034FRguX1hjhDZ7ZMEBnbm）で、Devin compat が
  `tool_info` に `file_path` 以外を載せないことが実測済み。metadata
  消失地点は Devin 側 compat 変換で cheaper 制御外 — 到達する field の
  防御的受容と、縮退状態を失敗時点・検出時点で surface する remediation
  が cheaper 側の残る修正境界。
- 現行 windsurf adapter は共有 normalize より alias coverage が狭い
  （`start_line`/`max_lines` 不達、`end_line`/`view_range` 未対応、
  `pattern`→`Search` 経路無し）という実装側 gap が discovery で確定した。

## Users and Value

- **Devin CLI consumer（cheaper hook 利用者）**: 範囲 field が到達する
  compat payload で bounded 評価が発動し、未到達でも deny 時に
  「なぜ拒否されたか・どう直すか（`hook-config verify` → native 登録 +
  legacy 除去）」を失敗時点で得られる。
- **実 Windsurf consumer**: 実 Windsurf が alias・pair/array 形式の範囲
  field を送る場合に bounded 評価へ到達できる。検出 finding は
  windsurf 登録を「conflict」とだけではなく「bounded eval 未到達の
  併存条件」として説明される。
- **maintainer**: compat emitter の field 表現が変化しても共有 normalize
  と同じ受理面で受けるため、再発報告の調査コストが下がる。

## Scope

### In Scope

- `windsurf_normalize`（`pre_read_code`）の `ReadRange` 抽出 coverage
  拡張 — offset aliases `offset`/`line_offset`/`start_line`、limit
  aliases `limit`/`line_limit`/`max_lines`、pair `start_line`+`end_line`
  （inclusive → `limit = end - start + 1`）、array `view_range`
  = `[startLine, endLine]`（1-based・inclusive、copilot `view` と同規約）。
  受理順・無効値の縮退規則は既存 `normalize_read_range` と同じ
  （未満 1・非整数・空文字は不採用、`offset == 1 && limit == None` は
  全文評価へ畳む等）。優先順位は「明示 offset/limit aliases → pair/array
  形式 → `end_line` 単独（offset=1, limit=end_line）」の方針で design で
  固定するが、いかなる経路でも過少評価には縮退しない（曖昧・無効値は
  常により広い評価=全文側へ倒す）。
- `pre_read_code` `tool_info` への `pattern`（string）検出と
  `RoutingRequest::Search` への正規化。`path`/`file_path`/
  `max_results`/`context`/`glob`/`output_mode`/`case_insensitive` の
  写像は共有 `normalize_search` と同一規約。`pattern` を持たない
  payload は従来通り `FileRead`。
- windsurf FileRead Deny（`range:None`）への remediation 付記 —
  deny reason へ compat firing の範囲不達の可能性と `hook-config
  verify` 誘導を追記。`range:Some` の deny・allow・他 target 不変。
- `hook-config verify` — `ownEntry` absent/missing かつ windsurf 系
  legacy 登録検出時の compat-degraded finding 追加（bounded eval が
  compat 経路では発動しない旨＋ remediation: native 登録 + 競合 legacy
  除去/import 無効化）。`sync`/`remove` の動作・own-entry-only 境界は不変。
- 文書化 — windsurf `tool_info` field contract、compat 経路の限界、
  dual-registration trap（共存では bounded eval が有効にならない）、
  shell eval policy の説明を `docs/guides/distribution.md` 等へ。

### Out of Scope

- Devin CLI 本体の compat 変換（native→windsurf payload 翻訳）の修正 —
  cheaper 制御外。
- legacy 登録ファイル（`.windsurf/hooks.json` 等）の自動除去・
  `read_config_from` 設定の自動変更 — consumer 側設定境界であり、
  cheaper は own-entry-only の所有境界を維持する。
- bounded shell read 評価（`head`/`tail -n`・pipe 下流 filter を
  bounding として解釈する評価）— `exec` 経由の deny は shell eval
  policy 由来で compat 固有の問題ではない。別 feature として defer
  （Deferred Design Concerns 参照）。
- bounded 評価本体（`routing_policy` の tokenize・閾値・dedup
  fingerprint）の変更 — #107 で導入済みの評価ロジックは正常であり、
  threshold を弱めたり評価を bypass したりしない。
- Devin 3000.11.92 の compat wire shape 実測 — consumer 側 wire probe
  が必要で本 scope 外（defensive normalization がこの不確定性を
  カバーする）。

## Requirements

- R1: `windsurf_normalize` の `pre_read_code` 経路で、`tool_info` の
  `offset`/`line_offset`/`start_line`（offset 系）、`limit`/`line_limit`/
  `max_lines`（limit 系）を受理し `ReadRange` へ写像する。
- R2: `tool_info` の `start_line`+`end_line`（両方整数）または
  `view_range` array `[start, end]` を 1-based・inclusive 区間として
  `ReadRange{offset:start, limit:end-start+1}` へ写像する。
  `end_line` 単独は `{offset:1, limit:end_line}` として受理してよい。
- R3: R1/R2 の無効値（1 未満・非整数・型不一致・`end < start`）は
  既存規約どおり field 無視で縮退し、曖昧な入力が過少評価に繋がる
  ことはない。
- R4: `pre_read_code` `tool_info` に `pattern`（string）がある場合は
  `RoutingRequest::Search` を生成し、bounded 検索評価へ到達させる。
  `pattern` 無しは `FileRead`。
- R5: windsurf target で `range:None` の `FileRead` が Deny の場合、
  deny reason に Devin compat の範囲不達の可能性と `cheaper
  hook-config verify` 誘導を付記する。`range:Some` deny・allow・
  他 target の reason は従来通り。
- R6: `hook-config verify` は `ownEntry` absent/missing かつ windsurf 系
  cheaper 登録が legacy conflicts に存在する場合、compat 経路では
  bounded 評価が発動しない旨の finding を出力する。exit code 規約
  （finding 種別 → code）は従来どおり。
- R7: `docs/guides/distribution.md`（必要に応じ skill 文書も）に、
  windsurf `tool_info` field contract、bounded eval の登録要件
  （native entry + 競合 legacy 除去または import 無効化）、compat 経路
  の限界（grep metadata 不達、exec は shell eval policy）を記載する。
- R8: 既存の native 経路（devin/pretooluse/windsurf の offset/limit
  bounded 動作、invalid range の全文縮退、Search・Shell 評価）を
  一切回帰させない。

## Acceptance Criteria

- [ ] `tool_info{file_path, start_line:1, end_line:20}` で threshold 超過
  file の 20 行 window が bounded 評価され allow になる（現行: 全文
  deny）。`view_range:[1,20]` でも同様。
- [ ] `tool_info{file_path, offset, limit}` および `start_line`+
  `limit`/`max_lines` 組合せが従来どおり bounded 評価され、window が
  threshold 内なら allow。
- [ ] `tool_info{file_path, pattern:"稀", max_results:5}` が Search 評価へ
  到達し、実体化結果が threshold 内なら allow（現行: FileRead 全文
  deny）。
- [ ] `tool_info{file_path}`（範囲無し）の Deny reason が、Devin compat
  firing の範囲不達と `hook-config verify` 誘導を含む。
- [ ] `.devin/config.json` 不在 + `.windsurf/hooks.json` 登録ありの
  fixture で `hook-config verify` が compat-degraded finding を出力する。
- [ ] 無効な `start_line`/`end_line`/`view_range` は field 無視で全文
  評価へ縮退し deny 方向のみになる（過少評価しない）。
- [ ] 既存 integration tests（`hook::` devin/pretooluse/windsurf bounded
  動作、invalid range 縮退、search/shell）が非回帰。
- [ ] `mise run check` が pass する。

## Deferred Design Concerns

- **bounded shell read 評価** | `head`/`tail -n`・pipe 下流 filter を
  bounding として解釈する shell eval 拡張 — exec 症状の方向的解消に
  なるが compat 機序とは無関係（native でも同 verdict）で別 feature
  相当。再開契機: shell read bounding の需要報告。無視した場合の
  リスク: `exec` 経由の大 file 読み取りが引き続き全文 deny になる。
- **compat firing の環境識別** | `DEVIN_PROJECT_DIR` 等の env marker が
  compat firing にも立つか未確認 — 確定すれば deny 案内を Devin
  context に絞り込める。再開契機: wire/env 観測の取得。無視した場合の
  リスク: 実 Windsurf 環境の deny にも案内行が載る（内容は条件付き
  表現で実害は軽微）。
- **Devin 3000.11.92 wire shape 実測** | defensive normalization が
  救済する alias が実際に送られるかの確認。再開契機: consumer 側
  wire probe 実施。無視した場合のリスク: 拡張した alias が dead code
  化する可能性（共有 normalize との parity 自体に価値があるため
  許容）。

## Sources and Relations

- upstream discovery:
  `docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md`
  （DISC-t3TdC7pUJUGdYgfkpkmhMU、`derives-from`）— 発火経路・metadata
  消失地点・dual-registration trap・exec 分類の確定根拠。
- 併走 ADR:
  `docs/adr/defensive-compat-normalization-with-registration-surfacing.md`
  （ADR-034KwuE0Xnp3O6EUXacFG8、`related`）— 選択した修正境界と棄却
  option（defer-on-registered 等）の理由。
- 前提・関連する既存決定（cross-chain、`references`/`related` のみで
  lineage は張らない）:
  - SPEC-034H2WAU6TQG6N9dmmmKNA（bounded eval + managed registration）
  - ADR-034H2WBlp8VXoEE7KVlGt5（ReadRange/Search canonical request）
  - ADR-034H2WBNtC4We8dyC5lQZJ（hook-config lifecycle・own-entry-only）
  - ADR-034Ie3HFd6tIJms6Z2NIow（dedup fingerprint = canonical request）
- issue: https://github.com/scarecrow173/cheaper/issues/161
