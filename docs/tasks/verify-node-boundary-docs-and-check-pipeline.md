---
id: "TASK-034LSvYSW97wwisapzR9Uo"
type: "task"
status: "done"
title: "Verify Node boundary docs and run check pipeline"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  implemented-by: []
  depends-on:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    - "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
    - "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  defers: []
  deferred-by: []
---

# Verify Node boundary docs and run check pipeline

## Work

T1(boundary docs + comment 修正)と T2(follow-up 起票 + 記録)の着地済み
変更に対し、宣言≒実態の一致と canonical verification contract
(`mise run check`)の不変 pass を確認する収束 task。新規 code 変更は無い。

確認観点(TSPEC-034LSvGcYkAp0ImKqx2Alh の Rules に対応):

1. **宣言の実態整合** — T1 の diff が doc/comment のみであること、
   AGENTS.md 段落が完全列挙(4 系統)+ policy 要約 + canonical 参照を
   持つこと、両 script の header が実 pipeline 順・実 import と一致する
   ことを grep と目視で確認する。
2. **pipeline 不変** — `mise run check` が exits 0(node 経由 step を
   含む現行 10 step、Option A の不変 sanity)。`mise.toml` の
   `node = "26"` pin が `mise.lock` と整合して残ることを確認。
3. **boundary invariant** — product path node-free の維持:
   `git grep -n -w 'node' -- 'crates/*/src/*'` で node spawn 相当箇所が
   0 件(既知の語彙/comment hit は 〜60 件程度・時点依存 —
   `INTERACTIVE_COMMANDS` 語彙・comment・`nodepath` module・
   `NodeIoError`・schema/glob の `node` 語彙 — は目視で除外。
   assertion は spawn 相当箇所 0 件)、`git grep -n 'Command::new("node"' --
   'crates/*/src/*'` が 0 件、`node scripts/check-apm-payload.mjs` が
   exits 0 で `packages/*/.apm/hooks/` に `.js`/`.mjs` が無いこと。
   なお `'crates/*/src'`(末尾 glob 無し)は `src` directory 自体にしか
   match せず 0 match に縮退して vacuous になる — git の generic
   pathspec 意味論で platform 固有の問題ではないため、`src/*` まで
   書くこと。
4. **doc 整合** — `node scripts/check-doc-ids.mjs` exits 0、
   `audit_docs.js --type plan` / `--type test-spec` / `--type task` で
   本 chain の新規 file(plan・test-spec・task×3)に findings が無いこと
   (repo 全体の pre-existing findings は対象外、新規 file に限定して数える)。
5. **follow-up 記録整合** — F1・F5 の issue が 4 節 body で open、
   plan の Deferred Follow-up Tracking 表に URL・gate・F6 消費記録が
   整合、issue #182 へのサマリ comment が投稿済みであること。
   issue body の照合は T2 が起票時に `gh issue view --jq .body` で
   capture した記録で代替可(未 capture の場合のみ T3 で
   `gh issue view <n> --repo scarecrow173/cheaper --json body --jq .body`
   を実行し 4 節を確認)。
6. **index・importer 整合** — chain 各 doc(discovery resolved・
   spec approved・adr accepted・design approved・plan approved・
   test-spec draft・task×3 todo)の front-matter status が
   `docs/*/README.md` の index status cell と一致すること
   (specs/adr README の修正済み cell を含めて全 chain で照合)。
   併せて `grep -rn "release-artifacts" scripts/check-version.mjs
   scripts/set-version.mjs` で両 file が同 lib を import することを
   確認する(positive assertion — header 修正の 0 件 check と対)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md (T3 slice・Verification matrix)
- docs/designs/node-dev-tooling-boundary-contract-issue-182.md (§1 非退行契約・Verification Notes)
- docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md (Rules・Examples)

## Dependencies

- docs/tasks/update-node-boundary-docs-and-stale-comments.md (T1)
- docs/tasks/file-deferred-node-tooling-follow-up-issues.md (T2)

## Affected Files

- なし(検証のみ。確認対象: `AGENTS.md`・`scripts/check-lockfile.mjs`・
  `scripts/lib/release-artifacts.mjs`・`mise.toml`・plan/task doc・
  GitHub issue 2 件)

## Done When

- [x] `mise run check` が exits 0(pipeline 不変の sanity)
      — 2nd run exits 0。1st run は `proc_command.rs`
      `abort_kills_and_preserves_reason` の flaky fail(分離再実行・
      全量再実行ともに pass — Notes 参照)
- [x] `mise.toml` の `node = "26"` pin が `mise.lock` と整合して残る
      (mise.toml:10 / mise.lock `tools.node` = 26.12.1 全 platform)
- [x] AGENTS.md・両 script header の宣言が実態と一致(TSPEC Rules の
      grep 系 assertion を全て通る)
- [x] `crates/*/src` に node spawn 相当箇所が無く、
      `check-apm-payload.mjs` が pass する(product path node-free 不変)
- [x] `node scripts/check-doc-ids.mjs` が exits 0、新規 file の doc
      audit findings が 0(T1/T2 の tasks README index stale cell を
      `done` へ修正した後に 0 — 修正前は index-status finding 2 件)
- [x] chain 各 doc の front-matter status と `docs/*/README.md` index
      status cell が一致する(T1/T2 の stale `todo` cell を `done` へ
      修正済み — 本 task の status 遷移と同 commit で index 整合)
- [x] F1・F5 issue が 4 節 body で open・URL 記録済み、F2-F4 が
      未起票(gated 記録のみ)。issue #182 サマリ comment は未投稿 —
      PR 未 open(`gh pr list --head` = 0 件)のため T2 記録どおり
      orchestrator が PR open 後に投稿する残 criterion(冪等:
      投稿前に `--json comments` で既存確認して skip)
- [x] `git diff main...HEAD` の tracked 変更が doc/comment + 新規 doc +
      README index + upstream front matter のみで実行 code 差分が 0
      (task/plan doc は branch 途中で commit されるため `git status`
      では出ない — main との diff で見る)

## Verification

- [x] `mise run check` exits 0(2nd run — 全 10 step pass。
      1st run の `abort_kills_and_preserves_reason` fail は flaky:
      分離再実行 pass・2nd 全量 pass、docs/comment-only diff で
      実行 code に触れないため本 change 起因ではない)
- [x] `node scripts/check-doc-ids.mjs` exits 0(1211 markdown files scanned)
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js --type plan --json` / `--type test-spec --json` / `--type task --json` (+ `--type design` / `--type adr` / `--type discovery` / `--type spec` も実施)で新規 file に findings 0
      — index 修正前の task audit で T1/T2 の index-status finding 2 件
      を検出し `docs/tasks/README.md` を修正、再実行で 0
- [x] `grep -n "最後尾" scripts/check-lockfile.mjs` が 0 件、`grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs` が 0 件、`grep -n "gen-glob-tables.cjs" AGENTS.md` が match(:83)
- [x] `grep -rn "release-artifacts" scripts/check-version.mjs scripts/set-version.mjs` が両 file の import を返す(positive assertion — check-version.mjs:23・set-version.mjs:23。check-apm-payload.mjs は importer でないことを確認)
- [x] `git grep -n -w 'node' -- 'crates/*/src/*'` で spawn 相当 0 件(59 hit 全て目視除外済み: `INTERACTIVE_COMMANDS` 語彙・schema walker `fn node`・glob node loop var・`NodeIoError`・telemetry target・`executable.rs` の `#[cfg(test)]` fake-file fixture・comment)・`git grep -n 'Command::new("node"' -- 'crates/*/src/*'` が 0 件
- [x] `node scripts/check-apm-payload.mjs` exits 0(`packages/*/.apm/hooks/` に `.js`/`.mjs` 無し — `.ps1`・`.sh`・`hooks.json` のみ)
- [x] `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1 in:title" --state all --json number,title` / `"Node dev-tooling F5 in:title"` で #188(F1)・#189(F5)が返る(`in:title` は search 文字列内に含める必要あり — 独立 arg だと unknown argument になる)
- [x] chain 各 doc の front-matter status と `docs/*/README.md` index の status cell が一致(目視照合 + audit_docs task type での機械検出を併用)
- [x] `git diff main...HEAD --name-status` で tracked 変更が doc/comment + 新規 doc + README index + upstream front matter のみ(実行 code 0 行)

## Notes

- `mise run check` は ~数分かかる canonical contract — 途中失敗があれば
  本 PR の変更(doc/comment)とは無関係の pre-existing failure かを
  切り分けて記録する(本変更は実行 code に触れないため、check 失敗が
  出た場合はほぼ確実に外部要因 — 再実行と既存 issue との照合を行う)。
- POSIX shell 前提の `find`/`grep` 系は Windows では Git Bash 経由で
  実行する(design §4 の観測手順 note と同じ前提)。
- 検証結果(pass/fail・各 command の exit・issue URL)を Notes へ追記して
  `done` へ進める。

### 実施記録(2026-10-25)

- `mise run check`: **exits 0**(2nd run、全 10 step pass — biome /
  fmt / clippy / check-version / check-doc-ids / cargo test --workspace /
  node --test scripts/test(43 tests) / node --test benchmarks/test(12) /
  check-lockfile / check-apm-payload)。1st run は
  `crates/cheaper-tools/tests/proc_command.rs::abort_kills_and_preserves_reason`
  が `assertion failed: !alive(pid)` で 1 件 fail — 分離再実行
  (`cargo test -p cheaper-tools --test proc_command abort_kills_and_preserves_reason`)
  で pass、2nd 全量 run でも pass。本 branch は実行 code 差分 0 のため
  flaky な Windows process-kill timing race と切り分け。該当 test 名での
  既存 issue 検索は 0 件(flaky 関連は closed #157 のみ・別 test) —
  flaky test の tracking issue 起票は orchestrator/maintainer 判断に委ねる。
- `mise.toml` `node = "26"`(:10) ↔ `mise.lock` `tools.node` =
  26.12.1(linux/macos/windows 全 platform entry)整合を確認。
- `git diff main...HEAD --name-status`: AGENTS.md・6 README index・
  2 script(comment のみ — `最後尾`→`check-apm-payload の直前`・
  `check-apm-payload.mjs` importer 誤記の除去)・新規 chain doc ×9・
  upstream ADR/design front-matter — 実行 code 差分 **0 行**。
- AGENTS.md Node 段落: 完全列挙 4 系統(scripts/・benchmarks/・
  gen-glob-tables.cjs・deploy 済み doc-suite skills)+ policy 要約 +
  design/discovery/ADR 参照 + ambient 3 site 記述は design §3 と
  整合。mise.toml の check pipeline(:69-75)で check-lockfile が
  check-apm-payload 直前(9 番目)であることも実照合済み。
- Boundary invariant: `git grep -n -w 'node' -- 'crates/*/src/*'` =
  59 hit、全て語彙/comment/test fixture(`INTERACTIVE_COMMANDS`
  語彙・schema walker・glob node・`NodeIoError`・telemetry target・
  `executable.rs` `#[cfg(test)]` fake-file fixture)。`Command::new("node"`
  = 0 件。`packages/*/.apm/hooks/` に `.js`/`.mjs` 無し。
  `check-apm-payload.mjs` exits 0。
- Doc audit: `--type plan/test-spec/task/design/adr/discovery/spec` を
  `--json` で全量実行。初回の task type で chain finding 2 件を検出
  (T1・T2 の tasks README index cell が front-matter `done` に対し
  stale `todo`)→ `docs/tasks/README.md` の 2 cell を `done` へ修正後、
  再実行で chain finding **0**。他 type の出力は全て pre-existing
  の `inconsistent-reciprocal-relation` warning(非 chain file)のみ。
- `node scripts/check-doc-ids.mjs`: exits 0(1211 markdown files)。
- F1/F5: `gh issue view 188` = OPEN「Node dev-tooling F1: cargo test の
  ambient node 結合を fixture binary へ repoint」、`gh issue view 189` =
  OPEN「Node dev-tooling F5: brainstorming skill の node 依存
  drop/replace」— 両 body に `## Scope`・`## Acceptance Criteria`・
  `## Dependencies`・`## 参照` の 4 節を確認。plan の Deferred
  Follow-up Tracking 表の URL 欄(:283 F1→#188、:287 F5→#189)と
  F2-F4 gated・F6 消費記録も整合。
- issue #182 サマリ comment: `gh issue view 182 --json comments` =
  **0 件(未投稿)**。`gh pr list --head chore/node-deps-inventory-issue-182`
  も 0 件で PR 未 open — T2 Notes の記録どおり comment は PR link を
  参照するため PR open 後に orchestrator が投稿する残 criterion
  (T2 Done-When も「schedule 済み」で done 済み)。post 時は
  `--json comments` で既存を確認して冪等 skip すること。
- index 整合: discovery=resolved・spec=approved・adr=accepted・
  design=approved・plan=approved・test-spec=draft・task×3=done の
  front-matter ↔ 各 README index cell が一致(本 task の status 遷移と
  README cell を同時更新)。
- 変更 file: `docs/tasks/README.md`(T1・T2 index cell → done、
  本 task cell → done)・本 task doc(status・Done-When/Verification・
  Notes)のみ。commit は orchestrator が実施。
