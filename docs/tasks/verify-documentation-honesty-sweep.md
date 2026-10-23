---
id: "TASK-034L7LPMgAWmjPhefil8Dn"
type: "task"
status: "done"
title: "documentation honesty sweep 最終検証 — grep 検証・doc-id・audit_docs・mise run check"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
  changes:
    added: []
    modified:
      - type: "file"
        path: "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
      - type: "file"
        path: "docs/tasks/correct-guides-and-skills-honesty-sweep.md"
      - type: "file"
        path: "docs/tasks/append-refine-notes-to-adr-and-design-docs.md"
      - type: "file"
        path: "docs/tasks/correct-specs-and-annotate-test-specs.md"
      - type: "file"
        path: "docs/tasks/create-migration-guide-and-index.md"
      - type: "file"
        path: "docs/tasks/verify-documentation-honesty-sweep.md"
      - type: "file"
        path: "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
      - type: "file"
        path: "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  implemented-by: []
  depends-on:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/tasks/correct-guides-and-skills-honesty-sweep.md"
    - "docs/tasks/append-refine-notes-to-adr-and-design-docs.md"
    - "docs/tasks/correct-specs-and-annotate-test-specs.md"
    - "docs/tasks/create-migration-guide-and-index.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/documentation-honesty-sweep-and-migration-guide.md"
  references:
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep 最終検証 — grep 検証・doc-id・audit_docs・mise run check

## Work

plan `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
(PLAN-034L7LkYl6qzs5eZ9mEJZL)の T5。T1-T4 で完了した sweep が
design(DESIGN-034Lrgl6KUIjrLaU6BBL2U)の Verification Notes・
spec(SPEC-034L6i3XmfCaWRHAiw8TTT)の Acceptance Criteria どおりに
成立していることを、grep 検証・TSPEC Rule 対応確認・full
verification pipeline で閉じる。対象 doc の追加編集はしない
(検証と doc status 更新のみ — gap 検出時のみ該当 task へ差し戻し
または該当 doc へ軽微な補記)。

### 1. spec Acceptance Criteria ↔ TSPEC Rule の対応確認

spec(SPEC-034L6i3XmfCaWRHAiw8TTT)の Acceptance Criteria 各項目に
対応する検証が存在し pass していることを確認する
(TSPEC-034L7LgKjsa5dwb8PstQX9 の Rules に対応 — 欠落があれば
該当 task の責務として差し戻し):

| AC | 対応検証領域 |
|----|-------------|
| AC1(issue AC1): 「cheaper は OS sandbox ではない」・「子プロセスの fs 副作用は強制不能」が migration guide・command・managed-process guide の所定節に明記 | TSPEC Rule「canonical disclaimer 配置」— `grep -n "OS sandbox" docs/guides/migration.md docs/guides/command.md docs/guides/managed-process.md` が 3 file hit + canonical 項目 1-2 の文言確認 |
| AC2(issue AC2): 「sandbox/boundary/protection」表現と実強制の整合が guides・refine 対象 ADR/spec で取れている(掃き出し inventory 全件処理) | TSPEC Rule「guides stale absence-grep」+「refine 反映 — doc 種別別方式」— §2 の absence-grep + `Refine 注記`/annotation presence-grep |
| AC3(issue AC3): spawn shell-free と ENOEXEC→`/bin/sh` fallback の区別が exec 系 guide に記述 | `grep -n "ENOEXEC" docs/guides/command.md docs/guides/managed-process.md` が両 file hit + 「/bin/sh」「OS/libc 既定動作」「保証対象外」の文言確認 + ADR7 注記内の ENOEXEC 区別 |
| AC4(issue AC4): 全 16 対象 doc に訂正注記があり front-matter 規約(R6)を破っていない | 18 doc `Refine 注記` + 12 doc in-place/annotation + `related` backlink 4 件 + `refines`/`refined-by`/`supersedes`/`superseded-by` 新規 0 の `git diff` 確認 |
| AC5(issue AC5): migration.md に破壊的変更一覧と移行手順があり README に index row | `docs/guides/migration.md` 存在 + #163-#168 6 節 × 3 小節 + `docs/README.md` index row |
| AC(文書のみ): 差分が文書のみで `check-doc-ids` pass(README status 整合含む) | `git diff` file 一覧が docs/*.md・SKILL.md のみ + `node scripts/check-doc-ids.mjs` exits 0 |

design Verification Notes の対応: VN1 = disclaimer 配置
(AC1)、VN2 = 訂正 absence/presence-grep(AC2)、VN3 =
ENOEXEC 区別(AC3)、VN4 = refine 反映 + backlink + lineage
edge 0(AC4)、VN(d-2)(a)-(g) = off-list 定点検証 7 項目
(§2)、VN5 = migration guide + index(AC5)、末尾 = 文書のみ
差分 + check-doc-ids + README status 整合(R8)。

### 2. design Verification Notes の grep 検証(全項目)

- **disclaimer 配置(VN1)**: `grep -n "OS sandbox"
  docs/guides/migration.md docs/guides/command.md
  docs/guides/managed-process.md` が 3 file hit・
  `command.md` :141-153・`managed-process.md` :47-49 の
  所定節に canonical 項目 1-2 の文言。
- **stale absence(VN2)**: `command.md` に「native
  magic（MZ」・「shell metachar 拒否」・「workspace 相対
  path は拒否」が残存 0、`managed-process.md` に「native
  PE/ELF/Mach-O」・「shell 演算子、script shim を拒否」
  が残存 0。16 対象 doc それぞれに `## Refine 注記` 節
  または in-place 訂正(`issue #16` 注釈)がある。
- **ENOEXEC(VN3)**: `grep -n "ENOEXEC"` が
  `managed-process.md`・`command.md`・ADR7 注記で hit・
  「/bin/sh」・「OS/libc 既定動作」・「保証対象外」の
  区別記述。
- **refine 反映 + lineage(VN4)**: front-matter 付き全対象
  doc(ADR 10・design 8・spec 6・test-spec 6)に refine
  反映(注記節・in-place 訂正・annotation blockquote)
  があり、各 `related` に backlink 4 件(§(b) path 規約 —
  `docs/designs/` 配下は `./`・他は `../designs/`)。
  `git diff` で `refines:`/`refined-by:`/`supersedes:`/
  `superseded-by:` の新規出現が全差分内に 0。
- **off-list 定点(VN(d-2))**:
  - (a) `grep -l "issue #169" docs/test-specs/{policy-gated-
    command-tool,invocation-scoped-command-grants,managed-
    process-task-authorization,route-resolution,substitute-
    exec-route-mode,route-grant-set-projection}.md` が 6
    件を返し blockquote 配置(file-end 節ではない)である
  - (b) `grep -n "no-grant\|unroutable" docs/adr/defer-non-
    exec-routing-to-permission-model-redesign.md` が注記
    節内に hit(:225-229 falsify 訂正)
  - (c) `docs/specs/policy-gated-command-tool.md` :82/:104
    が writeScope+command fail-closed を「決定済み」として
    記述し Deferred 体裁を脱している(当該行に `Deferred`
    残存 0)
  - (d) `docs/specs/invocation-scoped-command-grants.md`
    :115/:173 が `RuntimePolicy.exec`・`inner.exec`/
    `exec: None` へ rename 済み(`RuntimePolicy.command`・
    `inner.command` 残存 0 — file-level `command.grants`
    shape 言及は残ってよい)
  - (e) `docs/designs/overview.md`・`docs/designs/
    workspace-filesystem-boundary-sandboxed-tools.md`・
    `docs/specs/cheaper-v1-worker-runtime-cli.md` が
    canonical file tool surface 限定へ補正済み
    (`grep -n "Refine 注記\|issue #169"` で各 file hit)
  - (f) `packages/apm/.apm/skills/cheaper-tool-operate/
    SKILL.md` :82-85・`docs/guides/usage.md` :207 の
    deny-list 言及が opt-in 条件付き
  - (g) off-list ADR 3 件 + off-list design 4 件(managed-
    process-canonical・invocation-scoped・workspace-
    filesystem-boundary・overview)に `## Refine 注記` 節
- **migration guide(VN5)**: `docs/guides/migration.md`
  存在・`#163`〜`#168` 6 節・各節「変更内容/利用者影響/
  修復」3 小節・`docs/README.md` User guides に
  `migration.md` 行。
- **文書のみ差分 + index(R8)**: `git diff` の対象 file が
  `docs/**/*.md`・`packages/apm/.apm/skills/*/SKILL.md`
  のみで code・fixture・test 差分 0。`node
  scripts/check-doc-ids.mjs` pass。`docs/designs/README.md`
  本 design row + 既存 chain 行(specs=…・adr=…・
  discovery=resolved)が front-matter status と整合。

### 3. 整合確認・full verification・status 更新

- **doc 内容の意味確認(grep を超える部分)**: 注記本文が
  design §(c)/(d)/(d-2) の注記要点と矛盾しないこと・
  guides の訂正後記述が同 file 内で自相矛盾しないこと・
  canonical 文言の 3 要素が全配置先で意味等価であること・
  deny-list 命名規約違反(「保護」「sandbox」「containment」
  「防御」での `execDenyPatterns` 呼称)が差分内にないこと。
- **既存の正直な記述の保持**: `command.md:46-100/:119-123/
  :148/:63`・`managed-process.md:47/:49`・
  `parent-agent-integration.md`・`configuration.md` が
  差分に含まれないまたは均質化に留まること
  (G-9 変更不要一覧 — `git diff` 確認)。
- `mise run check` が pass すること(biome・fmt・clippy
  `--all-targets`・version・doc-ids・`cargo test
  --workspace`・`node --test`・lockfile・apm-payload —
  `--features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  付きの canonical pipeline。文書のみ差分だが canonical
  pipeline で閉じる)。
- T1-T5 の task status を `done`・plan status を
  `completed` へ更新する(実測結果を各 task の
  `## Verification` 欄へ記録)。spec
  (SPEC-034L6i3XmfCaWRHAiw8TTT)・ADR
  (ADR-034L6ihibvbjmSnKmSffWH)は現行 `proposed` — status
  昇格(approved/accepted)を spec chain 慣行に従い行う
  (#168 先例 — planning commit 後の status 追随で
  spec=approved・ADR=accepted)。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (PLAN-034L7LkYl6qzs5eZ9mEJZL — T5)
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U — Verification Notes が
  AUTHORITATIVE)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT — Acceptance Criteria)
- `docs/test-specs/documentation-honesty-sweep-and-migration-guide.md`
  (TSPEC-034L7LgKjsa5dwb8PstQX9 — 全 Rules の最終確認)

## Dependencies

- `docs/tasks/correct-guides-and-skills-honesty-sweep.md`
  (T1) — guides + skills 訂正が完了していること
- `docs/tasks/append-refine-notes-to-adr-and-design-docs.md`
  (T2) — 18 doc の注記追記が完了していること
- `docs/tasks/correct-specs-and-annotate-test-specs.md`
  (T3) — 12 doc の訂正が完了していること
- `docs/tasks/create-migration-guide-and-index.md`
  (T4) — migration.md + index が完了していること
- (T1-T4 は相互に file disjoint で並列 — 本 task は全者の
  完了後)

## Affected Files

- `docs/tasks/correct-guides-and-skills-honesty-sweep.md`・
  `docs/tasks/append-refine-notes-to-adr-and-design-docs.md`・
  `docs/tasks/correct-specs-and-annotate-test-specs.md`・
  `docs/tasks/create-migration-guide-and-index.md`・
  `docs/tasks/verify-documentation-honesty-sweep.md`
  (status 更新)
- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (status 更新)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`・
  `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (status 追随 — spec chain 慣行)
- 観測 gap が見つかった場合のみ、該当 doc へ軽微な補記
  (本来は T1-T4 の責務 — 大規模な訂正のやり直しはしない)

## Done When

- [x] spec Acceptance Criteria AC1-AC5 + 文書のみ AC 全項目
      に対応する検証が存在し pass(§1 対応表)
- [x] design Verification Notes 全項目(VN1-VN5・
      (d-2)(a)-(g)・R8 末尾)が grep/`git diff` で実測
      されている(§2)
- [x] 注記本文・guides 訂正の意味確認(自相矛盾なし・
      canonical 等価・命名規約遵守)が取れている(§3)
- [x] 差分が docs/*.md・SKILL.md のみで code・fixture・
      test 変更 0 件
- [x] `node scripts/check-doc-ids.mjs` exits 0
- [x] `audit_docs` が本 issue の新規 doc で error 0
- [x] `mise run check` exits 0
- [x] 5 task + plan(+ spec/ADR の status 追随)が更新され
      ている

## Out of Scope

- 対象 doc の大規模な再訂正(実装漏れが見つかった場合は
  該当 task へ差し戻す — T5 で訂正作業をやり直さない)
- code 側不整合の修正(「doc が正しく実装が誤り」は
  follow-up issue 化 — doc を実装へ合わせる訂正はしない)
- 16 対象 + §(d-2) 一覧外の stale 補正(継続 sweep —
  spec Deferred)
- Option E exec-less route・OS-sandbox backed mode
- release/version bump・migration guide の継続運用方式
- `mise run check` 以外の新規検証基盤

## Verification

- [x] 上記 §1-§3 の全項目が pass(コマンド実測を記録)
- [x] `node scripts/check-doc-ids.mjs` exits 0(実測を記録)
- [x] `node .agents/skills/doc-status/scripts/audit_docs.js
      --type all --json` で本 issue の新規 doc が error 0
      (実測を記録)
- [x] `mise run check` exits 0(実測を記録)
- status 更新: T1-T5 task=`done`・plan=`completed`・
  spec=`approved`・ADR=`accepted`(#168 chain 慣行 —
  spec AC checkbox・plan task checkbox は spec chain 側
  管理のため未チェックのまま)・各 README index 追随

## 実施記録（issue #169 sweep 実施時）

### §1 AC ↔ 検証対応（全項目 pass）

- AC1 → VN1 disclaimer 配置 pass（下記 grep 実測）。
- AC2 → VN2 stale absence 0 件 + refine presence 確認 pass。
- AC3 → VN3 ENOEXEC 区別が command・managed-process・migration
  guide + ADR 注記で確認 pass。
- AC4 → VN4 refine 反映 20 doc + backlink 4 件/doc +
  prohibited lineage edge 新規 0 件 pass。
- AC5 → VN5 migration.md 6 節 × 3 小節 + README index row pass。
- AC(文書のみ) → 差分一覧が `docs/**/*.md`・`SKILL.md` のみ +
  `check-doc-ids` pass。

### §2 grep 実測（2026-10-24 実測値）

- **VN1**: `grep -n "OS sandbox" docs/guides/migration.md
  docs/guides/command.md docs/guides/managed-process.md` →
  3 file hit。`migration.md:7-8`（前提節・canonical 項目 1-2）、
  `command.md:143`（limitation 節）・`:154`（out of scope 節）、
  `managed-process.md:47`（authorization boundary 節）で
  「cheaper は OS sandbox ではない」+「spawn 子プロセスの
  fs 副作用は強制・完全観測できない」を確認。
- **VN2**: `command.md`/`managed-process.md`/`workers.md`/
  `usage.md` への stale 肯定文言 grep — `native magic`・
  マジックバイト審査・metachar/shim/interpreter 拒否・
  workspace 相対 executable 拒否・`PE/ELF`/`Mach-O` いずれも
  0 hit（現行記述は「拒否しない」「viability のみ判定」
  の否定形のみ残存）。
- **VN3**: `grep -n "ENOEXEC"` → `command.md:152`・
  `managed-process.md:27`/`:47`・`migration.md:9` に加え、
  ADR 注記側は `documentation-honesty-sweep-and-migration-guide.md:156`・
  `fix-tool-runtime-…-upfront.md:239` で hit。全箇所で
  「`/bin/sh` retry」「OS/libc 既定動作（glibc `execvpe` 系、
  musl は retry しない残留差分あり）」「保証対象外」の
  3 要素が揃う。
- **VN4**: `grep -l "Refine 注記" docs/adr/*.md docs/designs/*.md`
  → 20 file（対象 ADR 10 + design 8 + 本 sweep 自身の
  ADR/design が注記節または format 定義本文として含有）。
  各 refine 対象の `related` に sweep spec・sweep ADR・
  migration guide・sweep design の backlink 4 件を確認
  （例: `workspace-filesystem-boundary-cheaper-sandboxed-tool.md:34-37`）。
  `git diff 9a919eb..HEAD` + 未コミット差分で
  `^+\s*(refines|refined-by|supersedes|superseded-by):\s*\S` は
  0 hit — prohibited lineage edge 新規追加なし。
- **VN(d-2)**:
  - (a) `grep -l "issue #169"` が 6 test-spec 全件を返す
    （policy-gated-command-tool・invocation-scoped-command-grants・
    managed-process-task-authorization・route-resolution・
    substitute-exec-route-mode・route-grant-set-projection）。
    配置は対象 bullet/Example 直下の blockquote。
  - (b) `defer-non-exec-routing-…md:233`/`:260` で
    `no-grant`/`unroutable` の falsify 訂正を確認
    （builtin deny 除去 → opt-in `execDenyPatterns` 記述）。
  - (c) `policy-gated-command-tool.md:82`/`:104` が
    writeScope × `command` fail-closed「確定済み」記述
    （`registry.rs:1167`・issue #169 注記付き）。
  - (d) `invocation-scoped-command-grants.md:119` が
    `RuntimePolicy.exec`・`:177` が `inner.exec`/`exec: None`。
    `RuntimePolicy.command`/`inner.command` 残存 0。
  - (e) `overview.md:151`・`workspace-filesystem-boundary-
    sandboxed-tools.md:117`・`cheaper-v1-worker-runtime-cli.md
    :75`/`:79`/`:135` で canonical file tool 限定の補正確認。
  - (f) `cheaper-tool-operate/SKILL.md:84`（"is declared —
    are re-applied"）・`usage.md:208`（opt-in 注記）で
    deny-list が opt-in 条件付き記述であることを確認。
  - (g) off-list ADR 3 件 + off-list design 4 件
    （managed-process-canonical・invocation-scoped・
    workspace-filesystem-boundary・overview）に
    `## Refine 注記` 節あり — VN4a 一覧で確認。
- **VN5**: `grep -nE "^#{1,3} " docs/guides/migration.md` →
  `# Migration guide` + `前提` 節 + `#163`〜`#168` の 6 節、
  各節 `変更内容`/`利用者影響`/`修復・移行手順` の 3 小節
  （### 18 個）。`docs/README.md:27` に index row 確認。
- **R8 文書のみ差分**: `git diff 9a919eb..HEAD` + 未コミット
  差分 + 未追跡の和集合が全て `docs/**/*.md`・
  `packages/apm/.apm/skills/*/SKILL.md` — code・fixture・
  test 差分 0 件。`docs/designs/README.md` は本 design row が
  既に `approved` で front-matter と整合（無改訂でよい）。

### §3 整合確認

- 注記本文が design §(c)/(d)/(d-2) の注記要点と矛盾しないこと・
  guides 訂正後記述の自相矛盾がないこと・canonical 文言
  3 要素（OS sandbox 否定・file tool 強制面・spawn 子 process
  観測限界）が全配置先で意味等価であることを目視確認。
- `execDenyPatterns` を「保護」「sandbox」「containment」
  「防御」と呼ぶ記述は差分内に 0（deny-list 命名規約遵守）。
- G-9 変更不要一覧（`parent-agent-integration.md`・
  `configuration.md` 等）は差分に含まれないことを file 一覧で確認。

### コマンド実測

- `node scripts/check-doc-ids.mjs` → exit 0
  （`doc id check OK: 1192 markdown files scanned`）。
- `node .agents/skills/doc-status/scripts/audit_docs.js
  --type all --json` → exit 0。1030 file scan・
  findings 839 件は全て `warning`（既存 repo-wide の
  inconsistent-reciprocal-relation 系）で `error` 0 件。
  本 issue 関連 doc の findings は 15 件で全て warning
  （`related`/`derives-from` の非相互 link — blocking ではない）。
- `mise run check` → exit 0（biome・cargo fmt --check・
  clippy --all-targets・check-version 1.0.0・
  check-doc-ids 1192 files・`cargo test --workspace`・
  `node --test` scripts/benchmarks・check-lockfile・
  check-apm-payload — 全段 pass）。
- status 更新: T1-T5 task front-matter を `done`、plan を
  `completed`、spec を `approved`、ADR を `accepted` へ
  昇格。`docs/tasks/README.md`・`docs/plans/README.md`・
  `docs/specs/README.md`・`docs/adr/README.md` の index row を
  追随（`docs/designs/README.md` は既に approved で無改訂）。
