---
id: "TSPEC-034IQa6tpizWQ1s8A9Dvh0"
type: "test-spec"
status: "proposed"
title: "hook-config orphan sweep ownership contract"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
    - "https://github.com/scarecrow173/cheaper/issues/128"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
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
    - "docs/plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  derived-by: []
  verifies:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  verified-by: []
  references:
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
  defers: []
  deferred-by: []
---

# hook-config orphan sweep ownership contract

## Purpose

issue #137 に対して、PLAN-034IQ9W3TwoNQIp08Cn0SS が実装する orphan sweep
所有境界 narrowing が SPEC-034IOh5OATJ4AcprA5u47p の R1-R6 と対応
Acceptance Criteria を満たすことを固定する test contract。具体的には「`hook-config
sync` が削除するのは cheaper が実際に生成しうる `config.json.*.tmp` のみで
あり、`.bak` suffix の file(手動 backup・移行 snapshot・他 tool の残滓 —
producer extinct のため全て foreign file)は sync・verify・`--event-stdin`
lifecycle のいずれの経路でも一切削除されない」こと、および「verify は
`.bak` を informational orphan として報告し続ける(read-only の非破壊的
可視性)が、その finding 文言は管理外である旨を伝え "safe to delete" を
含まない」ことを保証する。

本 test spec は、sweep の削除対象が再び cheaper 非生成の file(`.bak` 等)
へ広げられたとき・verify の `.bak` informational 報告が除去されたとき・
または atomic write の temp 命名規約が変わって `{name}.` prefix + `.tmp`
suffix 対称が崩れたときに retire / 再検討してよい。それまでは spec/design
が再編されても「mutating command が削除するのは自身の所有 domain 内の
file のみ」という所有境界の不変条件を保存する役割を持つ。

## Feature

`cheaper hook-config` の orphan 扱い: `sweep_orphans` は `{config_name}.`
prefix + `.tmp` suffix に一致する sibling file のみを sync 開始時
(`run_sync` の upsert 前、毎回実行)に物理削除して `orphansRemoved` へ
載せ、`find_orphans` は `{config_name}.` prefix + `.tmp`/`.bak` suffix の
走査条件を維持して verify の `orphans` field + `orphan:` finding へ
report する(read-only・削除しない)。finding 文言は `.tmp`(atomic write
残留 — 次回 sync で自動回収)と `.bak`(cheaper 管理外 — 削除判断は user)
で区分される。`--event-stdin` の `post-install`/`post-update` は同一 code
arm で `run_sync` + `run_verify` を呼ぶため同じ contract を継承する。

## Rules

### Test layer(全 Rule に共通する前提)

- **Rust integration 層**: `tests/integration/hook_config.rs` の既存
  `fixture_with_cheaper_on_path` PATH shim パターン(`CARGO_BIN_EXE_cheaper`
  を tempdir `bin/` へ copy し `PATH` を shim のみに固定)と
  `fx.run(&["hook-config", ...])` の in-process `run_cli` 経路、および
  `fx.stdin.data` 差替えによる `--event-stdin` 連続実行 pattern を再利用
  し、exit code・envelope(`ok`・`result` map)・file 存在を assert する。
  sibling orphan file 作成用に小さな helper(`write_orphan_file(fx, name)`
  相当 — `.devin/` dir 作成 + 任意名 file 書込み)を足す。

### AC 対応 Rule

- Rule(R1-R3 / AC: `.tmp` sweep): `config.json.` prefix + `.tmp` suffix の
  orphan(`config.json.12345.tmp` 等)が存在する状態で `sync` を実行すると、
  当該 file が削除され `result.orphansRemoved` にその file 名が載り
  exit 0 となる。`.tmp` orphan 回収という本来の設計意図は維持される。
- Rule(R1・R3 / AC: `.bak` 保全): `config.json.bak`・
  `config.json.manual.bak` のように suffix が `.bak` の任意名 file は
  sync の前後を通じて一切削除されず、`orphansRemoved` に `.bak` 名を
  一切含まない(orphan 無しなら key 不在 — 出現条件は削除 1 件以上で
  insert の不変規約)。`.tmp`+`.bak` 混在時も `.tmp` のみが削除され
  `orphansRemoved` は `.tmp` 名のみを含む。
- Rule(R4 / AC: event-stdin): `--event-stdin` の `post-install`・
  `post-update` 両経路で `.bak` file が削除されない — 同一 code arm が
  `run_sync` を呼ぶため lifecycle 経路の contract 継承を pin する。
- Rule(R5 / AC: verify 報告): `.bak` file が存在する状態で `verify` を
  実行すると `result.orphans` field と `orphan:` prefix の finding に
  当該 file 名が報告され、file は削除されない。`.tmp` orphan も verify
  後に残存する(read-only 契約)。`orphan:` finding は informational で
  `clean` flag・exit code に影響しない(blocker 系 finding の有無のみが
  `clean` を決める)。
- Rule(R6 / AC: finding 文言区分): `.bak` orphan の finding 文言は
  "safe to delete" を含まず、「cheaper が管理しない file — 不要なら手動
  で削除」旨(例: "not managed by cheaper; remove manually if unwanted")
  を伝える。`.tmp` orphan の finding は「atomic write の残留 — 次回
  `hook-config sync` で自動回収」旨(例: "auto-removed by the next
  `hook-config sync`")を示す文言。`orphan:` prefix・findings の string
  array 形式は維持し、severity field 等の新構造は導入しない。
- Rule(後方互換): `orphansRemoved`(sync result)・`orphans`(verify
  result)の key 名・shape(string array・file 名)・出現条件は不変。
  変わるのは `orphansRemoved` に載りうる値の範囲(`.tmp` 名のみ)と
  finding 文字列の文言のみ。

## Examples

- `hook_config_sync_sweeps_tmp_orphan` — `config.json.12345.tmp` を sibling
  に配置 → `sync` → exit 0・file 削除済み・`result.orphansRemoved` が
  `config.json.12345.tmp` を含む。
- `hook_config_sync_preserves_bak_files` — `config.json.bak` と
  `config.json.manual.bak` を配置 → `sync` → exit 0・両 file 残存・
  `orphansRemoved` が `.bak` 名を一切含まない(削除対象が無いため key
  自体が不在でもよい)。
- `hook_config_sync_sweeps_only_tmp_among_mixed` — `config.json.<x>.tmp` +
  `config.json.bak` 混在 → `sync` → `.tmp` のみ削除・`.bak` 残存・
  `orphansRemoved` が `.tmp` 名のみ。
- `hook_config_verify_reports_bak_orphan_informational` — 先に converged
  config(own entry 登録済み)を用意した上で `config.json.bak` 配置 →
  `verify` → `result.orphans` と `orphan:` finding が
  `config.json.bak` を含む・file は削除されない・該当 finding が
  "safe to delete" を含まず "not managed by cheaper" を含む・
  `.bak` orphan 単独では `clean:true`・exit 0(informational — config
  不在時は `absent-config` blocker が `clean:false` にするため config
  前提が必須)。
- `hook_config_verify_tmp_orphan_wording_and_readonly` —
  `config.json.<x>.tmp` 配置 → `verify` → `orphan:` finding が
  `auto-removed` と `sync` を含む文言・verify 実行後も `.tmp` file が
  残存する(verify が read-only であることの pin)。
- `hook_config_event_stdin_preserves_bak` — `config.json.bak` 配置 →
  stdin `{"event":"post-install","packages":["cheaper"]}` → exit 0・
  `.bak` 残存・`result.sync.orphansRemoved` に `.bak` 名を含まない。
  続けて `{"event":"post-update","packages":["cheaper"]}` → 同様に
  `.bak` 残存(両 event が同一 arm の contract を継承することを pin)。

## Guarantees

- mutating command(sync・`--event-stdin` lifecycle)が削除する file は
  cheaper が生成しうる `config.json.*.tmp` のみ — `.bak` file は任意名
  で一切削除されず、own-entry-only 所有境界の不変条件が sweep にも一致
  する。
- `.bak` file の存在は verify の `orphans` field・`orphan:` finding で
  常に可観測のまま — 削除を止めても sibling の残骸・backup の可視性は
  失われない。
- `.bak` orphan の finding は foreign file を「safe to delete」と断じない
  — cheaper 管理外である旨と削除判断が user にあることを伝える。
- `orphansRemoved`・`orphans`・`findings` の key・shape・出現条件は不変
  で後方互換 — `.bak` が混在しても `orphansRemoved` には `.tmp` 名のみ
  が載る。
- 全経路で同一挙動 — sync 直接実行・`post-install`・`post-update` の
  いずれでも同じ predicate が効き、flag 伝達路の無い lifecycle 経路でも
  安全な既定動作になる。

## Non-goals

- `find_orphans` の走査条件 narrowing の検証 — glob は `.tmp`/`.bak` を
  含む現行維持であり、`.bak` の informational 報告は本 contract が保持を
  要求する側(R5)。
- `config.json.` prefix + `.tmp` suffix に一致する外部 file(非 pid 命名
  `.tmp`)の巻き込み防護 — spec/design が受容した残存リスクであり、
  strict `config.json.<pid>.tmp` narrowing は deferred concern。
- 複合 suffix・case の edge(`config.json.bak.tmp` は `.tmp` 終端のため
  削除対象、`config.json.BAK` は case sensitive で無視)— 挙動は規約
  どおりだが専用 test は張らない。
- `remove_file`/`read_dir` 失敗時の経路(現行構造の維持で新規分岐なし)、
  blocker 系 finding と `clean` の関係全般(`orphan:` が informational
  である点のみ pin し、blocker semantics は legacy-conflict 系 test が
  担保)。
- R7 の歴史的 doc amend・SKILL.md clarify — 文書整合の要件であり実行時
  挙動の guarantee ではないため本 test spec の検証対象外(plan/task T2 と
  review で担保)。
- `.bak` 削除の opt-in flag・severity field・findings 機械可読区分 —
  spec/design の Out of Scope(却下済み)であり検証対象外。

## Risk

- 本 contract を外すと以下を回帰として検出できない: (a) predicate の誤
  拡大 — `.bak` や他 suffix が削除対象に復帰して user file が同意なく
  消える(issue #137 の復活)、(b) `.bak` 報告の消失 — `find_orphans`
  glob を同期して narrow すると verify から残骸の可視性が消える、(c)
  文言の誤誘導 — `.bak` finding が "safe to delete" を含むまま戻り
  foreign file を安全と断じる、(d) lifecycle 経路の乖離 — event-stdin
  arm が別経路を通って sync 直実行と異なる sweep 挙動になる、(e)
  `orphansRemoved` の shape 変化 — field 名・出現条件の変更で後方互換が
  壊れる。いずれも「所有するものだけを削除し、管理外は報告のみ」という
  所有境界の意図を静かに壊す。
