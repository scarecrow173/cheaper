---
id: "SPEC-034IKGkW4dQEVw6QigMFxQ"
type: "spec"
status: "approved"
title: "hook-config legacy conflict gate demotion to warning"
created: "2026-10-19"
updated: "2026-10-20"
owners: ["scarecrow173"]
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
    - "./hook-duplicate-firing-single-evaluation-dedup.md"
    - "../adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "../adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md"
    - "../designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "../discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./hook-config-orphan-sweep-ownership-boundary.md"
    - "../plans/restrict-hook-config-orphan-sweep-to-cheaper-owned-temp-files.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "docs/guides/distribution.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# hook-config legacy conflict gate demotion to warning

## What Should Be Built

`cheaper hook-config` の legacy conflict gate を blocker から
warning へ降格する。issue #127 の hook eval dedup（canonical
request fingerprint による verdict cache + leader election、PR #131、
`crates/cheaper-cli/src/hook/eval_cache.rs`）により、複数の登録
path（`.devin/config.json` native + `.claude/settings.json`
import 等）からの発火は、fingerprint が一致する同一 canonical
request の間で評価 1 回へ収束し、全 firing が同一の canonical
verdict を返す。ただし fingerprint が一致しない firing はそれぞれ
独立に評価される（cost は per-request bounded）ため、correctness
の根拠は「全 firing の合一」ではなく request 単位の verdict
決定性にある。また normalize は target 間で divergence しうる
（例: devin の cwd 解決は payload.cwd→`DEVIN_PROJECT_DIR`→
deps.cwd の順、pretooluse は deps.cwd のみ —
`crates/cheaper-cli/src/hook/targets.rs:535-541`,`:651`）ため、
dedup の coverage を「あらゆる重複 firing の合一」とは主張しない。
その上で、現行 gate の前提（「重複発火は unsafe なので未然に
防ぐ」）は除去されている。

求める振る舞い（command 別 contract 変更）:

1. `sync` は legacy conflict を検出しても own entry の upsert を
   継続し、result envelope に `legacyConflicts` array と
   `warning` 文字列を載せて exit 0 を返す。conflict 解消用の
   flag は追加しない。
2. `verify` は conflict を informational finding として報告する
   （blocker にしない）。finding が conflict のみの状態では
   `clean:true`・exit 0 を返す。`legacyConflicts` array は
   引き続き populate される。
3. `remove` の residual 報告（`residualRegistrations` + note）は
   informational のまま維持し、文言のみ新前提へ整合させる。
4. `--event-stdin` の `post-install`/`post-update` は conflict が
   あっても verify の exit code（0）を返して収束する。

## Why Now

- 既に cheaper hook 登録を持つ consumer project（例: Claude
  Code の `.claude/settings.json`）では、`cheaper hook-config
  sync` が同 gate に block され、`.devin/config.json` への native
  登録と既存 hook の共存ができない（issue #128）。
- dedup 導入（issue #127 / PR #131）で gate の前提が除去された。
  現行 gate を維持すると、Devin の `read_config_from` import
  merge のような cheaper 制御外の重複登録を user へ解消強制
  し続ける設計になる。
- APM lifecycle `--event-stdin` 経路には flag 伝達路が無く、
  conflict 有りで sync error または verify exit 2 となり
  lifecycle が収束しない。primary use case である lifecycle 自動
  収束を実現するには blocker → warning 降格が必須。
- 大枠の方針は issue #128 で採択済み（blocker → warning 降格、
  `--allow-conflict` flag 却下）であり、本 spec は採択済み方針の
  確定 contract を定義する。

## Users and Value

- **multi-agent 環境で cheaper を導入する consumer / parent
  coding agent（Devin + Claude Code 等）**: 既存 hook 登録を
  残したまま `.devin/config.json` へ native 登録でき、native +
  import 共存が警告付きで許可される。
- **APM lifecycle 経路**: `post-install`/`post-update` が conflict
  有りでも自動収束し、人手介入なしで登録が完了する。
- **cheaper CLI maintainer**: conflict 情報は `legacyConflicts`・
  finding・`warning` として引き続き可観測であり、blocker 降格で
  診断性を失わない。

## Scope

### In Scope

- `run_sync`・`run_verify`・`run_remove`・`run_event_stdin`
  （`crates/cheaper-cli/src/hook_config.rs`）の conflict 扱い変更、
  result envelope field・finding 文言・exit code 規約の更新。
- integration test の更新・追加
  （`crates/cheaper-cli/tests/integration/hook_config.rs`）。
- `scripts/test/devin-hooks-install.test.mjs` の更新 — :269-285 の
  "stops on legacy cheaper registrations in known sources" case が
  旧 gate 挙動（install 非ゼロ exit・config 非作成・出力に
  `/legacy|conflict/i`）を assert しており、降格後の sync が
  exit 0 + config 書込みになると `node --test` pipeline
  （`mise run check` 内、本 repo は Windows-hosted で実行対象）を
  壊すため、exit 0・config 書込み済み・warning/conflict が出力に
  表れる assert へ書き換える。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の記述更新 —
  同 file は consumer へ配布される skill の source であり、
  :156-208 に hook-config + lifecycle 節を持つ。:167-171 の
  "`sync` refuses to write while a legacy cheaper read-hook
  registration coexists … resolve duplicates explicitly" という旧
  gate 記述と、:194-199 の post-uninstall 段落を新規約へ整合
  させる。repo 内 `.agents/skills/cheaper-usage/SKILL.md` は deploy
  後の写しで同節を持たない（確認済み）— 更新対象は packages 側
  source であり、discovery が「節が存在しない」と記録したのは
  deployed copy の方である。
- `docs/guides/distribution.md` の記述更新（`sync` 安全条件・
  lifecycle 節・移行節）と旧 binary caveat の追記。

### Out of Scope

- conflict 検出時の foreign registration 自動除去（auto-cleanup）。
  `hook-config` は own entry のみを管理し他 agent / user の entry
  を触らない所有境界を越えるため却下（discovery の alternative
  (d)）。
- `--allow-conflict` 等の flag 追加。`--event-stdin` の lifecycle
  経路に flag 伝達路が無く dead weight になるため issue #128 で
  却下済み。
- capability gate（PATH 上の cheaper が `hook targets --json` で
  devin target + `hookConfig` を報告する check）の変更。
- dedup 機構・verdict 意味論・canonical matcher / own entry shape
  の変更。
- `.devin/hooks.v1.json` の生成・rewrite（shadow 報告のみの現行
  維持）。`shadow` finding は shadow file が own entry を実質
  inert にしうる場合でも informational のままとし、observability
  のみで gate へ戻さない。
- 旧 binary（dedup 無し）への実行時対策。影響は duplicate
  evaluation（同一 tool call の token 計測・bounded
  materialization・bounded search scan が firing 毎に独立実行
  される — `eval_cache.rs` module doc :4-7）のみで verdict は
  同一のため、docs 記述のみの mitigation とする（R11）。
- 歴史的 doc の遡及更新 — 旧 blocker semantics を記述する
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:144`・
  `docs/tasks/cli-owned-hook-config-lifecycle.md:81-82` は降格前の
  記録（historical record）としてそのまま残し、更新対象にしない。

## Requirements

### Envelope / Field Contract

- R1: `sync` result は従来の key（`configPath`・`scope`・
  `changed`・`matcher`・`orphansRemoved`・`note`）に加え、
  `legacyConflicts`（string array、conflict を含む file path
  一覧）を常に含める。ここでの「常に」は sync が成功して result
  map を返す場合に限る — `ok:false` error envelope には result
  map が無いため適用外とする。key 名・shape は verify result の
  同名 field と同一とし、表現ドリフトを起こさない。conflict 無し
  では空 array。
- R2: `sync` result は conflict 検出時のみ `warning`（string、
  「legacy file に cheaper hook 登録が残る。重複発火は評価時
  dedup で無害化されるが、単一登録へ収束したい場合は除去
  可能」旨の人間可読文言）を含める。`warning` は result JSON
  のみに載せ、別途 stderr channel は設けない。
  `warning`・`legacyConflicts` は `--quiet` の有無に関わらず出力
  する（既存 `note` の quiet gate は維持）。
- R3: `sync` は conflict 検出で `invalid_request` を返さず
  upsert を実行する。conflict 以外の既存 error 条件（capability
  gate 失敗・malformed/unreadable config・modified own entry・
  duplicate own entry・外部変更検出・symlink/reparse 経路）は
  従来どおり error（envelope `ok:false`・exit 2）。
- R4: `verify` result の `legacyConflicts` array は現行どおり常に
  populate される。conflict 検出は blocker を立てず、
  informational finding として `findings` に載せる。
- R5: verify の conflict finding は `legacy-conflict:` prefix を
  維持するが、文言は新前提へ書き換える。現行の
  "(remove it before sync — duplicate firing is unsafe)" は dedup
  導入後に偽となるため、「重複発火は評価時に dedup されるため
  共存は安全。単一登録へ収束したい場合は除去可能」旨の文言に
  する。同じく verify result の `remediation` 文字列
  （`hook_config.rs:730-738`、"run `cheaper hook-config sync`
  after resolving any legacy conflicts; see docs/guides/distribution.md
  for the APM lifecycle wiring"）も書き換え対象とする — 降格後は
  conflict 共存下でも sync が書き込むため、conflict 解消を sync
  の前提条件のように読ませる現行文言は偽になる。
- R6: findings は従来どおり string array のままとし、
  informational / blocker の区分に新しい severity field は
  導入しない。`clean` flag は blocker 系 finding
  （`config-unreadable`・`missing`・`modified`・`duplicate`・
  `absent-config`）のみから導出する。`matcher-drift`・`shadow`・
  `orphan`・`legacy-conflict` は informational で `clean` に
  影響しない（`matcher-drift` は現行でも blocker を立てずに
  finding 化される — `hook_config.rs:637-641`。informational
  finding と `clean:true` の併存は既存 semantics）。
- contract note: `legacy_conflicts`（`hook_config.rs:362-394`）は
  parse 不能な sibling JSON file を黙って skip する（:376-378）—
  conflict 検出は best-effort であり、malformed な legacy file 内の
  登録は `legacyConflicts`・finding のいずれにも現れない。この
  best-effort 規約自体は変更しないが、同所 :377 の code comment
  （"parse 不能 file は verify の finding で別途報告"）は stale で
  ある — verify は sibling file の parse 失敗を finding として
  報告しない（`config-unreadable` は target config path のみを
  対象とする :611-617）。実装時に comment を実動作へ修正する。
- R7: `remove` の residual 報告は key 名 `residualRegistrations`
  と informational note を維持し、note 文言のみ新規約へ整合
  させる（rename・構造変更はしない）。
- R8: `--event-stdin` の `post-install`/`post-update` は event
  result の envelope 構造（`sync`/`verify` subresult key + verify
  exit code の passthrough）を変えない — `result.sync` 自体は
  R1/R2 のとおり `legacyConflicts`・`warning` を得て、
  `result.verify` は R4-R6 の informational finding を載せる。
  conflict 有りでも verify が 0 を返すため lifecycle は収束する。

### Exit Code Contract

| command | state | exit | envelope |
| --- | --- | --- | --- |
| `sync` | conflict のみ | 0 | upsert 実行、`legacyConflicts` 非空 + `warning`（変更） |
| `sync` | config 不在 | 0 | config 新規作成（現行維持） |
| `sync` | malformed / unreadable config | 2 | `invalid_request` error（現行維持） |
| `sync` | own entry modified / duplicate-own | 2 | `invalid_request` error（現行維持） |
| `sync` | capability gate 失敗 | 2 | `invalid_request` error（現行維持） |
| `verify` | conflict のみ | 0 | `clean:true`、`legacyConflicts` 非空、informational finding（変更） |
| `verify` | missing own entry | 2 | `clean:false` + `missing` finding（現行維持） |
| `verify` | absent config | 2 | `clean:false` + `absent-config` finding（現行維持） |
| `verify` | malformed config | 2 | `clean:false` + `config-unreadable`（現行維持） |
| `verify` | modified / duplicate-own | 2 | `clean:false`（現行維持） |
| `verify` | matcher drift / shadow / orphan のみ | 0 | informational finding のみ（現行維持） |
| `remove` | residual conflict | 0 | `residualRegistrations` + note（現行維持、文言のみ整合） |
| `remove` | own absent / config 不在 | 0 | `changed:false`（現行維持） |
| `remove` | modified / duplicate-own | 2 | error（現行維持） |
| `--event-stdin` `post-install`/`post-update` | conflict のみ | 0 | `result.verify.clean:true`（変更 — 現行は sync error または verify 2） |
| `--event-stdin` `post-install`/`post-update` | verify blocker 有り | 2 | verify code を透過（現行維持） |
| `--event-stdin` `post-uninstall`（`packages` に cheaper 含有） | residual conflict | 0 | `run_remove` code を透過 — `residualRegistrations` + informational note（現行維持） |
| `--event-stdin` `post-uninstall`（`packages` 判定不能） | conflict のみ residual | 0 | `run_verify` code を透過 — conflict-only で exit 2→0（変更） |
| `--event-stdin` `post-uninstall`（`packages` 判定不能） | verify blocker 有り | 2 | `run_verify` code を透過（現行維持） |
| `--event-stdin` `post-uninstall`（cheaper 非含有）/ `pre-*` / unknown | — | 0 noop（現行維持） | `action:"noop"` + `reason`（現行維持） |
| `--event-stdin` | malformed payload | 2 | error（現行維持） |

- `post-uninstall` の `packages` 判定不能 branch（missing/非
  array）は remove を行わず `run_verify` の exit code をそのまま
  返す（`hook_config.rs:1023-1036`）ため、conflict-only の residual
  state では verify の新規約を継承して exit 2→0 に変わる。これは
  `packages` に cheaper が含まれる branch（:1010-1014 →
  `run_remove`）が residual conflict を `residualRegistrations` +
  note で informational に報告し exit 0 を返す現行挙動と整合する
  — 同一 event の両 branch が残留 conflict を non-blocking に
  扱う規約となる。
- 表は finding 由来の outcome のみを示す。finding を経ない
  non-finding error（`--event-stdin` 内での sync 内部 error 伝播
  `hook_config.rs:1002` の `?`、verify の arg/path 解決 error
  :592-593、capability gate・reparse point・外部変更検出などの
  `?` 伝播）は `ok:false` error envelope の exit 2 として
  propagate し、finding-based outcome とは別系統である。

### Backward Compatibility

- R9: verify の `legacyConflicts`・findings の `legacy-conflict:`
  prefix・`clean` field・exit code 規約の枠組みは維持する。
  sync result への `legacyConflicts`・`warning` 追加と finding /
  note 文言の変更は additive であり、既存 key の削除・rename・
  型変更は行わない。
- R10: conflict-only 状態での `clean:false→true`・exit 2→0 は
  schema ではなく behavioral contract の意図的な変更であり、
  本 spec の主変更として docs に明記する。

### Documentation

- R11: `docs/guides/distribution.md` を更新する — `sync` の安全
  条件の「conflict → 停止」記述を warning 化へ改訂し、lifecycle
  節・移行節（"conflict gate が発火しうる" の記述）を新規約へ
  整合させる。加えて、dedup を持たない旧 cheaper binary が PATH
  上に居て重複登録が発火した場合の影響は duplicate
  evaluation（measurement/materialization/scan の重複のみで
  verdict は同一 — `eval_cache.rs` module doc :4-7）であり
  correctness は維持される旨の caveat を lifecycle 節に記す
  （documentation-only mitigation）。

## Acceptance Criteria

- [ ] legacy conflict が存在する状態で `hook-config sync` を
  実行すると、config へ own entry が upsert され exit 0 で終了し、
  result に非空の `legacyConflicts` と `warning` が含まれることが
  test で固定される（既存
  `hook_config_sync_fails_on_legacy_conflict` —
  `crates/cheaper-cli/tests/integration/hook_config.rs:132` —
  の書き換え）。
- [ ] legacy conflict のみの状態（own entry present・他 blocker
  無し）で `hook-config verify` が exit 0・`clean:true` を返し、
  `legacyConflicts` が populate され、finding が
  "duplicate firing is unsafe" を含まないことが test で固定される。
- [ ] conflict 有りの状態で `hook-config --event-stdin` に
  `post-install` payload を与えると exit 0 で終了し config が
  書き込まれることが、新規 event-stdin × conflict test で固定
  される。
- [ ] 同じく conflict 有りで `post-update` payload を与えても
  exit 0 で終了し config が書き込まれることが test で固定される
  （post-install と同一 mapping の収束をカバーする）。
- [ ] `post-uninstall` payload で `packages` が判定不能
  （missing/非 array）かつ conflict-only residual の状態で
  `--event-stdin` が exit 0 を返すことが test で固定される
  （`run_verify` code の passthrough 継承 — `hook_config.rs:1023-1036`）。
- [ ] `hook-config sync --quiet` が conflict 有りでも exit 0 で
  result に `legacyConflicts`・`warning` を含めることが test で
  固定される — R2 のとおり両 field は quiet gate 対象外であり、
  既存 `note` の quiet gate（`hook_config.rs:852-858`）の書き方を
  そのまま複製しないことを guard する。
- [ ] conflict 有りでも malformed config・modified own entry・
  duplicate own entry の各条件で `hook-config sync` が引き続き
  exit 2・`ok:false` error envelope を返す（conflict 降格が既存
  error 条件を緩めない）ことが test で固定される。
- [ ] `scripts/test/devin-hooks-install.test.mjs` の
  "stops on legacy cheaper registrations in known sources" case
  （:269-285）が、exit 0・config 書込み済み・warning/conflict が
  出力に表れることを assert する形へ書き換えられ、`node --test`
  pipeline（`mise run check`）が pass する。
- [ ] conflict 以外の blocker 条件（missing own entry・absent
  config・malformed config・modified / duplicate own entry）では
  verify が引き続き exit 2・`clean:false` を返すことが test で
  固定される。
- [ ] `remove` の residual 報告が `residualRegistrations` key を
  維持しつつ note 文言が新規約に整合することが test で固定
  される。
- [ ] `docs/guides/distribution.md` に warning 化・共存許可・旧
  binary caveat（duplicate evaluation のみ、verdict 同一）が
  反映される。
- [ ] `packages/apm/.apm/skills/cheaper-usage/SKILL.md` の
  hook-config 節（:167-171 の "sync refuses to write while a
  legacy cheaper read-hook registration coexists" 記述と
  :194-199 の post-uninstall 段落）が新規約へ整合する。

## Deferred Design Concerns

- 旧 gate 方針を記録した ADR-034H2WBNtC4We8dyC5lQZJ
  （`docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`）
  との関係整理 | 本 spec と並行して降格の構造決定を記録する
  ADR-034IKJq6tKWWAep0Q0ONzK
  （`docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md`）
  が切られているため、旧 ADR への amend/supersede 関係はそちらで
  確定する | ADR 作成時 | 記録上の矛盾が残るリスク。
- informational finding の構造化（severity field 等） | 現行は
  string finding + `clean` flag で足りており追加構造は YAGNI |
  findings を機械消費する consumer が severity 区別を要求した時点 |
  consumer は prefix 規約で informational / blocker を読み分ける
  必要がある。
- legacy file 側の登録を cheaper 側から収束させる仕組み | 所有境界
  上 auto-cleanup は却下済み | Devin 側 import 制御や明示的な
  cleanup command の要求が上がった時点 | 重複登録が残り続ける
  （実害は dedup で吸収済み）。

## Sources and Relations

- issue #128: <https://github.com/scarecrow173/cheaper/issues/128>
  （本 spec の起点 — gate 降格 proposal 採択済み）
- upstream discovery:
  `docs/discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  （DISC-034IKBmCjJBeLD5qG9DoUM）— 検証済み file:line evidence、
  採択済み設計判断、alternatives、gap 分析の記録。本 spec はその
  promotion candidate「spec-doc needed」を引き継ぎ、Open
  Questions の未確定事項（finding 表現形態・sync result shape・
  warning の出力先）を R1-R8 で確定する。
- dedup（前提除去）の #127 chain artifact（`related` 参照 —
  `derives-from` には含めず route_graph の focus lineage を分離）:
  `docs/discovery/hook-duplicate-firing-dedup-via-canonical-request-fingerprint.md`
  （DISC-034IdYnTDqD7Ao1nG3aoY9）、
  `docs/specs/hook-duplicate-firing-single-evaluation-dedup.md`
  （SPEC-034Ie1ZEdUR2aBFVoqkBay、Out of Scope / Deferred Design
  Concerns で本降格を follow-up として明記）、
  `docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md`
  （DESIGN-034IefWW6OJ9rvESaNvKUK）、
  `docs/adr/dedup-concurrent-hook-firings-via-canonical-request-fingerprint-verdict-cache.md`
  （ADR-034Ie3HFd6tIJms6Z2NIow）。
- 並行 ADR:
  `docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  （ADR-034IKJq6tKWWAep0Q0ONzK）— 同一 discovery から dual-track
  で作成される降格の構造決定 ADR（skill の spec + ADR parallel
  規約）。
- gate の定義元（本変更で方針部分が更新される）:
  `docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md`
  （ADR-034H2WBNtC4We8dyC5lQZJ、issue #107）、
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md`
  （DESIGN-034H3c448Lvz0CfBjd5Xlz）。
- code evidence: `crates/cheaper-cli/src/hook_config.rs`
  （`CONFLICT_PATTERN` :38、`KNOWN_LEGACY_FILES` :42-49、
  `legacy_conflicts` :362 / unparseable skip :376-378、
  `run_verify` :587 / arg/path error :592-593 / `matcher-drift`
  :637-641 / conflict 処理 :668-682 / `remediation` :730-738、
  `run_sync` :743 / gate :760-768 / `note` quiet gate :852-858、
  `run_remove` :864 / residual :926-939、`run_event_stdin` :973 /
  mapping :1001-1007 / `post-uninstall` 判定不能 :1023-1036 /
  unknown noop :1044-1050）、
  `crates/cheaper-cli/src/hook/eval_cache.rs`（dedup 実装 —
  module doc :4-7）、`crates/cheaper-cli/src/hook/targets.rs`
  （devin cwd 解決 :535-541、pretooluse :651 — target 間
  normalize divergence）。
- test evidence: `crates/cheaper-cli/tests/integration/hook_config.rs:132`
  （`hook_config_sync_fails_on_legacy_conflict` — 書き換え対象）、
  `scripts/test/devin-hooks-install.test.mjs:269-285`
  （"stops on legacy cheaper registrations in known sources" —
  旧 gate 挙動 assert、書き換え対象）。
- docs 対象: `docs/guides/distribution.md`（130-141・148-169・
  171-182 行）、`packages/apm/.apm/skills/cheaper-usage/SKILL.md`
  （:156-208 の hook-config + lifecycle 節 — :167-171 の旧 gate
  記述・:194-199 の post-uninstall 段落）。
