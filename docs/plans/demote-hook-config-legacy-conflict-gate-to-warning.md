---
id: "PLAN-034IM2ZjSyCYQSJ8xF1c6L"
type: "plan"
status: "approved"
title: "Demote hook-config legacy conflict gate to warning"
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
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/hook_config.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/hook_config.rs"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "docs/guides/distribution.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "docs/designs/hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "docs/adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-legacy-conflict-warning-contract.md"
  references:
    - "crates/cheaper-cli/src/hook_config.rs"
    - "crates/cheaper-cli/src/hook/eval_cache.rs"
    - "crates/cheaper-cli/tests/integration/hook_config.rs"
    - "scripts/test/devin-hooks-install.test.mjs"
    - "docs/guides/distribution.md"
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "docs/discovery/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  defers: []
  deferred-by: []
---

# Demote hook-config legacy conflict gate to warning Implementation Plan

## Goal

`cheaper hook-config` の legacy conflict gate を blocker から warning へ
降格する — issue #127 の eval dedup(canonical request fingerprint verdict
cache)が「重複発火は unsafe」という gate の前提を除去したため、`sync` は
conflict 有りでも own entry の upsert を継続して `legacyConflicts` +
`warning` を result に載せ exit 0、`verify` は conflict を informational
`legacy-conflict:` finding として報告し `clean` は blocker 系 finding のみ
から導出する。これにより既存 cheaper 登録を持つ consumer project で sync が
block される問題(issue #128)と APM lifecycle `--event-stdin` が conflict 有りで
収束しない問題を解消する。

## Scope

単一 implementation stream — `hook_config.rs` 内の 4 action + 2 comment の
振る舞い変更と、その test・docs 追従。critical path と release intent が同一
(1 PR でship する behavior 変更)のため分割しない。

In scope:

- `crates/cheaper-cli/src/hook_config.rs` —
  `run_verify` の conflict blocker 除去(:668-682)・`remediation` 文言
  (:730-738)、`run_sync` の `invalid_request` early return 除去(:757-768)
  + result への `legacyConflicts`(常時)・`warning`(conflict 時のみ)追加
  (:850-860 付近)、`run_remove` の residual `note` 文言(:932-938)、
  stale comment 修正(`legacy_conflicts` :377・`run_sync` doc-comment :742)。
- `crates/cheaper-cli/tests/integration/hook_config.rs` — :132 の
  `hook_config_sync_fails_on_legacy_conflict` を warns variant へ書き換え +
  event-stdin×conflict・`--quiet` field 持続・retained error・verify blocker
  retention・remove residual の test 追加。
- `scripts/test/devin-hooks-install.test.mjs` — :269-285 "stops on legacy
  cheaper registrations in known sources" を warns 版 assert へ書き換え。
- `docs/guides/distribution.md` — sync 安全条件(:130-141)・lifecycle 節
  (:148-169)・移行節(:171-182)の整合 + 旧 binary caveat 追記(R11)。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — :167-171 の旧 gate
  記述と :194-199 の post-uninstall 段落の整合(配布 source のみ — repo 内
  `.agents/skills/cheaper-usage/SKILL.md` は deploy 写しで同節を持たない)。

Out of scope(spec / design と同じ境界):

- conflict 検出時の foreign registration 自動除去(auto-cleanup)・
  `--allow-conflict` 等の flag・severity field・stderr channel の追加。
- capability gate・dedup 機構(`hook/eval_cache.rs`)・verdict 意味論・
  canonical matcher / own entry shape の変更。
- `.devin/hooks.v1.json` の生成・rewrite(`shadow` finding は informational
  のまま)。
- 旧 binary(dedup 無し)への実行時対策 — docs 記述のみの mitigation(R11)。
- 歴史的 doc(`bounded-hook-request-evaluation-…` design :144・
  `cli-owned-hook-config-lifecycle` task :81-82)の遡及更新。
- `install-devin-hooks.ps1` wrapper 自体の変更 — header comment :5 の
  「legacy conflict gate」表現は降格後 stale となるが wrapper が scope 外の
  ため変更せず drift として記録する。

## Upstream Documents

- `docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (SPEC-034IKGkW4dQEVw6QigMFxQ, approved) — R1–R11 要件・exit code 表・
  Acceptance Criteria。拘束力のある contract。
- `docs/adr/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (ADR-034IKJq6tKWWAep0Q0ONzK, accepted) — Option 1「gate を warning へ
  降格」の構造決定(Option 2-4 却下の根拠を含む)。

## Design Inputs

- `docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (DESIGN-034ILuADFr70oZoUROR13q, approved) — touch point 単位の実装青写真・
  Wording Contract(新文言の確定文字列)・Test Design。本 plan は design を
  implementation order へ転記したもので独自判断を足さない。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `crates/cheaper-cli/src/hook_config.rs` | modify | `run_verify` conflict informational 化(:668-682・`remediation` :730-738)・`run_sync` gate 除去 + result field 追加(:757-768・:850-860)・`run_remove` note 文言(:932-938)・stale comment 修正(:377・:742) | 検出規則(`CONFLICT_PATTERN` :38・`KNOWN_LEGACY_FILES` :42-49・`legacy_conflicts` :362-394)は不変。`run_event_stdin`(:973-1053)はコード変更なし |
| `crates/cheaper-cli/tests/integration/hook_config.rs` | modify | :132 rewrite + event-stdin×conflict・`--quiet`・retained error・verify blocker retention・remove residual の test 追加 | `fixture_with_cheaper_on_path` PATH shim パターンを再利用 |
| `scripts/test/devin-hooks-install.test.mjs` | modify | :269-285 の旧 gate assert を warns 版へ書き換え(wrapper `-Apply` exit 0 + config 書込み + direct `verify` spawn で `legacyConflicts`/finding/`clean:true` assert、optional `sync` direct spawn は PATH env 渡し) | win32 + built `target/debug/cheaper(.exe)`  gated describe block |
| `docs/guides/distribution.md` | modify | sync 安全条件(:130-141)・lifecycle 節(:148-169)・移行節(:171-182)を新規約へ整合 + 旧 binary caveat 追記 | R11 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | modify | :167-171 の旧 gate 記述・:194-199 の post-uninstall 段落を新規約へ整合 | 配布 source のみ |

## Implementation Sequence

task slice は T1 → T2 の直列(T2 の mjs test は T1 の code 変更済み binary を
spawn して assert するため実依存。docs 文言も T1 で確定する result field /
finding 文字列を引用する)。

### T1 — `hook_config.rs` 降格 + Rust integration tests

- [ ] `run_verify`(:668-682)から `blocker = true` を除去し、per-file
      `legacy-conflict:` finding を Wording Contract の新文言へ置き換える:
      `legacy-conflict: {f} contains a cheaper hook registration (safe to keep — duplicate firings are deduplicated at eval time for identical requests; remove that registration to converge on a single registration)`
      — `legacyConflicts` の常時 populate(:679-682)は維持
- [ ] `remediation`(:730-738)の文言を
      `run \`cheaper hook-config sync\` to converge the registration; see docs/guides/distribution.md for the APM lifecycle wiring`
      へ置き換える — `!quiet && blocker` の emit 条件は維持(conflict-only
      では blocker が立たず emit されない)
- [ ] `run_sync` の legacy gate(:757-768)を除去し、scan を write 前位置で
      保持(`let conflicts = legacy_conflicts(&root_dir, &config_path);` +
      降格 comment)。upsert を継続し、result 組立(:850 付近、`changed`/
      `matcher` insert 直後・quiet gated `note` の**外**)で
      `legacyConflicts`(string array、conflict 無しでも常時)+ conflict
      検出時のみ `warning` を insert — `note` の quiet gate(:852-858)を
      複製しない(R1/R2)。`warning` 文言は Wording Contract の確定文字列:
      `legacy cheaper hook registrations remain (see legacyConflicts); coexisting is safe — duplicate firings are deduplicated at eval time for identical requests, so removal is needed only to converge on a single registration. A pre-dedup cheaper binary on PATH still evaluates each firing independently (duplicate evaluation only — the verdict is identical); see docs/guides/distribution.md`
- [ ] `run_remove` の `note`(:932-938)を新文言へ置き換える:
      `remove deleted only the canonical entry; other files still contain cheaper registrations (safe to keep — duplicate firings are deduplicated at eval time for identical requests); run \`cheaper hook-config verify\` to inspect`
      — `residualRegistrations` key・emit 条件は不変(R7)
- [ ] stale comment 修正: `legacy_conflicts` :377 の「parse 不能 file は
      verify の finding で別途報告」を実動作(verify は sibling の parse
      失敗を報告しない best-effort skip)へ修正し、`run_sync` doc-comment
      :742 を「canonical own entry の冪等 upsert + legacy conflict の
      informational 報告」系の降格後責務表現へ修正
- [ ] `tests/integration/hook_config.rs` — :132 を
      `hook_config_sync_warns_on_legacy_conflict` へ rewrite(sync → exit 0・
      `ok:true`・`legacyConflicts` len 1・`warning` 非空・config 書込み済み
      own entry;verify → exit 0・`clean:true`・`legacy-conflict:` finding・
      全 finding が "duplicate firing is unsafe" を含まない)
- [ ] 新規 `hook_config_event_stdin_converges_on_legacy_conflict` —
      `.windsurf/hooks.json` conflict + `{"event":"post-install",
      "packages":["cheaper"]}` → exit 0・config 書込み・
      `result.sync.legacyConflicts` 非空・`result.sync.warning` 存在・
      `result.verify.clean == true`。続けて `post-update` payload → exit 0
      + `config_path(&fx).exists()` と own entry(canonical matcher +
      `OWN_COMMAND`)の存在を assert
- [ ] 新規 post-uninstall 判定不能 × conflict-only residual test — own entry
      書込み済み + conflict 残りで `{"event":"post-uninstall"}`(packages
      欠落)→ exit 0・`result.action == "skipped"`・
      `result.verify.clean == true`(降格前は exit 2)
- [ ] 新規 `hook_config_sync_quiet_keeps_conflict_fields` — conflict setup +
      `sync --quiet` → exit 0・`legacyConflicts` 非空・`warning` 存在・
      `note` 不在(R2 guard)
- [ ] retained error test — conflict 併存下で malformed config / 未知
      matcher の modified own entry / duplicate own entry の各条件で sync が
      exit 2・`ok:false`(1 test に fixture 再 setup でまとめても良い)
- [ ] verify blocker retention test — conflict + missing → exit 2・
      `clean:false`・findings が `missing` と `legacy-conflict` の両方を
      含む;modified own entry → exit 2・`modified:` finding;duplicate
      own → exit 2・`duplicate:` finding(absent-config・malformed は既存
      test :62-76・:222-254 が担保)
- [ ] remove residual test — conflict setup + own entry 書込み済みで
      `remove` → exit 0・`residualRegistrations` 非空・`note` が新文言
      ("deduplicated at eval time" を含む)
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      hook_config::` が exit 0、`cargo fmt --check`・clippy が pass

### T2 — docs + mjs test(depends T1)

- [ ] `docs/guides/distribution.md` — :135-138 の「conflict → 停止」を
      `legacyConflicts` + `warning` で報告・upsert 継続へ改訂し共存安全・
      収束 optional を明記;:148-169 lifecycle 節に conflict 有りでも verify
      exit 0 passthrough で収束する旨 + 旧 binary caveat(dedup 無し binary
      では firing 毎に独立評価 — duplicate evaluation のみで verdict 同一・
      correctness 維持)を追記;:171-182 移行節を「再 import 旧登録は
      warning 報告のみで block しない・単一収束の除去は optional」へ改訂
- [ ] `packages/apm/.apm/skills/cheaper-usage/SKILL.md` — :167-171 の
      "refuses to write while a legacy … coexists" 記述を「sync は共存しても
      canonical entry を書き込み `legacyConflicts` + `warning` で報告」へ
      書き換え、:194-199 post-uninstall 段落に residual legacy 登録は
      informational で lifecycle を失敗させない(packages 判定不能時も
      verify passthrough で exit 0)旨を追記
- [ ] `scripts/test/devin-hooks-install.test.mjs` — :269-285 を "warns on
      legacy cheaper registrations in known sources" へ書き換え: wrapper
      `-Apply` → `r.code === 0`・config 存在・canonical entry
      (CANON_MATCHER + CANON_COMMAND)を assert。warning surfacing は
      wrapper が成功時に sync envelope を echo しないため direct spawn で
      検証 — `spawnSync(cliExe, ["hook-config","verify","--project-root",
      root])` の stdout JSON で `result.legacyConflicts` が windsurf file
      を含み・`result.findings` に `legacy-conflict` entry・
      `result.clean === true`。`warning` field 自体を assert する場合は
      `sync` を direct spawn し env に `PATH: <cliExe dir> + delimiter +
      process.env.PATH` を渡す(`check_capability` が PATH 上の cheaper を
      要求するため)。旧 assert(非ゼロ exit・config 非作成・
      `/legacy|conflict/i`)は全て削除
- [ ] `node --test scripts/test/devin-hooks-install.test.mjs` が pass
      (**win32 + built CLI gated** — 本 repo は Windows-hosted で
      `mise run check` の `node --test` step が実効対象。非 Windows /
      未 build 環境では describe skip で既存 coverage model どおり)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 hook_config.rs 降格 + Rust tests | — | T2 |
| T2 docs + mjs test | T1 | — |

critical path は T1 → T2 の直列で並行 stream は無い — T2 の mjs test は
T1 済みの `target/debug` binary へ spawn して新 contract を assert し、docs
文言も T1 が着地させる result field・finding 文字列を引用する。file 集合は
disjoint(`hook_config.rs` + Rust test vs docs 2 file + mjs test)だが、assert
対象の実行物・文言の唯一 source は T1 のため `depends-on` で直列化する。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | conflict 見落とし — blocker 除去で exit code に現れなくなり consumer が warning を見落とす | med | low | `warning`・`legacy-conflict:` finding・`residualRegistrations` note で可観測性を維持(ADR Consequences で受容済み)。文言に除去 optional と docs pointer を含める |
| 2 | 旧 binary(dedup 無し)が PATH 上に残り重複発火が独立評価される | low | low | 影響は duplicate evaluation(token 計測・bounded materialization・bounded scan の重複)のみで verdict 同一・correctness 維持 — `warning` 文言内 pointer + distribution.md caveat(R11)の documentation-only mitigation |
| 3 | 降格が既存 error 条件を緩める — malformed/modified/duplicate/capability 系が通り抜ける回帰(breaking-change 方向の逆) | low | med | gate 除去は conflict early return のみに限定し他 `?` 伝播・`?` error は無変更。retained error test(T1)で conflict 併存下の exit 2・`ok:false` を固定 |
| 4 | `absent-config` 併発 — config 不在 + conflict で降格前は抑止されていた `absent-config` blocker finding が `legacy-conflict` と併発して exit 2 となる意図的副次変化を見落とす | low | low | design Data and Control Flow で確定済みの意図的変更。verify blocker retention test が blocker 系 semantics の維持を固定 |
| 5 | mjs test の warning 不可視 — wrapper が成功時に sync envelope を echo しないため `r.combined` への assert が不可能 | — | low | 確定済みの制約。design が direct `verify` spawn(`legacyConflicts`+finding+`clean:true` で spec AC 充足)と optional `sync` spawn(PATH env 必須)の検証経路を指定済み |
| 6 | `sweep_orphans` 実行範囲の拡大 — early return 除去で conflict 有り sync でも orphan sweep が走る | low | low | 副次的変化は orphan temp/bak 掃除が増えるのみの harmless/beneficial side effect(design 済)。contract 追加は意図しない |
| 7 | 外部依存 availability — 新規 crate dep・env var・flag 無し。外部状態は fixture tempdir と `target/debug` binary のみ | low | low | Rust test は `CARGO_BIN_EXE` PATH shim で隔離。mjs test は win32+cliBuilt gate で環境不在時に skip |

### Rollback

- **Trigger**: 降格後の sync/verify/lifecycle 収束に障害(誤って conflict で
  block される、または conflict 以外の blocker が緩む、warning が出ない等)。
- **Procedure**: revert unit は T1/T2 の commit — `git revert` で
  `hook_config.rs` の gate 復帰 + test・mjs・docs を降格前へ戻す。
  schema 変更・migration・共有 state は無く、consumer 側 config は own entry
  upsert が残るのみ(再 sync・remove で収束可能)のため `git revert` で完全に
  戻せる不可逆操作は存在しない。
- **Verification**: revert 後に `mise exec -- cargo test -p cheaper-cli
  --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test
  integration hook_config::` と `node --test
  scripts/test/devin-hooks-install.test.mjs`(win32+built CLI)が pass する
  こと。

## Task Breakdown

task doc へ分割する候補(2 task — file 集合が disjoint で review 単位を分ける
が、実行は T1 → T2 直列):

- T1: `hook_config.rs` 降格(verify informational・sync proceed+warning・
  remove note・stale comment)+ Rust integration test 群
  — `docs/tasks/demote-legacy-conflict-gate-in-hook-config.md`
- T2: docs 2 file(distribution.md・cheaper-usage SKILL.md)+
  `devin-hooks-install.test.mjs` rewrite
  — `docs/tasks/update-conflict-demotion-docs-and-mjs-test.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| AC: conflict 有り sync が upsert 実行・exit 0・`legacyConflicts` 非空 + `warning`(R1-R3) | `hook_config_sync_warns_on_legacy_conflict`(:132 rewrite) | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration hook_config::` |
| AC: conflict-only verify が exit 0・`clean:true`・`legacyConflicts` populate・"duplicate firing is unsafe" 非含有(R4-R6/R9) | 同上 test の verify 部 | 同上 |
| AC: `post-install`/`post-update` × conflict で exit 0・config 書込み(R8) | `hook_config_event_stdin_converges_on_legacy_conflict` | 同上 |
| AC: `post-uninstall` packages 判定不能 × conflict-only residual で exit 0・`action:"skipped"` | 新規 post-uninstall test | 同上 |
| AC: `sync --quiet` で `legacyConflicts`・`warning` 持続・`note` 不在(R2) | `hook_config_sync_quiet_keeps_conflict_fields` | 同上 |
| AC: conflict 併存下で malformed/modified/duplicate が exit 2・`ok:false`(R3) | retained error test | 同上 |
| AC: conflict 以外の blocker(missing/modified/duplicate + 既存 absent-config/malformed coverage)で verify が exit 2・`clean:false`(R6/R10) | verify blocker retention test + 既存 :62-76・:222-254 | 同上 |
| AC: `remove` residual が `residualRegistrations` 維持・note 文言整合(R7) | remove residual test | 同上 |
| AC: mjs test が新規約(exit 0・config 書込み・warning/conflict 可観測)を assert | "warns on legacy cheaper registrations in known sources"(win32+built CLI gated — 非 Windows では skip) | `node --test scripts/test/devin-hooks-install.test.mjs` |
| AC: distribution.md に warning 化・共存許可・旧 binary caveat、SKILL.md が新規約へ整合(R11) | review check | 手動 review + `mise run check` の doc 系 step |
| 全体 | biome・`cargo fmt --check`・clippy・version/doc-id・`cargo test --workspace`・`node --test`・lockfile/payload check | `mise run check` |

## Open Questions / Gaps

- design が wording・scan-before-write 位置・test 構成を確定済みのため、実装
  上の不明点は無い。
- 記録済みの意図的変更(implementation 時に驚かないための明示):
  - `absent-config` blocker が config 不在 + conflict 状態で `legacy-conflict`
    と併発するようになる(降格前は conflict の `blocker = true` で抑止されて
    いた)— exit 2 維持で contract 違反ではない。
  - `sweep_orphans` が conflict 有り sync でも実行されるようになる(harmless
    side effect)。
  - `install-devin-hooks.ps1:5` の header comment(「legacy conflict
    gate」)は wrapper scope 外のため stale のまま残す — drift 記録済み。
- mjs test の `sync` direct spawn は optional — spec AC は `verify` spawn
  assert で充足する(`warning` field の直接 assert が欲しい場合のみ PATH env
  付きで sync を spawn する)。

## Self-Review

- [x] Every spec requirement maps to a task.(R1-R11・全 AC → Verification
      matrix と T1/T2 checklist を対応付け済み)
- [x] No placeholders remain.(文言は design Wording Contract の確定文字列を
      転記)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation — T1 は
  code+test の self-contained slice、T2 は docs+mjs の追従 slice で、各 slice
  が review checkpoint として独立する。
- Independent work streams: なし(T1 → T2 直列。file 集合は disjoint だが
  T2 の assert 対象・引用文言は T1 の着地が前提)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in
  the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and
  verification matrix.

## Review Handoff

- 前提: upstream 3 doc(spec approved・design approved・ADR accepted)が
  wording・scan 位置・test 構成まで確定済み。plan は design を implementation
  order へ転記したもので独自判断を足していない。
- 重点確認: (a) `run_sync` の `legacyConflicts`/`warning` insert が quiet
  gated `note` の外にあること(R2 guard)、(b) `run_verify` で `blocker =
  true` のみ除去し finding push と `legacyConflicts` populate が維持されて
  いること、(c) `warning`・finding・note・`remediation` の文言に
  "duplicate firing is unsafe" が残っていないこと、(d) mjs test が wrapper
  の echo 制約を踏まえ direct verify spawn で assert していること。
- status note: 本 plan は upstream spec/design/ADR が全て approved/accepted
  で design が実装精度まで確定済みのため、作成時点で `approved` とした。
  task-doc gate(approved/in-progress/completed の plan が前提)を満たす。
