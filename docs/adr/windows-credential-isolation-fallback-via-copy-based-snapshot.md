---
id: "ADR-034D9BG9UKuUaVNeCIKfNo"
type: "adr"
status: "accepted"
title: "Windows credential isolation fallback via copy-based snapshot"
created: "2026-10-09"
updated: "2026-10-09"
owners: []
relations:
  source:
    - "../discovery/devin-provider-windows-credential-isolation-limitation.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/devin-provider-windows-credential-contract.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/devin-local-provider-via-cli-delegated-auth-and-isolated-data-dir.md"
    - "../specs/devin-provider-windows-credential-contract.md"
    - "../plans/devin-provider-windows-credential-snapshot-sync.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/devin-provider-windows-credential-isolation-limitation.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: []
    consulted: []
    informed: []
---

# Windows credential isolation fallback via copy-based snapshot

## Context and Problem Statement

`cheaper-provider-devin` は Devin CLI を isolated env
（`XDG_DATA_HOME` + Windows は `LOCALAPPDATA` 併用）の child
process として起動し、credential を cheaper data dir に閉じ込める
設計だった（ADR-034r05iHrZUE6WqcIaH1CU）。しかし Devin CLI
3000.x（実測 3000.11.3）は Windows で credentials.toml を
known-folder API（RoamingAppData）で解決し、env override
（`XDG_DATA_HOME`・`LOCALAPPDATA`・`APPDATA`）を全て無視する。
`--config` flag は config.json のみ redirect、redirect 系 env
（`DEVIN_HOME` 等）も binary に存在しない。`USERPROFILE` override
は config 解決自体を壊す。すなわち Windows での真の隔離は原理的に
不可能であり、かつ現行の `devin auth logout` 無条件呼出しは global
session を破壊する副作用を持つ。

## Decision Drivers

* cheaper が API に使う `api_key` は cheaper 管理領域から供給する
  という原 contract を維持したい。
* `cheaper auth logout devin` が利用者の global Devin session を
  破壊してはならない（副作用の境界）。
* global session の account 切替・logout に cheaper credential が
  追随せず、stale copy を明示的に検出したい。
* 非 Windows 経路（`XDG_DATA_HOME` 隔離が機能する）を変えない。

## Considered Options

* Option 1: copy-based snapshot — login で global toml を cheaper
  dir へ copy、reuse は key 一致判定、logout は cheaper copy のみ
  削除
* Option 2: Windows では global toml を直接読む（隔離放棄）
* Option 3: Windows では login 不可とし manual copy のみ案内
  （現状維持 + logout 修正のみ）
* Option 4: Devin CLI 側の修正を待つ

## Decision Outcome

Chosen option: "Option 1: copy-based snapshot", because cheaper が
供給する `api_key` の供給源を cheaper 管理領域に保ちつつ、global
session への副作用（logout 破壊）を排除でき、reuse 判定で account
切替を検出できる唯一の選択肢だから。

### Consequences

* Good, because `read_devin_credentials`・`resolve_auth_material`・
  `refresh` の既存経路が変わらず、API key 供給源は cheaper toml
  に統一される。
* Good, because `devin auth logout` を Windows では呼ばないことで
  global session 保護が構造で担保される。
* Good, because key 一致判定は **login 境界**で global logout・
  account 切替後の stale copy 利用を防ぐ。runtime 経路では
  snapshot semantics として cheaper toml の key を pin し続け、
  stale 化は API auth failure として顕在化して次回 login で再同期
  する（この scope 限定は spec の Out of Scope に明記）。
* Good, because `devin auth status` の `File:` 行から global toml
  path を得ることで CLI 自身の known-folder 解決に追随でき、
  `%APPDATA%` env との乖離リスクを CLI 到達時には排除できる。
* Bad, because Windows では CLI 側の書込先は依然 global であり、
  「隔離」という表現が厳密には成立しない（snapshot 管理方式への
  緩和）。
* Bad, because global toml を freshness 判定のために読む（ただし
  key 供給には使わない）。
* Bad, because `PluginAuth::logout` に `AuthInteraction` を渡す
  trait signature 拡張が必要になる（impl は devin + test fake の
  みで変更面は小さい）。

### Confirmation

* Windows host で `cheaper auth login devin` → cheaper toml 生成と
  key 一致を確認。
* fake runner test で Windows contract の logout 経路が
  `devin auth logout` を argv に含まないことを検証（platform は
  injectable seam で Linux CI から検証）。
* key 一致/不一致・global 不在・login abort の reuse/copy 判定を
  unit test で検証。

## Implementation Plan

* Affected paths: `crates/cheaper-provider-devin/src/credentials.rs`
  （login・status・logout・global toml 解決・copy）、`plugin.rs`
  （logout notify・platform dispatch）、
  `crates/cheaper-runtime/src/plugin.rs`（`PluginAuth::logout`
  signature に `AuthInteraction` 追加）と
  `crates/cheaper-runtime/src/auth/service.rs`・
  `crates/cheaper-cli/src/auth.rs`（logout 呼出に interaction
  を供給）。
* Patterns to follow: `read_devin_credentials` の既存 parse 経路を
  global toml にも再利用。credential copy は
  `crates/cheaper-runtime/src/auth/store.rs` の temp + rename
  atomic write と同じ convention。platform 判定は
  `cfg!(windows)` を plugin 側の 1 箇所の dispatch に限定し、
  分岐先は platform flag を引数で受ける pure な関数とする。
* Patterns to avoid: global toml を API key 供給源にしない、
  Windows で `devin auth logout` を spawn しない、非 Windows
  経路の条件分岐を増やさない、`cfg!(windows)` を分岐先関数に
  散らばせない（CI が Ubuntu のため検証不能になる）、
  `%APPDATA%` env のみで global path を決め打ちしない
  （known-folder API との乖離リスク）。
* Dependencies or configuration: 新規依存なし。global path は
  `devin auth status` 出力の `File:` 行 parse で解決し、CLI
  未到達時のみ `%APPDATA%\devin\credentials.toml` に fallback。

## Verification

* [ ] `cargo test -p cheaper-provider-devin`（fake runner + 注入
  platform seam で login copy・exit-code gate・reuse 判定・
  logout 非 spawn・notify を検証）
* [ ] `cargo test --workspace`（`PluginAuth::logout` signature
  変更の非 regression）
* [ ] `mise run check`
* [ ] Windows host での手動 smoke（`cheaper auth login devin` →
  `auth logout devin` で global session が残ること）

## Pros and Cons of the Options

### Option 1: copy-based snapshot

* Good, because cheaper-owned credential 供給源を維持し、global
  session を破壊せず、account 切替を検出できる。
* Good, because 既存の credential 読取・refresh 経路を再利用できる。
* Bad, because copy の実装と key 一致判定という新しい概念が増える。
* Bad, because global toml の読取が新たに必要になる。

### Option 2: global toml を直接読む

* Good, because 実装が最小。
* Bad, because「global credential を暗黙利用しない」原 contract を
  破り、global session の account 切替にそのまま追随する。
* Bad, because cheaper credential store との整合が取れない
  （stored credential と実 key が乖離しうる）。

### Option 3: login 不可 + manual copy 案内（現状維持 + logout 修正）

* Good, because 変更が最小で安全側。
* Bad, because UX が悪く（毎回手動 copy）、login-reuse 判定が
  global session を見る誤判定問題は解消しない。

### Option 4: upstream 修正待ち

* Good, because 根本解決。
* Bad, because timeline 不明で現行版では issue が継続する。

## More Information

* 実測: `devin 3000.11.3` で `XDG_DATA_HOME`・`LOCALAPPDATA`・
  `APPDATA` override 全てが `C:\Users\<user>\AppData\Roaming\devin\
  credentials.toml` 解決を変えず、`--config` も credentials を
  redirect しない。`USERPROFILE` override は
  `Failed to determine config directory`。
* `docs/discovery/devin-provider-windows-credential-isolation-limitation.md`
* GitHub issue #80
