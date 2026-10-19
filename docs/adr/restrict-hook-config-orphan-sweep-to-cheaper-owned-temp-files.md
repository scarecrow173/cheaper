---
id: "ADR-034IOqXM5YpqVgFZKatIXN"
type: "adr"
status: "accepted"
title: "Restrict hook-config orphan sweep to cheaper-owned temp files"
created: "2026-10-19"
updated: "2026-10-19"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
    - "https://github.com/scarecrow173/cheaper/issues/128"
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
    - "./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-cli/tests/integration/hook_config.rs"
    - "../../scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# Restrict hook-config orphan sweep to cheaper-owned temp files

## Context and Problem Statement

`cheaper hook-config sync` は temp + rename atomic write の前に
`sweep_orphans`(`crates/cheaper-cli/src/hook_config.rs:482-503`)を
実行し、`{config_name}.` prefix + `.tmp`/`.bak` suffix に一致する
file を無条件で `remove_file` する(glob 条件 :496-497、呼出は
`run_sync` の upsert 前 :763)。削除した file 名は result の
`orphansRemoved` field に載る(:774-784)。sweep は upsert 成否・
変更有無にかかわらず毎回実行され、`--event-stdin` の APM
lifecycle 経路(`post-install`/`post-update` → `run_sync` +
`run_verify`、:1013-1019)でも発火する — 人間が sync を直接実行
しなくても自動収束の過程で走る。

しかし **`.bak` file を cheaper が書き出す経路は codebase 内に一切
存在しない**。`.bak` sweep の出自は foreign tool の残滓処理では
なく、旧 `install-devin-hooks.ps1` が
`[IO.File]::Replace(src, dst, backupPath)` の明示 backup path で
残し `finally` で除去していた自前 backup の crash orphan 回収を
Rust port が sweep 規約ごと引き継いだものである
(`docs/impl/ir/register-devin-hooks-from-apm-skill.md:113-115`・
`:139-142` — ADR-034H2WBNtC4We8dyC5lQZJ :142 の「書き込み規則は
PowerShell script の既存 contract を引き継ぐ」で継承)。旧 ps1 は
write-less thin wrapper(`hook-config` へ委譲)へ縮退し、自身の
atomic write が作るのは `config.json.<pid>.tmp` のみ
(`write_config_atomic` :529+、temp name 規約 :565-571 が
「sweep_orphans の `{name}.` prefix + `.tmp` suffix 規約に
合わせる」と明示) — `.bak` producer は消滅し、glob の本来目的は
extinct となった。つまり `.bak` 側の sweep 対象は全て外部/
ユーザー作成 file であり、`config.json.bak` のような手動 backup・
`config.json.pre-migration.bak` のような移行 snapshot・他 tool の
backup が sync 実行時に同意なく削除される(issue #137 —
#128 の review で accepted residual として記録済み)。

これは `hook-config` の所有境界の不変条件と矛盾する: sync の
note は "hook-config manages only the entry it owns; unrelated
hooks are preserved"(:863-870)と明示し、remove は strict-shape
でない entry を拒否する(:922-924)。「所有するものだけを触る」
境界に対し、`.bak` の削除は mutating command が所有 domain 外の
file を消す唯一の経路になっている。`orphansRemoved` による事後の
可視化は、同意のない不可逆削除を正当化しない。

verify 側の `find_orphans`(:506-525、glob :520)は同一条件で走査
するが read-only で削除しない。ただし finding 文言は
`"orphan: {o} (leftover temp/backup; safe to delete)"`(:711-713)
であり、外部 `.bak` を "safe to delete" と一律に断じている点は
文言として不正確。

## Decision Drivers

* **所有境界(deciding criterion)**: mutating command が削除して
  よいのは自身の ownership domain 内の file のみ。cheaper は
  `.bak` を一切生成しないため、削除対象に `.bak` を含める限り
  全ての対象が必然的に foreign file になる。削除対象を「cheaper
  が実際に生成しうる file」(`config.json.*.tmp`)と一致させる。
* **lifecycle 経路の収束**: primary use case は APM lifecycle
  `--event-stdin` 経由の自動収束であり、stdin は event JSON の
  みで flag 伝達路が無い。安全な挙動は flag 無しの既定動作で
  実現されなければならない(#128 で `--allow-conflict` が同じ
  理由で却下済み — ADR-034IKJq6tKWWAep0Q0ONzK)。
* **可視性の維持**: `.bak` の削除は止めるが、verify の
  `find_orphans` が `.bak` を informational orphan として報告
  し続けることで、sibling に残る backup/残骸の存在は隠さない。
* **後方互換**: sync result の `orphansRemoved` field と verify
  result の `orphans` field の名前・shape を変更しない。

## Considered Options

* Option 1: `sweep_orphans` の削除対象を `config.json.*.tmp` のみ
  に絞る。`.bak` は `find_orphans`/verify の informational 報告
  として維持する。
* Option 2: `.bak` 削除を維持し、cheaper 所有の命名 pattern(例:
  `config.json.<pid>.bak`)に限定する。
* Option 3: `.bak` 削除を opt-in flag(例: `--sweep-bak`)で制御する。
* Option 4: 現行維持 + 文書化のみ(コード変更なし)。

## Decision Outcome

Chosen option: "Option 1 — 削除対象を `config.json.*.tmp` のみに
限定"。削除対象を cheaper が実際に生成しうる file(atomic write
lifecycle の `config.json.<pid>.tmp` orphan)に一致させ、所有境界
の不変条件を根源から回復する唯一の選択肢であるため(issue #137
の提案 A、discovery の Option A 採択を確定)。

command 別 contract:

- `sync`: `sweep_orphans` は `{config_name}.` prefix + `.tmp`
  suffix のみを削除する。`.bak` は削除対象から完全に外す。
  `.tmp` orphan 回収という本来の設計意図は完全に保たれ、sweep
  の呼出位置(`run_sync` upsert 前)と「毎回実行」の規約は不変。
- `verify`: `find_orphans` は非破壊のまま走査条件を変更せず、
  `.bak` を informational orphan として報告し続ける。ただし
  finding 文言は `.tmp`(atomic write 残留 — sync で自動回収)と
  `.bak`(cheaper 管理外 file — 削除判断は user)で区分し、
  `.bak` への "safe to delete" 表現は廃止する。
- `--event-stdin`: `post-install`/`post-update` は `run_sync` を
  そのまま呼ぶため、新 sweep contract は lifecycle 経路へ自動
  継承される。コード変更・新入力経路は不要。
- `remove`: 挙動変更なし。

### Scope and Non-Goals

- `.bak` 削除用の opt-in flag・設定・env var は追加しない
  (Option 3 却下 — `--event-stdin` に flag 伝達路が無く dead
  weight になる)。
- `.bak` を cheaper 所有と見なす命名 pattern は定義しない
  (Option 2 却下 — cheaper は `.bak` を書かないため実在しない)。
- `find_orphans` の走査 glob の narrowing はしない — verify の
  `.bak` 報告は可視性として維持する(文言区分のみ修正)。
- `orphansRemoved`・`orphans` field の rename・構造変更、
  findings への severity field 等の新構造は導入しない
  (表現ドリフト回避・YAGNI)。
- `config.json.` prefix + `.tmp` suffix に一致する外部 file への
  追加防護はしない — glob は pid 成分を要求しないため
  `config.json.tmp`・`config.json.*.tmp` 命名の外部 file は依然
  sweep されうるが、`.bak` の任意 backup より衝突可能性は狭い。
  `config.json.<pid>.tmp` への strict な narrowing は実害報告時の
  将来候補とし、本変更の要件には含めない。
- 歴史的 doc の「`.bak` orphan sweep」記述の amend 方式・文言の
  詳細は spec(SPEC-034IOh5OATJ4AcprA5u47p)の R7 で確定する
  事項であり、本 ADR は所有境界の方向のみを固定する。

### Consequences

* Good, because mutating command が削除する file は cheaper が
  生成しうる `config.json.*.tmp` のみになり、own-entry-only
  所有境界の不変条件が回復する。`config.json.bak` 等の user
  file は sync・APM lifecycle のいずれでも削除されない。
* Good, because flag 伝達路の無い `--event-stdin` 経路を含め
  全経路で安全な既定動作になり、同意を得る channel を必要と
  しない。
* Good, because `orphansRemoved` の field 名・shape は不変で
  後方互換 — field に載りうるのは `.tmp` 名のみになる。
* Good, because verify の `.bak` 報告は残るため、sibling の
  backup/残骸の可視性は失われない。
* Bad, because 外部 tool が残した `config.json.*.bak` の掃除は
  cheaper の責務から外れる(もともと責務外と解釈すべき)。
* Bad, because verify の `.bak` orphan finding は informational
  のままで exit code に現れない — 文言区分で対象が foreign
  file である旨を明示して緩和する。

### Confirmation

* `.tmp` orphan は sync で削除され `orphansRemoved` に載り、
  `.bak` file は任意名でも削除されず `orphansRemoved` に含まれ
  ないことを integration test で固定する。
* `.bak` file が verify の `orphans` field と `orphan:` finding
  に報告され file は削除されないこと、および `.bak` の finding
  文言が "safe to delete" を含まないことを test で固定する。
* `--event-stdin` `post-install`・`post-update` の両経路で `.bak`
  が削除されない regression test を追加する(同一 code arm
  :1013-1019 を共有 — lifecycle 経路での contract 継承)。
* `mise run check` が pass すること。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-cli/src/hook_config.rs`
    - `sweep_orphans`(:482-503): glob 条件 :496-497 から
      `.bak` suffix を除去し `{name}.` prefix + `.tmp` suffix
      のみにする。doc comment(:480-481)の記述も整合させる。
    - `run_verify`(:707-719): `orphan:` finding 文言(:711-713)
      を `.tmp`/`.bak` で区分し、"leftover temp/backup; safe to
      delete" の一律表現を廃止する。走査 glob・`orphans` field
      は変更しない。
    - `run_sync`(:763・:774-784)・`run_event_stdin`(:1013-1019)
      はコード変更不要 — 新 contract は `sweep_orphans` 経由で
      自動継承される。
  - `crates/cheaper-cli/tests/integration/hook_config.rs`
    - sweep の `.tmp` 削除 / `.bak` 保全、混在時に
      `orphansRemoved` が `.tmp` 名のみを含むこと、verify の
      `.bak` 報告と文言区分、`--event-stdin` 経路の `.bak` 保全
      を pin する test を新規追加(現状 orphan sweep の coverage
      はゼロ)。
  - `scripts/test/devin-hooks-install.test.mjs`
    - `.bak` 系 assertion は現状無い。必要に応じて install
      lifecycle 経路での `.bak` 保全を確認する。
  - 歴史的 doc(`.bak` sweep を現行挙動として記述する文 — spec
    R7 の amend 対象。amend は `.bak` 除去のみならず「起動時 +
    失敗時に実行」の記述も「sync 開始時に 1 回実行 — 失敗時
    sweep なし」へ正す):
    - `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md:147`
    - `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md:274`
    - `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:149`
      および `:265`
* Patterns to follow:
  - own-entry-only 所有境界(sync note :863-870、remove の
    strict-shape 拒否 :922-924)— 削除対象を「自身が生成しうる
    file」に一致させる。
  - `write_config_atomic` の temp name 規約(:565-571)—
    `{name}.<pid>.tmp` は sweep 規約と対称に設計済みであり、
    その対称を保つ。
  - JSON-first CLI contract — result field の追加・文言変更で
    報告し、field 名・shape を変えない(表現ドリフト回避)。
  - informational finding と blocker の区分は #128 系
    (ADR-034IKJq6tKWWAep0Q0ONzK)の降格規約と同じ構造を保つ。
* Patterns to avoid:
  - `.bak` 削除用の新規 CLI flag(`--sweep-bak` 等)— lifecycle
    経路で届かず dead weight になる(#128 の `--allow-conflict`
    却下と同一理由)。
  - `find_orphans` の走査条件の narrowing — `.bak` 報告は非破壊
    の可視性として残す。
  - findings の機械可読区分(severity field・別 field)の導入 —
    string finding + `orphans` field で足りる(YAGNI)。
  - 外部 tool の同名 `.tmp` を警戒した sweep 条件の追加拡張 —
    glob は `config.json.` prefix + `.tmp` suffix のみを要求し、
    `config.json.<pid>.tmp` への strict narrowing は実害報告が
    出た時点の将来候補(YAGNI)。
* Dependencies or configuration: 新規依存・新規 env var・新規
  flag なし。
* Expected tests:
  - `config.json.<pid>.tmp` 相当の orphan 存在下で sync → 削除 +
    `orphansRemoved` 記載。
  - `config.json.bak`・`config.json.manual.bak` 存在下で sync →
    両 file 残存・`orphansRemoved` に `.bak` 名を含まない。
  - `.tmp` + `.bak` 混在で sync → `.tmp` のみ削除。
  - `.bak` 存在下で verify → `orphans` field + `orphan:` finding
    に報告・file 非削除・文言が "safe to delete" を含まない。
  - `--event-stdin` `post-install`・`post-update` payload 経路で
    `.bak` 非削除。

## Verification

* [ ] `config.json.` prefix + `.tmp` suffix の orphan file が存在
      する状態で `cheaper hook-config sync` を実行すると、当該
      file が削除され result の `orphansRemoved` に載ることが
      `crates/cheaper-cli/tests/integration/hook_config.rs` の
      test で確認される。
* [ ] `config.json.bak`・`config.json.manual.bak` 等の任意
      `*.bak` が存在する状態で sync を実行しても file が残存し、
      `orphansRemoved` に `.bak` 名が一切含まれないことが test
      で確認される。
* [ ] `.tmp` orphan と `.bak` が混在する状態で sync した場合、
      `.tmp` のみ削除され `orphansRemoved` が `.tmp` 名のみを
      含むことが test で確認される。
* [ ] `.bak` 存在下で `cheaper hook-config verify` を実行すると
      `orphans` field と `orphan:` finding に file 名が報告され、
      file が削除されないことが test で確認される。
* [ ] `.bak` orphan の finding 文言が "safe to delete" を含まず
      cheaper 管理外である旨(削除判断は user)を伝える文言、
      `.tmp` orphan の文言が sync による自動回収を示す文言に
      なっていることが test で確認される。
* [ ] `hook-config --event-stdin` に `post-install`・`post-update`
      の各 payload を与えた両経路で `.bak` file が削除されない
      ことが test で確認される。
* [ ] 歴史的 doc 3 箇所の記述が「sweep 対象 `config.json.*.tmp`
      のみ・sync 開始時に実行(失敗時 sweep なし)」の contract
      へ amend されていることが review で確認される(spec R7)。
* [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration` と `mise run check` が pass する。

## Pros and Cons of the Options

### Option 1 — 削除対象を `config.json.*.tmp` のみに限定(chosen)

* Good, because 削除対象を「cheaper が実際に生成しうる file」に
  一致させ、所有境界の不変条件を最小変更で回復できる。`.tmp`
  orphan 回収の設計意図は完全に保たれ、verify の可視性も残る。
  lifecycle 経路を含め全経路で安全な既定動作になる。
* Bad, because 外部 tool が残した `config.json.*.bak` の掃除は
  cheaper の責務から外れる(もともと責務外と解釈すべき)。verify
  の finding 文言は `.bak` 向けに修正が要る。

### Option 2 — `.bak` 削除を cheaper 所有の命名 pattern に限定

* Good, because `.bak` 掃除の構造は残る。
* Bad, because cheaper は `.bak` を一切書かないため、その
  pattern に一致する file は現実に存在しない — Option 1 と
  同等の効果で pattern 定義・保守コストだけが乗る(YAGNI)。

### Option 3 — `.bak` 削除を opt-in flag で制御

* Good, because 挙動を変えずに危険性だけ抑制できる。
* Bad, because `--event-stdin` の lifecycle 経路には flag 伝達路
  が無く(stdin は event JSON のみ)、primary use case である APM
  自動収束では使えない dead weight になる — #128 で
  `--allow-conflict` が同じ理由で却下済み
  (ADR-034IKJq6tKWWAep0Q0ONzK)。default を維持すれば結局
  user file は消えるため deciding criterion を満たさない。

### Option 4 — 現行維持 + 文書化のみ

* Good, because コード変更不要。
* Bad, because 「同意なくユーザー file を削除する」挙動は文書化
  しても所有境界違反のままで、`orphansRemoved` を見るまで検知
  不能。破壊的操作を docs で正当化する形になり、根因を解消
  しない。

## More Information

* issue #137: <https://github.com/scarecrow173/cheaper/issues/137>
  — 本決定の起点(`.bak` sweep の所有境界問題と修正提案。#128
  の review で accepted residual として記録済み)。
* upstream discovery:
  [hook-config-sweep-orphans-deletes-foreign-bak-files.md](../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md)
  (DISC-034IO86RVnmQ8AU5Lac8bw) — 検証済み file:line 証跡・
  alternatives 比較(A/B/C/D)・open questions を含む。
* sibling spec:
  [hook-config-orphan-sweep-ownership-boundary.md](../specs/hook-config-orphan-sweep-ownership-boundary.md)
  (SPEC-034IOh5OATJ4AcprA5u47p) — 本決定を R1-R7 の contract
  へ展開した並行 spec。文言区分・test 要件・歴史的 doc の amend
  方式の詳細はこちらで確定する。
* **ADR-034H2WBNtC4We8dyC5lQZJ との関係**: 本 ADR は
  [cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md](./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md)
  (issue #107)の `.bak` orphan sweep 記述(:147)を部分的に
  amend する。ただし旧 ADR の core 決定(CLI が所有する
  registration lifecycle・atomic write・own-entry-only 境界)は
  維持されるため `supersedes`/`superseded-by` ではなく `related`
  のみで link し、amend 関係は本文で記述する。`refines`/
  `refined-by` は route_graph の lineage edge であり、chain の
  異なる doc 間で張ると focus component が merge されるため
  使わない(前 chain の ADR-034IKJq6tKWWAep0Q0ONzK と同じ規約)。
  旧 ADR の status と決定本文の位置付けは変更しない —
  加えるのは front matter `related` への back-link と、body 内の
  `.bak` sweep 記述文(:147-149)の R7 amend による訂正のみ。
* issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  — 本問題を accepted residual として記録した review 元。
  「`--event-stdin` に flag 伝達路が無い」制約は Option 3 却下の
  根拠として継承(関連 ADR:
  [hook-config-legacy-conflict-gate-demotion-to-warning.md](./hook-config-legacy-conflict-gate-demotion-to-warning.md)
  ADR-034IKJq6tKWWAep0Q0ONzK)。
* issue #107: <https://github.com/scarecrow173/cheaper/issues/107>
  — hook-config ownership / lifecycle の導入元。
* spec の Deferred Design Concerns は「dual-track ADR は不要と
  見なす」と記録していたが、本変更は mutating command の所有
  境界という不変条件の再定義であり、chain の判断で decision
  record として本 ADR を切ることになった。spec 側の R7 amend・
  test 要件と矛盾する内容は無い。
