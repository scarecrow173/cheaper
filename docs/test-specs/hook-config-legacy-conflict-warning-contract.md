---
id: "TSPEC-034IM5B88Ph3dNfw748cmj"
type: "test-spec"
status: "approved"
title: "hook-config legacy conflict demotion contract"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
    - "https://github.com/scarecrow173/cheaper/issues/127"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
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
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
    - "docs/test-specs/hook-eval-dedup-verdict-cache-contract.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  verified-by: []
  references:
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "scripts/test/devin-hooks-install.test.mjs"
  defers: []
  deferred-by: []
---

# hook-config legacy conflict demotion contract

## Purpose

issue #128 に対して、PLAN-034IM2ZjSyCYQSJ8xF1c6L が実装する hook-config
legacy conflict gate の blocker → warning 降格が
SPEC-034IKGkW4dQEVw6QigMFxQ の R1–R11・exit code 表・Acceptance Criteria を
満たすことを固定する test contract。具体的には「legacy conflict が共存する
状態でも `sync` が own entry の upsert を実行して exit 0・`verify` が
`clean:true`・exit 0 で収束し、conflict 情報は `legacyConflicts`・
informational `legacy-conflict:` finding・`warning`・
`residualRegistrations` note として引き続き可観測」であること、および降格が
既存の blocker 系 finding(missing・modified・duplicate・absent-config・
config-unreadable)と sync の retained error 条件を一切緩めないことを保証
する。

本 test spec は、gate が blocker へ復帰したとき・conflict 報告経路
(`legacyConflicts` field・finding・note)が除去されたとき・または dedup
の前提(同一 canonical request で verdict 同一)が崩れて共存が unsafe に
戻ったときに retire / 再検討してよい。それまでは spec/design が再編されても
「降格で共存を許可しつつ conflict を可観測に保ち、他の blocker 規約を
緩めない」という意図を保存する役割を持つ。

## Feature

`cheaper hook-config` の conflict 扱い: `legacy_conflicts()` の検出
(`KNOWN_LEGACY_FILES` + target file・`CONFLICT_PATTERN`)は維持したまま、
`sync` は conflict 有りでも upsert を継続して result に `legacyConflicts`
(常時)・`warning`(conflict 時のみ)を載せ exit 0、`verify` は conflict を
informational `legacy-conflict:` finding として報告し `clean` は blocker 系
finding のみから導出、`remove` の `residualRegistrations` + note は
informational のまま文言のみ整合、`--event-stdin` の
`post-install`/`post-update`/`post-uninstall`(packages 判定不能)は verify
code passthrough により conflict-only で exit 0 へ収束する。

## Rules

### Test layer 分割と隔離(全 Rule に共通する前提)

- **Rust integration 層**: `tests/integration/hook_config.rs` の既存
  `fixture_with_cheaper_on_path` PATH shim パターン(`CARGO_BIN_EXE_cheaper`
  を tempdir `bin/` へ copy し `PATH` を shim のみに固定)と
  `fx.run(&["hook-config", ...])` の in-process `run_cli` 経路を再利用し、
  exit code・envelope(`ok`・`result` map)・config file 内容を assert する。
- **mjs 層**: `scripts/test/devin-hooks-install.test.mjs` の
  "install-devin-hooks.ps1 + real cheaper CLI" describe block
  (`{ skip: !runPsAvailable || !cliBuilt }` — win32 + built
  `target/debug/cheaper(.exe)` 必須、非 Windows/未 build では skip の既存
  coverage model)で、wrapper `-Apply` run と実 CLI の direct `verify`/
  `sync` spawn により end-to-end の収束を assert する。

### AC 対応 Rule

- Rule(R1-R3 / AC1): conflict 有りで `sync` は own entry の upsert を実行し
  exit 0・`ok:true`。result に `legacyConflicts`(string array、conflict
  file path 一覧、verify result の同名 field と同一 shape)が**常に**含まれ
  (success result map 限定 — `ok:false` envelope は適用外)、conflict 検出時
  のみ `warning` string を含む。`configPath`・`scope`・`changed`・`matcher`・
  `orphansRemoved`・`note` の既存 key は不変。
- Rule(R2 / AC: quiet): `warning`・`legacyConflicts` は `--quiet` の有無に
  関わらず出力する — 既存 `note` の quiet gate のみが quiet で抑制され、
  降格 field が quiet gate を複製していないことを guard する。`warning` は
  result JSON のみで stderr channel は設けない。
- Rule(R4-R6・R9 / AC: verify): conflict-only の状態では `verify` が
  exit 0・`clean:true`・`legacyConflicts` populate・`legacy-conflict:`
  prefix の informational finding を返し、finding・`warning`・`note`・
  `remediation` の文言に "duplicate firing is unsafe" が残らない。
  `clean` は blocker 系 finding(`config-unreadable`・`missing`・
  `modified`・`duplicate`・`absent-config`)のみから導出され、
  informational(`matcher-drift`・`shadow`・`orphan`・`legacy-conflict`)
  は `clean` に影響しない。severity field は導入しない(prefix 規約)。
- Rule(R6・R10 / AC: blocker retention): informational `legacy-conflict`
  が blocker 系 finding を覆わない — conflict 併存下でも own entry
  missing・modified・duplicate の各 blocker で verify は exit 2・
  `clean:false`。config 不在 + conflict では blocker `absent-config` と
  informational `legacy-conflict` が併発して exit 2(降格の意図的副次変化
  — 降格前は conflict の blocker で `absent-config` が抑止されていた)。
- Rule(R3 / AC: retained errors): conflict 併存下でも malformed/unreadable
  config・modified own entry(未知 matcher)・duplicate own entry・
  capability gate 失敗・外部変更検出・symlink/reparse 経路で `sync` は
  `ok:false` error envelope・exit 2 を返す — 降格は conflict early return
  のみを除去し他 error 経路を緩めない。capability/reparse/外部変更経路は
  既存の非 conflict test が担保し、新規 test は
  malformed/modified/duplicate-own を固定する。
- Rule(R8 / AC: event-stdin): `{"event":"post-install","packages":
  ["cheaper"]}` と `{"event":"post-update","packages":["cheaper"]}` は
  conflict 有りでも exit 0 で収束し、`result.sync` が `legacyConflicts`・
  `warning` を、`result.verify` が informational finding・`clean:true` を
  載せ、config file が書き込まれ own entry(canonical matcher + own
  command)が存在する。`{"event":"post-uninstall"}`(packages 欠落/判定不能)
  × conflict-only residual は `run_verify` code passthrough を継承して
  exit 0・`action:"skipped"`(降格前は exit 2)。
- Rule(R7 / AC: remove): conflict setup + own entry 書込み済みで `remove`
  は exit 0・`residualRegistrations` 非空・`note` が新文言(residual が
  「残っても安全 — duplicate firings are deduplicated at eval time」旨を
  含み verify を観察先として指す)。key 名・emit 条件は不変。

## Examples

- `hook_config_sync_warns_on_legacy_conflict`(既存 :132
  `hook_config_sync_fails_on_legacy_conflict` の rewrite)—
  `.windsurf/hooks.json` に legacy 登録を残す fixture で `sync` → exit 0・
  `ok:true`・`result.legacyConflicts` が len 1 で windsurf path を含む・
  `result.warning` が非空 string・config file が存在し own entry
  (canonical matcher + `OWN_COMMAND`)が書き込まれている。続けて `verify`
  → exit 0・`clean:true`・`legacyConflicts` populate・findings に
  `legacy-conflict:` prefix entry・全 finding が "duplicate firing is
  unsafe" を含まない。
- `hook_config_event_stdin_converges_on_legacy_conflict` — conflict setup +
  stdin `{"event":"post-install","packages":["cheaper"]}` → exit 0・config
  書込み・`result.sync.legacyConflicts` 非空・`result.sync.warning` 存在・
  `result.verify.clean == true`。続けて `{"event":"post-update","packages":
  ["cheaper"]}` → exit 0 + `config_path(&fx).exists()` と own entry
  (canonical matcher + `OWN_COMMAND`)の存在を assert(post-install と同一
  mapping の冪等経路でも conflict で落ちない収束を固定)。
- post-uninstall 判定不能 × conflict-only residual(新規)— own entry 書込み
  済み + conflict 残りで stdin `{"event":"post-uninstall"}`(packages 欠落)
  → exit 0・`result.action == "skipped"`・`result.verify.clean == true`
  (`run_event_stdin` :1023-1036 の verify passthrough 継承 — 降格前は
  exit 2)。
- `hook_config_sync_quiet_keeps_conflict_fields`(新規)— conflict setup +
  `sync --quiet` → exit 0・`result.legacyConflicts` 非空・`result.warning`
  存在・`result.note` 不在(R2 — `warning`/`legacyConflicts` が quiet gate
  対象外であることの guard)。
- retained errors(conflict 併存)— `.windsurf` conflict を置いた fixture で
  malformed `.devin/config.json` / 未知 matcher の modified own entry /
  duplicate own entry の各条件で `sync` が exit 2・`ok:false`(1 test に
  fixture 再 setup でまとめても良い)。
- verify blocker retention(新規)— conflict + own entry missing → exit 2・
  `clean:false`・findings が `missing` と `legacy-conflict` の両方を含む;
  own entry に余分な key(または未知 matcher)→ exit 2・`modified:` prefix
  finding;own entry group 2 件 → exit 2・`duplicate:` finding。
  absent-config・config-unreadable は既存 test(:62-76・:222-254)が担保。
- remove residual — conflict setup + own entry 書込み済み → `remove` →
  exit 0・`residualRegistrations` 非空・`note` が "deduplicated at eval
  time" を含む新文言。
- mjs "warns on legacy cheaper registrations in known sources"(:269-285
  rewrite)— wrapper `-Apply` run → `r.code === 0`・`existsSync(
  configAt(root)) === true`・parse した config が canonical entry
  (`CANON_MATCHER` + `CANON_COMMAND`)を持つ。warning surfacing は wrapper
  が成功時に sync envelope を echo しないため direct spawn で assert:
  `spawnSync(cliExe, ["hook-config","verify","--project-root",root])` の
  stdout JSON で `result.legacyConflicts` が windsurf file を含み・
  `result.findings` に `legacy-conflict` entry・`result.clean === true`。
  `warning` field 自体の assert が欲しい場合のみ `sync` を direct spawn
  (idempotent で再実行しても `warning` は返る)し、env に
  `PATH: <cliExe dir> + path.delimiter + process.env.PATH` を渡す
  (`check_capability` → `find_on_path` が PATH 上の cheaper binary を要求
  するため)。旧 assert(非ゼロ exit・config 非作成・`/legacy|conflict/i`)
  は全て削除。

## Guarantees

- legacy conflict の共存は sync・verify・lifecycle event のいずれでも
  block されず、conflict 情報は result field・finding・note として常に
  可観測 — exit code から外れても stale 登録の存在が隠れない。
- 降格は conflict のみに適用され、blocker 系 finding(missing・modified・
  duplicate・absent-config・config-unreadable)と sync の retained error
  (capability・malformed・modified・duplicate・外部変更・reparse)は exit
  2・`ok:false`/`clean:false` のまま一切緩まない。
- `sync` result の `legacyConflicts` は success result map に常時存在し
  (空 array 可・`changed:false` の冪等経路でも emit)、`warning` は
  conflict 検出時のみ・`--quiet` に関わらず出力される。
- `--event-stdin` の envelope 構造(`sync`/`verify`/`remove` subresult key・
  verify code passthrough・`action:"skipped"`/`"noop"` + `reason`)は不変
  で、conflict-only の `post-install`/`post-update`/`post-uninstall`
  判定不能が全て exit 0 へ収束する。
- 文言の新旧置換は完全 — どの経路の finding/warning/note/remediation にも
  "duplicate firing is unsafe" が残らない。

## Non-goals

- conflict 検出規則自体(`CONFLICT_PATTERN`・`KNOWN_LEGACY_FILES`・
  `legacy_conflicts` の best-effort skip 規約 — parse 不能 sibling file は
  `legacyConflicts`・finding のいずれにも現れない)の検証。検出側は不変で
  あり、本 spec は「検出結果の surface 方法」のみを固定する。
- dedup 機構(`hook/eval_cache.rs`)の correctness・verdict 同一性 —
  `hook-eval-dedup-verdict-cache-contract.md` が担う upstream 契約であり、
  本 spec はその前提に依存するのみ。
- wrapper(`install-devin-hooks.ps1`)経路での `warning` 可観測性 —
  wrapper は成功時に sync envelope を echo しない既知の制約であり、mjs
  test は direct CLI spawn で surfacing を検証する。
- 非 Windows / 未 build 環境での mjs coverage — describe block の
  `skip: !runPsAvailable || !cliBuilt` gate は既存 coverage model どおり。
- auto-cleanup・`--allow-conflict` flag・severity field・stderr channel —
  spec/design の Out of Scope(却下済み)であり検証対象外。
- `post-uninstall` packages 判定不能 × **conflict 以外の** residual 状態・
  `Some(false)` noop・`pre-*`/unknown event noop・malformed payload —
  現行維持の規約であり、本 spec は conflict 降格が変える行のみを固定する。
- `install-devin-hooks.ps1:5` header comment の stale 表現 — wrapper scope
  外の drift 記録であり assert 対象外。

## Risk

- 本 contract を外すと以下を回帰として検出できない: (a) 降格の partial
  適用 — sync は進むが `warning`/`legacyConflicts` が emit されず conflict
  が静かに不可視化、(b) quiet gate の誤複製で `--quiet` 時に warning が
  消える、(c) blocker 系 finding の誤緩和 — informational 化が
  missing/modified/duplicate へ漏れて verify が壊れた config を clean と
  報告する、(d) lifecycle 不収束 — event-stdin passthrough 前提が壊れて
  conflict 有りで post-install が非 0 へ戻る(issue #128 の復活)、
  (e) 旧文言残存 — "duplicate firing is unsafe" が残り dedup 後の安全
  前提と矛盾する案内を consumer へ出し続ける。いずれも「共存を許可しつつ
  可観測に保つ」という降格の目的を静かに壊す。
