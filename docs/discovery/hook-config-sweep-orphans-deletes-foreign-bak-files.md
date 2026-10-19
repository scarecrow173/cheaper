---
id: "DISC-034IO86RVnmQ8AU5Lac8bw"
type: "discovery"
status: "resolved"
title: "hook-config sweep_orphans の .bak sweep が cheaper 非生成 file を削除する"
created: "2026-10-19"
updated: "2026-10-19"
owners: []
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
    - "./hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../specs/bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/hook-config-orphan-sweep-ownership-boundary.md"
    - "../adr/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# hook-config sweep_orphans の .bak sweep が cheaper 非生成 file を削除する

## Exploration Goal

`cheaper hook-config sync` は temp + rename atomic write の直前で
`sweep_orphans` を実行し、`config.json.<pid>.tmp` のような中途 orphan を
回収する設計になっている(起源は ADR-034H2WBNtC4We8dyC5lQZJ 系、issue
#107)。しかし実装の glob は `config.json.*` prefix + `.tmp`/`.bak`
suffix で、**`.bak` を cheaper が書き出す経路は codebase 内に一切存在
しない**。結果として、ユーザーが project `.devin/` dir に置いた
`config.json.bak` や `config.json.pre-migration.bak` のような手動
backup、あるいは他 tool が残した backup が、`sync` 実行時に同意なく
削除される(issue #137 — #128 の review で accepted residual として
記録済み)。

本 discovery が扱う問い: **orphan sweep の所有境界をどう修正するか —
`.bak` の削除対象を cheaper が実際に生成しうる `.tmp` orphan のみに
絞るか、cheaper 所有の命名 pattern に限定するか、opt-in flag で制御
するか、あるいは現行を維持して文書化で済ませるか。** あわせて verify
側の `.bak` 報告の扱いと、設計起源 docs(ADR/spec/design)の「`.bak`
orphan sweep」記述との整合方針を整理する。

方針の採択自体は issue #137 側の判断であり、本書は確定方針ではなく
証拠・代替案・未解決点・昇華先を整理する探索記録である。

## Key Issues and Assumptions

- **sweep_orphans は任意 `*.bak` を無条件削除する(confirmed)**:
  `sweep_orphans`(`crates/cheaper-cli/src/hook_config.rs:482-503`)は
  `config.json.` prefix + `.tmp`/`.bak` suffix に一致する file を
  `remove_file` で削除する(glob 条件は :496-497)。`run_sync` から
  upsert 処理の前に呼ばれ(:763)、削除した file 名は result の
  `orphansRemoved` field に載る(:774-784)。sweep は upsert 成否・
  変更有無にかかわらず毎回実行されるため、no-change な sync でも
  `.bak` は消える。
- **`.bak` を cheaper が生成する経路は存在しない(confirmed)**:
  repo-wide に `.bak` の書き出し経路は無く、自身の atomic write が
  作るのは `config.json.<pid>.tmp` のみ
  (`write_config_atomic` :529、temp name 規約の注記 :565-571)。
  つまり `.bak` 側の sweep 対象は全て外部/ユーザー作成 file であり、
  「自身の atomic write lifecycle の orphan 回収」という設計意図を
  超えて所有境界外の file を削除している。
- **verify 側は read-only で非破壊(confirmed)**:
  `find_orphans`(`hook_config.rs:506-525`、glob :520)は同一条件で
  走査するが削除せず、`orphans` finding/field として報告するのみ
  (:708-719)。ただし finding 文言は
  `"orphan: {o} (leftover temp/backup; safe to delete)"`(:711-713)で、
  外部 `.bak` を "safe to delete" と断じている点は文言として
  再検討対象。
- **`--event-stdin` 経路でも削除が走る(confirmed)**:
  `run_event_stdin`(:986)は `post-install`/`post-update` で
  `run_sync` + `run_verify` を実行する(:1015-1016)。つまり APM
  lifecycle の自動収束でもユーザー `.bak` が消える — 人間が sync を
  直接実行しなくても発火する。
- **所有境界の不変条件と矛盾(confirmed)**: `hook-config` は own
  entry のみを管理し無関係な登録・file を保全する規約
  (sync の note: "hook-config manages only the entry it owns;
  unrelated hooks are preserved" :863-870、remove の strict-shape
  拒否 :922-924、remove は own entry のみ除去 :874)。`.bak` の
  削除はこの「所有するものだけを触る」境界を越える。
- **設計起源 docs は `.bak` sweep を明示(confirmed)**:
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md:147`、
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md:274`
  (orphan cleanup 一般は :313-314 にも)、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:149`
  と `:265`。`.bak` sweep の実際の起源は foreign tool/legacy の
  残滓処理ではなく、旧 `install-devin-hooks.ps1` が
  `[IO.File]::Replace(src, dst, backupPath)` の明示 backup path で
  残し `finally` で除去していた自前 backup file — Rust port の
  sweep はその crash orphan の回収責務を sweep 規約ごと引き継いだ
  もの(`docs/impl/ir/register-devin-hooks-from-apm-skill.md:113-115`・
  `:139-142`、旧 ADR `:142`「書き込み規則は PowerShell script の
  既存 contract を引き継ぐ」)。旧 ps1 は現在 write-less thin
  wrapper(`hook-config` へ委譲)で `.bak` producer は消滅しており、
  glob の本来目的は extinct — 残るのは foreign file 削除の副作用
  のみ。
- **orphan sweep の test coverage はゼロ(confirmed)**:
  `crates/cheaper-cli/tests/integration/hook_config.rs` と
  `scripts/test/devin-hooks-install.test.mjs` のいずれにも
  `.bak`/orphan 系 assertion が無い。修正時に振る舞いを pin する
  test が新規に必要。
- **関連する前作業**: issue #107 で hook-config ownership と
  lifecycle が導入され、#128 で legacy conflict gate が warning へ
  降格された(#137 はその review の accepted residual)。#128 と同じ
  「`--event-stdin` には flag 伝達路が無い」制約が option 評価に効く。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (A) `sweep_orphans` の削除対象を `config.json.*.tmp` のみに絞る。`.bak` は `find_orphans`/verify の informational 報告として維持 | 削除対象を「cheaper が実際に生成しうる file」に一致させ、所有境界の不変条件を回復できる。最小変更で、verify の可視性は残る(非破壊)。`.tmp` orphan 回収という本来の設計意図は完全に保たれる | 外部 tool が残した `config.json.*.bak` の掃除は cheaper の責務から外れる(もともと責務外と解釈すべき)。verify の finding 文言("safe to delete")は `.bak` に対しては再検討が要る | yes(推奨方向) |
| (B) `.bak` 削除を維持し、cheaper 所有の命名 pattern(例: `config.json.<pid>.bak`)に限定する | `.bak` 掃除の構造は残る | cheaper は `.bak` を一切書かないため、そのような pattern の file は現実に存在しない — 実質 (A) と同等の効果で、pattern 定義・保守コストだけが余計に載る(YAGNI) | no |
| (C) `.bak` 削除を opt-in flag(例: `--sweep-bak`)で制御 | 挙動を変えずに危険性だけ抑制できる | `--event-stdin` の lifecycle 経路には flag 伝達路が無く、primary use case である APM 自動収束では使えない dead weight になる — #128 で `--allow-conflict` が同じ理由で却下済み。default を維持すれば結局ユーザー file は消える | no(rejected analog) |
| (D) 現行維持 + 文書化のみ(削除仕様を docs で明記) | コード変更不要 | 「同意なくユーザー file を削除する」挙動は文書化しても不変条件違反のままで、`orphansRemoved` を見るまで検知不能。破壊的操作を docs で正当化する形になり、問題の根(所有境界違反)を解消しない | no |

## Tentative Conclusions and Hypotheses

- **方向 (A) が最も妥当(hypothesis)**: sweep の削除対象を
  `config.json.*.tmp` — cheaper の atomic write が実際に残しうる
  唯一の形 — に限定し、所有境界の不変条件を回復する。削除 consent
  の問題は「消していいものだけを消す」設計に戻すことで根源から
  解消される。#128 の降格と同じく、lifecycle 経路での収束を阻害
  しない点も整合する。
- **verify の `.bak` 報告は残す方向(hypothesis)**: `find_orphans`
  は非破壊なので可視性として残す価値があるが、`.bak` を含めて
  "safe to delete" と一律に断じる文言はユーザー file に対して
  不正確 — finding 文言の区分(tmp = 自動回収対象 / bak = 外部 file
  で判断はユーザー)を spec で確定する必要がある。
- **設計起源 docs の扱い(hypothesis)**: ADR/spec/design の
  「`.bak` orphan sweep」記述は実装の所有境界と齟齬するため、
  amend/supersede による訂正か drift-note での明記が必要 —
  新 ADR を切るか spec のみで扱うかは未確定(Open Questions 参照)。
- **test gap(hypothesis)**: 修正時に `sweep_orphans` の
  「`.tmp` は消す / `.bak` は消さない」挙動と verify の `.bak`
  finding を pin する integration test を
  `crates/cheaper-cli/tests/integration/hook_config.rs` に追加する。

## Open Questions

- verify の `find_orphans` が `.bak` を finding として報告し続けるべきか。
  報告を残す場合、`.tmp` と `.bak` を文言・field 上で区別するか
  (現行は同一 `orphans` array + 一律 "safe to delete" 文言)—
  spec で確定する。
- 設計起源 docs への訂正方法 — ADR-034H2WBNtC4We8dyC5lQZJ・
  spec・design の「`.bak` orphan sweep」記述を amend/supersede
  する新 artifact を切るか、drift-note で済ませるか(#128 での
  ADR 扱いと同じ構造の未確定点)。
- sync result の `orphansRemoved` field の命名・後方互換 —
  `.tmp` のみになることで field 名はそのままで良いか、
  削除内容の内訳(tmp/bak 別)を報告する必要があるか。
- `.bak` 以外に sweep が巻き込みうる pattern の有無 —
  `config.json.` prefix + `.tmp` suffix に絞った後も、外部 tool が
  同名規則の `.tmp` を残す可能性をどこまで考慮するか。glob が
  要求するのは prefix + `.tmp` suffix のみで pid 成分は条件に
  含まれないため、`config.json.tmp`・`config.json.<任意>.tmp` 命名の
  外部 file は依然削除対象となる — `.bak` の任意 backup より衝突
  可能性は狭いがゼロではない。`config.json.<pid>.tmp` への strict
  narrowing は実害報告が出た場合の将来候補。

## Promotion Candidates

- [ ] spec-doc needed: `hook-config sync`/`verify` の orphan 取り扱い
  contract(削除対象の所有境界、verify finding の文言・区分、
  result field、test 要件)を formalize する
- [ ] adr-doc needed: かもしれない — `.bak` sweep を記録した
  ADR-034H2WBNtC4We8dyC5lQZJ 系の記述を訂正する形を
  Open Questions のとおり未確定

## Sources and Relations

- issue #137: <https://github.com/scarecrow173/cheaper/issues/137>
  (本 discovery の起点 — `.bak` sweep の所有境界問題と修正提案)
- issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  (本問題を accepted residual として記録した review 元 — flag
  伝達路不存在の制約もここで確立)
- issue #107: <https://github.com/scarecrow173/cheaper/issues/107>
  (hook-config ownership / lifecycle の導入元)
- 起源 artifact(`related` 参照。本 chain の `derives-from` には
  含めない — lineage edge は chain 内のみに限定して graph focus を
  壊さないため):
  `docs/discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (DISC-034IKBmCjJBeLD5qG9DoUM、#128 系 chain)、
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  (ADR-034H2WBNtC4We8dyC5lQZJ、`.bak` sweep の設計根拠 :147)、
  `docs/specs/bounded-native-hook-evaluation-and-managed-devin-registration.md`
  (`:274`・`:312`)、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  (`:149`・`:265`)
- code evidence: `crates/cheaper-cli/src/hook_config.rs`
  (`sweep_orphans` :482-503 / glob :496-497、`find_orphans`
  :506-525 / glob :520、verify orphan 報告 :708-719、
  `run_sync` の sweep 呼出 :763 / `orphansRemoved` :774-784、
  `write_config_atomic` temp name :565-571、所有境界 note
  :863-870・:922-924、`run_event_stdin` :986 / sync 呼出 :1015)
- test evidence: `crates/cheaper-cli/tests/integration/hook_config.rs`・
  `scripts/test/devin-hooks-install.test.mjs` — orphan sweep の
  coverage は現状ゼロ(修正時に新規追加対象)
