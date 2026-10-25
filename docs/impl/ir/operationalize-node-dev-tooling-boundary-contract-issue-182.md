---
id: "IMPL-034LTxBKOIF2rELRSlh5SJ"
type: "impl"
status: "completed"
title: "Operationalize Node dev-tooling boundary contract (issue #182)"
created: "2026-10-25"
updated: "2026-10-25"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/182"
  changes:
    added:
      - type: "file"
        path: "docs/impl/ir/operationalize-node-dev-tooling-boundary-contract-issue-182.md"
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "scripts/check-lockfile.mjs"
      - type: "file"
        path: "scripts/lib/release-artifacts.mjs"
      - type: "file"
        path: "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
      - type: "file"
        path: "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
      - type: "file"
        path: "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
      - type: "file"
        path: "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
    - "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  implemented-by: []
  depends-on:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/node-dev-tooling-inventory-issue-182.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "docs/specs/node-dev-tooling-scope-issue-182.md"
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Operationalize Node dev-tooling boundary contract (issue #182)

## Summary

issue #182 の Option A 決定(Node.js を pinned dev-tooling runtime として維持、
ADR-034LRjX5iYZHrubmKgKAL1)を boundary contract design
(DESIGN-BSEuqAM7QQUmRoTUGOMxbH)どおりに operationalize する
PLAN-034LSvLN7KnVMrtavZGKiv の実装記録。T1-T3 の全 task が `done`。

- **T1 — boundary docs + stale comment 修正**: `AGENTS.md` の Node 段落を
  task doc 収録の replacement 文案で verbatim 置換(repo 所有 Node entry
  4 系統の完全列挙 + 境界 policy 要約 + design/discovery/ADR への
  canonical 参照。現 :81-100)、併せて :27 toolchain 列挙から `python` を
  除去(mise.toml:19-21 の非含有 note と整合する stale 解消)。
  `scripts/check-lockfile.mjs:4-7` header の「check pipeline の最後尾」
  claim を「`check-apm-payload` の直前」(相対位置、序数なし)へ修正。
  `scripts/lib/release-artifacts.mjs:1-2` header の shared-by 列挙から
  `check-apm-payload.mjs` を除去(実 importer =
  `check-version.mjs:23`・`set-version.mjs:23` のみ)。実行 code 非接触。
- **T2 — deferred follow-up 起票**: design §5 F-plan の ordering 制約
  どおり、即時可の F1・F5 を `gh issue create` で起票 —
  **#188**(F1: cargo test ambient node 結合の fixture binary repoint)・
  **#189**(F5: brainstorming skill の node 依存 drop/replace)。両 body は
  task doc 収録の 4 節(`## Scope`・`## Acceptance Criteria`・
  `## Dependencies`・`## 参照`)を verbatim 転記。発行 URL を plan の
  Deferred Follow-up Tracking 表(:283・:287)と T2 Notes へ write-back。
  F2-F4 は gate 未達で起票せず追跡表で記録のみ、F6 は本 plan で消費。
- **T3 — 全体検証**: `mise run check` exits 0(2nd run)、
  `check-doc-ids.mjs` exits 0、doc audit 新規 chain findings 0、
  boundary invariant grep(spawn 相当 0 件)・宣言 grep・index/status
  整合を全 pass。

`git diff main...HEAD` の tracked 変更は doc/comment + 新規 chain doc +
README index + upstream front matter のみで**実行 code 差分 0 行** —
Option A「現行を維持する」決定どおり、check pipeline・`mise.toml`
`node = "26"` pin・product runtime の挙動は一切変更していない。

## Context

承認済み plan(PLAN-034LSvLN7KnVMrtavZGKiv, approved)の 3 task —

- T1: TASK-034LSvSfFCNY9T3y1exy8S(boundary docs + stale comment 修正)
- T2: TASK-034LSvTlOcJm8dAlxd1fqP(F1/F5 起票 + 記録)
- T3: TASK-034LSvYSW97wwisapzR9Uo(全体検証)

に従う。検証契約は TSPEC-034LSvGcYkAp0ImKqx2Alh(declaration accuracy
Rules)。upstream chain は DISC-034LRjLlw4UAOWypGTWGJu(resolved)→
SPEC-034LRjQlZwIik0Fp58p5hV(approved)→ ADR-034LRjX5iYZHrubmKgKAL1
(accepted)→ DESIGN-BSEuqAM7QQUmRoTUGOMxbH(approved)で確定済み。
本 IR は issue #182 chain 内の doc 集合で閉じ、foreign chain への
lineage edge は持たない。

## Implementation

### Changes

- `AGENTS.md`(:81-100): Node 段落を boundary contract 要約へ置換 —
  `scripts/`・`benchmarks/`・`crates/cheaper-tools/gen-glob-tables.cjs`
  (glob table codegen の regex oracle)・deploy 済み doc-suite skills
  (`.agents/skills/`・`.claude/skills/`、upstream 所有)の 4 系統を列挙し、
  product/distribution/hook 経路の Node 非依存・新規 ambient 結合の原則
  禁止・`mise.toml` `node` 行の removal gate・canonical 参照
  (design/discovery・ADR-034LRjX5iYZHrubmKgKAL1)を明記。(:27)
  toolchain 列挙から `python` を除去 — `grep "python" AGENTS.md` =
  file 全体で 0 件。
- `scripts/check-lockfile.mjs`(:4-7): header の pipeline 位置 claim を
  「最後尾」→「`check-apm-payload` の直前」へ修正(mise.toml:74 = 10
  step 中 9 番目、最後尾は :75 の `check-apm-payload.mjs`)。drift 検出
  根拠(先行 build/test の lockfile 再生成捕捉)は維持。comment 行のみ。
- `scripts/lib/release-artifacts.mjs`(:1-2): header の shared-by 列挙を
  実 importer 2 file(`check-version.mjs`・`set-version.mjs`)のみへ修正。
  `check-apm-payload.mjs` は同 lib を import しない。comment 行のみ。
- GitHub `scarecrow173/cheaper`: F1→**issue #188**、F5→**issue #189** を
  起票(title・body 4 節とも task doc 収録分を verbatim 転記、
  label/milestone/assignee 未指定 — #151 と同じ判断)。
- `docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md`:
  Deferred Follow-up Tracking 表の F1/F5 行 URL 欄へ #188/#189 を
  write-back。F2-F4 gated・F6 消費の記録と整合。
- `docs/tasks/` ×3: status → `done`、Done-When/Verification チェック
  完了、T1 Notes に drift 実測観測・T2 Notes に issue URL と #182
  comment defer 記録・T3 Notes に実施記録を追記。
- `docs/tasks/README.md`: T1・T2・T3 の index status cell を `done` へ
  修正(T3 の audit finding 駆動)。
- `.agents/skills/cheaper-usage/SKILL.md` の旧 hook path drift は deploy
  物を編集せず(upstream 所有)、lock-pinned stale copy という実測観測を
  T1 Notes・plan F6 行・ADR Correction note に記録するに留めた。

### Decisions During Implementation

- **dedup query の形を訂正** — plan/task が記した `gh issue list
  --search "Node dev-tooling F1" in:title` は `in:title` が独立 arg と
  なり `gh` に unknown argument として拒否される。実際は
  `--search "Node dev-tooling F1 in:title"`(`in:title` を search 文字列
  内に含める)で重複確認・起票後照合を実行。T2/T3 の Verification に
  訂正済み query を記録。
- **T2 step-4(issue #182 サマリ comment)を post-PR へ defer** —
  comment が本 PR の link を参照するため、orchestrator の sequencing で
  PR open 後の投稿へ変更。投稿時は `gh issue view 182 --json comments`
  で既存を確認して冪等 skip する手順を T2 Done-When/T3 Notes に記録。
  T3 時点では PR 未 open(`gh pr list --head` = 0 件)・comment 0 件を
  確認済み。
- **F6 は GitHub issue 化せず本 plan で消費** — design §5 F6 は drift
  note + comment 修正 2 件の trivial issue だったが、comment 半分は
  T1 の「宣言と実態の整合」と同一変更面・同一検証経路のため本 PR で
  実施。drift 半分は `.agents/skills/cheaper-usage/SKILL.md:43` が
  upstream `scarecrow173/apm-packages` 由来の lock-pinned stale copy
  (deploy sha256 `b0edb6bc…a74da2` が `apm.lock.yaml`
  `local_deployed_file_hashes` の pin 値と一致、`apm install` 再実行
  でも解消しない 2026-10-25 実測)と判明し、cheaper repo の issue
  対象ではなく upstream 側対応事項として記録(plan F6 行・T1 Notes・
  ADR :225-232/design :302-306 の Correction note)。
- **design §5 F5 の wording を transitive-dep 実態で起票** —
  「brainstorming skill の node 依存 drop/replace」は、root `apm.yml`
  の direct dep ではなく `apm.lock.yaml` 実測(`repo_url:
  obra/superpowers`・`is_virtual: true`・`depth: 2`・`resolved_by:
  scarecrow173/apm-packages`)に基づく transitive dep として記述し、
  実 lever を 3 択((a) maintainer 所有 upstream の package 宣言変更 +
  apm dep bump、(b) apm exclusion/override 機構 — 存在未確認で前提と
  しない、(c) 親 dep drop)で明示。design F5 row 自体も同じ実測構造で
  authored されており、filed issue #189 はその転記。
- **check-lockfile header は序数を書かない** — 「9 番目/10」は再び
  stale 化し得るため「`check-apm-payload` の直前」という相対位置のみを
  記述(task 指定どおり)。
- **tasks README index cell 修正** — T3 の初回 task audit で T1/T2 の
  index cell が front-matter `done` に対し stale `todo` のままという
  finding 2 件を検出し、`docs/tasks/README.md` を修正後に再実行で 0。

## Related Experiments

### Adopted

なし(known-plan execution のため Experiment Log は未作成。
`metadata.experiments.adopted` は空)。

### Rejected

なし。

## Validation

- `mise run check`: **exits 0**(2nd run、全 10 step pass — biome /
  fmt / clippy / check-version / check-doc-ids / `cargo test --workspace`
  / `node --test` scripts/test(43) / `node --test` benchmarks/test(12)
  / check-lockfile / check-apm-payload)。1st run は
  `crates/cheaper-tools/tests/proc_command.rs::abort_kills_and_preserves_reason`
  が `assertion failed: !alive(pid)` で 1 件 fail — 分離再実行・2nd
  全量 run ともに pass。本 branch は実行 code 差分 0 のため flaky な
  Windows process-kill timing race と切り分け済み(同名既存 issue 0 件)。
- `mise.toml` `node = "26"`(:10)↔ `mise.lock` `tools.node` = 26.12.1
  (全 platform)整合 — Option A の pin 維持を確認。
- `node scripts/check-doc-ids.mjs`: exits 0(1211 markdown files)。
- `node .agents/skills/doc-status/scripts/audit_docs.js --json` を
  `--type plan/test-spec/task/design/adr/discovery/spec` で全量実行 —
  本 chain の新規 file に findings 0(index 修正後。他 type は
  pre-existing の `inconsistent-reciprocal-relation` warning のみ)。
- 宣言 grep(TSPEC Rules): `grep -n "最後尾" scripts/check-lockfile.mjs`
  = 0 件、`grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs`
  = 0 件、`grep -n "gen-glob-tables.cjs" AGENTS.md` = :83 match、
  `grep -n "python" AGENTS.md` = 0 件。
- importer positive assertion: `grep -rn "release-artifacts"
  scripts/check-version.mjs scripts/set-version.mjs` = 両 file の import
  行(:23)を返す。`check-apm-payload.mjs` は importer でないことを確認。
- boundary invariant: `git grep -n -w 'node' -- 'crates/*/src/*'` = 59
  hit、全て語彙/comment/test fixture(`INTERACTIVE_COMMANDS` 語彙・
  schema walker `fn node`・glob node・`NodeIoError`・telemetry target・
  `executable.rs` `#[cfg(test)]` fixture)で spawn 相当 0 件。
  `git grep -n 'Command::new("node"' -- 'crates/*/src/*'` = 0 件。
  `node scripts/check-apm-payload.mjs` exits 0(`packages/*/.apm/hooks/`
  に `.js`/`.mjs` 無し)。
- GitHub 実測: `gh issue view 188` = OPEN(F1)・`gh issue view 189` =
  OPEN(F5)、両 body に 4 節を確認。F2-F4 は未起票(追跡表で gated
  記録のみ)。
- index/status 整合: discovery=resolved・spec=approved・adr=accepted・
  design=approved・plan=approved・test-spec=draft・task×3=done の
  front-matter ↔ 各 `docs/*/README.md` index cell が一致。
- `git diff main...HEAD --name-status`: 実行 code 差分 0 行を確認。

証跡 commit: `07e9a6a`(chain doc: discovery/spec/ADR/design + index)→
`e809976`(plan/test-spec/task×3 + AGENTS.md/script comment + 起票
write-back)→ `b3514e3`(検証 evidence + index done cell)。

## Risks

- AGENTS.md の Node 段落が将来の entry/pipeline 変更で再び drift する
  可能性 — TSPEC-034LSvGcYkAp0ImKqx2Alh の Rules が列挙 4 系統と
  canonical 参照を pin し、design §1 非退行契約の diff 確認が guard。
- F2-F4 は gate 未達のまま未起票 — 対応 trigger(issue #183・T3・T1)
  発火時の別 chain が起票を担う。忘失リスクは plan の Deferred
  Follow-up Tracking 表で軽減済み。
- upstream `scarecrow173/apm-packages` の `cheaper-usage` SKILL.md
  drift は repo 内から解消不能 — 解消観測は design §4 T1 の dep-bump
  時 `apm.lock.yaml` `deployed_files` diff に依存。
- `abort_kills_and_preserves_reason` の flaky は本 change 非起因と
  切り分け済みだが、Windows CI での再発可能性は残る。

## Follow-ups

- **issue #182 分解結果サマリ comment**(残 criterion)— PR open 後に
  orchestrator が投稿(F1/F5 URL・F2-F4 gated・F6 消費・PR link)。
  投稿前に `gh issue view 182 --json comments` で既存確認して冪等
  skip する。
- **F2(benchmark harness Rust 化)** — issue #183 結合前提で未起票。
  plan 追跡表で gate 記録済み。
- **F3(`gen-glob-tables.cjs` oracle 再設計)** — trigger T3 発火時のみ。
- **F4(check-*.mjs・`node --test` 系 Rust/native 化)** — trigger T1
  発火後。
- **upstream `cheaper-usage` SKILL.md 修正** — `scarecrow173/apm-packages`
  側の upstream 対応事項。issue 起票は maintainer 判断(plan で起票
  しないことを決定として記録済み)。
- flaky test `abort_kills_and_preserves_reason` の tracking issue 起票
  判断、PR open・merge、issue #182 の close 判断は
  orchestrator/maintainer 所有。
