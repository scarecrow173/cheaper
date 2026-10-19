---
id: "IMPL-034IQb7Nk2ZxRv8WtPfYsCd"
type: "impl"
status: "completed"
title: "Update hook-config docs and devin-hooks-install test for conflict demotion"
created: "2026-10-20"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/update-conflict-demotion-docs-and-mjs-test.md"
    modified:
      - type: "file"
        path: "docs/guides/distribution.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
      - type: "file"
        path: "docs/tasks/update-conflict-demotion-docs-and-mjs-test.md"
      - type: "file"
        path: "docs/impl/ir/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/update-conflict-demotion-docs-and-mjs-test.md"
  implemented-by: []
  depends-on:
    - "docs/impl/ir/demote-legacy-conflict-gate-in-hook-config.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-legacy-conflict-warning-contract.md"
  references:
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Update hook-config docs and devin-hooks-install test for conflict demotion

## Summary

plan の T2: T1(commit 2440f11)が着地させた legacy conflict gate の
blocker → warning 降格へ、consumer 向け docs 2 file と `node --test`
pipeline の mjs test を追従させた(issue #128)。docs は T1 が ship した
実際の result field 名(`legacyConflicts`・`warning`・`legacy-conflict:`
finding・`residualRegistrations` note)と dedup 安全性の文言を引用し、
mjs test は旧 gate assert(非ゼロ exit・config 非作成)を新 contract
(exit 0・config 書込み・direct verify spawn での informational
finding)へ書き換えた。

## Context

- task: `docs/tasks/update-conflict-demotion-docs-and-mjs-test.md`
  (TASK-034IM7WZt5kW1QHzLRcugI)
- plan: `docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md`
  (T2 — docs + mjs test slice)
- spec: `docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (SPEC-034IKGkW4dQEVw6QigMFxQ — R11・exit code 表・AC)
- design: `docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md`
  (DESIGN-034ILuADFr70oZoUROR13q — Documentation Design 節・mjs 書き換え方針)
- T1 IR: `docs/impl/ir/demote-legacy-conflict-gate-in-hook-config.md`
  (IMPL-034IMWFzhYCEc3udAKl2xZ — code 側の降格実装)

## Implementation

### Changes

- `docs/guides/distribution.md`
  - `sync` の安全条件 bullet(:135-141)— 「conflict file を報告して停止
    する — 二重発火を防ぐため利用者が明示的に解消する」を、conflict 検出は
    `legacyConflicts` array + `warning` 文字列として result に報告しつつ
    upsert を継続する旨へ改訂。eval-time dedup(issue #127)で共存は安全、
    単一登録への収束は optional と明記。
  - APM lifecycle 節(:174-189 へ 2 段落追加)— `post-install`/`post-update`
    が conflict 有りでも `verify` exit 0 passthrough で収束する旨、
    `post-uninstall` の `packages` 判定不能 branch も conflict-only
    residual で exit 0 となる旨を追記(issue #128)。さらに複数登録 path
    からの firing が fingerprint 一致 request 単位で評価 1 回・同一
    verdict へ収束する dedup の記述と、R11 の旧 binary caveat(PATH 上の
    pre-dedup binary では firing 毎に独立評価 — duplicate evaluation
    のみ・verdict 同一・correctness 維持)を追記。
  - 移行節(:197-201)— 「旧登録が再 import され conflict gate が発火
    しうる → import 無効化か entry 除去」を、再 import された旧登録は
    `legacyConflicts` + `warning` で報告されるのみで `sync`/lifecycle を
    block しない旨へ改訂。除去は optional 手順として残した。
- `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
  - :167-178 — "`sync` refuses to write while a legacy cheaper
    read-hook registration coexists … resolve duplicates explicitly" を、
    sync は legacy 登録と共存しても canonical entry を書き込み
    `legacyConflicts` + `warning` で報告する旨へ書き換え。dedup が重複
    発火を無害化するため共存は安全・除去は optional、`/hooks` で全
    loaded source の確認案内は維持。
  - :198-206(post-uninstall 段落)— `packages` 判定不能時は `verify`
    passthrough で residual legacy 登録が informational finding
    (`clean: true`・exit 0)となり lifecycle を失敗させない旨を追記。
    `verify` が残余状態を報告する fallback である記述は維持。
- `scripts/test/devin-hooks-install.test.mjs`(:269-330)
  - "stops on legacy cheaper registrations in known sources" →
    "warns on legacy cheaper registrations in known sources" へ rewrite。
    wrapper `-Apply` run で `r.code === 0`・`existsSync(configAt(root))
    === true`・config が canonical entry(`CANON_MATCHER` +
    `CANON_COMMAND`)を持つことを assert。
  - warning surfacing: wrapper は成功時に sync envelope を echo しない
    ため、実 CLI を direct spawn した `hook-config verify --project-root`
    (capability check 無し・PATH 加工不要)で `result.legacyConflicts` が
    windsurf file を含むこと・`result.findings` に `legacy-conflict:`
    entry・`result.clean === true` を assert。`legacyConflicts` の path
    は混在 separator で返りうるため `path.resolve` で正規化して比較。
  - `warning` field 自体は `hook-config sync --project-root` の direct
    spawn で確認する(task の optional 項目、:312-329)。sync の
    `check_capability` が PATH 上の cheaper を要求するため、spawn env の
    `PATH` 先頭へ `path.dirname(cliExe)` を連結して渡す。`warning` が
    非空 string で `legacyConflicts` が windsurf file を含むことを
    assert。
  - 旧 assert(非ゼロ exit・config 非作成・`/legacy|conflict/i` match)
    は全て削除。win32 + `cliBuilt` gating は不変。
- `docs/tasks/update-conflict-demotion-docs-and-mjs-test.md` — status
  `done`、checklist 全項目 checked。
- `docs/impl/ir/README.md` — `doc_maintenance.js apply --type impl` で
  本 IR の index row を再生成。

### Decisions During Implementation

- docs の引用文言は design Wording Contract ではなく T1 が ship した
  `hook_config.rs` の実文字列(`legacyConflicts`・`warning`・
  `legacy-conflict:` finding)を確認した上で field 名を引用 — task の
  binding どおり実装の確定形を source とした。
- mjs test の `legacyConflicts` path 比較は `path.resolve` 正規化を使用 —
  Rust 側が `root.join(rel)` の display を返すため separator 混在
  (`dir\.windsurf/hooks.json`)が起こりうる環境差を吸収する。
- `warning` field 自体の sync direct-spawn assert(task の optional 項目)
  も追加した — `check_capability` の PATH 要件は spawn env の `PATH`
  先頭へ `path.dirname(cliExe)` を連結して満たし、`warning` 非空
  string + `legacyConflicts` に windsurf file を含むことを sync 側でも
  pin した(test-spec の「assert が欲しい場合のみ sync を direct
  spawn」の選択肢を採用)。

## Related Experiments

### Adopted

なし(既知の docs/test 追従 task のため Experiment Log は作成していない)。

### Rejected

なし。

## Validation

- `node --test scripts/test/devin-hooks-install.test.mjs` — 11 passed /
  0 failed(win32 + built `target/debug/cheaper.exe` で real-CLI describe
  block を含む全 test が実効実行)。
- `grep -n "refuses to write while a legacy" packages/apm/.apm/skills/cheaper-usage/SKILL.md` — 0 件。
- `grep -n "duplicate firing is unsafe\|conflict gate\|二重発火\|conflict file" docs/guides/distribution.md packages/apm/.apm/skills/cheaper-usage/SKILL.md` — 0 件。
- `mise exec -- biome check scripts/test/devin-hooks-install.test.mjs` —
  clean(format・lint 共に pass)。
- `node scripts/check-doc-ids.mjs` — pass(legacy sequential ID 無し)。
- `target/debug/cheaper.exe hook-config verify` の smoke 実行で T1 の
  informational `legacy-conflict:` finding・`legacyConflicts` populate を
  実 binary で確認済み。

## Risks

- wrapper 経路の `warning` 不可視は変わらず残る
  (`install-devin-hooks.ps1` は成功時に sync envelope を echo しない —
  design で受容済みの residual risk。`verify` が観察経路)。
- `install-devin-hooks.ps1:5` header comment の「legacy conflict gate」
  表現は wrapper scope 外のため stale のまま — drift として design に
  記録済み(本 task でも変更しない)。
- 歴史的 doc(旧 blocker semantics を記述する
  `docs/designs/bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md:144`・
  `docs/tasks/cli-owned-hook-config-lifecycle.md:81-82`)は spec の
  Out of Scope どおり遡及更新していない。

## Follow-ups

なし — plan T1/T2 で降格の contract・docs・test 追従は完結。
