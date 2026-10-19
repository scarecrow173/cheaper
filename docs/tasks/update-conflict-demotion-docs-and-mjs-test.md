---
id: "TASK-034IM7WZt5kW1QHzLRcugI"
type: "task"
status: "done"
title: "Update hook-config docs and devin-hooks-install test for conflict demotion"
created: "2026-10-19"
updated: "2026-10-20"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/128"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/guides/distribution.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "scripts/test/devin-hooks-install.test.mjs"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
  implemented-by: []
  depends-on:
    - "docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md"
    - "docs/tasks/demote-legacy-conflict-gate-in-hook-config.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/hook-config-legacy-conflict-warning-contract.md"
  references:
    - "docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
    - "docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md"
  defers: []
  deferred-by: []
---

# Update hook-config docs and devin-hooks-install test for conflict demotion

## Work

plan の T2: 降格後 contract へ docs 2 file と `node --test` pipeline の mjs
test を追従させる。引用する文言・field 名は T1
(`demote-legacy-conflict-gate-in-hook-config.md`)が design Wording Contract
どおりに着地させた最終形を前提とする — 文言確定の source は design であり
T1 がそれを code へ反映するため、docs 側は design の確定文字列をそのまま
引用してよい。

### docs(`docs/guides/distribution.md`)

- [x] :135-138(`sync` 安全条件 bullet)— 「conflict file を報告して停止
      する — 二重発火を防ぐため利用者が明示的に解消する」を、conflict
      検出は `legacyConflicts` + `warning` として result に報告され
      upsert は継続される旨へ改訂。重複発火は eval-time dedup で無害化
      されるため共存は安全で、単一登録への収束は optional と明記
- [x] :148-169(APM lifecycle 節)— `post-install`/`post-update` が
      conflict 有りでも verify exit 0 passthrough で収束する旨を追加。
      さらに旧 binary caveat を追記(R11)— PATH 上の cheaper binary が
      dedup 導入前の version の場合、重複登録は firing 毎に独立評価される
      が影響は duplicate evaluation(token 計測・bounded
      materialization・bounded scan の重複)のみで verdict は同一・
      correctness は維持される(`eval_cache.rs` module doc :4-7)
- [x] :171-182(移行節)— 「旧登録が再 import され conflict gate が
      発火しうる → import 無効化か entry 除去」を、再 import された旧
      登録は warning として報告されるのみで sync/lifecycle を block しない
      旨へ改訂。単一登録へ収束したい場合の除去は optional 手順として残す

### skill(`packages/apm/.apm/skills/cheaper-usage/SKILL.md`)

配布 source の同 path のみを更新 — repo 内 `.agents/skills/cheaper-usage/
SKILL.md` は deploy 写しで同節を持たない(consumer 側生成物)。

- [x] :167-171 — "`sync` refuses to write while a legacy cheaper
      read-hook registration coexists … resolve duplicates explicitly" を、
      sync は legacy 登録と共存しても canonical entry を書き込み
      `legacyConflicts` + `warning` で報告する旨へ書き換え。dedup が重複
      発火を eval 時に無害化するため共存は安全で、単一登録への収束は
      optional(`/hooks` で全 loaded source の確認案内は維持)
- [x] :194-199(post-uninstall 段落)— residual な legacy 登録は
      informational で lifecycle を失敗させない(`packages` 判定不能時も
      verify passthrough で exit 0)旨を追記し、`verify` が残余状態を
      報告する fallback である記述を維持

### mjs test(`scripts/test/devin-hooks-install.test.mjs`)

- [x] :269-285 "stops on legacy cheaper registrations in known sources" を
      "warns on legacy cheaper registrations in known sources" へ書き換え:
      wrapper `-Apply` run → `r.code === 0`・`existsSync(configAt(root))
      === true`・parse した config が canonical entry(`CANON_MATCHER` +
      `CANON_COMMAND`)を持つことを assert
- [x] warning surfacing の assert — wrapper は成功時に sync envelope を
      echo しない(`install-devin-hooks.ps1:84-93` — `Write-Host $out` は
      `ConvertFrom-Json` 失敗時のみ)ため `r.combined` への `warning`
      assert は不可能。実 CLI を直接 spawn して検証する:
      `spawnSync(cliExe, ["hook-config","verify","--project-root",root])`
      の stdout JSON で `result.legacyConflicts` が windsurf file を含み・
      `result.findings` に `legacy-conflict` entry・`result.clean ===
      true`。verify は `check_capability`(PATH 上の cheaper)を持たない
      ため PATH 加工なしで動き、この verify-spawn assert だけで spec AC
      は充足する
- [x] (optional)`warning` field 自体を assert する場合のみ `sync` を
      direct spawn — idempotent なため再実行しても `warning` は返るが、
      `sync` は `check_capability` → `find_on_path` で PATH 上の cheaper
      binary を要求するため env に
      `PATH: `${path.dirname(cliExe)}${path.delimiter}${process.env.PATH}``
      を渡す
- [x] 旧 assert(非ゼロ exit・`existsSync(configAt(root)) === false`・
      `assert.match(r.combined, /legacy|conflict/i)`)は全て削除 — 新
      contract では全て偽になる
- [x] win32 gating 維持 — describe block `{ skip: !runPsAvailable ||
      !cliBuilt }` はそのまま(Windows + built `target/debug/cheaper(.exe)`
      必須。非 Windows では skip の既存 coverage model)

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/demote-hook-config-legacy-conflict-gate-to-warning.md (T2 節)
- docs/test-specs/hook-config-legacy-conflict-warning-contract.md (mjs 層
  Rule/Example)
- docs/designs/hook-config-legacy-conflict-gate-demotion-to-warning.md
  (Documentation Design 節 — 各 file の改訂方針)
- docs/specs/hook-config-legacy-conflict-gate-demotion-to-warning.md (R11)

## Dependencies

- `docs/tasks/demote-legacy-conflict-gate-in-hook-config.md` (T1)— mjs test
  の direct spawn は T1 済みの `target/debug` binary が前提(降格済み
  verify/sync を spawn しないと新 assert が偽になる)。docs 引用文言は
  design Wording Contract で確定済みのため T1 完了前に drafting は可能だが、
  verify/mjs 実行と最終文言照合は T1 着地後に行う。

## Affected Files

- modify: `docs/guides/distribution.md`
- modify: `packages/apm/.apm/skills/cheaper-usage/SKILL.md`
- modify: `scripts/test/devin-hooks-install.test.mjs`

Out of scope(本 task では触らない): `hook_config.rs`・Rust integration
test(T1)、`install-devin-hooks.ps1` wrapper 本体(:5 header comment の
stale「legacy conflict gate」表現は drift 記録済みで変更しない)、歴史的
doc(旧 blocker semantics を記述する既存 design/task)の遡及更新、repo 内
`.agents/skills/cheaper-usage/SKILL.md`(deploy 写しで同節を持たない)。

## Done When

- [x] distribution.md の sync 安全条件・lifecycle 節・移行節が warning 化後
      の規約を記述し、旧 binary caveat(duplicate evaluation のみ・verdict
      同一)が追記されている
- [x] cheaper-usage SKILL.md の hook-config 節が「sync は共存しても
      書き込み `legacyConflicts` + `warning` で報告」へ整合し、
      post-uninstall 段落に residual が informational で lifecycle を
      失敗させない旨が追記されている
- [x] mjs test が exit 0・config 書込み済み・direct verify spawn での
      `legacyConflicts`/`legacy-conflict` finding/`clean:true` を assert
      し、旧 gate assert が残っていない
- [x] `node --test` pipeline が pass(win32 + built CLI 環境)

## Verification

- [x] `node --test scripts/test/devin-hooks-install.test.mjs` が exit 0
      (**win32 + built `target/debug/cheaper(.exe)` gated** — 非 Windows /
      未 build では describe block が skip されるため、検証には
      `mise exec -- cargo build -p cheaper-cli` 済みの Windows 環境が必要)
- [x] `grep -n "refuses to write while a legacy" packages/apm/.apm/skills/cheaper-usage/SKILL.md` が 0 件
- [x] `grep -n "duplicate firing is unsafe\|conflict gate\|二重発火\|conflict file" docs/guides/distribution.md packages/apm/.apm/skills/cheaper-usage/SKILL.md` が 0 件
- [x] `mise run check` が exit 0(biome・`cargo fmt --check`・clippy・
      version/doc-id・`cargo test --workspace`・`node --test`・
      lockfile/payload check — canonical pipeline 全体)
