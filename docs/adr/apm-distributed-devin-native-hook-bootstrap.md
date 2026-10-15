---
id: ADR-034GssAufIiId3hgh4jrSQ
type: adr
status: accepted
title: APM-distributed Devin native hook bootstrap
created: '2026-10-15'
updated: '2026-10-15'
owners:
  - scarecrow173
relations:
  source:
    - 'https://github.com/scarecrow173/cheaper/issues/107'
    - 'https://github.com/scarecrow173/cheaper/issues/107#issuecomment-6183636932'
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - >-
      ./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - ../specs/case-insensitive-native-tools-and-devin-registration.md
    - ../specs/apm-delegation-skills-and-read-hook.md
  refines:
    - >-
      ./normalize-native-hook-payloads-to-a-canonical-routing-request-via-per-target-ada.md
  refined-by: []
  derives-from:
    - ../discovery/bounded-hook-read-request-contract.md
  derived-by:
    - ../designs/devin-native-hooks-and-apm-bootstrap.md
    - ../plans/implement-native-tool-normalization-and-devin-bootstrap.md
  verifies: []
  verified-by:
    - ../test-specs/apm-distributed-devin-registration-preservation.md
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers:
      - scarecrow173
    consulted: []
    informed: []
---

# APM-distributed Devin native hook bootstrap

## Context and Problem Statement

issue #107 の fixture 再現で確定した（
`docs/discovery/bounded-hook-read-request-contract.md`、
issuecomment-6183636932）: Devin CLI 3000.11.3 の native
`PreToolUse` wire は `tool_input` に `offset`/`limit`/`max_results`
等の bounds を含むが、Windsurf 互換 `pre_read_code` payload は
`tool_info` が `file_path` のみ（root・`tool_info` とも
`otherKeyCount: 0`）であり、tool の種類・範囲・検索上限は Devin 側の
native → Windsurf compat 変換で消失する。cheaper の Rust adapter が
受信する前の境界であり、Rust 側に bounds 読み取りを足しても届かない。

そのため Devin consumer には Windsurf 互換を介さない native
`PreToolUse` 登録が必要だが、現行の配布経路は APM の
Windsurf-compat `hooks.json` のみで、native `.devin/config.json` への
登録手段がない。一方、APM 0.28.0 の `install --help` の target には
Devin がなく、`core/target_catalog.py` の catalog は固定
MappingProxy、`integration/hook_integrator.py` の
`_MERGE_HOOK_TARGETS` も組込であり、公開された custom-target plugin
機構は存在しない。`agent-skills` target（`integration/targets.py`
L815-829）は skills のみを `.agents/skills/` へ deploy し hooks は
含まない。

## Decision Drivers

* APM を distribution source として維持する（別 repo である APM 本体の
  変更は今回の scope 外）。
* ADR-034rzTJYDRmre6D5OS8HDW の per-target adapter + shared policy
  単一化と APM verbatim deploy の方針を継承する。
* consumer 環境への登録は preview 既定・冪等・非破壊であること。
  既存 config / 他 event / 他 hook group を保持し、曖昧さや重複は
  自動修復せず停止して通知する。
* `.devin/hooks.v1.json` は生成しない（同一 `.devin/` 内で legacy
  `hooks.json` を shadow する挙動を fixture で実測済み）。
* 新規 dependency なし、APM 0.28.0 pin 維持。

## Considered Options

* Option 1: APM 本体に Devin target を追加し、`apm install --target
  devin` が `.devin/config.json` の `hooks.PreToolUse` を直接生成する。
* Option 2: APM の shared skill 配布（`agent-skills` target）で
  PowerShell script を resource として届け、その script が限定された
  native 登録補助（preview + `-Apply`）を行う。
* Option 3: 独立 installer の作成、または Claude 向け出力の機械 copy
  を登録 source とする。

## Decision Outcome

Chosen option: "Option 2: APM shared skill 配布＋限定 native 登録補助"。

`packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
を既存 skill bundle の resource として APM から配布する。消費側の
操作は consumer の状態で分ける:

- **新規 Devin-only consumer**: `apm install --target agent-skills`
  の後に `powershell -NoProfile -ExecutionPolicy Bypass -File
  .agents/skills/cheaper-usage/scripts/install-devin-hooks.ps1
  [-ProjectRoot <repo-root>] [-Apply]` を実行する。
- **既存 consumer（manifest で他 target を所有）**: 既存 targets を
  保持したまま通常の `apm install` を実行し、APM が deploy した
  cheaper-usage helper を実際の deploy path から呼ぶ。target を
  置き換えて helper の場所を得ようとしない。`--legacy-skill-paths`
  を選んでいる場合も、target を切り替えず既存 target の deploy
  path を使う。

これ以外の package resolver / copier / download、独自 APM install、
APM internals の monkeypatch は作らない。`apm install` step 自体は
APM の dropped-target reconciliation 副作用を持ち得るため
（`install --target` は manifest targets を上書きし、外れた target
の deployed artifact が cleanup され得る）、この副作用と helper の
非破壊保証は別の責務として扱う。helper 自身は既存 config / 他
event / 他 hook group / legacy read hook を変更しない。

script の契約: default は preview（既存 config 値をログに出さず変更
概要のみ。CLI 不在でも「何を追加するか」は表示してよい）。
`-Apply` で `.devin/config.json` の `hooks.PreToolUse` に
`command: "cheaper hook pre-tool-use --target devin"`、matcher は
`read`/`grep` の ASCII case-insensitive exact（char class）、hook
`type: "command"` を追加する。root 既定は cwd。source 環境・global
設定は変更しない。apply 前 preflight として read-only capability
query `cheaper hook targets --json`（exit 0 + stdout 1 行
`{"schemaVersion":1,"targets":[...]}`。stdin / model / usage /
policy を触らず、`targets` は `HookTarget` の canonical supported
list を単一 source とする）を実行し、exit 0・JSON object・
`schemaVersion: 1`・`targets` が string array で exact `"devin"` を
含むことを確認してから書き込む。executable 不在・nonzero exit・
empty / malformed 出力・unknown `schemaVersion`・`devin` 欠落は
非破壊 error とし、Cargo install/update 案内を出して config を一切
書かない。version string では判定しない（APM skill と Cargo binary
は独立 install され、version は現行と同一になり得るため executable
存在だけでは devin 対応を保証できない）。既存の無関係 config・他
event・hook group は保持し、同一 command+matcher の再実行は
no-op、改変済み・曖昧な既存 own entry は非破壊 error。既存 JSON の
parse/schema 異常や symlink / reparse point の path には書かず
error。書込みは temp file + atomic replace。初版は Windows
PowerShell 5.1 向けで POSIX installer は非 scope。

登録前に、プロジェクトの既知 legacy cheaper read hook 登録元
（`.windsurf/hooks.json` / `.devin/hooks.json` /
`.devin/hooks.v1.json` / `.devin/config.local.json` /
`.claude/settings*.json` 等の既知の登録元）を限定 scan し、cheaper
互換 read / 他 target の重複があれば apply を停止して場所と解消手順を
通知する。削除・provider 一括無効化は行わず、既存 legacy shell hook
も削除しない。global / ancestor の完全検出は保証せず、ユーザーへ
`/hooks` builtin での確認を促す旨を表示する。

この登録は APM lockfile の ownership に記録されない補助登録であり、
APM uninstall 後の登録解除はこの command group の手動削除が必要で
あることを script と配布 skill に明記する。

ADR-034rzTJYDRmre6D5OS8HDW の Option 3 却下理由（installer が APM の
verbatim deploy 前提を崩す点）に対し、本 ADR は限定 refinement とし
て記録する: **APM 未対応 target の config 登録のみを補助し、skill /
package の install 経路は置き換えない**。skill の配布自体は引き続き
APM が担い、script は `.devin/config.json` への hook entry 登録だけを
担当する。黙って原則を覆すものではなく、この限定条件の下でのみ
適用する。

### Consequences

* Good, because APM 本体を変更せずに Devin native `PreToolUse` 登録へ
  到達でき、native wire の bounds をそのまま受け取る経路が得られる。
* Good, because preview 既定・冪等・非破壊の補助 script により
  consumer の既存 config を壊さず、重複登録元の存在を検出して停止
  できる。
* Good, because legacy Windsurf 経路を削除せず並行維持できるため、
  既存 consumer を破壊しない。
* Bad, because 登録は APM lockfile の ownership に入らず、uninstall
  時の解除は手動になる。
* Bad, because global / ancestor の既存登録は完全に検出できず、
  `/hooks` による利用者確認に依存する。

### Confirmation

* 本 ADR と SPEC-034GssA0IjLKSiDdIHpAC8 の acceptance criteria が
  実装と一致すること（installer の preview/apply/冪等/非破壊 error・
  重複検出停止・`.devin/hooks.v1.json` 非生成）。
* fixture での script 実行（repo 内 test fixture のみ）により、
  `.devin/config.json` への意図した entry 追加と preview 出力が
  確認できること。

## Implementation Plan

* Affected paths:
  * `packages/apm/.apm/skills/cheaper-usage/scripts/install-devin-hooks.ps1`
    （新規。script は repo 内 test fixture に対してのみ実行して
    検証する）。
  * `packages/apm/.apm/skills/cheaper-usage/`（配布 skill 側に利用手順
    と uninstall 時の手動解除 notice を追記）。
  * `crates/cheaper-cli/` — `HookTarget::Devin`、CLI `--target devin`
    の enum parse / `as_str` / dispatcher、native normalize → 共有
    evaluate → `{"decision":"block","reason":...}` rendering の接続、
    および read-only capability query `cheaper hook targets --json`
    （対象要件は SPEC-034GssA0IjLKSiDdIHpAC8。normalizer 構成詳細は
    design 段階）。
  * `packages/apm/.apm/hooks/hooks.json` の `PreToolUse` matcher を
    ASCII case-insensitive exact に更新。
* Patterns to follow:
  * 既存 per-target adapter の parse / normalize / response 変換の
    分離（shared policy を複製しない）。
  * canonical launcher（`cheaper.ps1`）と同じ PATH 解決の前提。
  * temp file + atomic replace、preflight の fail-closed 検査。
* Patterns to avoid:
  * `.devin/hooks.v1.json` の生成（legacy shadow の既観測）。
  * APM internals の monkeypatch / 独自 install 機構 / package
    resolver・copier・download の新規実装。
  * preview での既存 config 値の出力、既存 legacy hook の削除、
    他 provider の一括無効化。
  * native exec / shell tool の hook 登録（今回対象外）。
* Dependencies or configuration:
  * 新規 dependency なし。APM 0.28.0 pin 維持。
  * consumer 側前提: `cheaper` が PATH 上に存在すること（preflight
    で検査）。

## Verification

* [ ] `install-devin-hooks.ps1` が default で preview のみを行い、
      `-Apply` 時に `.devin/config.json` の `hooks.PreToolUse` へ
      指定 entry（command / matcher / `type: "command"`）を追加する。
* [ ] 同一 command+matcher の再実行が no-op、無関係な既存 config・
      他 event・他 hook group が保持される。
* [ ] 既存 JSON の parse/schema 異常・symlink / reparse point・
      改変済み own entry で非破壊 error。
* [ ] `-Apply` が `cheaper hook targets --json` の成功（exit 0・
      `schemaVersion: 1`・`targets` に exact `"devin"`）を書込み条件
      とし、CLI 不在・旧 binary・nonzero・malformed・`devin` 欠落で
      非破壊 error + Cargo install/update 案内となること。
* [ ] 既知 legacy 登録元の重複検出時に apply が停止し、場所と解消
      手順を通知する（自動削除・無効化しない）。
* [ ] `.devin/hooks.v1.json` が生成されない。
* [ ] Devin CLI の `/hooks` で `pre_tool` 登録が確認できる
      （fixture での確認方法として記録）。

## Pros and Cons of the Options

### Option 1（APM 本体に Devin target 追加）

* Good, because `apm install --target devin` という正規の位置づけに
  なり、lockfile ownership に登録される。
* Bad, because APM は別 repo であり今回の scope 外。catalog /
  `_MERGE_HOOK_TARGETS` は固定組込で custom-target plugin 機構が
  無いため、upstream change が前提条件になる。

### Option 2（APM shared skill 配布＋限定 native 登録補助）— 採用

* Good, because 既存の `agent-skills` deploy 経路（skills のみ）を
  そのまま使い、APM 本体を変更せずに script だけを配布できる。
* Good, because preview 既定・冪等・非破壊の契約を script 側で
  強制できる。
* Bad, because native config 登録が APM lockfile の ownership に
  記録されず、解除は手動になる。
* Bad, because consumer が 2 step（`apm install` + script 実行）を
  必要とする。

### Option 3（独立 installer / Claude 出力の機械 copy）

* Good, because APM を介さず任意の config shape を書ける。
* Bad, because 独自 installer は配布・保守・drift 管理を別途要し、
  「APM を distribution source として維持する」前提と矛盾する。
* Bad, because Claude 向け schema の機械 copy は Devin の登録形式と
  一致せず、`.claude/settings.json` 側の unknown variant parse error
  が既に観測されているため provider 間 copy は根拠にならない。

## More Information

* Issue #107 / comment `issuecomment-6183636932`: fixture 再現結果
  （native bounds 存在・legacy file_path のみ・22419 tokens deny）。
* `docs/discovery/bounded-hook-read-request-contract.md`
  （`derives-from`、wire 観測の canonical discovery）。
* SPEC-034GssA0IjLKSiDdIHpAC8（`related`、tool 名正規化と Devin
  target の要件側）。
* ADR-034rzTJYDRmre6D5OS8HDW（`depends-on` / `refines`、per-target
  adapter + shared policy・APM verbatim deploy の継承元。本 ADR は
  APM 未対応 target の config 登録補助に限定した refinement）。
* Review record（2026-10-15、review-protocol の role-separated
  pass。修正後の文書と監査結果を再確認し、未解消の confirmed P0/P1 が
  ないため ACCEPT。ユーザー指定の承認代替を適用）:
  Reviewer A（cooperative）→ Reviewer B（adversarial）→ Validator →
  Judge の順で 10 findings を評価。P1 の F8（旧 CLI で
  `HookTarget::parse("devin")` → silent allow のため guard が無断
  無効化）と F9（`apm install --target agent-skills` の無限定記述が
  dropped-target reconciliation で既存 target artifact を drop
  し得る）に対し、capability handshake（`cheaper hook targets
  --json`）と consumer 状態別の手順分離を must-fix として適用。
  P2 以下は非 blocking とし、`tool_name` のみへの限定、per-tool
  routing 明確化、`read` fail-open 明記、配布元 path 明記、回帰
  suite AC、`Verdict::Deny.reason` passthrough、重複 `refines` の
  dedupe、accepted ADR frontmatter 書式の復元を easy fix として
  適用した。
* Devin hooks docs:
  https://docs.devin.ai/cli/extensibility/hooks/overview /
  https://docs.devin.ai/cli/extensibility/hooks/lifecycle-hooks
