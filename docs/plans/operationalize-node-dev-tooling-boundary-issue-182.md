---
id: "PLAN-034LSvLN7KnVMrtavZGKiv"
type: "plan"
status: "approved"
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
        path: "docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md"
      - type: "file"
        path: "docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
      - type: "file"
        path: "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
      - type: "file"
        path: "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
      - type: "file"
        path: "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
    modified:
      - type: "file"
        path: "AGENTS.md"
      - type: "file"
        path: "scripts/check-lockfile.mjs"
      - type: "file"
        path: "scripts/lib/release-artifacts.mjs"
      - type: "file"
        path: "docs/specs/node-dev-tooling-scope-issue-182.md"
      - type: "file"
        path: "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
      - type: "file"
        path: "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
      - type: "file"
        path: "docs/discovery/node-dev-tooling-inventory-issue-182.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/discovery/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
      - type: "file"
        path: "docs/test-specs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/node-dev-tooling-scope-issue-182.md"
  implemented-by:
    - "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
    - "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/phase-6-remove-typescript-node-pi.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/node-dev-tooling-scope-issue-182.md"
    - "docs/designs/node-dev-tooling-boundary-contract-issue-182.md"
    - "docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md"
  derived-by:
    - "docs/tasks/update-node-boundary-docs-and-stale-comments.md"
    - "docs/tasks/file-deferred-node-tooling-follow-up-issues.md"
    - "docs/tasks/verify-node-boundary-docs-and-check-pipeline.md"
  verifies: []
  verified-by:
    - "docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md"
  references:
    - "docs/discovery/node-dev-tooling-inventory-issue-182.md"
    - "AGENTS.md"
    - "CONTRIBUTING.md"
    - "mise.toml"
    - "scripts/check-lockfile.mjs"
    - "scripts/lib/release-artifacts.mjs"
  defers: []
  deferred-by: []
---

# Operationalize Node dev-tooling boundary contract (issue #182) Implementation Plan

## Goal

issue #182 の toolchain 決定(Option A — Node.js を pin した dev-tooling
runtime として維持、ADR-034LRjX5iYZHrubmKgKAL1)を boundary contract design
(DESIGN-BSEuqAM7QQUmRoTUGOMxbH)の規定どおり operationalize する。具体的には
(1) AGENTS.md の Node 段落を boundary contract の要約 + canonical 参照を持つ
宣言へ更新し、(2) design §5 F6 が拾った stale comment 2 件を実態へ整合させ、
(3) F-plan の先送り implementation 候補を ordering 制約どおりに起票・記録する。
変更は doc/comment のみで、check pipeline・mise.toml node pin・product runtime
の挙動は一切変えない — Option A は「現行を維持する」決定であり、Node 移行は
発生しない。

## Scope

単一 implementation stream — 宣言(doc/comment)の実態整合・follow-up 起票・
検証が同一 critical path(Option A 決定の運用固定)と同一 release intent
(issue #182 の完了)を共有するため分割しない。

In scope(design In Scope の operationalize 分のみ):

- AGENTS.md:81-83 の Node 段落更新 — repo 所有 Node entry の完全列挙
  (`scripts/`・`benchmarks/`・`crates/cheaper-tools/gen-glob-tables.cjs`・
  deploy 済み doc-suite skills)・境界 policy 要約(repo tooling = Node 可、
  product path = Node 不可・新規 ambient 結合の原則禁止・`mise.toml`
  node 除去 gate = required entrypoint 全 node-free + §1 維持 entry の
  移行完了)・design/discovery への canonical 参照。現行段落は既に「repo
  tooling のみ・product path 非依存」の骨格を持つが、`gen-glob-tables.cjs`
  が列挙から抜けており、boundary contract 本体への pointer が無い。
  併せて AGENTS.md:27 の toolchain 列挙から `python` を除去
  (mise.toml:19-21 の非含有 note と整合する stale 解消)。
- `scripts/check-lockfile.mjs:4-6` header の「check pipeline の最後尾に
  置き」claim を実際の位置(10 step 中 9 番目、`check-apm-payload` 直前)
  へ修正。
- `scripts/lib/release-artifacts.mjs:1-2` header の「shared by
  check-version.mjs / set-version.mjs / check-apm-payload.mjs」claim を
  実 importer(`check-version.mjs`・`set-version.mjs` の 2 file — grep
  で `check-apm-payload.mjs` が同 lib を import しないことを確認済み)
  へ修正。
- design §5 F-plan の先送り候補を ordering 制約どおりに処理: 即時可の
  F1・F5 を `gh issue create` で起票し URL を本 plan・task へ記録、
  gate 未達の F2・F3・F4 は追跡表で記録のみ、F6 は本 plan で消費
  (後述 deviation 参照)。
- 検証: `mise run check` 不変 pass・`node scripts/check-doc-ids.mjs`・
  触った file の doc audit・boundary invariant の grep 確認。

Out of scope(spec/design の境界 + Option A の非変更面):

- product runtime・hook 実行経路・配布 payload・`mise.toml`(`node = "26"`
  pin 維持)・`[tasks.check]` pipeline wiring・test 意味論の一切の変更。
- `.agents/skills/`・`.claude/skills/` 配下の deploy 物の編集(upstream
  所有、`apm install` で上書きされる)。
- F2(benchmark harness Rust 化 — issue #183 結合前提)・F3(glob-table
  codegen oracle 再設計 — trigger T3)・F4(check-*.mjs 系 Rust/native 化 —
  trigger T1)の起票。design §5 の ordering 制約により gate 未達。
- upstream doc-suite の node-free 化・`obra/superpowers` 側の対応。
- `CONTRIBUTING.md` — 同 file の Node 宣言(:55-57)は非列挙(「等」)の
  ため stale でなく、canonical 参照は AGENTS.md 側に集約する(編集不要)。

## Upstream Documents

- `docs/specs/node-dev-tooling-scope-issue-182.md`
  (SPEC-034LRjQlZwIik0Fp58p5hV, approved) — issue #182 の done 定義・
  Deferred Design Concerns(F1 相当の衛生 issue 候補・Option A 即時実装不要)。
- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
  (ADR-034LRjX5iYZHrubmKgKAL1, accepted) — Option A 決定・revisit
  trigger・follow-up issue 候補の列挙。
- `docs/discovery/node-dev-tooling-inventory-issue-182.md`
  (DISC-034LRjLlw4UAOWypGTWGJu, resolved) — 全 Node entry の inventory・
  stale comment 2 件の検出証拠・呼出し graph。

## Design Inputs

- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
  (DESIGN-BSEuqAM7QQUmRoTUGOMxbH, approved) — §1 非退行契約・§2 領域別
  境界 policy・§3 ambient 結合契約・§4 revisit trigger 観測点・§5
  F-plan(F1-F6)が本 plan の authoritative HOW。

design との差分(review 確定の justified deviation):

- **F6 の stale comment 半分を本 PR に吸収** — design §5 F6 は
  drift note + comment 修正 2 件を束ねた trivial issue だったが、
  comment 修正は本 PR の「宣言と実態の整合」作業と同一変更面・同一
  検証経路のため先行実施する。comment-only で risk は無視できる。
- **F6 の drift note 前提を訂正** — design は「`.agents/skills/
  cheaper-usage/SKILL.md` の旧 `node .windsurf/hooks/cheaper/cli/bin/
  cheaper.mjs` 参照は `apm install` 再実行で解消する」としたが、
  2026-10-25 の実測(`mise exec -- apm install` 実行後も :43 の旧参照が
  残存、file sha256 `b0edb6…` が `apm.lock.yaml`
  `local_deployed_file_hashes` の記録値と一致)により、当該 deploy 物は
  upstream `scarecrow173/apm-packages` の lock-pinned 内容であり repo
  内からは解消不能と判明した。本 plan は drift を「upstream 所有の
  cosmetic stale copy であり実 node 依存ではない」観測として task の
  Notes に記録するに留め、F6 の GitHub issue 化は行わない(comment
  半分は本 PR で消費、drift 半分は upstream 側対応事項として追跡表に
  記録 — cheaper repo の issue として起票する対象ではない)。
- **起票範囲を ordering 制約で限定** — design §5 の順序サマリ
  (F6・F1 独立、F5 third-party upstream 非依存、F2 は issue #183 結合前提、
  F3・F4 は trigger T3・T1 発火が前提)に従い、本 plan で起票するのは
  F1・F5 のみ。F2-F4 は gate 未達のまま起票すると後で close/reopen の
  ノイズになるため追跡表で固定する。

## File Map

| Path | Change | Responsibility | Notes |
| --- | --- | --- | --- |
| `AGENTS.md` | modify | Node 段落(81-83 行目)を boundary contract 要約 + canonical 参照へ更新 | replacement 文案は task doc に収録 |
| `scripts/check-lockfile.mjs` | modify (comment only) | header の pipeline 位置 claim を実態(9 番目/10、check-apm-payload 直前)へ修正 | :4-6。実行 code 非接触 |
| `scripts/lib/release-artifacts.mjs` | modify (comment only) | header の shared-by claim を実 importer 2 file へ修正 | :1-2。実行 code 非接触 |
| (GitHub) `scarecrow173/cheaper` の新規 issue ×2 | create | F1・F5 の起票 + deps/AC 記述 | `gh issue create`。body 全文は task doc に収録 |
| `docs/plans/operationalize-node-dev-tooling-boundary-issue-182.md` | add/modify | 本 plan。Deferred Follow-up Tracking の URL 欄を T2 で追記 | |
| `docs/test-specs/node-dev-tooling-boundary-declaration-accuracy.md` | add | 宣言≒実態の検証契約(TSPEC-034LSvGcYkAp0ImKqx2Alh) | |
| `docs/tasks/` ×3 | add | T1-T3 の task doc | |
| upstream chain doc front matter | modify (relations のみ) | spec/design/ADR へ derived-by/verified-by/implemented-by の reciprocal link | doc-workflow 慣行(#180 chain と同型) — doc 作成時に適用済み、本 plan の task 対象外 |

## Implementation Sequence

3 task — T1・T2 は互いに独立で並列可、T3 は両者の着地をまとめて検証する。

### T1 — boundary docs + stale comment 修正

- [ ] AGENTS.md:81-83 の Node 段落を task doc 収録の replacement 文案で
      更新し、:27 toolchain 列挙から `python` を除去
- [ ] `scripts/check-lockfile.mjs` header の「最後尾」claim を実位置へ修正
- [ ] `scripts/lib/release-artifacts.mjs` header の importer claim を実態へ修正
- [ ] `.agents/skills/cheaper-usage/SKILL.md` drift の実測観測(upstream
      lock-pinned・`apm install` 非解消・実依存なし)を task Notes へ記録
- [ ] `git diff` が doc/comment のみであることを確認(実行 code 差分 0)

### T2 — deferred follow-up の起票と記録

- [ ] `gh issue list --search "<title prefix>" in:title` で F1・F5 の
      重複確認後、`gh issue create` で 2 issue を起票(body 全文は
      task doc 収録分を転記)
- [ ] 発行 URL を本 plan の Deferred Follow-up Tracking 表と task Notes へ記録
- [ ] F2・F3・F4 の gated 記録と F6 の消費済み記録が追跡表で整合している
      ことを確認
- [ ] issue #182 へ分解結果サマリ comment(起票 URL・gated 一覧・本 PR
      link)を投稿

### T3 — 全体検証

- [ ] `mise run check` exits 0(pipeline 不変の sanity — node 経由 step
      を含む現行 10 step のまま)
- [ ] `node scripts/check-doc-ids.mjs` exits 0
- [ ] `audit_docs.js --type plan` / `--type test-spec` / `--type task` で
      新規 file に findings 0
- [ ] boundary invariant grep(`crates/*/src/*` の spawn 相当 0 件・comment
      修正の実態照合・AGENTS.md 宣言の完全列挙確認)
- [ ] `node scripts/check-apm-payload.mjs` exits 0(slim payload 契約不変)

## Dependency Graph

| Slice | Depends on | Blocks |
| --- | --- | --- |
| T1 boundary docs + comment 修正 | — | T3 |
| T2 follow-up 起票 + 記録 | — | T3 |
| T3 全体検証 | T1・T2 | — |

critical path は T1∥T2 → T3。T1 と T2 は file 面が交差しない
(AGENTS.md/scripts と GitHub issue + plan 追記)ため並列可。

## Dependencies and Risks

### Risk Register

| # | Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- | --- |
| 1 | AGENTS.md 段落の更新が実際の entry 集合と再び drift する | low | med | TSPEC-034LSvGcYkAp0ImKqx2Alh の Rules で列挙必須 4 カテゴリと canonical 参照を pin。将来の pipeline/entry 変更時は design §1 非退行契約の diff 確認が guard |
| 2 | `gh` 認証失敗・重複起票 | low | low | 起票前に `gh issue list --search "Node dev-tooling F1" in:title` / `"Node dev-tooling F5" in:title` の unique-prefix query で重複確認し既存なら URL 記録へ進む(冪等再開、#151 plan と同じ手順) |
| 3 | F2-F4 を誤って起票し ordering 制約に反する | low | med | task の起票対象を F1・F5 に限定し body 全文を収録。追跡表で gate 条件を明記 |
| 4 | comment 修正が意図せず実行 code を触る | low | low | task の Done When で `git diff` が comment 行のみであることを確認。biome/`mise run check` が不変 pass することを T3 で検証 |
| 5 | stale drift を「repo の実依存」と誤認して upstream deploy 物を編集する | low | low | T1 の Notes に upstream-locked 実測を記録。deploy 物は編集禁止(`.agents/`・`.claude/` は upstream 所有) |

### Rollback

- **Trigger**: AGENTS.md/comment 更新が実態と矛盾することが判明、
  または誤った issue が起票された。
- **Procedure**: doc/comment 変更は `git revert` で本 change の commit
  ごと戻せる(実行 code 非接触のため完全に戻る)。誤起票 issue は
  `gh issue close <n> --comment "<理由>"` で閉じる。
- **Verification**: revert 後に `mise run check` が pass すること。

## Deferred Follow-up Tracking

design §5 F-plan の disposition(T2 が URL 欄を埋める):

| ID | 候補 issue | disposition | gate / 根拠 | issue URL |
| --- | --- | --- | --- | --- |
| F1 | `cargo test` ambient `node` 結合の fixture binary repoint | **本 PR で起票** | 独立・小規模衛生 | (T2 が記録) |
| F2 | benchmark harness の Rust 化 | 起票しない(gated) | issue #183 結合前提 — redesign 機会に畳み込む | — |
| F3 | `gen-glob-tables.cjs` codegen oracle 再設計 | 起票しない(gated) | trigger T3 発火時のみ | — |
| F4 | check-*.mjs・`node --test` 系の Rust/native 化(Option C repo 側) | 起票しない(gated) | trigger T1 発火後 — upstream deploy script が残る間 removal payoff なし | — |
| F5 | brainstorming skill の node 依存 drop/replace | **本 PR で起票** | 独立・third-party(`obra/superpowers`)非依存 unblock path — transitive dep で `scarecrow173/apm-packages`(同一 maintainer 所有)側の宣言変更 + dep bump が主 lever | (T2 が記録) |
| F6 | stale deploy drift note + stale comment 修正 | **本 plan で消費** | comment 半分は T1 で実施。drift 半分は upstream(`scarecrow173/apm-packages`)所有の lock-pinned stale copy と実測確認済み(`apm install` 再実行でも解消せず、2026-10-25) — cheaper repo の issue 対象ではなく upstream 側対応事項として記録 | — |

## Task Breakdown

- T1: boundary docs + stale comment 修正 —
  `docs/tasks/update-node-boundary-docs-and-stale-comments.md`
- T2: deferred follow-up 起票 + 記録 —
  `docs/tasks/file-deferred-node-tooling-follow-up-issues.md`
- T3: 全体検証 —
  `docs/tasks/verify-node-boundary-docs-and-check-pipeline.md`

## Verification

| Spec AC / Requirement | Test / Check | Command |
| --- | --- | --- |
| AGENTS.md が boundary contract を反映(完全列挙・policy 要約・canonical 参照) | grep + 目視 | `grep -n "gen-glob-tables.cjs" AGENTS.md` が match、`docs/designs/node-dev-tooling-boundary-contract-issue-182.md` への link がある |
| `check-lockfile.mjs` header が実 pipeline 位置を述べる | comment 実態照合 | `grep -n "最後尾" scripts/check-lockfile.mjs` が 0 件、mise.toml:74(9 番目)・:75(最後尾 check-apm-payload)と整合する記述 |
| `release-artifacts.mjs` header が実 importer のみ名指す | comment 実態照合 | `grep -n "check-apm-payload" scripts/lib/release-artifacts.mjs` が 0 件、`grep -rn "release-artifacts" scripts/*.mjs` で importer = check-version/set-version のみ |
| F1・F5 が scope・AC・deps 付きで起票された | GitHub 実測 | `gh issue list --repo scarecrow173/cheaper --search "Node dev-tooling F1" in:title --state all --json number,title` / `"Node dev-tooling F5" in:title` で title が F1・F5 entry を含む issue が返り、各 body が `## Scope`・`## Acceptance Criteria`・`## Dependencies`・`## 参照` を持つ |
| check pipeline 不変(Option A の sanity) | canonical verification contract | `mise run check` exits 0 |
| doc-id・index 整合 | lint | `node scripts/check-doc-ids.mjs` exits 0、`audit_docs.js --type plan/test-spec/task` で新規 file の findings 0 |
| product path node-free 不変 | boundary invariant | `git grep -n -w 'node' -- 'crates/*/src/*'` で spawn 相当箇所 0 件(既知の語彙/comment hit は 〜60 件程度・時点依存のため目視除外 — assertion は spawn 相当 0 件。pathspec は `src/*` が必要、`crates/*/src` は `src` directory 自体にしか match せず git の generic pathspec 意味論で 0 match に縮退)、`git grep -n 'Command::new("node"' -- 'crates/*/src/*'` が 0 件、`node scripts/check-apm-payload.mjs` exits 0 |
| README index status が front-matter と一致 | index 整合 | `docs/*/README.md` の #182 chain 行の status cell が各 doc の front-matter status と一致(discovery=resolved・spec=approved・adr=accepted・design=approved・plan=approved・test-spec=draft・task×3=todo) |

## Open Questions / Gaps

- F1・F5 の issue 番号は T2 実行まで不明 — 追跡表の URL 欄は T2 が埋める。
- F2-F4 の起票は trigger/#183 発火後の別 chain。revisit trigger の観測手順は
  design §4 が authoritative。
- upstream `scarecrow173/apm-packages` の `cheaper-usage` SKILL.md drift の
  追跡機構は design §4 T1 の dep-bump 観測(dep bump 時に
  `apm.lock.yaml` `deployed_files` diff で解消を確認)。upstream 側への
  issue 起票は maintainer 判断に委ねる — 本 plan では起票しないことを
  決定として記録する。

## Self-Review

- [x] Every spec requirement maps to a task.(spec の done 定義 —
      boundary 明文化・follow-up 起票・検証 — が T1-T3 に対応。inventory・
      ADR・design の doc 本体は chain で完成済みのため本 plan の変更対象外)
- [x] No placeholders remain.(comment 修正の対象行・文案、issue body、
      verification command は全て確定値。URL 欄のみ T2 実行時に確定する
      記録欄で placeholder ではない)
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: inline execution — 変更は doc/comment 1 段落 + 2 行と
  `gh` 2 回の機械的作業。T1・T2 は並列可だが量が小さく逐次で十分。
- Independent work streams: T1・T2(両者の後に T3)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- 前提: upstream 4 doc(discovery resolved・spec approved・ADR accepted・
  design approved)が確定済み。plan は design を implementation order へ
  転記し、上記 3 件の justified deviation(F6 comment 吸収・F6 drift 前提
  訂正・起票範囲の ordering 限定)以外の独自判断を足していない。
- 重点確認: (a) AGENTS.md replacement 文案が design §2 の境界 policy を
  正確に要約しているか(特に「`crates/*/tests` の新規 ambient 結合は
  原則不可」と codegen 条件付き可の扱い)、(b) F6 drift が upstream-locked で
  `apm install` 非解消という実測が task Notes に記録されるか、
  (c) F1/F5 の issue body が design §5 の scope/acceptance sketch を
  転記し再解釈していないか、(d) F2-F4 が起票されず gate 記録のみで
  あるか。
- 既知の受容事項: `check-apm-payload.mjs` の走査範囲が `packages/*/
  .apm/hooks/` に限られ skills payload は human review 依存、
  `biome.jsonc` が `*.cjs` を lint coverage 外とする既知 gap(design
  §2.3)は本 plan でも変更しない — 新たな機械的検査を追加しない。
