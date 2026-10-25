---
id: "TASK-034LSvYSW97wwisapzR9Uo"
type: "task"
status: "todo"
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

- [ ] `mise run check` が exits 0(pipeline 不変の sanity)
- [ ] `mise.toml` の `node = "26"` pin が `mise.lock` と整合して残る
- [ ] AGENTS.md・両 script header の宣言が実態と一致(TSPEC Rules の
      grep 系 assertion を全て通る)
- [ ] `crates/*/src` に node spawn 相当箇所が無く、
      `check-apm-payload.mjs` が pass する(product path node-free 不変)
- [ ] `node scripts/check-doc-ids.mjs` が exits 0、新規 file の doc
      audit findings が 0
- [ ] chain 各 doc の front-matter status と `docs/*/README.md` index
      status cell が一致する
- [ ] F1・F5 issue が 4 節 body で open・URL 記録済み、F2-F4 が
      未起票(gated 記録のみ)、issue #182 へサマリ comment 済み
- [ ] `git diff main...HEAD` の tracked 変更が doc/comment + 新規 doc +
      README index + upstream front matter のみで実行 code 差分が 0
      (task/plan doc は branch 途中で commit されるため `git status`
      では出ない — main との diff で見る)

## Verification

- [ ] `mise run check` exits 0
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `node .agents/skills/doc-status/scripts/audit_docs.js --type plan --json` / `--type test-spec --json` / `--type task --json` で新規 file に findings 0
- [ ] `grep -n "最後尾" scripts/check-lockfile.mjs` が 0 件、`grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs` が 0 件、`grep -n "gen-glob-tables.cjs" AGENTS.md` が match
- [ ] `grep -rn "release-artifacts" scripts/check-version.mjs scripts/set-version.mjs` が両 file の import を返す(positive assertion)
- [ ] `git grep -n -w 'node' -- 'crates/*/src/*'` で spawn 相当 0 件・`git grep -n 'Command::new("node"' -- 'crates/*/src/*'` が 0 件(既知の語彙/文字列 hit は目視除外)
- [ ] `node scripts/check-apm-payload.mjs` exits 0
- [ ] `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1" in:title --state all --json number,title` / `"Node dev-tooling F5" in:title` で title が F1・F5 entry を含む issue が返る
- [ ] chain 各 doc の front-matter status と `docs/*/README.md` index の status cell が一致(目視照合)
- [ ] `git diff main...HEAD --name-status` で tracked 変更が doc/comment + 新規 doc + README index + upstream front matter のみ(実行 code 0 行)

## Notes

- `mise run check` は ~数分かかる canonical contract — 途中失敗があれば
  本 PR の変更(doc/comment)とは無関係の pre-existing failure かを
  切り分けて記録する(本変更は実行 code に触れないため、check 失敗が
  出た場合はほぼ確実に外部要因 — 再実行と既存 issue との照合を行う)。
- POSIX shell 前提の `find`/`grep` 系は Windows では Git Bash 経由で
  実行する(design §4 の観測手順 note と同じ前提)。
- 検証結果(pass/fail・各 command の exit・issue URL)を Notes へ追記して
  `done` へ進める。
