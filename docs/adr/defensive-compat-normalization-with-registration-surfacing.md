---
id: "ADR-034KwuE0Xnp3O6EUXacFG8"
type: "adr"
status: "proposed"
title: "Defensive compat normalization with registration surfacing"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "../discovery/devin-compat-range-fields-dropped-read-code-deny.md"
    - "https://github.com/scarecrow173/cheaper/issues/161"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md"
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
    - "./range-aware-file-read-and-bounded-search-requests-in-hook-routing.md"
    - "./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "./dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../guides/distribution.md"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Defensive compat normalization with registration surfacing

## Context and Problem Statement

issue #161（#107 再発）。Devin CLI 環境で `.devin/config.json` の native
登録が無いまま `.windsurf/hooks.json` の legacy 登録経由で発火した場合、
Devin の windsurf-compat 変換が `tool_info` に `file_path` 以外を載せ
ない（#107 wire 観測で確定済み、3000.11.92 は未確認）ため、範囲指定
read・grep が `FileRead{range:None}` に縮退し全文 token 数で拒否される。

#107 で導入した bounded 評価（`ReadRange`・`Search`・dedup fingerprint）
自体は正常であり、native payload（`tool_input.offset`/`limit`）では
範囲評価が発動することを再現確認済み。再発したのは metadata 消失地点が
新しい経路（Devin→windsurf compat 変換）だからであり、evaluation 層の
欠陥ではない。

cheaper 側に残る gap は 2 点:

1. windsurf adapter の範囲 field alias coverage が共有
   `tool_use_normalize` より狭い（`start_line`/`end_line`/`view_range`/
   `max_lines` 不達、`pattern`→`Search` 経路無し）— compat emitter が
   別名や pair/array 形式で送る場合にも範囲を拾えない。
2. 縮退状態（compat 経路で bounded eval が発動しない登録状態）が失敗
   時点・検出時点に surface されない。`hook-config verify` は
   `absent-config` finding を返すが誰も呼ばず、deny reason には
   登録案内が無い。また native 登録を追加するだけでは legacy 登録が
   残る限り compat deny が bounded eval を無効化する
   （dual-registration trap — fingerprint は `range` を含むため
   diverge した request は dedup されない）。

どこまでを cheaper の修正境界とし、どこを consumer/環境側の責務として
残すかを決める。

## Decision Drivers

* metadata 消失地点（Devin 側 compat 変換）は cheaper 制御外 — 到達する
  field の防御的受容と、縮退の可視化が残る cheaper 側の境界。
* 3000.11.92 の compat wire shape は未観測 — 「送らない」が確定済みで
  ない以上、受容面は広く取るべきだが、曖昧値が過少評価に繋がっては
  いけない。
* 修正は #107 で確立した責務境界を崩さない — eval policy・threshold・
  fingerprint 契約・own-entry-only の所有境界は維持する。
* consumer が「なぜ bounded eval が効かないか」を失敗時点で診断できる
  ことが重要 — deny reason と verify finding が remediation の入口。
* fail-open になる設計（登録の存在を allow の根拠にする等）は採らない。

## Considered Options

* **Option A: 防御的正規化のみ** — windsurf adapter の `ReadRange`
  alias coverage を共有 normalize と parity に拡張し、`tool_info.pattern`
  → `Search` 正規化を追加する。field が届くケースを全て救済する。
* **Option B: A + remediation surfacing** — A に加えて、windsurf
  FileRead deny（range 無し）への登録案内付記、`hook-config verify`
  の compat-degraded finding、tool_info contract・dual-registration
  trap の文書化を行う。
* **Option C: defer-on-registered** — compat firing が `.devin/config.json`
  の own entry 登録を検出した場合に allow へ defer し、native eval に
  評価を委譲する。consumer 操作なしに bounded eval へ一本化できるが、
  registered ≠ fired（Devin が native hook を実際に呼ぶ保証が無い）
  ため fail-open の穴を開ける。
* **Option D: 文書のみ（status quo + docs）** — tool_info contract と
  運用手順だけを文書化し、コードを変更しない。

## Decision Outcome

Chosen option: "**Option B: 防御的正規化 + remediation surfacing**",
because metadata 消失地点は cheaper 制御外であるため、cheaper が制御
できる 2 つの残存 gap（normalize coverage・検出/案内）の両方を閉じる
必要がある。Option A 単独では観測済みの主流ケース（`file_path` のみ
到達）で症状が消えず、Option C は fail-open、Option D は gap を残す。
Option B は #107 の責務境界（eval 層無変更・own-entry-only 登録管理・
fail-closed 評価）を維持しつつ、到達する field は救済し、到達しない
場合は deny/verify の失敗面で remediation へ誘導する。

### Consequences

* 範囲 field が届く compat/実 Windsurf payload（alias・pair/array 形式）
  で bounded 評価が発動するようになり、grep metadata が届く場合は
  `Search` 評価へ到達する。
* `file_path` のみ到達する主流ケースでは引き続き全文 deny だが、deny
  reason に `hook-config verify` への誘導が載り、verify が compat-
  degraded 状態を明示する finding を返すため、consumer が native 登録
  + legacy 除去の必要操作を診断できる。
* dual-registration trap（native 追加だけでは compat deny が残る）が
  文書化され、「共存は安全」の解釈が bounded eval に限定されることが
  明確になる。
* deny reason 文字列の追加に伴い、理由文字列を assert する既存 test の
  追随が必要。実 Windsurf 環境の range 無し deny にも案内行が載る
  （条件付き表現で実害は軽微）。
* 主流ケース（field 未到達）の挙動自体は変わらず fail-closed のまま —
  症状を消すのは compat 変換側または登録修復であり、本変更は救済
  可能なケースの coverage と診断可能性を上げる。

### Confirmation

* `cargo test -p cheaper-cli --test integration hook::` で windsurf の
  `start_line`/`end_line`/`view_range`/`max_lines`/`pattern` 各 fixture
  が bounded 評価へ到達し、既存 native/windsurf の bounded・縮退挙動が
  非回帰であること。
* `hook-config verify` の integration test で compat-degraded finding
  の出力と exit code 規約を確認する。
* `docs/guides/distribution.md` に field contract・登録要件・
  dual-registration trap が記載されていること。

## Implementation Plan

* Affected paths:
  `crates/cheaper-cli/src/hook/targets.rs`（`windsurf_normalize` の
  `ReadRange` alias 拡張・pair/array 変換・`pattern`→`Search` 分岐）、
  `crates/cheaper-cli/src/hook/routing_policy.rs`（windsurf FileRead
  deny の reason 付記。付記は target 情報を持つ dispatch 側で行うか、
  `source_tool` から windsurf 系を判別 — 実装時に最小差分で決める）、
  `crates/cheaper-cli/src/hook_config.rs`（verify の compat-degraded
  finding）、`crates/cheaper-cli/tests/integration/hook.rs`・
  `hook_config.rs`（fixture 追加）、`docs/guides/distribution.md`。
* Patterns to follow: `normalize_read_range` の既存縮退規則・
  `copilot_view_range` の inclusive pair→`ReadRange` 変換・
  `normalize_search` の field 写像規約・`hook_config.rs` の既存
  finding/remediation 構造。
* Patterns to avoid: 登録済みを根拠にした allow defer（fail-open）、
  eval/threshold/fingerprint の変更、legacy 登録 file の自動除去、
  `source_tool` 文字列の fuzzy 照合で他 target を巻き込む付記。
* Dependencies or configuration: なし（crate 依存追加なし）。

## Verification

* [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook::`
* [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::`
* [ ] `mise run check`

## Pros and Cons of the Options

### Option A: 防御的正規化のみ

* Good, because field が届く経路を全て救済し、共有 normalize との
  非対称という実装 gap を最も直接的に潰せる。eval/policy 層への
  波及が無く最小の coherent 変更。
* Bad, because #107 観測の主流 shape（`file_path` のみ）では効果が
  無く、「なぜ効かないか」を consumer が依然診断できない。

### Option B: A + remediation surfacing（採用）

* Good, because 救済可能なケースを救済しつつ、救済不能なケースでは
  失敗面（deny reason・verify finding・文書）から remediation へ
  誘導できる。cheaper の所有境界内で完結し fail-closed を維持する。
* Bad, because deny reason の変更で test 追随が要り、案内行が実
  Windsurf 環境にも載る。主流ケースの deny 自体は消えない。

### Option C: defer-on-registered

* Good, because consumer 操作なしに混在環境を bounded eval へ一本化
  できる唯一の cheaper 側経路。
* Bad, because registered ≠ fired で、native firing が実際に来ない
  場合に compat firing の defer が無防備な allow になる fail-open
  の穴。firing 毎の config 読込も必要で複雑性が割に合わない。

### Option D: 文書のみ

* Good, because 実装リスクがゼロ。
* Bad, because normalize coverage・検出 gap のどちらも解消せず、
  文書を読まない consumer には効果が無い。

## More Information

- upstream discovery:
  `docs/discovery/devin-compat-range-fields-dropped-read-code-deny.md`
  （DISC-t3TdC7pUJUGdYgfkpkmhMU）
- 併走 spec:
  `docs/specs/windsurf-compat-range-normalization-and-devin-registration-surfacing.md`
  （SPEC-034KwuiFzl4WvVQKdjfSnM）
- 関連する #107 系の決定（cross-chain、`references`/`related` で参照）:
  ADR-034H2WBlp8VXoEE7KVlGt5（canonical request への ReadRange/
  Search 導入）、ADR-034H2WBNtC4We8dyC5lQZJ（hook-config lifecycle・
  own-entry-only）、ADR-034Ie3HFd6tIJms6Z2NIow（dedup fingerprint
  契約 — range を含むため diverge した compat/native request は
  dedup されない根拠）、ADR-034IKJq6tKWWAep0Q0ONzK（legacy
  conflict gate の warning demotion —「共存は安全」の根拠文書）。
- issue: https://github.com/scarecrow173/cheaper/issues/161
