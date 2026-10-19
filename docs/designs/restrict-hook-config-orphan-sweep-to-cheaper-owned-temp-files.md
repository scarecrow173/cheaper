---
id: "DESIGN-034IPoKswcH3LaUZdnb0XT"
type: "design"
status: "approved"
title: "Restrict hook-config orphan sweep to cheaper-owned temp files"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/137"
    - "https://github.com/scarecrow173/cheaper/issues/128"
    - "https://github.com/scarecrow173/cheaper/issues/107"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
      - type: "file"
        path: "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
      - type: "file"
        path: "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "docs/adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/hook-config-sweep-orphans-deletes-foreign-bak-files.md"
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-cli/tests/integration/hook_config.rs"
    - "../../scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# Restrict hook-config orphan sweep to cheaper-owned temp files

## Context

`cheaper hook-config sync` は `run_sync` 開始時（upsert 前、
`crates/cheaper-cli/src/hook_config.rs:763`）に毎回
`sweep_orphans`（:482-503）を実行し、`{config_name}.` prefix +
`.tmp`/`.bak` suffix に一致する sibling file を無条件 `remove_file`
する（glob :496-497）。しかし cheaper が `.bak` を書き出す経路は
codebase に一切存在しない — `.bak` sweep は旧
`install-devin-hooks.ps1` の `[IO.File]::Replace` backup path が残し
うた crash orphan の回収責務を Rust port が規約ごと引き継いだもので、
旧 ps1 の write-less wrapper 化により producer は extinct となった
（DISC-034IO86RVnmQ8AU5Lac8bw）。結果、ユーザー手動 backup
（`config.json.bak`、`config.json.pre-migration.bak` 等）や他 tool の
backup が sync・APM lifecycle（`--event-stdin` :1013-1019 — flag 伝達
路なし）の自動収束で同意なく削除される（issue #137）。

本 design は ADR-034IOqXM5YpqVgFZKatIXN の Option 1（削除対象を
cheaper が実際に生成しうる `config.json.*.tmp` のみに限定）を
SPEC-034IOh5OATJ4AcprA5u47p の R1-R7 へ展開し、所有境界の不変条件
（sync note "hook-config manages only the entry it owns" :863-870、
remove の strict-shape 拒否 :922-924）を sweep にも一致させる。

## Scope

### In Scope

- `crates/cheaper-cli/src/hook_config.rs` —
  `sweep_orphans` の is_orphan predicate（:496-497）から `.bak` suffix
  条件を除去し `{name}.` prefix + `.tmp` suffix のみにする変更と、
  stale な doc comment（:480-481）の 2 箇所訂正、および
  `run_verify` の `orphan:` finding 文言（:711-713）の
  `.tmp`/`.bak` 区分修正 — 併せて直上の inline comment
  （`// orphan temp/bak。` :707）を区分後の contract に整合する
  内容へ更新する。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`（:160-161）—
  verify の説明 "orphan temp files" を 1 行 clarify: `.bak` orphan
  は verify が報告するが sync が自動除去しない旨を追記する
  explainability 上の wording 変更のみ（挙動変更なし）。
- `crates/cheaper-cli/tests/integration/hook_config.rs` — sweep の
  `.tmp` 削除 / `.bak` 保全・混在時の `orphansRemoved` 内容・verify
  の `.bak` informational 報告と文言区分・`--event-stdin` 両 event
  経路の `.bak` 保全を pin する新規 test（現状 orphan coverage は
  ゼロ）。
- 歴史的 doc 3 箇所（spec R7）— `.bak` sweep を現行挙動として記述
  する文を「対象 `config.json.*.tmp` のみ・sweep は sync 開始時に
  1 回実行（失敗時の別途 sweep なし）」へ amend:
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  （:274-277、requirement R17 内 — 規範的記述のため文そのものを
  訂正）、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  （:149-153 および :265-268）、
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  （:148-150）。各修正に SPEC-034IOh5OATJ4AcprA5u47p / issue #137
  への dated amend-note を付す。

### Out of Scope

spec の Out of Scope をそのまま引き継ぐ:

- `.bak` 削除を制御する opt-in flag・設定・env var の追加
  （Option C — `--event-stdin` に flag 伝達路が無く dead weight、
  #128 の `--allow-conflict` 却下と同一理由）。
- `.tmp` sweep 挙動自体の変更 — 呼出位置（`run_sync` upsert 前
  :763）、「upsert 成否・変更有無にかかわらず毎回実行」の規約、
  `{name}.` prefix 条件は不変（R2）。
- `find_orphans`（:506-525、glob :520）の走査条件 narrowing —
  verify が `.bak` を informational orphan として報告し続ける
  非破壊の可視性は維持する（R5）。
- `remove` command の scope 変更（own entry のみ除去の現行維持）。
- `orphansRemoved`（:774-784）・`orphans`（:716-719）field の
  rename・shape 変更、findings への severity field 等新構造の導入
  （表現ドリフト回避・YAGNI）。
- `config.json.` prefix + `.tmp` suffix に一致する外部 file への
  追加防護 — `config.json.<pid>.tmp` への strict narrowing は
  deferred concern（spec Deferred Design Concerns 参照）。
- issue #128 系の legacy conflict gate・dedup 機構への変更
  （cross-chain — `related` 参照のみ）。
- 別 chain の時点記録
  （`docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md:258-262`・
  `docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md:268`）
  の `.bak` sweep 言及 — #128 変更時点の side-effect を記述した
  historical change record であり現在挙動の規範的記述ではないため、
  spec R7 の amend 対象から除外されている。

## Components and Boundaries

変更は `hook_config.rs` 内の 2 関数 + 1 doc comment に閉じる:

| Component | 責務 | 変更 |
| --- | --- | --- |
| `sweep_orphans`（:482-503） | sync 開始時の orphan 物理削除 — 所有境界内のみ | predicate 内部 narrowing（signature・戻り値 `Vec<String>`・呼出位置不変）+ doc comment 訂正 |
| `find_orphans`（:506-525） | verify 用 read-only 走査 — 可視性 | 変更なし（`.tmp`/`.bak` glob 維持） |
| `run_verify`（:707-719） | finding 文言の組立 | `orphan:` finding を `.tmp`/`.bak` で文言区分 |
| `run_sync`（:763・:774-784） | sweep 呼出・`orphansRemoved` 組立 | コード変更なし — 新 contract は predicate 経由で継承 |
| `run_event_stdin`（:1013-1019） | lifecycle event → sync+verify 写像 | コード変更なし — 同一経路で継承 |
| `write_config_atomic`（:565-571） | temp name 規約 | コード変更なし — コメントは既に `{name}.` prefix + `.tmp` suffix 規約のみを述べており `.bak` への言及が無いため訂正不要（詳細は Detailed Design） |

境界の原則: 削除してよいのは cheaper の atomic write lifecycle が
実際に生成しうる file のみ。`.bak` は producer が extinct のため削除
domain から完全に外す — 所有境界違反を下流で補償するのではなく、
predicate という source で正す。

## Detailed Design

### `sweep_orphans` — predicate narrowing（:496-497）+ doc comment 訂正（:480-481）

現行:

```rust
let is_orphan =
    fname.starts_with(&tmp_prefix) && (fname.ends_with(".tmp") || fname.ends_with(".bak"));
```

新:

```rust
// 削除対象は自身の atomic write が残しうる `{name}.` prefix +
// `.tmp` suffix のみ — `.bak` は cheaper が生成しない（所有境界外）。
let is_orphan = fname.starts_with(&tmp_prefix) && fname.ends_with(".tmp");
```

signature `fn sweep_orphans(config_path: &Path) -> Vec<String>`、
`removed` への push 条件（`remove_file` 成功時のみ :498-499）、
`read_dir` 失敗時の empty 返却（:491-493）等の構造は不変 —
internal predicate の narrowing のみ（R1）。`orphansRemoved` は
field 名・shape（削除 file 名の string array）・出現条件（1 件
以上で insert :774-784）を維持し、載る内容が `.tmp` 名のみになる
（R3）。

doc comment（:480-481）は 2 箇所とも stale で訂正する:

```rust
// 現行:
/// `config.json.<pid>.tmp` / `config.json.*.tmp`/`*.bak` の orphan を
/// 削除する。戻り値は削除した file 名(verify の finding 用)。

// 新:
/// `{name}.` prefix + `.tmp` suffix（`config.json.*.tmp` 相当）の
/// orphan を削除する。戻り値は削除した file 名 — sync result の
/// `orphansRemoved` field 用（verify が使う走査は `find_orphans`）。
```

- (a) 対象記述: 実際の glob は `{name}.` prefix + `.tmp` suffix で
  pid 成分を要求しない — `config.json.<pid>.tmp` 表現は過度に狭く、
  `*.bak` 列挙は削除する挙動とともに除去する。
- (b) 戻り値説明: 「verify の finding 用」は誤り — verify の finding
  は `find_orphans` 由来であり、本戻り値は sync result の
  `orphansRemoved` field（:774-784）へ渡る。

### `find_orphans` — 変更なし（:506-525）

走査条件（`{name}.` prefix + `.tmp`/`.bak` suffix、:520）は現行
維持（R5）。verify が `.bak` を informational orphan として報告
し続けることで、sibling に残る backup/残骸の可視性を失わない
（ADR Decision Drivers「可視性の維持」）。read-only・削除しない
性質も不変。

### `run_verify` — `orphan:` finding 文言の区分（:708-719）

現行（:710-714）は全 orphan に一律
`"orphan: {o} (leftover temp/backup; safe to delete)"` を push
する。これを suffix で区分する:

```rust
// orphan temp/bak — `.tmp` は自身の atomic write 残留で次回 sync が
// 自動回収、`.bak` は cheaper 管理外 file で削除判断は user。
let orphans = find_orphans(&config_path);
if !orphans.is_empty() {
    for o in &orphans {
        if o.ends_with(".tmp") {
            findings.push(Value::from(format!(
                "orphan: {o} (leftover atomic-write temp; auto-removed \
                 by the next `hook-config sync`)"
            )));
        } else {
            findings.push(Value::from(format!(
                "orphan: {o} (not managed by cheaper; remove manually \
                 if unwanted)"
            )));
        }
    }
}
```

- `find_orphans` の返す名前は `.tmp`/`.bak` のみなので `ends_with
  (".tmp")` 分岐で網羅される — else 側は `.bak`（cheaper 管理外）
  文言。
- R6 の contract: `.tmp` orphan は「sync で自動回収」を示し、
  `.bak` orphan は「cheaper が管理しない — 不要なら手動削除」を
  示す。`.bak` 向けの "safe to delete" 表現は廃止（foreign file を
  安全と断じる不正確さの解消）。
- `orphan:` prefix・findings の string array 形式・informational
  扱い（blocker へ触れず `clean` flag に影響しない）は不変。
  severity field 等新構造は導入しない（YAGNI）。
- `orphans` field populate（:716-719）はそのまま。
- 直上の inline comment（現行 `// orphan temp/bak。` :707）は文言
  区分に合わせて上記 code block の comment へ更新する。

### `run_sync` / `run_event_stdin` — コード変更なし（:763・:774-784・:1013-1019）

- sweep 呼出位置（`run_sync` の `legacy_conflicts` scan 後・
  `load_config` 前 :763）と「毎回実行」規約は不変（R2）。
- `orphansRemoved` は `sweep_orphans` 戻り値をそのまま載せるため
  predicate narrowing が自動継承される（R3）。
- `--event-stdin` の `post-install`/`post-update` は同一 arm
  （:1013-1019）で `run_sync` + `run_verify` を呼ぶため、両 event
  が新 contract を共有する（R4）。event payload・subresult
  envelope（`sync`/`verify` key + verify exit code passthrough）の
  構造変更・新入力経路は無い。

### `write_config_atomic` temp name comment（:565-571）— 訂正不要と確認

:565-566 の comment は「temp name は sweep_orphans の `{name}.`
prefix + `.tmp` suffix 規約に合わせる」と `.tmp` 規約のみを述べて
おり、`.bak` sweep への言及は無い — narrowing 後も正確なまま
残る（むしろ temp name 側が sweep 規約に従う方向の記述であり、
対称性の記述として正しい）。`.bak` 言及が無いことを確認した上で
変更しない。

## Wording Contract

finding・doc comment の文言契約（before → after）:

| 対象 | before | after |
| --- | --- | --- |
| verify finding（`.tmp` orphan） | `orphan: {o} (leftover temp/backup; safe to delete)` | `orphan: {o} (leftover atomic-write temp; auto-removed by the next \`hook-config sync\`)` — sync 自動回収を示す |
| verify finding（`.bak` orphan） | 同上（一律） | `orphan: {o} (not managed by cheaper; remove manually if unwanted)` — 管理外 + 判断は user。"safe to delete" を含まない |
| `sweep_orphans` doc comment | `config.json.<pid>.tmp`/`*.bak`・「verify の finding 用」 | `config.json.*.tmp` 相当・「`orphansRemoved` field 用」 |

文言の必須条件（test が pin する semantic contract）は spec R6 に
従う — `.tmp` は自動回収を、`.bak` は cheaper 管理外 + 手動削除
の判断を伝え、"safe to delete" は `.bak` finding に含めない。上表
の after 文言はその contract を満たす具体案であり、細部の言い回し
は実装で調整してよいが semantic 要件から外れないこと。

## Documentation Design（R7 amend）

`.bak` sweep を現行挙動として記述する 3 箇所の規範的記述を、文
そのものを訂正する方式（drift-note 併記ではない）で amend する。
訂正後の文は R7 が定める 2 点を述べる: 「sweep 対象は cheaper が
生成しうる `config.json.*.tmp` のみ」「sweep は sync 開始時
（load 前）に 1 回実行し、失敗時の別途 sweep は行わない —
rename 失敗時の temp 除去は `write_config_atomic` 自身
（:574-575）が担う」。各修正箇所に dated amend-note を付す。
note の形式は repo 既存の blockquote 規約
（`docs/specs/unified-version-source-of-truth.md:50-55` の
「Superseded clauses」note）に倣うが、旧文の扱いは同型ではなく
正反対である点に注意: 同先例は原文を歴史的記録として温存した
まま blockquote で superseded 条項を注記する annotate-only の
方式であり、本 R7 amend は現行の規範文をその場で書き換えて
dated note を添える。in-place 訂正とするのは spec R7 の明示に
よる — 対象文は現行挙動を定める規範的記述（bounded spec :274
の記述は descriptive sentence ではなく requirement R17 の一部）
であり、drift-note 併記では矛盾する 2 つの requirement が併存
するため（本 chain の ADR-034IOqXM5YpqVgFZKatIXN :361-371 も
旧 ADR への変更を「`.bak` sweep 記述文の R7 amend による
訂正のみ」と位置付け、この方式を裏付ける）。note は修正文の
直後へ:

```text
> Amend（2026-10-20 — SPEC-034IOh5OATJ4AcprA5u47p / issue #137）:
> orphan sweep の対象を `config.json.*.tmp`（cheaper 生成 temp）に
> 限定し、実行時点を「sync 開始時の 1 回（失敗時の別途 sweep
> なし）」へ訂正 — 旧記述の `.bak` sweep と「起動時 + 失敗時」
> timing は実装と齟齬していた。
```

対象別の before → after:

1. `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md:148-150`

   before: 「…temp+rename の atomic replace（`fs::rename` は dest
   存在時に置換）、capability 確認との間の外部変更検出、
   `config.json.*.tmp`/`.bak` の orphan sweep（起動時 + 失敗時）、
   無関係 entry の保持、…」

   after: 「…temp+rename の atomic replace（`fs::rename` は dest
   存在時に置換）、capability 確認との間の外部変更検出、
   `config.json.*.tmp` のみの orphan sweep（sync 開始時に 1 回 —
   失敗時の別途 sweep は行わず、rename 失敗時の temp 除去は
   atomic write 自身が行う）、無関係 entry の保持、…」+ amend-note。

2. `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md:274-277`
   （requirement R17 内 — 規範的記述なので文を訂正し amend-note
   で provenance を残す）

   before: 「…`hook-config` の書き込みは temp file + atomic
   replace（`fs::rename` は dest 存在時に置換）で、capability 確認
   との間の外部変更検出・`config.json.*.tmp`/`.bak` の orphan
   sweep を持つ。」

   after: 「…`hook-config` の書き込みは temp file + atomic
   replace（`fs::rename` は dest 存在時に置換）で、capability 確認
   との間の外部変更検出・`config.json.*.tmp` のみの orphan sweep
   （sync 開始時に 1 回 — 失敗時の別途 sweep は行わず、rename
   失敗時の temp 除去は atomic write 自身が行う）を持つ。」
   + amend-note。

3. `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
   — 2 箇所を amend（同一 file 内の両箇所で同じ contract を述べる
   形にし、amend-note は各箇所に付す）:

   a. :149-153（`sync` bullet 内）
      before: 「…temp+rename で atomic replace、
      `config.json.*.tmp`/`.bak` の orphan sweep を起動時と失敗時
      に行う。」
      after: 「…temp+rename で atomic replace、`config.json.*.tmp`
      のみの orphan sweep を sync 開始時に 1 回行う（失敗時の別途
      sweep は行わず、rename 失敗時の temp 除去は atomic write
      自身が行う）。」+ amend-note。

   b. :265-268（Non-Functional Constraints）
      before: 「…`hook-config` の書き込みは temp+rename で atomic
      replace、外部変更を capability 確認との間で検出して中断し、
      `config.json.*.tmp`/`.bak` の orphan を sweep する。」
      after: 「…`hook-config` の書き込みは temp+rename で atomic
      replace、外部変更を capability 確認との間で検出して中断し、
      `config.json.*.tmp` のみの orphan を sync 開始時に 1 回
      sweep する（失敗時の別途 sweep は行わず、rename 失敗時の
      temp 除去は atomic write 自身が行う）。」+ amend-note。

実装時の注意: 上記の行番号 citation は対象 file の改訂で ±2 行
程度 drift しうる。対象文は行番号ではなく各 before 引用文の
verbatim 一致で特定すること（上記 before 文は全て対象 file で
verbatim 確認済み）。

## Test Design

`crates/cheaper-cli/tests/integration/hook_config.rs` へ新規 test を
追加（orphan sweep の coverage は現状ゼロ — 同 file・
`scripts/test/devin-hooks-install.test.mjs` のいずれにも `.bak`/
orphan 系 assertion が無い）。既存 pattern に従う: fixture は
`fixture_with_cheaper_on_path()`（`CARGO_BIN_EXE` copy の PATH
shim）、config 経路は `config_path(&fx)`、実行は in-process
`fx.run(&[...])` で envelope JSON を assert、sibling file 作成用に
小さな helper（`write_orphan_file(fx, name)` 相当 —
`.devin/` dir 作成 + 任意名 file 書込み）を足す。

test matrix（spec Acceptance Criteria に対応）:

| test（命名案） | 配置する file | 実行 | assert |
| --- | --- | --- | --- |
| `sync` の `.tmp` sweep | `config.json.<pid>.tmp` 相当（例: `config.json.12345.tmp`） | `hook-config sync` | file 削除・`result.orphansRemoved` に `.tmp` 名が載る・exit 0 |
| `sync` の `.bak` 保全 | `config.json.bak` + `config.json.manual.bak` | `hook-config sync` | 両 file 残存・`orphansRemoved` が `.bak` 名を一切含まない（orphan 無しなら key 不在 — 出現条件 :774-784） |
| `.tmp`+`.bak` 混在 | `config.json.<x>.tmp` + `config.json.bak` | `hook-config sync` | `.tmp` のみ削除・`.bak` 残存・`orphansRemoved` が `.tmp` 名のみ含む |
| verify の `.bak` 報告 | `config.json.bak` | `hook-config verify` | `result.orphans` + `orphan:` finding に file 名・file 非削除・finding が "safe to delete" を含まず "not managed by cheaper" を含む・`clean`/exit code は blocker 有無のみで決まる（informational のまま） |
| verify の `.tmp` 文言 | `config.json.<x>.tmp` | `hook-config verify` | `orphan:` finding が sync 自動回収を示す文言（`auto-removed` + `sync` 等）・verify 実行後も `.tmp` file が残存する（read-only 契約の pin — R5） |
| `--event-stdin` 経路 | `config.json.bak` | `post-install` payload → `post-update` payload を同 fixture で連続実行（`fx.stdin.data` 差替え — :334 の pattern） | 両実行とも `.bak` file 残存・`result.sync.orphansRemoved` に `.bak` 名を含まない（同一 arm :1013-1019 の contract 継承を pin） |

## Error and Edge Handling

- `config.json.bak` 完全一致: `config.json.` prefix + `.bak` suffix
  に一致するため `find_orphans` の報告対象には残り、sweep 対象から
  は外れる — 本変更の核心 case で test が pin する。
- 非 pid `.tmp` 名の残存リスク（正直な residual）: glob が要求する
  のは `config.json.` prefix + `.tmp` suffix のみで pid 成分は条件に
  無いため、`config.json.tmp`・`config.json.backup.tmp`・
  `config.json..tmp` 命名の外部 file は post-fix でも削除されうる。
  `.bak` の任意 backup より衝突可能性は狭いがゼロではない — spec
  が受容を決めた残存リスクとして Deferred Design Concerns に記録。
- `config.json.bak.tmp` のような複合 suffix: `.tmp` で終わるため
  sweep 対象 — 命名上 temp 形なので新 contract では削除が妥当
  （所有境界の表現として一貫）。
- `.bak` の大文字化（`config.json.BAK`）: `ends_with` は case
  sensitive で glob に一致しない — 従来どおり無視（変化なし）。
- `read_dir` 失敗（dir 不在等）: empty 返却の現行構造を維持
  （:491-493・:515-517）。
- `remove_file` 失敗（lock・権限）: `is_ok()` gate で `removed` に
  載せず処理継続の現行規約を維持（:498-499）— sweep 失敗で sync
  自体を落とさない。
- `.tmp` orphan が `run_verify` で報告されても `clean` には影響
  しない（informational）— 文言区分後も同じ。

## Non-Functional Constraints

- 新規依存・flag・env var・result field・severity 構造の追加なし
  （YAGNI・表現ドリフト回避）。
- sweep の計算量は sibling entry 走査 O(n) のまま — predicate
  narrowing は suffix check 1 項の除去で cost 減方向のみ。
- 後方互換: `orphansRemoved`・`orphans`・`findings` の key・shape・
  出現条件は不変。変わるのは finding 文字列の文言（機械可読
  contract ではない）と `orphansRemoved` に載りうる値の範囲
  （`.tmp` 名のみ）のみ。
- 全経路で同一挙動: sync 直接実行・`--event-stdin` lifecycle の
  いずれでも同じ predicate が効く — consent channel を必要としない
  安全な既定動作。

## Risks and Trade-offs

- 外部 tool が残した `config.json.*.bak` は cheaper の掃除責務から
  外れる — verify の informational 報告で可視性を維持する緩和
  （もともと責務外と解釈すべき; ADR Consequences）。
- verify の `.bak` orphan finding は informational のままで exit
  code に現れない — 文言区分で「cheaper 管理外・判断は user」を
  明示し、誘導としての弱さを緩和する。
- verify ↔ sync の報告乖離は意図的かつ永続する: post-fix も
  verify は `.bak` orphan を実行のたび（`--event-stdin` lifecycle
  の各実行を含む）報告し続けるが、sync はそれを一切除去しない —
  非破壊の可視性を取った trade-off であり、新 finding 文言
  （「cheaper 管理外 — 不要なら手動削除」）が乖離の理由そのもの
  を説明する役割を担う。`orphans` field 名・`orphan:` prefix は
  R3/R6 の後方互換のため維持し、乖離を解消する構造変更は
  行わない。
- finding 文言の変更は string を機械消費する consumer に見える
  差分だが、findings は人間向け message であり shape は不変 —
  severity 構造化は YAGNI（deferred）。
- `config.json.<pid>.tmp` strict narrowing を行わないため理論上の
  同名 `.tmp` 巻き込みリスクが残る — 実害報告時の将来 tightening
  候補（spec deferred concern と同一記録）。

## Deferred Design Concerns

- `config.json.` prefix + `.tmp` suffix に一致する外部 file の取り
  扱い | glob は pid 成分を要求しないため非 pid 命名の外部 `.tmp`
  は依然 sweep されうる — `.bak` より衝突可能性は狭いがゼロでは
  ない | 外部 file の同名衝突被害が報告された時点（strict
  `config.json.<pid>.tmp` narrowing の将来候補） | 残存する理論上
  の sweep 巻き込みリスク。
- 並行 writer の in-flight `.tmp` 巻き込み | sweep は同一 glob 面で
  sibling を走査するため、concurrent な sync/remove の
  `write_config_atomic` が `std::fs::write`（:572）と `fs::rename`
  （:574）の間に置いた `config.json.<pid>.tmp` を削除しうる —
  競合 writer 側の rename が失敗に転じる。pre-existing の残存
  リスクであり本変更による増減はなく mitigation は scope 外 |
  並行実行での実害報告があった時点 | 理論上の racing writer
  巻き込みリスク。
- orphan finding の機械可読区分（`.tmp`/`.bak` 別 field・severity
  構造化） | string finding + `orphans` field で足りており追加構造
  は YAGNI — #128 spec と同じ findings 構造維持判断 | findings を
  機械消費する consumer が区分を要求した時点 | consumer は文言
  から判断する必要がある。

## Verification Notes

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` が pass する（新規 test を含む）。
- [ ] `.tmp` orphan が sync で削除され `orphansRemoved` に載り、
  `.bak` file（`config.json.bak`・`config.json.manual.bak`）が任意名
  でも削除されず `orphansRemoved` に含まれないことが test で固定
  される（R1-R3）。
- [ ] verify が `.bak` を `orphans` field + `orphan:` finding に報告
  し file を削除せず、`.bak` finding が "safe to delete" を含まず
  cheaper 管理外（手動削除の判断は user）を伝え、`.tmp` finding が
  sync 自動回収を示すことが test で固定される（R5-R6）。
- [ ] `--event-stdin` の `post-install`・`post-update` 両経路で
  `.bak` が削除されないことが test で固定される（R4 — 同一 arm
  :1013-1019 の contract 継承）。
- [ ] 歴史的 doc 3 箇所が「対象 `config.json.*.tmp` のみ・sync
  開始時に 1 回実行（失敗時の別途 sweep なし）」へ amend され、
  SPEC-034IOh5OATJ4AcprA5u47p / issue #137 への dated 参照を
  伴うことが review で確認される（R7）。
- [ ] `mise run check` が pass する。
