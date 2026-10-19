---
id: "DISC-034IKBmCjJBeLD5qG9DoUM"
type: "discovery"
status: "draft"
title: "hook-config legacy conflict gate demotion to warning"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
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
    - "./hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
    - "../specs/hook-duplicate-firing-single-evaluation-dedup.md"
    - "../designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "docs/guides/distribution.md"
    - ".agents/skills/cheaper-usage/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# hook-config legacy conflict gate demotion to warning

## Exploration Goal

issue #127 で導入された hook-eval dedup(canonical request fingerprint
による verdict cache + leader election、PR #131、
`crates/cheaper-cli/src/hook/eval_cache.rs`)により、複数の登録 path
(`.devin/config.json` native + `.claude/settings.json` import 等)が
同一 canonical request(fingerprint 一致)で発火しても評価は 1 回に
収束し、全 firing が同一の canonical verdict を返すようになった
(非一致 fingerprint の firing は独立評価 — correctness の根拠は
request 単位の verdict 決定性)。これは現行 legacy conflict
gate の前提(「重複発火は unsafe なので未然に防ぐ」)を取り除く。

一方、既に cheaper hook 登録を持つ consumer project(例: Claude
Code の `.claude/settings.json`)では、`cheaper hook-config sync` が
同 gate に block され、`.devin/config.json` への native 登録と既存
hook の共存ができない(issue #128)。

本 discovery が扱う問い: **dedup 導入後の前提のもとで、legacy
conflict gate を blocker から warning へ降格し、native + import 登録の
共存を許すには `hook-config` の contract をどう変更するか。**

大枠の設計判断は issue #128 側で採択済み(blocker → warning 降格、
`--allow-conflict` flag 却下)であり、本書はその確定方針・適用境界・
代替案・未解決点・昇華先を整理する探索記録である。降格自体の可否は
ここでは再審しない。

## Key Issues and Assumptions

- **gate の現行挙動(confirmed)**:
  `legacy_conflicts()`(`hook_config.rs:362`)が
  `KNOWN_LEGACY_FILES`(`.windsurf/hooks.json`・`.devin/hooks.json`・
  `.devin/hooks.v1.json`・`.devin/config.local.json`・
  `.claude/settings.json`・`.claude/settings.local.json`、
  `hook_config.rs:42-49`)+ target file を走査し、command-ish な
  JSON 文字列に対して
  `CONFLICT_PATTERN`(`cheaper[^\r\n]*hook[^\r\n]*(pre-read-code|pre-tool-use)`、
  `hook_config.rs:38`)を適用する。target file 内では
  `OWN_COMMAND` 一致のみ除外される(`hook_config.rs:384-388`)。
- **verify は conflict を blocker 扱い(confirmed)**:
  `run_verify`(`hook_config.rs:587`)は conflict 検出で
  `blocker = true` → `clean:false`・exit 2、finding は
  `"legacy-conflict: {f} ... (remove it before sync — duplicate firing is unsafe)"`
  (`hook_config.rs:668-678`)。
- **sync は conflict で拒否(confirmed)**:
  `run_sync`(`hook_config.rs:743`)は conflict 検出で
  `invalid_request` error を返し upsert 自体を実行しない
  (`hook_config.rs:760-768`)。
- **remove は既に informational(confirmed)**:
  `run_remove`(`hook_config.rs:864`)は残余 conflict を
  `residualRegistrations` + note として報告するのみ
  (`hook_config.rs:926-939`)。挙動変更は不要で文言整合のみ。
- **`--event-stdin` は verify の exit code を返す(confirmed)**:
  `run_event_stdin`(`hook_config.rs:973`)は
  `post-install`/`post-update` で `run_sync` + `run_verify` を順に
  実行し verify の exit code をそのまま返す
  (`hook_config.rs:1001-1007`)。現状では conflict があると sync が
  error で落ちるか verify が exit 2 を返し、lifecycle が収束しない。
- **dedup が gate の前提を除去(confirmed)**: `eval_cache.rs` は同一
  canonical request・同一 file 状態・同一 threshold の firing 間で
  計測・実体化を高々 1 回へ dedup し、全 firing に同一 canonical
  verdict を返す(PR #131  merged、issue #127 系 artifact 群を
  参照)。「重複発火は unsafe」はもはや成立しない。なお dedup が
  収束させるのは fingerprint 一致の canonical request 間のみで、
  非一致 firing は独立に評価される(cost は per-request bounded)
  — correctness の根拠は request 単位の verdict 決定性であり、
  「全 firing の合一」ではない。target 間で normalize も
  divergence しうる(例: devin の cwd は payload.cwd→
  `DEVIN_PROJECT_DIR`→deps.cwd、pretooluse は deps.cwd のみ —
  `targets.rs:535-541`,`:651`)。
- **lifecycle 経路には flag 伝達路が無い(confirmed)**:
  `--event-stdin` の入力は stdin の event JSON のみであり、APM の
  lifecycle script から追加 flag を渡す経路が無い。primary use case
  が lifecycle 自動収束である以上、conflict 解消用の opt-in flag は
  実質使えない。
- **所有境界(confirmed)**: `hook-config` は own entry のみを管理し、
  他 agent / 無関係 hook の entry は保全する
  (`hook_config.rs:855-858` の note、remove の strict-shape 規約
  `hook_config.rs:911-913`)。sync が legacy file 内の他登録を消す
  auto-cleanup はこの境界を越える。
- **旧 binary caveat(confirmed)**: dedup を持たない旧 cheaper
  binary が PATH 上に居て重複登録が発火した場合、影響は
  duplicate evaluation(token 計測・bounded materialization・
  bounded search scan の重複 — `eval_cache.rs` module doc)のみ —
  verdict は同一で correctness は維持される。docs で明記対象。

## Current vs Desired Behavior

| command | 現行 | 目標 |
| --- | --- | --- |
| `sync` | conflict 検出で `invalid_request` error、upsert 不実行、exit 2(`hook_config.rs:760-768`) | upsert を実行し、result に `legacyConflicts` array + warning note を載せて exit 0 |
| `verify` | conflict → blocker → `clean:false`・exit 2・"duplicate firing is unsafe" finding(`hook_config.rs:668-678`) | conflict を informational finding として報告。conflict-only finding では `clean:false`/exit 2 にしない(他 blocker の扱いは不変) |
| `remove` | residual conflict → `residualRegistrations` + informational note(`hook_config.rs:926-939`) | 挙動不変。文言を新規約へ整合させるのみ |
| `--event-stdin` (`post-install`/`post-update`) | conflict 有りで sync error または verify exit 2 → lifecycle 非収束(`hook_config.rs:1001-1007`) | conflict があっても verify が exit 0 を返し収束する |
| `--event-stdin` (`post-uninstall`、`packages` 判定不能) | remove せず `run_verify` code を透過 — conflict-only residual で exit 2(`hook_config.rs:1023-1036`) | verify code passthrough を継承し conflict-only で exit 0 |

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| (a) gate を warning へ降格 — sync は upsert 継続し `legacyConflicts` + warning note を result で報告、verify は informational | dedup により「重複発火は unsafe」の前提が消え、correctness を保ったまま native + import 共存を許可できる。lifecycle 経路が flag 無しで収束する。issue #128 の採択案 | conflict 情報の見落としリスクは残るが note/finding で緩和できる。旧 binary caveat の文書化が必要 | yes(issue で採択済み) |
| (b) 現行 gate 維持(conflict を blocker のまま) | 変更不要。重複登録を強制的に排除し続ける | dedup 導入後は前提が消えており、正当な共存ケース(`.devin/config.json` native + Claude import)を永久に block する。Devin import は cheaper 制御外で、user へ解消を強制し続ける設計になる | no |
| (c) `--allow-conflict` flag を追加して opt-in bypass | gate を残しつつ明示的に bypass できる | APM lifecycle `--event-stdin` に flag 伝達路が無く、primary use case の lifecycle 自動収束では使えない dead weight になる — issue #128 で却下済み | no(rejected) |
| (d) sync 側 auto-cleanup(conflict 検出時に legacy entry を自動削除して収束) | user 操作不要で単一登録へ収束する | 他 agent が管理する登録を所有境界を越えて削除する設計であり、`hook-config` の own-entry-only 不変条件に反する。import が再有効な環境では再生成されてイタチごっこになる | no |

## Tentative Conclusions and Hypotheses

以下は issue #128 で採択済みの設計判断(decided — 本書は再審せず記録する):

- **gate は blocker → warning へ降格(decided)**: `sync` は upsert を
  実行し、result の `legacyConflicts` array + warning note で conflict
  を報告して exit 0。`--allow-conflict` flag は付けない
  (lifecycle 経路に flag 伝達路が無く dead weight になるため却下)。
- **verify は informational finding(decided)**: conflict 検出は
  finding として報告するが blocker にはしない。conflict-only finding
  では `clean:false`/exit 2 にしない。
- **remove は文言調整のみ(decided)**: residual 報告は既に
  informational であり、挙動変更は不要。
- **`--event-stdin` は収束(decided)**: `post-install`/`post-update`
  が conflict 有りでも verify exit 0 で終了する。
- **caveat の文書化(decided)**: dedup 無しの旧 binary が重複登録で
  発火しても影響は duplicate evaluation のみ(verdict は同一、
  correctness 維持)である旨を `docs/guides/distribution.md` の
  lifecycle 節に記す。cheaper-usage skill については訂正する:
  deployed copy の `.agents/skills/cheaper-usage/SKILL.md` には
  lifecycle/hook-config 節が存在しない(確認済み)が、consumer へ
  配布される source の
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md` には
  :156-208 に hook-config + lifecycle 節があり、:167-171 に旧
  gate 記述("`sync` refuses to write while a legacy cheaper
  read-hook registration coexists … resolve duplicates
  explicitly")と :194-199 に post-uninstall 段落があるため
  packages 側 source が更新対象。

gap 分析(confirmed、実装対象の file:line 差分):

- `run_sync`(`hook_config.rs:760-768`): conflict 検出 →
  `invalid_request` early return を除去し、upsert 継続 +
  result 報告へ変更。`legacyConflicts` field は verify result に
  既存(`hook_config.rs:679-682`)なので sync result にも同名で載せる
  形が表現ドリフトを起こさない(hypothesis — spec で確定)。
- `run_verify`(`hook_config.rs:668-678`): conflict 検出を
  `blocker = true` から外し informational finding 化。finding 文字列の
  "(remove it before sync — duplicate firing is unsafe)" という文言は
  新前提(重複発火は dedup で無害化)と矛盾するため書き換え対象。
  findings は現行 `Vec<Value>` の文字列配列であり、informational /
  blocker の区分をどう表現するか(prefix 変更か構造化か)は未確定。
- `run_remove`(`hook_config.rs:926-939`): `residualRegistrations`
  note の文言のみ新規約へ整合。
- `run_event_stdin`(`hook_config.rs:1001-1007`): コード変更は
  verify 側の exit code 変化に依存して自然に収束する見込み
  (verify_code をそのまま返す構造のため)。要 regression test。
  `post-uninstall` の `packages` 判定不能 branch(:1023-1036)も
  `run_verify` の code を透過するため、conflict-only residual で
  exit 2→0 に変わる — `packages` 含有時の `run_remove` が
  residual を informational 報告して exit 0 の現行規約と整合。
- test(`crates/cheaper-cli/tests/integration/hook_config.rs:132`):
  `hook_config_sync_fails_on_legacy_conflict` は sync exit 2 +
  no write + verify conflict 報告を assert しており新規約と矛盾 —
  書き換え対象。`--event-stdin` × conflict の収束 test の追加が必要。
- test(`scripts/test/devin-hooks-install.test.mjs:269-285`):
  "stops on legacy cheaper registrations in known sources" が旧
  gate 挙動(非ゼロ exit・config 非作成・`/legacy|conflict/i`
  出力)を assert — 降格後は install が exit 0 + config 書込みに
  なるため `node --test` pipeline(`mise run check` 内)を壊す。
  exit 0・config 書込み済み・warning/conflict 出力の assert へ
  書き換え対象。
- docs(`docs/guides/distribution.md`): `sync` の安全条件
  (130-141 行、conflict → 停止の記述)、lifecycle 節(148-169 行)、
  移行節(171-182 行、"conflict gate が発火しうる" の記述)が現行
  gate 前提の記述 — dual-firing dedup 動作と warning 化を反映する。
- docs(`packages/apm/.apm/skills/cheaper-usage/SKILL.md`): 上記
  訂正のとおり :156-208 の hook-config + lifecycle 節が旧 gate
  前提の記述を含む — :167-171("sync refuses to write …")と
  :194-199 の post-uninstall 段落を新規約へ整合させる。
- 元の gate 判断を記録した
  ADR-034H2WBNtC4We8dyC5lQZJ(`docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`、
  issue #107)の方針部分が本変更で更新される — amend/supersede の
  扱いは未確定。

## Open Questions

- verify の findings における informational / blocker の表現形態
  (finding 文字列の prefix 規約か、severity field の追加か)—
  spec で確定する。
- sync result の `legacyConflicts` と warning note の JSON shape・文言、
  および remove の `residualRegistrations` との用語整合(同じ概念に
  2 つの field 名がある)— spec で確定する。
- 旧 gate 方針を記録した ADR-034H2WBNtC4We8dyC5lQZJ への扱い —
  降格は「dedup が前提を除去した」という因果を伴う方針更新であり、
  新 ADR(amend/supersede 関係)を切るか spec のみで扱うか。
- sync 成功時の warning note が人間可読 stderr でも出るべきか、
  result JSON のみか(quiet 時の振る舞いを含む)— spec で確定する。

## Promotion Candidates

- [ ] spec-doc needed: gate 降格後の `hook-config` 各 command の
  contract(sync/verify/remove/`--event-stdin` の exit code・result
  field・finding 文言・収束条件)と旧 binary caveat の文書化要件を
  formalize する
- [ ] adr-doc needed: かもしれない — blocker 前提を記録した
  ADR-034H2WBNtC4We8dyC5lQZJ の方針更新を ADR として残すかどうかは
  Open Questions のとおり未確定

## Sources and Relations

- issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  (本 discovery の起点 — gate 降格 proposal 採択済み)
- issue #127 / PR #131: <https://github.com/scarecrow173/cheaper/issues/127>・
  <https://github.com/scarecrow173/cheaper/pull/131>
  (dedup 実装 — gate の前提を除去した upstream 変更)
- #127 系 artifact(`related` 参照。本 chain の `derives-from` には
  含めない — lineage を分離して route_graph の focus detection を
  壊さないため):
  `docs/discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md`
  (DISC-034IdYnTDqD7Ao1nG3aoY9、gate 降格を hypothesis として記録)、
  `docs/specs/hook-duplicate-firing-single-evaluation-dedup.md`
  (SPEC-034Ie1ZEdUR2aBFVoqkBay、Out of Scope / Deferred Design
  Concerns で本降格を follow-up として明記)、
  `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
  (DESIGN-034IefWW6OJ9rvESaNvKUK)、
  `docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md`
  (ADR-034Ie3HFd6tIJms6Z2NIow)
- gate の定義元:
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  (ADR-034H2WBNtC4We8dyC5lQZJ、issue #107)、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  (DESIGN-034H3c448Lvz0CfBjd5Xlz)
- code evidence:
  `crates/cheaper-cli/src/hook_config.rs`
  (`CONFLICT_PATTERN` :38、`KNOWN_LEGACY_FILES` :42-49、
  `legacy_conflicts` :362 / unparseable skip :376-378、
  `run_verify` :587 / conflict 処理 :668-682 / `remediation`
  :730-738、`run_sync` :743 / gate :760-768 / `note` quiet gate
  :852-858、`run_remove` :864 / residual :926-939、
  `run_event_stdin` :973 / mapping :1001-1007 / `post-uninstall`
  判定不能 :1023-1036 / unknown noop :1044-1050)、
  `crates/cheaper-cli/src/hook/eval_cache.rs`(dedup 実装 —
  module doc :4-7)、`crates/cheaper-cli/src/hook/targets.rs`
  (devin cwd 解決 :535-541、pretooluse :651)
- test evidence:
  `crates/cheaper-cli/tests/integration/hook_config.rs:132`
  (`hook_config_sync_fails_on_legacy_conflict` — 書き換え対象)
- docs 対象: `docs/guides/distribution.md`(130-141・148-169・
  171-182 行)、`packages/apm/.apm/skills/cheaper-usage/SKILL.md`
  (:156-208 の hook-config + lifecycle 節 — 更新対象)。
  `.agents/skills/cheaper-usage/SKILL.md` は deployed copy で
  lifecycle/hook-config 節を持たず(確認済み)、本 repo 内の更新
  対象は packages 側 source である
- test 対象:
  `crates/cheaper-cli/tests/integration/hook_config.rs:132` に加え
  `scripts/test/devin-hooks-install.test.mjs:269-285` も旧 gate
  assert の書き換え対象(後者は `node --test` pipeline 経由で
  `mise run check` を壊しうる)
