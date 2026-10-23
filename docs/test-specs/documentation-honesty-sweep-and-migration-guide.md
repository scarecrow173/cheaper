---
id: "TSPEC-034L7LgKjsa5dwb8PstQX9"
type: "test-spec"
status: "approved"
title: "documentation honesty sweep + migration guide の検証契約 (issue #169)"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
  changes:
    added: []
    modified: []
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
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "docs/test-specs/trusted-route-flag-issuer-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
  derived-by: []
  verifies:
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/adr/enforce-declared-contracts-over-content-audit.md"
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "docs/impl/ir/trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/guides/command.md"
    - "docs/guides/managed-process.md"
    - "docs/README.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep + migration guide の検証契約 (issue #169)

## Purpose

issue #169(権限モデル再設計 #151 分解の最終 slice 7/7)で確定した
「**claim ≤ 実強制** の文書正直性 sweep + 破壊的変更 migration
guide」を、canonical disclaimer 配置・guides stale 訂正・refine
対象 doc への訂正反映・front-matter 規約・migration guide 構成・
文書のみ差分の各面で固定する。
SPEC-034L6i3XmfCaWRHAiw8TTT の R1-R8・Acceptance Criteria
(AC1-AC5)と DESIGN-034Lrgl6KUIjrLaU6BBL2U の Verification
Notes(grep 検証可能形)が authoritative。

保つべき意図は 4 つ。(1) **claim ≤ 実強制**: 提供しない保護
(OS sandbox・子プロセス fs 副作用の強制・内容審査・builtin
deny)を文書で誇張しない — 訂正後の文言が実装強制範囲を超えない。
(2) **履歴非改竄**: ADR/design/test-spec の本文・Rule/Example は
決定時点の記録として保持し、file-end `## Refine 注記` 節または
in-place annotation blockquote で補正する — supersede・
superseded status 変更なし。(3) **graph lineage 保護**: cross-
chain の意味 link は `related` backlink のみ — 新規 `refines`/
`refined-by`/`supersedes`/`superseded-by` edge ゼロ(commit
9d76d40 規約)。(4) **文書のみの差分**: code・fixture・test・
`.agents/`・`apm_modules/` 生成物を一切変更しない(spec R8)。

本 spec の「test」は cargo test ではなく **grep ベースの doc
check**(presence-grep / absence-grep / `git diff` 確認 /
`check-doc-ids` / `audit_docs`)である — design Verification
Notes が全検証項を grep 可能形で固定している。

退役条件: 次の破壊的変更・権限モデル再変更で canonical
disclaimer 文言(ADR D1)・refine 規約(ADR D3)・migration guide
置き場所(ADR D4)が見直されるまで存続する。16 対象一覧外の
stale 補正は継続 sweep 対象であり本契約の検証対象外
(spec Deferred)。

## Feature

権限モデル再設計(#163–#168)で除去・降格・再設計された機構
(内容審査 7 項目・builtin executable deny list・self-declared
`trusted` shadowing・C9 single-grant 投影・`RuntimePolicy.command`)
を前提とした stale 記述が、38 file + 新設 migration.md へ
design の per-doc 訂正表どおりに反映される。訂正方式は doc 種別
で分岐する: ADR/design は file-end `## Refine 注記（issue
#169）` 節追記(本文保持)、spec は要件本文の in-place 訂正 +
`（issue #16X — …）` inline 注釈、test-spec は該当 bullet
直下の `> （issue #169 注記 — …）` blockquote、guide・skill は
in-place 書換。front-matter を持つ全対象 doc へ `related`
backlink 4 件(本 spec・本 ADR・migration guide・本 design —
§(b) path 規約)を追加する。

## Rules

- Rule: **canonical disclaimer 配置 — 3 file 所定節**(spec R1・
  issue AC1・design VN1 — presence-grep)
  - `docs/guides/migration.md`(前提節)・`docs/guides/command.md`
    (trust/sandbox 限界節 :141-153)・`docs/guides/
    managed-process.md`(caveat 節 :47-49)に ADR D1 の canonical
    文言相当が配置される — `grep -n "OS sandbox" <file>` が 3
    file とも hit。
  - 3 要素すべてを含む: (1)「cheaper は OS sandbox ではない」
    (2)「canonical file tool(write/edit/ls/read/find/grep)の
    workspace boundary(E1/E2)・`filesystem.denyPaths`・worker
    `writeScope` は cheaper 所有の tool surface で強制されるが、
    spawn した子プロセスの fs 副作用は OS sandbox 無しでは
    強制・完全観測できない」(3) shell-free / ENOEXEC 区別。
    逐語再掲でなくてよいが意味の等価性を保つ。
  - 既存の正直な記述(`command.md:143/:145/:148`・
    `managed-process.md:47/:49`)と重複・矛盾しない —
    `command.md` は節冒頭を canonical 形へ強化 + ENOEXEC
    bullet 追加、`managed-process.md` は caveat を canonical
    項目 1・2 へ揃え :27 への cross-reference 1 行を添えてよい。

- Rule: **spawn shell-free と ENOEXEC→`/bin/sh` fallback の
  区別**(spec R2・issue AC3・design VN3 — presence-grep)
  - `managed-process.md`・`command.md` に「cheaper の spawn は
    shell を介さず argv を直接渡す(shell metachar は executable
    の literal argv として渡り cheaper が解釈・拒否しない)」と
    「POSIX で shebang を持たない +X file の ENOEXEC に libc が
    `/bin/sh` で retry するのは OS/libc 既定動作(glibc
    `execvpe` 系・musl は retry しない残留差分)で cheaper の
    保証対象外」の区別がある — `grep -n "ENOEXEC"` で両 guide
    hit。ADR7(`fix-tool-runtime…`)の refine 注記本文にも
    ENOEXEC 区別を含む(§(a) 配置表)。

- Rule: **guides stale 4 箇所の訂正 — absence-grep**(spec R3・
  issue AC2・design VN2)
  - `docs/guides/command.md` に「native magic（MZ」・「shell
    metachar 拒否」・「workspace 相対 path は拒否」が**残存
    しない**(G-1/G-2)。訂正後は 3 形態受理(workspace 相対は
    boundary 内へ解決・bare name は ambient PATH・absolute は
    直接 viability)・内容審査なし・残存 bounds は input schema
    `INVALID_INPUT`・唯一の executable 審査は opt-in
    `security.execDenyPatterns`。
  - `docs/guides/managed-process.md` に「native PE/ELF/Mach-O」・
    「shell 演算子、script shim を拒否」が**残存しない**
    (G-3/G-4)。訂正後は宣言 `command` 配列の内容審査なし・
    win32 `.cmd`/`.bat`/`.ps1` は CreateProcess viability 制約・
    unix +X file は内容無審査で解決・pnpm 助言は保持。
  - 同 file 内で記述が自相矛盾しない(`command.md:100` の
    migration 注記・:46-100 execDenyPatterns 節との整合)。

- Rule: **deny-list 命名規約**(spec R4・ADR D2 — sweep 全編集
  の wording 確認)
  - `security.execDenyPatterns` は「deterministic executable
    deny-list」(opt-in・既定 `[]`・executable basename stem
    への anchored case-insensitive 完全一致・3 適用点)と呼ぶ。
    「保護」「sandbox」「containment」「防御」とは呼ばない。
  - 列挙を免れる経路(改名・代替 binary・plugin/extension)の
    但書は `command.md:63`・`:148` の既存記述を維持・再利用
    (新規の誇張表現を足さない)。

- Rule: **refine 対象 16 doc + off-list 追加への訂正反映 —
  doc 種別別方式**(spec R5・issue AC2/AC4・design VN2/VN4 —
  presence-grep + 内容確認)
  - ADR 10 件(ADR1-7 + off-list `managed-process-named-task-
    lifecycle`・`invocation-scoped-command-grants-as-a-synthetic-
    trusted-policy-layer`・`opt-in-soft-gating-…`)と design 8
    件(D-1〜D-4 + off-list `managed-process-canonical-tool`・
    `invocation-scoped-command-grants`・`workspace-filesystem-
    boundary-sandboxed-tools`・`overview`)に file-end
    `## Refine 注記` 節 — `grep -n "Refine 注記" <file>` が
    全 18 doc で hit。節は 1 つ・最終節の**後**(特定節名の
    後ではない — D-2 は `## Coordinator review amendments`
    の後)。canonical 形の必須要素: issue #169・変更点・現行
    真実・governing link(ADR-034L6ihibvbjmSnKmSffWH・
    SPEC-034L6i3XmfCaWRHAiw8TTT・migration guide)。
  - spec 6 件(S-1〜S-5 + off-list `cheaper-v1-worker-runtime-
    cli`)は要件本文の in-place 訂正 + `（issue #16X — …）`
    inline 注釈。S-1 は差分が大きいため末尾 `## Refine 注記`
    併置推奨。S-3(route spec)は backlink のみ + 残差分の軽微
    訂正(R8 routing candidacy 層は別層 — 訂正不要、見つかれば
    層区別 1 行補記)。
  - test-spec 6 件(TS-1〜TS-6)は該当 bullet/Example 直下の
    `> （issue #169 注記 — …）` blockquote — file-end 節を
    設けない。TS-5 は同一 falsified 機構の cluster 末尾へ
    1 本集約可。
  - 各 doc の訂正内容は design §(c)/(d)/(d-2) の per-doc 表の
    anchor・訂正方向・注記要点どおり(ADR1 = #163/#164 降格・
    ADR4/D-4 = grant-set 一般化 + `Route.trusted` 削除・
    ADR6 = 採否確定 + :225-229 falsify・D-3 = B5 除去・
    TS-4/5/6 = #168 merge 無条件化・presence 拒否)。

- Rule: **front-matter backlink 規約 — `related` のみ**(spec
  R6・ADR D3・issue AC4・design VN4 — `git diff` + grep)
  - front-matter を持つ全対象 doc(ADR 10・design 8・spec 6・
    test-spec 6)の `relations.related` へ backlink 4 件追加:
    本 spec(`../specs/documentation-honesty-sweep-and-breaking-
    change-migration-guide.md`)・本 ADR(`../adr/documentation-
    honesty-sweep-and-migration-guide.md`)・migration guide
    (`../guides/migration.md`)・本 design(`docs/designs/` 配下
    は `./documentation-honesty-sweep-and-migration-guide.md`・
    他 dir は `../designs/…`)。
  - 新規 `refines:`/`refined-by:`/`supersedes:`/`superseded-by:`
    edge は全対象 doc の差分内に存在しない — `git diff` で
    front-matter 差分を検査し該当 key の新規出現が 0 である
    こと。superseded status 変更もなし。

- Rule: **off-list 追加 scope の定点検証**(design VN (d-2) —
  grep 7 項目)
  - (a) test-spec 6 file 全てに `issue #169` 注記がある —
    `grep -l "issue #169" docs/test-specs/{policy-gated-command-
    tool,invocation-scoped-command-grants,managed-process-task-
    authorization,route-resolution,substitute-exec-route-mode,
    route-grant-set-projection}.md` が 6 件を返す。
  - (b) `defer-non-exec-routing…` ADR の末尾注記が :225-229 の
    deny-list 恒久 unroutable 主張を訂正 — `grep -n "no-grant\
    |unroutable" <file>` が注記節内に hit。
  - (c) `specs/policy-gated-command-tool.md` :82/:104 が
    writeScope+command fail-closed を「決定済み」として記述し
    Deferred 体裁を脱している — 当該行に `Deferred` が残存
    しない。
  - (d) `specs/invocation-scoped-command-grants.md` :115/:173
    が `RuntimePolicy.exec`・`inner.exec`/`exec: None` へ
    rename — `RuntimePolicy.command`・`inner.command` の残存
    なし(file-level `command.grants` shape 言及は残ってよい)。
  - (e) `designs/overview.md`・`designs/workspace-filesystem-
    boundary-sandboxed-tools.md`・`specs/cheaper-v1-worker-
    runtime-cli.md` が canonical file tool surface 限定へ
    補正済み — `grep -n "Refine 注記\|issue #169"` で各 file
    hit。
  - (f) `cheaper-tool-operate/SKILL.md` :82-85・`usage.md`
    :207 の deny-list 言及が opt-in 条件付き —
    `grep -n "execDenyPatterns" <file>` で当該箇所に条件付記。
  - (g) off-list ADR 3 件 + off-list design 5 件(うち spec は
    cheaper-v1 — `## Refine 注記` 節あり:`managed-process-
    named-task-lifecycle`・`invocation-scoped-command-grants-
    as-a-synthetic-…`・`opt-in-soft-gating-…`・`managed-process-
    canonical-tool`・`invocation-scoped-command-grants`)。

- Rule: **migration guide 構成契約**(spec R7・ADR D4・issue
  AC5・design VN5 — presence-grep + 構成確認)
  - `docs/guides/migration.md` が存在し front matter 無しの
    guide 規約(`release.md`・`distribution.md` 同形)に従う。
  - 冒頭に「前提: cheaper が強制するもの・しないもの」節
    (canonical disclaimer 全 3 項目)+ `#163`〜`#168` の 6
    issue 節があり、各節が「変更内容/利用者影響/修復・移行
    手順」の 3 小節を持つ。
  - 収録項は design §(f) 表どおり: #163 = 内容審査 7 項目除去
    (受理側挙動変化)、#164 = builtin deny 削除 → opt-in
    `security.execDenyPatterns` + seed 例(旧 regex verbatim・
    分類別 seed・shell 名 predicate は別追加の注記)、#165 =
    unified `exec` grant namespace(loader 写像 + alias 存続・
    名衝突 `InvalidRuntimePolicy`・`fixed`/`scope` CLI spec
    拒否・project task = `scope:"project"`)、#166 =
    `grants:[name,…]` trusted 層限定 + `kind:"route"` record
    `{command, executed}` split(`executed.argv` 平文注意・
    additive field)、#167 = `substitute:{grant,argv}` trusted
    層限定 + exit は「実行した command」の exit、#168 =
    project 層 `trusted` key presence 拒否(load 全滅・hook
    fail-open で `route resolve` まで不可視・修復 = key 削除
    1 行 or `<userConfigDir>/` vendor)。
  - `docs/README.md` User guides リストに `migration.md` の
    行がある(`managed-process.md`・`command.md` の後・
    `workflow-usage.md` の前あたり)。

- Rule: **文書のみ差分 + index 整合 + full pipeline**(spec R8・
  issue AC(文書のみ)・design VN 末尾・ADR Confirmation)
  - 差分が `docs/**/*.md`・`packages/apm/.apm/skills/*/SKILL.md`
    のみで code・fixture・test 変更なし — `git diff` の対象
    file 一覧で確認。
  - `node scripts/check-doc-ids.mjs` pass。
  - `audit_docs` が本 chain 新規 doc で error 0。
  - README index 行の status 整合: `docs/designs/README.md`
    の本 design row + `docs/specs/README.md`(SPEC=…)・
    `docs/adr/README.md`(ADR=…)・`docs/discovery/README.md`
    (DISC=resolved)が front-matter status と整合。
  - `mise run check` exits 0(biome・fmt・clippy・version・
    doc-ids・cargo test・node --test・lockfile・apm-payload
    — 文書のみ差分だが canonical pipeline で閉じる)。

## Examples

- Example: `grep -n "OS sandbox" docs/guides/migration.md
  docs/guides/command.md docs/guides/managed-process.md` が 3
  file とも hit し、`command.md` は trust/sandbox 限界節冒頭が
  canonical 形へ強化・ENOEXEC bullet 追加済み(issue AC1・
  design VN1/VN3)。
- Example: `grep -n "native magic（MZ\|shell metachar 拒否\|
  workspace 相対 path は拒否" docs/guides/command.md` が 0
  hit、`grep -n "native PE/ELF/Mach-O\|shell 演算子、script
  shim を拒否" docs/guides/managed-process.md` が 0 hit
  (spec R3・design VN2)。
- Example: `docs/adr/command-grant-trust-boundary-and-managed-
  process-reuse.md` の末尾に `## Refine 注記（issue #169）`
  節があり、「#163 で内容審査 7 項目が mandatory 層から除去・
  #164 で opt-in `security.execDenyPatterns` へ降格・Option A
  の grant 境界は維持」を含み governing link 3 件がある
  (ADR D3・design §(c) ADR1)。
- Example: `docs/specs/invocation-scoped-command-grants.md`
  :115 が `RuntimePolicy.exec`・:173 が `inner.exec`/
  `exec: None` を記述し、`grep -n "RuntimePolicy.command\|
  inner.command"` が残存しない — file-level `command.grants`
  言及は残る(表現 drift 境界・design VN(d-2)(d))。
- Example: `docs/test-specs/substitute-exec-route-mode.md` の
  merge 節・Guarantees・Risk の各 cluster 末尾に `> （issue
  #169 注記 — …）` blockquote があり、file-end `## Refine
  注記` 節は**ない**(§(d-2) placement convention)。
- Example: `git diff` で対象 doc の front-matter に
  `related:` への 4 link 追加のみがあり、`refines:`/
  `refined-by:`/`supersedes:`/`superseded-by:` の新規行が 0
  (spec R6・commit 9d76d40 規約)。
- Example: `docs/guides/migration.md` が `#163`〜`#168` の
  6 節を持ち、#168 節に「hook 経路 fail-open で症状が
  `route resolve` まで不可視」・「修復 = key 削除 1 行 or
  `<userConfigDir>/` へ vendor」がある(spec R7・design
  VN5)。
- Example: `node scripts/check-doc-ids.mjs` exits 0 +
  `git status` で差分 file が docs/*.md・SKILL.md のみ
  (spec R8)。

## Guarantees

- 実施者が design を再解釈しなくても、canonical disclaimer
  配置・guides 訂正・18 doc の refine 注記・12 doc の in-place
  訂正/annotation・migration guide 構成・backlink 規約・
  文書のみ差分の全項目が grep / `git diff` / doc-id check で
  機械的に検証できる(design Verification Notes が全項 grep
  可能形)。
- 「提供しない保護」を誇張する記述が guides・refine 対象
  doc から機械的に除かれ(absence-grep)、代わりに canonical
  文言が 3 file 所定節に揃う — issue AC1-AC3。
- ADR/design/test-spec の本文が保持されたまま訂正が施される
  ことが注記節/annotation blockquote の存在 + `git diff` の
  追加主体差分で確認できる(履歴非改竄 — ADR D3)。
- cross-chain lineage edge が 0 であることが front-matter
  差分検査で保たれ、9d76d40 の復旧事案を反復しない。
- migration guide が #163–#168 の全収録項を「変更内容→
  利用者影響→修復手順」で列挙し、利用者が症状から修復へ
  辿れる単一入口になる(issue AC5)。
- 差分が文書のみであることが `git diff` file 一覧で pin され、
  code・fixture・test への副作用がない(spec R8)。

## Non-goals

- code・fixture・test の変更検証(文書のみ差分 — cargo test
  の対象挙動変更なし。`mise run check` は pipeline 全体の
  無改訂 pass を確認するのみ)。
- 16 対象 + §(d-2) 一覧外 doc の stale 捕捉(以後の発見分は
  継続 sweep — spec Deferred Design Concerns)。
- Option E exec-less route・OS-sandbox backed mode の検証
  (親 chain open/deferred)。
- migration guide の継続運用方式・version 番号紐付け
  (release 時の責務 — 現時点は issue 番号起点)。
- impl/ir・plans・tasks・handoff・過去 discovery 等 history
  記録の訂正(対象外 — spec Out of Scope)。
- 検証コマンド・実行結果の記録 — task T5 の `## Verification`
  と impl record の責務。

## Risk

- per-doc 表の「訂正方向」を超えた書換で維持要件(trusted
  boundary・pin・argv pattern・lifecycle)を消すと、living
  contract としての spec が本来の契約を失う — absence-grep
  だけでは検出できないため、`git diff` の行単位確認と
  per-doc 表の「維持する項目」転記が防衛線。
- test-spec へ file-end `## Refine 注記` 節を設けると
  §(d-2) placement convention 違反 —「6 file に `issue
  #169` 注記がある」だけでは配置が検証できないため、
  blockquote 形式・file-end 節非存在の目視確認が必要。
- `related` backlink の path 規約違反(`./` vs `../`)は
  link 切れ・audit warning の温床 — §(b) の dir 別規約と
  `check-doc-ids`/`audit_docs` で捕捉。
- 既存の正直な記述(`command.md:63/:148`・`managed-process.
  md:47/:49` 等)を canonical 化の名の下に書き潰すと
  「重複・矛盾しない」規約違反 — G-9 変更不要一覧と
  `git diff` の対象行確認が必要。
- 「doc が正しく実装が誤り」の code 側不整合を doc 訂正で
  隠すと実装バグを文書で追認する — flag → follow-up issue
  機構(design §Data and Control Flow)を編集者が遵守する
  必要がある。
- migration guide の収録項欠落・小節抜けは presence-grep で
  検出できるが、各節内容の正確性(seed 例の verbatim・修復
  手順の正しさ)は design §(f) 表との照合レビューが必要。
