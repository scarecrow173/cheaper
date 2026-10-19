---
id: "SPEC-034IOh5OATJ4AcprA5u47p"
type: "spec"
status: "approved"
title: "Restrict hook-config orphan sweep to cheaper-owned temp files"
created: "2026-10-19"
updated: "2026-10-19"
owners: ["scarecrow173"]
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
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "../discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
  derived-by:
    - "../designs/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# Restrict hook-config orphan sweep to cheaper-owned temp files

## What Should Be Built

`cheaper hook-config sync` が実行する orphan sweep の**削除対象を、
cheaper 自身が実際に生成しうる `config.json.*.tmp` のみ**に限定する。
`.bak` suffix の file は sweep の削除対象から外す。

背景: `sweep_orphans`（`crates/cheaper-cli/src/hook_config.rs:482-503`）
は `run_sync` の upsert 前に毎回呼ばれ（:763）、`{config_name}.`
prefix + `.tmp`/`.bak` suffix に一致する file を無条件で
`remove_file` する（glob 条件 :496-497）。この `.bak` 掃除の出自は
foreign tool の残滓処理ではなく、旧 `install-devin-hooks.ps1` が
`[IO.File]::Replace(src, dst, backupPath)` の明示 backup path で
作成し `finally` で除去していた自前 backup file の crash orphan
回収を、Rust port が sweep 規約ごと引き継いだものである
（`docs/impl/ir/register-devin-hooks-from-apm-skill.md:113-115`・
`:139-142`、旧 ADR
`docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md:142`
「書き込み規則は PowerShell script の既存 contract を引き継ぐ」）。
旧 ps1 は現在 write-less thin wrapper（`hook-config` へ委譲）へ
縮退し、Rust の atomic write が作るのは `config.json.<pid>.tmp`
のみ（`write_config_atomic` :529+、temp name 規約 :565-571）—
`.bak` producer は codebase 内に一切残っていない。つまり `.bak`
glob の本来目的は消滅しており、現在の `.bak` 側 sweep 対象は全て
外部/ユーザー作成 file — `config.json.bak` や
`config.json.pre-migration.bak` のような手動 backup、他 tool の
backup が sync 実行時に同意なく削除される（issue #137）。

求める振る舞い（discovery の Option A を採択）:

1. `sweep_orphans` は `config.json.*.tmp` のみを削除する — 「自身の
   atomic write が残しうる file」という所有境界と削除対象を一致
   させる。
2. verify 側の `find_orphans`（:506-525、glob :520）は現行条件を
   維持し、`.bak` を informational orphan として報告し続ける
   （read-only・非破壊の可視性は残す）。ただし finding 文言の
   "safe to delete"（:711-713）は foreign `.bak` に対して不正確
   なため修正する。
3. sync result の `orphansRemoved` field（:774-784）と verify
   result の `orphans` field（:716-719）は名前・shape を維持する。
4. `.bak` sweep を現行挙動として記述した歴史的 doc 3 箇所の記述文を、
   新しい所有境界（対象 `config.json.*.tmp` のみ・sweep は sync
   開始時に実行 — 「起動時 + 失敗時」の記述も不正確）に合わせて
   amend する（R7）。

却下した代替案（discovery の alternatives 評価を確定）:

- **Option B（`.bak` を cheaper 所有の命名 pattern に限定）**:
  cheaper は `.bak` を一切書かないため、その pattern に一致する
  file は現実に存在しない — Option A と同等の効果で pattern 定義・
  保守コストだけが乗る（YAGNI）。
- **Option C（`--sweep-bak` 等の opt-in flag）**: `--event-stdin`
  の APM lifecycle 経路には flag 伝達路が無く、primary use case
  で使えない dead weight になる — #128 で `--allow-conflict` が
  同じ理由で却下済み。
- **Option D（現行維持 + 文書化のみ）**: 「同意なくユーザー file
  を削除する」挙動は文書化しても所有境界違反のままであり、
  `orphansRemoved` を見るまで検知不能 — 根因を解消しない。

## Why Now

- issue #137 で報告済みの実害: `.devin/config.json` sibling に
  置かれた任意 `*.bak` が `hook-config sync` で黙って削除される。
  ユーザーが手動退避した backup や移行前 snapshot まで消える
  不可逆なデータ損失であり、発生に気づく術は result の
  `orphansRemoved` を事後確認するしかない。
- 本問題は #128 の review で accepted residual として記録済みで、
  #128 系（legacy conflict gate 降格）が settle した今が対処
  タイミング。
- sweep は `run_sync` が呼ばれる全経路で発火する。特に APM
  lifecycle の `--event-stdin`（`post-install`/`post-update` →
  `run_sync` + `run_verify`、:1013-1019）でも走るため、人間が
  sync を直接実行しなくても自動収束の過程で `.bak` が消える —
  同意を得る channel が一切存在しない。
- 所有境界の不変条件と矛盾する: `hook-config` は own entry のみを
  管理し無関係な file・登録を保全する規約（sync note
  "hook-config manages only the entry it owns; unrelated hooks are
  preserved" :863-870、remove の strict-shape 拒否 :922-924）。
  `.bak` 削除は「所有するものだけを触る」境界を越える。

## Users and Value

- **cheaper を consumer project に導入する user / parent coding
  agent**: `.devin/` に置いた手動 backup・移行 snapshot が sync や
  APM lifecycle で消失しなくなる。hook 設定 dir の file は自分が
  作ったものも含めて制御下に置ける。
- **APM lifecycle 経路（package install/update の自動収束）**:
  flag 伝達路なしでも安全な挙動がデフォルトになる — 破壊的操作を
  consent なしで行う経路が消える。
- **cheaper CLI maintainer**: verify の `.bak` 報告は残るため
  visibility は失われず、削除対象が「自分が生成しうる file」のみに
  一致して sweep の責務が明確になる。

## Scope

### In Scope

- `sweep_orphans`（`crates/cheaper-cli/src/hook_config.rs:482-503`）
  の削除条件から `.bak` suffix を除外し、`config.json.*.tmp` のみを
  削除対象にする変更。doc comment（:480-481）の整合も含む — 同
  comment は 2 箇所が stale: (a) 削除対象の記述が
  `config.json.<pid>.tmp`/`config.json.*.tmp`/`*.bak` となっている
  ため、実際の glob である `{name}.` prefix + `.tmp` suffix
  （`config.json.*.tmp` 相当）に修正する、(b) 戻り値の説明
  「verify の finding 用」は誤りで、実際は sync result の
  `orphansRemoved` field へ渡る値（verify が使うのは
  `find_orphans`）である旨に修正する。
- verify の `find_orphans` 報告経路の文言修正 — `orphan:` finding
  （:711-713）を `.tmp`（atomic write 残留、sync で自動回収）と
  `.bak`（cheaper 管理外 file、削除判断は user）で区分し、
  `.bak` への "safe to delete" 表現を除去する。走査 glob 自体は
  変更しない。
- 新規 integration test の追加
  （`crates/cheaper-cli/tests/integration/hook_config.rs`）—
  sweep の `.tmp` 削除 / `.bak` 保全、verify の `.bak` 報告、
  `orphansRemoved` の内容を pin する。現状 orphan sweep の test
  coverage はゼロ（`hook_config.rs`・
  `scripts/test/devin-hooks-install.test.mjs` のいずれにも
  `.bak`/orphan 系 assertion が無い）。
- 歴史的 doc の `.bak` sweep 記述の amend — 現行挙動を記述する文を
  所有境界に合わせて訂正する（対象は R7）。

### Out of Scope

- `.bak` 削除を opt-in で制御する flag・設定・環境変数の追加
  （Option C — `--event-stdin` に flag 伝達路が無く dead weight、
  #128 の `--allow-conflict` と同一理由で却下）。
- `.tmp` sweep 挙動自体の変更（タイミング・prefix 条件・毎回実行の
  現行規約は維持）。
- `remove` command の scope 変更（own entry のみ除去の現行維持）。
- `find_orphans` の走査条件の narrowing — verify が `.bak` を報告
  し続けること自体は維持する（非破壊の可視性）。
- `orphansRemoved`・`orphans` field の rename・構造変更、findings
  への severity field 等新構造の導入（表現ドリフト回避・YAGNI）。
- `config.json.` prefix + `.tmp` suffix に一致する外部 file への
  追加防護 — glob が要求するのは prefix + `.tmp` suffix のみで pid
  成分は条件に無いため、`config.json.tmp` や `config.json.<任意>.tmp`
  命名の外部 file は依然削除されうるが、`.bak` の任意 backup より
  衝突可能性は狭い（Deferred Design Concerns 参照）。
- issue #128 系の legacy conflict gate・dedup 機構への変更
  （cross-chain。`related` 参照のみ）。

## Requirements

### Sweep Contract（sync）

- R1: `sweep_orphans` の削除対象を `{config_name}.` prefix +
  `.tmp` suffix のみに限定する（現行 :496-497 の `.bak` suffix
  条件を除去）。`config.json.bak`・`config.json.manual.bak`・
  `config.json.pre-migration.bak` 等の任意 `*.bak` file は sync
  の前後を通じて一切削除されない。
- R2: `.tmp` orphan の回収は維持する — `write_config_atomic` が
  作りうる `config.json.<pid>.tmp`（temp name 規約 :565-571 が
  「sweep_orphans の `{name}.` prefix + `.tmp` suffix 規約に
  合わせる」と明示している対称）の中途残留を削除する。sweep の
  呼出位置（`run_sync` の upsert 前、:763）と「upsert 成否・
  変更有無にかかわらず毎回実行」の現行規約は変更しない。
- R3: sync result の `orphansRemoved` field は key 名・shape
  （string array、削除した file 名）・出現条件（削除 1 件以上で
  insert、:774-784）を変更しない。field に載る内容は `.tmp` 名
  のみとなる — `.bak` が混在していても含まれない。
- R4: `--event-stdin` の `post-install`/`post-update` は
  `run_sync` をそのまま呼ぶ（:1015）ため、新 sweep contract は
  lifecycle 経路へ自動的に継承される。event payload・subresult
  envelope 構造（`sync`/`verify` key + verify exit code
  passthrough）の変更はなく、flag 等の新入力経路も追加しない。

### Verify Contract

- R5: `find_orphans` の走査条件（`{config_name}.` prefix +
  `.tmp`/`.bak` suffix、:520）は変更しない。`.bak` file は
  引き続き `orphans` field（:716-719）と `orphan:` finding に
  報告される。verify が file を削除することはない（read-only
  維持）。
- R6: orphan finding の文言を `.tmp` / `.bak` で区分する。
  `.tmp` orphan は「cheaper の atomic write の残留 — 次回 sync で
  自動回収される」旨、`.bak` orphan は「cheaper が管理しない file
  — 不要なら手動で削除」旨を伝える文言とし、現行の一律
  "leftover temp/backup; safe to delete"（:711-713）は廃止する。
  `orphan:` prefix・findings の string array 形式・informational
  扱い（`clean` flag に影響しない）は維持し、severity field 等の
  新構造は導入しない。

### Documentation

- R7: `.bak` sweep を現行挙動として記述する 3 箇所の記述文を amend
  する。対象:
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md:274`
  （requirement R17 内）、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:149`
  および `:265`、
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md:147`。
  amend は `.bak` 部分だけでなく記述文の不正確な節全体を正す:
  cli-owned ADR :148-149 と bounded design :151-152 は sweep を
  「起動時 + 失敗時」に行うと記述するが、実装では sweep は sync
  開始時に 1 回実行されるのみ（`hook_config.rs:763`）で失敗時
  sweep は存在しない — rename 失敗時の temp 除去は
  `write_config_atomic` 自身（:574-575）が担う。訂正後の文は
  「sweep 対象は cheaper が生成しうる `config.json.*.tmp` のみ」
  「sweep は sync 開始時に実行（失敗時 sweep なし）」の 2 点を
  述べるものとする。対象文は現在の挙動を定める規範的記述である
  （bounded spec :274 は descriptive sentence ではなく requirement
  R17 の一部）ため、drift-note 併記ではなく文そのものを訂正する
  方式を採る — ただし訂正には本 spec / issue #137 への dated
  amend 参照を付し、decision record としての provenance を消さ
  ない。なお別 chain の時点記録 —
  `docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md:258-262`
  ・
  `docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md:268`
  — にも `.bak` sweep 言及があるが、これらは #128 変更時点の
  side-effect を記述した historical change record であり現在挙動の
  記述ではないため amend 対象から除外する。

## Acceptance Criteria

- [ ] `config.json.<pid>.tmp` 相当（`config.json.` prefix +
  `.tmp` suffix）の orphan file が存在する状態で
  `hook-config sync` を実行すると、当該 file が削除され result
  の `orphansRemoved` にその file 名が載ることが
  `crates/cheaper-cli/tests/integration/hook_config.rs` の test で
  固定される。
- [ ] `config.json.bak` と `config.json.manual.bak`（suffix が
  `.bak` の任意名）が存在する状態で `hook-config sync` を実行
  しても、両 file が削除されず残存し、`orphansRemoved` に `.bak`
  名が一切含まれないことが test で固定される。
- [ ] `.tmp` orphan と `.bak` file が混在する状態で sync した
  場合、`.tmp` のみが削除され `orphansRemoved` が `.tmp` 名のみ
  を含むことが test で固定される。
- [ ] `.bak` file が存在する状態で `hook-config verify` を実行
  すると、`orphans` field と `orphan:` finding に当該 file 名が
  報告され、file が削除されないことが test で固定される。
- [ ] verify の `.bak` orphan finding 文言が "safe to delete" を
  含まず、cheaper 管理外である旨（手動削除の判断は user）を伝える
  文言になっていることが test で固定される。`.tmp` orphan の
  finding は sync による自動回収を示す文言であることも併せて
  固定される。
- [ ] `hook-config --event-stdin` に `post-install` と
  `post-update` の各 payload を与えた両経路で `.bak` file が
  削除されないことが test で固定される（同一 code arm
  :1013-1019 を共有する両 event を pin — lifecycle 経路での同一
  contract 継承、R4）。
- [ ] R7 の 3 箇所の記述が「sweep 対象 `config.json.*.tmp` のみ・
  sweep は sync 開始時に実行（失敗時 sweep なし）」の contract
  へ amend され、本 spec / issue #137 への参照を伴うことが
  確認される。

## Deferred Design Concerns

- `config.json.` prefix + `.tmp` suffix に一致する外部 file の
  取り扱い | 外部 tool が同一命名規則の `.tmp` を残す可能性は理論
  上ある。glob が要求するのは `config.json.` prefix + `.tmp`
  suffix のみで pid 成分は条件に含まれないため、
  `config.json.tmp`・`config.json.backup.tmp`・`config.json..tmp`
  命名の外部 file は post-fix でも依然 sweep される — `.bak` の
  任意 backup より衝突可能性は狭いがゼロではなく、この残存
  リスクを受容する。`config.json.<pid>.tmp` への strict な
  narrowing は追加防護として YAGNI とし、実害報告が出た場合の
  将来の tightening 候補とする（本変更の要件には含めない）|
  外部 file の同名衝突被害が報告された時点 | 残存する理論上の
  sweep 巻き込みリスク。
- orphan finding の機械可読区分（`.tmp`/`.bak` 別 field、severity
  構造化） | 現行の string finding + `orphans` field で足りており
  追加構造は YAGNI — #128 spec と同じ findings 構造維持判断 |
  findings を機械消費する consumer が区分を要求した時点 | consumer
  は文言から判断する必要がある。
- `.bak` sweep を記録した ADR-034H2WBNtC4We8dyC5lQZJ の訂正を新
  ADR で行うか（解決済み）| 当初は所有境界違反の bug fix であり
  spec の requirements（R7 amend）で処理可能と判断したが、
  mutating command の所有境界という不変条件の再定義として chain
  の判断で decision record が要求された | 発火済み —
  ADR-034IOqXM5YpqVgFZKatIXN
  （`docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md`）
  を本 chain で作成し解決 | なし — ADR が存在し、旧 ADR の該当
  記述は R7 amend で訂正されるため drift は残らない。

## Sources and Relations

- issue #137: <https://github.com/scarecrow173/cheaper/issues/137>
  （本 spec の起点 — `.bak` sweep の所有境界問題と修正提案。
  proposal の Option A 採択を確定する）
- upstream discovery:
  `docs/discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md`
  （DISC-034IO86RVnmQ8AU5Lac8bw）— 検証済み file:line evidence、
  alternatives 評価（A/B/C/D）、未解決点の記録。本 spec はその
  promotion candidate「spec-doc needed」を引き継ぎ、Open
  Questions（verify の `.bak` 報告の扱い・文言区分、`orphansRemoved`
  後方互換、歴史的 doc の訂正方法）を R1-R7 で確定する。
- issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  （本問題を accepted residual として記録した review 元 —
  「`--event-stdin` に flag 伝達路が無い」制約は Option C 却下の
  根拠として継承）。
- issue #107: <https://github.com/scarecrow173/cheaper/issues/107>
  （hook-config ownership / lifecycle の導入元）。
- 関連 chain（`related` 参照。本 chain の `derives-from` には含めない
  — lineage edge は chain 内のみに限定して graph focus を壊さない
  ため）:
  `docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  （SPEC-034IKGkW4dQEVw6QigMFxQ、#128 系の降格 contract）、
  `docs/discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  （DISC-034IKBmCjJBeLD5qG9DoUM）。
- sweep の設計根拠・`.bak` sweep の記述元（`related` 参照、R7 で
  amend 対象）:
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  （ADR-034H2WBNtC4We8dyC5lQZJ、`:147`）、
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  （SPEC-034H2WAU6TQG6N9dmmmKNA、`:274`）、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  （`:149`・`:265`）。
- code evidence: `crates/cheaper-cli/src/hook_config.rs`
  （`sweep_orphans` :482-503 / glob :496-497、`find_orphans`
  :506-525 / glob :520、verify orphan 報告 :708-719 / finding
  文言 :711-713、`run_sync` の sweep 呼出 :763 / `orphansRemoved`
  :774-784、`write_config_atomic` :529+ / temp name 規約 :565-571、
  所有境界 note :863-870・:922-924、`run_event_stdin` :986 /
  sync+verify 呼出 :1013-1019）。
- test evidence: `crates/cheaper-cli/tests/integration/hook_config.rs`・
  `scripts/test/devin-hooks-install.test.mjs` — orphan sweep の
  coverage は現状ゼロ（本 spec で新規追加対象）。
