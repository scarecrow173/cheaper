---
id: "TASK-034LSvTlOcJm8dAlxd1fqP"
type: "task"
status: "done"
title: "File deferred Node-tooling follow-up issues (F1/F5)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  implemented-by: []
  depends-on:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  blocks:
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
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

# File deferred Node-tooling follow-up issues (F1/F5)

## Work

design §5 F-plan の ordering 制約に従い、即時起票可能な F1・F5 を
`gh issue create` で `scarecrow173/cheaper` へ起票し、発行された URL を
plan の Deferred Follow-up Tracking 表と本 task の Notes へ記録する。
gate 未達の F2-F4 は起票しない(追跡表で固定済み)、F6 は plan で消費
済み(comment 半分は T1、drift 半分は upstream lock-pinned 実測記録)。

起票手順(#151 plan と同じ冪等手順):

1. 各 issue について起票前に、title の unique prefix で重複確認:
   `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1" in:title --state all --json number,title,url`
   `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F5" in:title --state all --json number,title,url`
   既存に同題があれば作成を skip して URL 記録へ進む。
2. 下記 body 全文を temp file へ書き出し(転記のみ — 再解釈しない)、
   `gh issue create --repo scarecrow173/cheaper --title "<title>" --body-file <temp file>` で起票。
3. 発行 URL を plan の Deferred Follow-up Tracking 表の F1/F5 行と
   本 task の Notes へ記録。
4. issue #182 へ分解結果サマリ comment を 1 件投稿する。本 step は
   T1 の変更が commit されて PR が open された後に実行する
   (comment が PR link を参照するため。issue 起票自体の step 1-3 は
   T1 と並列実行可)。投稿前に
   `gh issue view 182 --repo scarecrow173/cheaper --json comments`
   で既存のサマリ comment を確認し、再実行時に既に投稿済みなら
   skip する(冪等):
   `gh issue comment 182 --repo scarecrow173/cheaper --body "<plan/task への link・F1/F5 issue URL・F2-F4 gated 記録・F6 消費の要約>"`。

### Issue 1(F1)

- title: `Node dev-tooling F1: cargo test の ambient node 結合を fixture binary へ repoint`
- body:

  ```markdown
  ## Scope

  `cargo test` が ambient PATH 上の `node` に暗黙結合している 3 site を
  fixture child binary / PATH 制御へ repoint し、test suite の toolchain
  独立性を上げる衛生改善。

  対象:

  - `crates/cheaper-tools/tests/proc_process.rs`
    `node_task_resolves_via_path` — node 不在時 fail(required)
  - `crates/cheaper-tools/tests/fixtures/process.json`
    `"bare node task resolves via PATH"` case — literal `node` command の
    differential golden、不在時 diverge → fail(required)
  - `crates/cheaper-workflow/tests/process.rs` `resolve_node()` /
    `compiles_policy_process_producer_and_executes_materialized_jobs` —
    node 不在時 silent skip(coverage 低下)
  - 併せて `crates/cheaper-tools/tests/fixtures/runtime.json` の dormant
    `"node"` task 宣言 6 件(現在 invoke されない latent coupling)を
    整理するか判断する。

  ## Acceptance Criteria

  - node 不在 PATH の裸 `cargo test` で ambient 依存 site が fail しない
    (fixture 経路で検証が継続する)。
  - `process.json` の golden は翻訳機構(`<NODE>` → fixture child 系)に
    統合される。
  - repoint 後も tool contract(bare 名 → PATH 解決 → spawn 成功)の検証
    意味が維持される。
  - `docs/designs/node-dev-tooling-boundary-contract-issue-182.md` §3 の
    ambient 結合表が新しい contract を反映して更新される。

  ## Dependencies

  - 独立。issue #183 および他の Node-tooling follow-up と無関係の小規模
    衛生 issue。いつでも実施可。

  ## 参照

  - Design:
    `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
    §3(ambient 結合契約)・§5 F1(scope・acceptance sketch の
    authoritative source)
  - ADR: `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
  - Inventory:
    `docs/discovery/node-dev-tooling-inventory-issue-182.md` §D
  - 親 issue: #182
  ```

### Issue 2(F5)

- title: `Node dev-tooling F5: brainstorming skill の node 依存 drop/replace`
- body:

  ```markdown
  ## Scope

  deploy 済み brainstorming skill(`.agents/skills/brainstorming/`・
  `.claude/skills/brainstorming/` の `helper.js`/`server.cjs` — local
  server 系の node 依存)を除去、または node-free な skill へ置換する。
  brainstorming は root `apm.yml` の direct dep ではなく transitive
  dep として到達する(`apm.lock.yaml`: `repo_url: obra/superpowers`・
  `is_virtual: true`・`depth: 2`・`resolved_by:
  scarecrow173/apm-packages` — root `apm.yml` に `obra/superpowers`
  エントリは無い)。実際の lever は:

  (a) maintainer 所有の upstream `scarecrow173/apm-packages` の
      package 宣言で brainstorming を drop/replace し、本 repo 側で
      apm dep bump(`resolved_commit`/`version` 更新)を行う。
  (b) apm に exclusion/override 機構がある場合はそれで transitive
      skill を除外する(存在は要検証 — 確認せず前提としない)。
  (c) 親 dep(該当 `scarecrow173/apm-packages` package)自体を drop
      する — ただし同 package が提供する他 skill も失う。

  `scarecrow173/apm-packages` は同一 maintainer 所有のため
  third-party(`obra/superpowers`)の対応を待たない — doc-suite の
  node-free 化(trigger T1)を待たずに本 repo 側で前進できる
  unblock path。

  ## Acceptance Criteria

  - `apm.lock.yaml` の `deployed_files` に brainstorming の
    `.js`/`.cjs`(`helper.js`・`server.cjs`)が出ないこと。
  - skill を落とす場合は doc workflow への影響(brainstorming skill が
    提供していた workflow)が issue 本文または migration note で案内され
    ていること。
  - `mise run setup`(`apm install`)が pass し、deploy 物が整合する
    こと。

  ## Dependencies

  - 独立。`scarecrow173/apm-packages` は同一 maintainer 所有のため
    third-party upstream 非依存で実施可能 — design trigger T1(全
    deploy 済み node-requiring skill の node-free 化 / drop)の
    unblock path の一部を前進させる。

  ## 参照

  - Design:
    `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
    §4 T1・§5 F5(scope・acceptance sketch の authoritative source)
  - ADR: `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
  - Inventory:
    `docs/discovery/node-dev-tooling-inventory-issue-182.md` §E
  - 親 issue: #182
  ```

### 起票しない候補(gated — plan の Deferred Follow-up Tracking で記録済み)

- F2(benchmark harness Rust 化): issue #183 結合前提。
- F3(`gen-glob-tables.cjs` oracle 再設計): trigger T3 発火時のみ。
- F4(check-*.mjs・`node --test` 系 Rust/native 化): trigger T1 発火後。
- F6(stale drift note + comment 修正): plan で消費済み — comment 半分は
  T1 で実施、drift 半分は upstream `scarecrow173/apm-packages` 所有の
  lock-pinned stale copy で cheaper repo の issue 対象外(T1 Notes 参照)。

## Classification

- `normal-plan-task`

## Upstream Documents

- docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md (T2 slice・Deferred Follow-up Tracking)
- docs/designs/node-dev-tooling-boundary-contract-issue-182.md (§5 F-plan)

## Dependencies

- なし(plan のみ — T1 とは独立、並列可)

## Affected Files

- (GitHub) `scarecrow173/cheaper` の新規 issue ×2
- `docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md`
  (Deferred Follow-up Tracking の URL 欄追記)
- 本 task doc(Notes への URL 記録)
- issue #182 へのサマリ comment 1 件

## Done When

- [x] F1・F5 が body 4 節(`## Scope`・`## Acceptance Criteria`・
      `## Dependencies`・`## 参照`)を持つ issue として起票された
      (または重複 check で既存 issue に dedup された)
- [x] 発行/既存 URL が plan の追跡表と本 task の Notes に記録された
- [x] F2-F4 が起票されず gated 記録のみであることが確認された
- [x] issue #182 へのサマリ comment は PR open 後に orchestrator が
      投稿する形で schedule 済み(post-PR sequencing — 投稿時に
      `gh issue view 182 --json comments` で既存 comment を検出して
      skip すること)

## Verification

- [x] `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1 in:title" --state all --json number,title` / `"Node dev-tooling F5 in:title"` で、title が F1・F5 entry を含む issue が返る(件数リテラルではなく title で判定 — `in:title` は search 文字列内に含める必要あり)
- [x] `gh issue view <F1 の番号> --repo scarecrow173/cheaper --json body --jq ".body"` が `## Scope`・`## Acceptance Criteria`・`## Dependencies`・`## 参照` を含む(F5 も同様)
- [x] `node scripts/check-doc-ids.mjs` exits 0(plan 追記に legacy ID を持ち込まない)
- [x] Manual: plan の Deferred Follow-up Tracking 表で F1/F5 の URL 欄・F2-F4 の gate・F6 の消費記録が整合(目視 supplement)

## Notes

### Issue URL 記録欄(実施時に記入)

- F1: https://github.com/scarecrow173/cheaper/issues/188
- F5: https://github.com/scarecrow173/cheaper/issues/189
- issue #182 comment: 未投稿 — step 4 は orchestrator の sequencing に
  より PR open 後へ defer(comment が PR link を参照するため)。
  本 task の残 criterion はこの comment 投稿のみ。

### 起票時の指定

- label・milestone・assignee は指定しない — repo の label 体系は
  未確認で、issue #182 自体も unlabeled(#151 plan と同じ判断)。
