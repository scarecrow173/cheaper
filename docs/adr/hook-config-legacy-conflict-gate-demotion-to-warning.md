---
id: "ADR-034IKJq6tKWWAep0Q0ONzK"
type: "adr"
status: "accepted"
title: "hook-config legacy conflict gate demotion to warning"
created: "2026-10-20"
updated: "2026-10-20"
owners:
  - scarecrow173
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
    - "https://github.com/scarecrow173/cheaper/issues/127"
    - "https://github.com/scarecrow173/cheaper/pull/131"
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
    - "./dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../../crates/cheaper-cli/src/hook_config.rs"
    - "../../crates/cheaper-cli/src/hook/eval_cache.rs"
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/tests/integration/hook_config.rs"
    - "../guides/distribution.md"
    - "../../packages/apm/.apm/skills/cheaper-usage/SKILL.md"
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

# hook-config legacy conflict gate demotion to warning

## Context and Problem Statement

`cheaper hook-config` は legacy conflict gate を持つ:
`legacy_conflicts()`(`crates/cheaper-cli/src/hook_config.rs:362`)が
`KNOWN_LEGACY_FILES`(`.windsurf/hooks.json`・`.devin/hooks.json`・
`.devin/hooks.v1.json`・`.devin/config.local.json`・
`.claude/settings.json`・`.claude/settings.local.json`、
`hook_config.rs:42-49`)+ target file を走査し、command-ish な
JSON 文字列へ `CONFLICT_PATTERN`
(`cheaper[^\r\n]*hook[^\r\n]*(pre-read-code|pre-tool-use)`、
`hook_config.rs:38`)を適用する。conflict を検出すると `sync` は
`invalid_request` error で upsert 自体を拒否し
(`hook_config.rs:760-768`)、`verify` は `blocker = true` →
`clean:false`・exit 2 で `"legacy-conflict: {f} ... (remove it
before sync — duplicate firing is unsafe)"` の finding を返す
(`hook_config.rs:668-678`)。

この gate は ADR-034H2WBNtC4We8dyC5lQZJ(issue #107)で導入され、
前提は「重複発火は unsafe なので未然に防ぐ」だった。しかし issue
#127 の dedup(canonical request fingerprint による verdict cache +
leader election、PR #131 merged、
`crates/cheaper-cli/src/hook/eval_cache.rs`、
ADR-034Ie3HFd6tIJms6Z2NIow)により、複数の登録 path
(`.devin/config.json` native + `.claude/settings.json` import 等)
からの発火は、fingerprint が一致する同一 canonical request の間で
評価 1 回へ収束し、全 firing が同一 canonical verdict を返す。
fingerprint 非一致の firing は独立に評価される(cost は
per-request bounded)ため、correctness の根拠は「全 firing の
合一」ではなく request 単位の verdict 決定性にある。また
normalize は target 間で divergence しうる(例: devin の cwd
解決は payload.cwd→`DEVIN_PROJECT_DIR`→deps.cwd の順、
pretooluse は deps.cwd のみ —
`crates/cheaper-cli/src/hook/targets.rs:535-541`,`:651`)ため、
dedup の coverage を「あらゆる重複 firing の合一」とは主張
しない。その上で gate の前提は無効化された。

一方、既に cheaper hook 登録を持つ consumer project(例: Claude
Code の `.claude/settings.json`)では、`cheaper hook-config sync`
が同 gate に block され、`.devin/config.json` への native 登録と
既存 hook の共存ができない(issue #128)。`--event-stdin` の
`post-install`/`post-update` は `run_sync` + `run_verify` を順に
実行して verify の exit code を返す(`hook_config.rs:1001-1007`)
ため、conflict 有りでは sync error または verify exit 2 で APM
lifecycle が収束しない。Devin 側の import merge は cheaper の制御
外であり、共存を user の手動解消に強制し続ける設計は維持不能。

## Decision Drivers

* **共存の許容**: dedup 導入後、native 登録と import 系統の共存は
  correctness 上安全。正当な共存ケースを block し続ける根拠が無い。
* **lifecycle 収束(deciding criterion)**: primary use case は APM
  lifecycle script 経由の post-install 自動収束。`--event-stdin`
  の入力は stdin の event JSON のみで flag 伝達路が無いため、
  収束は flag 無しの既定動作で実現されなければならない。
* **own-entry-only 所有境界**: `hook-config` は own entry のみを
  管理し、他 agent / 無関係 hook の entry は保全する
  (`hook_config.rs:855-858` の note、remove の strict-shape 規約
  `hook_config.rs:911-913`)。この境界は本決定でも維持する。
* **可視性の維持**: gate を外しても conflict 情報は引き続き
  result / finding として報告し、stale な他登録の存在を隠さない。

## Considered Options

* Option 1: gate を warning へ降格 — `sync` は upsert を継続して
  result の `legacyConflicts` array + warning note で報告、
  `verify` は conflict を informational finding とする。
* Option 2: 現行 gate を維持(conflict を blocker のまま)。
* Option 3: `--allow-conflict` flag を追加して opt-in bypass する。
* Option 4: sync 側 auto-cleanup — conflict 検出時に legacy file
  内の他登録を自動削除して単一登録へ収束させる。

## Decision Outcome

Chosen option: "Option 1 — gate を warning へ降格"。dedup が
「重複発火は unsafe」という gate の前提を除去しており、
correctness を保ったまま native + import 共存を許可でき、flag
無しで lifecycle 経路が収束する唯一の選択肢であるため(issue
#128 で採択済み)。

降格後の command 別 contract:

- `sync`: conflict があっても upsert を実行し、result に
  `legacyConflicts` array(verify result の既存 field
  `hook_config.rs:679-682` と同名 — 表現ドリフト回避)+ warning
  note を載せて exit 0。`--allow-conflict` 等の flag は追加しない。
- `verify`: conflict 検出は informational finding として報告し、
  blocker にしない。conflict-only finding では `clean:true`・
  exit 0 とする。他 blocker(own entry 不在・重複・parse 失敗等)
  の扱いは不変。finding 文字列の "duplicate firing is unsafe"
  文言は新前提と矛盾するため書き換える。informational / blocker
  の表現形態(prefix 規約か severity field か)は spec で確定する。
- `remove`: 挙動変更なし。residual conflict の `residualRegistrations`
  + informational note(`hook_config.rs:926-939`)は既存どおりで、
  文言のみ新規約へ整合させる。
- `--event-stdin` (`post-install`/`post-update`): verify が exit
  0 を返すようになるため、conflict 有りでも lifecycle が収束する。
  `run_event_stdin` 自体のコード変更は不要な見込み(verify_code
  をそのまま返す構造のため)だが、regression test で固定する。
  `post-uninstall` の `packages` 判定不能 branch も `run_verify`
  の code をそのまま透過する(`hook_config.rs:1023-1036`)ため、
  conflict-only の residual state では同様に exit 2→0 へ変わる —
  `packages` に cheaper が含まれる branch(:1010-1014 →
  `run_remove` が residual を `residualRegistrations` +
  informational note で報告して exit 0)の現行規約と整合する。

### Scope and Non-Goals

- `--allow-conflict` 等の opt-in flag は追加しない(Option 3 却下)。
- legacy file 内の他登録の自動削除は行わない(Option 4 却下) —
  own-entry-only 所有境界を越えるため。stale な foreign entry は
  verify の informational finding で可視化し続け、除去は user の
  判断に委ねる。
- `KNOWN_LEGACY_FILES`・`CONFLICT_PATTERN`・`legacy_conflicts()`
  の検出規則自体は変更しない(gate の severity のみ降格)。
- dedup 実装(`hook/eval_cache.rs`)・verdict semantics には
  一切触れない。
- findings の informational / blocker 表現形態・warning note の
  JSON shape・stderr 出力の有無(quiet 時の振る舞いを含む)は
  spec で確定する詳細であり、本 ADR は方向のみ固定する。

### Consequences

* Good, because Devin native(`.devin/config.json`)登録と
  Claude/APM 系統の import 登録の共存が support され、issue #128
  の block が解消される。
* Good, because APM lifecycle `post-install`/`post-update` が
  flag 伝達路無しのまま verify exit 0 で収束し、登録状態が
  lifecycle 経由で自動収束する primary use case が成立する。
* Good, because 共存時の重複発火は dedup により評価 1 回・全
  firing 同一 verdict に収束し、correctness は維持される。
* Good, because conflict 情報は `legacyConflicts` field と
  informational finding で引き続き報告され、stale な他登録の
  存在が隠れない。
* Bad, because PATH 上の cheaper binary が dedup 導入前の version
  の場合、複数登録がそれぞれ独立に発火して評価も個別に行われる。
  影響は duplicate evaluation(同一 tool call の token 計測・
  bounded materialization・bounded search scan が firing 毎に
  独立実行される — `eval_cache.rs` module doc)のみで verdict
  は同一・correctness は維持されるが、 residual risk として
  `docs/guides/distribution.md` に明記する。
* Bad, because stale な foreign entry は残り続ける — verify は
  informational finding として報告し続けるが、blocker でなくなった
  分だけ見落としリスクが残る。
* Bad, because conflict を消費者が放置しても気づきにくくなる
  (exit code に現れない)。warning note / finding の文言で緩和する。

### Confirmation

* conflict 有りで `sync` が upsert を実行して exit 0、result に
  `legacyConflicts` + warning note が載ることを integration test
  で固定する。
* conflict-only finding の `verify` が `clean:true`・exit 0 を
  返し、他 blocker では従来どおり `clean:false`・exit 2 になる
  ことを test で固定する。
* `--event-stdin` の `post-install`/`post-update` が conflict
  有りでも exit 0 で収束する regression test、および
  `post-uninstall` の `packages` 判定不能 branch が conflict-only
  residual で exit 0 を返す test を追加する。
* `docs/guides/distribution.md` が旧 binary caveat と新しい gate
  挙動を反映すること。
* `mise run check` が pass すること。

## Implementation Plan

* Affected paths:
  - `crates/cheaper-cli/src/hook_config.rs`
    - `run_sync`(:760-768): conflict 検出時の `invalid_request`
      early return を除去し、upsert 継続 + result への
      `legacyConflicts` array / warning note 追加へ変更。
    - `run_verify`(:668-678): conflict 検出を `blocker = true`
      から外し informational finding 化。finding 文字列の
      "(remove it before sync — duplicate firing is unsafe)" は
      新前提と矛盾するため書き換え対象。
    - `run_remove`(:926-939): `residualRegistrations` note の
      文言のみ新規約へ整合(挙動変更なし)。
    - `run_event_stdin`(:1001-1007): コード変更は不要な見込み
      (verify exit code をそのまま返す構造)。regression test で
      収束を固定する。
  - `crates/cheaper-cli/tests/integration/hook_config.rs`
    - :132 `hook_config_sync_fails_on_legacy_conflict` は sync
      exit 2 + no write を assert しており新規約と矛盾 —
      書き換え対象。`--event-stdin` × conflict の収束 test を
      追加する。
  - `scripts/test/devin-hooks-install.test.mjs`
    - :269-285 "stops on legacy cheaper registrations in known
      sources" が旧 gate 挙動(非ゼロ exit・config 非作成・
      `/legacy|conflict/i` 出力)を assert しており新規約と矛盾 —
      exit 0・config 書込み済み・warning/conflict が出力に表れる
      assert へ書き換え(`node --test` pipeline が `mise run
      check` 内で実行されるため必須)。
  - `docs/guides/distribution.md`
    - sync の安全条件(130-141 行)、lifecycle 節(148-169 行)、
      移行節(171-182 行)の現行 gate 前提の記述を warning 化 +
      dual-firing dedup 動作へ更新。旧 binary caveat(重複発火時
      は duplicate evaluation のみ、verdict は同一)を追記。
  - `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
    - :156-208 の hook-config + lifecycle 節を更新 — :167-171 の
      "`sync` refuses to write while a legacy cheaper read-hook
      registration coexists … resolve duplicates explicitly" 記述と
      :194-199 の post-uninstall 段落を新規約へ整合させる。
      更新対象は配布 source の同 path であり、deploy 写しの
      `.agents/skills/cheaper-usage/SKILL.md`(同節を持たない)
      ではない。
* Patterns to follow:
  - sync result の conflict field 名は verify result の既存
    `legacyConflicts`(`hook_config.rs:679-682`)をそのまま使い、
    同一概念に別名を作らない(表現ドリフト回避)。
  - own entry の識別・atomic write・strict-shape 規約など
    `hook_config.rs` の既存 contract は維持する。
  - JSON-first CLI contract(result object への field 追加で報告)
    に倣う。
* Patterns to avoid:
  - conflict bypass 用の新規 CLI flag(`--allow-conflict` 等) —
    lifecycle 経路で届かず dead weight になる。
  - legacy file 内の他登録を sync/remove が削除・改変する
    auto-cleanup — own-entry-only 所有境界違反。
  - findings の構造化(severity field 追加等)を ADR 段階で確定
    すること — spec の確定事項。
* Dependencies or configuration: 新規依存・新規 env var なし。
  dedup 側(`hook/eval_cache.rs`)の変更も不要。
* Expected tests:
  - conflict 有り `sync`: upsert 実行・exit 0・result に
    `legacyConflicts` + warning note。`--quiet` でも両 field が
    result に残ること(既存 `note` の quiet gate
    `hook_config.rs:852-858` を複製しない)。
  - conflict 有り `sync` の既存 error 条件: malformed config・
    modified own entry・duplicate own entry は引き続き exit 2・
    `ok:false` error envelope。
  - `verify`: conflict-only で `clean:true`・exit 0。own entry
    不在等他 blocker 併存時は従来どおり `clean:false`・exit 2。
  - `--event-stdin` `post-install`/`post-update` × conflict:
    exit 0 で収束。`post-uninstall`(`packages` 判定不能)×
    conflict-only residual: verify code passthrough で exit 0。
  - `scripts/test/devin-hooks-install.test.mjs`: legacy
    registration 共存時に install が exit 0・config 書込み済み・
    warning/conflict を出力に含める。
  - `remove`: residual 報告が従来どおり informational で、文言が
    新規約と整合すること。

## Verification

* [ ] conflict 有りの project で `cheaper hook-config sync` が
      upsert を実行し exit 0、result JSON に `legacyConflicts`
      array と warning note が含まれることを
      `cargo test -p cheaper-cli --test integration hook_config`
      の test で確認する。
* [ ] `verify` が conflict-only finding で `clean:true`・exit 0
      を返し、own entry 不在等他 blocker では `clean:false`・
      exit 2 のままであることを test で確認する。
* [ ] `--event-stdin` の `post-install`/`post-update` payload が
      conflict 有りでも exit 0 で終了し、`post-uninstall` で
      `packages` が判定不能かつ conflict-only residual のときも
      exit 0 を返す regression test が pass する。
* [ ] `run_remove` の挙動が不変で `residualRegistrations` note の
      文言のみ新規約へ整合していることを test で確認する。
* [ ] finding / warning note の文言に "duplicate firing is unsafe"
      が残っていないことを確認する。
* [ ] `docs/guides/distribution.md` と
      `packages/apm/.apm/skills/cheaper-usage/SKILL.md` が
      warning 化後の gate 挙動と旧 binary caveat(dedup 無しでは
      duplicate evaluation のみ、verdict 同一)を記述していることを
      review で確認する。
* [ ] `scripts/test/devin-hooks-install.test.mjs` が新規約
      (conflict 共存で install が exit 0・config 書込み済み)を
      assert し、`node --test` pipeline が pass する。
* [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
      integration` と `mise run check` が pass する。

## Pros and Cons

### Option 1 — gate を warning へ降格(chosen)

* Good, because dedup により「重複発火は unsafe」の前提が消え、
  correctness を保ったまま native + import 共存を許可できる。
  lifecycle 経路が flag 無しで収束し、issue #128 の採択案。
* Bad, because conflict 情報の見落としリスクは残る(note/finding
  で緩和)。旧 binary caveat の文書化が必要になる。

### Option 2 — 現行 gate 維持(conflict を blocker のまま)

* Good, because 変更不要であり、重複登録を強制的に排除し続けられる。
* Bad, because dedup 導入後は前提が消えており、正当な共存ケース
  (`.devin/config.json` native + Claude import)を永久に block
  する。Devin import は cheaper 制御外で、user へ解消を強制し
  続ける設計になる — 共存不能が実害。

### Option 3 — `--allow-conflict` flag で opt-in bypass

* Good, because gate を残しつつ明示的に bypass できる。
* Bad, because APM lifecycle `--event-stdin` には flag 伝達路が
  無く(stdin は event JSON のみ)、primary use case の post-install
  自動収束では使えない dead weight になる — deciding criterion
  を満たさないため棄却(issue #128 で却下済み)。

### Option 4 — sync 側 auto-cleanup(他登録を自動削除)

* Good, because user 操作不要で単一登録へ収束する。
* Bad, because 他 agent が管理する登録を所有境界を越えて削除する
  設計であり、`hook-config` の own-entry-only 不変条件
  (`hook_config.rs:855-858`、ADR-034H2WBNtC4We8dyC5lQZJ の
  ownership 決定)に反する。import が再有効な環境では再生成されて
  イタチごっこになる。

## More Information

* issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  — 本決定の起点(gate 降格 proposal 採択済み、`--allow-conflict`
  却下済み)。
* issue #127 / PR #131:
  <https://github.com/scarecrow173/cheaper/issues/127>・
  <https://github.com/scarecrow173/cheaper/pull/131> — dedup 実装。
  gate の前提を除去した upstream 変更。
* upstream discovery:
  [hook-config-legacy-conflict-gate-demotion-to-warning.md](../discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md)
  (DISC-034IKBmCjJBeLD5qG9DoUM) — 現行挙動の file:line 証跡・
  代替案比較・open questions を含む。
* **ADR-034H2WBNtC4We8dyC5lQZJ との関係**: 本 ADR は
  [cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md](./cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md)
  (issue #107)の conflict-gate 部分決定を amend する
  (blocker → warning)。旧 ADR 全体は replace されないため
  `superseded-by` ではなく `related` のみで link する。
  `refines`/`refined-by` は route_graph の lineage edge であり、
  chain の異なる doc 間で張ると focus component が merge される
  ため使わない(spec :392-394・discovery :263-265 と同じ規約)。
  旧 ADR の status(`accepted`)と body は変更しない —
  front matter の `related` に back-link を追加するのみ。
* 旧 ADR-034H2WBNtC4We8dyC5lQZJ の body には本降格と無関係な
  現行 code との drift が残る(unknown event を「明示 error」と
  記述 :137 vs 実装は noop で exit 0 `hook_config.rs:1044-1050`、
  post-install → `sync` のみと記述 :131 vs 実装は sync+verify
  `hook_config.rs:1001-1007`)が、本変更の scope 外としてそのまま
  残す。
* 関連 ADR:
  [dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md](./dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md)
  (ADR-034Ie3HFd6tIJms6Z2NIow) — 本決定の前提となる dedup
  決定。その Consequences で本降格を follow-up の余地として
  言及済み。
* spec で確定する詳細: verify findings の informational / blocker
  表現形態、sync result の warning note JSON shape・文言・
  stderr 出力有無、`legacyConflicts` と `residualRegistrations`
  の用語整合。
