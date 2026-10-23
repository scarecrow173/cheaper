---
id: "TASK-034L7LKoGPWs0X67ASTfsm"
type: "task"
status: "done"
title: "documentation honesty sweep — migration.md 新設 + README index"
created: "2026-10-24"
updated: "2026-10-24"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
  changes:
    added:
      - type: "file"
        path: "docs/guides/migration.md"
    modified:
      - type: "file"
        path: "docs/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  implemented-by: []
  depends-on:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  blocks:
    - "docs/tasks/verify-documentation-honesty-sweep.md"
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
    - "docs/adr/exec-contract-content-audit-removal.md"
    - "docs/adr/opt-in-exec-deny-patterns.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/adr/route-grant-set-projection.md"
    - "docs/adr/substitute-exec-route-mode.md"
    - "docs/adr/trusted-route-flag-issuer-redesign.md"
    - "docs/guides/release.md"
    - "docs/guides/distribution.md"
    - "docs/README.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep — migration.md 新設 + README index

## Work

plan `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
(PLAN-034L7LkYl6qzs5eZ9mEJZL)の T4。design
(DESIGN-034Lrgl6KUIjrLaU6BBL2U)§(f) migration guide 構成契約・
§(a) canonical disclaimer 配置表に従い、`docs/guides/migration.md`
を新設し `docs/README.md` User guides index へ row を追加する。
front matter 無しの guide 規約(`release.md`・`distribution.md`
同形)。記述は issue 番号起点 — version 番号への紐付けは release
時の責務。

### 1. `docs/guides/migration.md` 新設(§(f) 構成契約)

- [x] **構成**: `# Migration guide` 見出し + 1-2 行の導入
      (cheaper の破壊的変更と移行手順を issue 番号起点で
      列挙 — 現在の main に適用される変更群)+ 前提節 +
      #163–#168 の 6 issue 節(各節「変更内容 / 利用者影響 /
      修復・移行手順」の 3 小節構成)
- [x] **前提節「前提: cheaper が強制するもの・しない
      もの」**: canonical disclaimer(ADR D1)の全 3 項目を
      配置 — (1)「cheaper は OS sandbox ではない」
      (2)「canonical file tool(write/edit/ls/read/find/
      grep)の workspace boundary(E1/E2)・`filesystem.
      denyPaths`・worker `writeScope` は cheaper 所有の
      tool surface で強制されるが、spawn した子プロセスの
      fs 副作用は OS sandbox 無しでは強制・完全観測でき
      ない」(3)「cheaper の spawn は shell を介さず argv
      を直接渡す(metachar は literal argv)。POSIX +
      shebang 無し +X file の ENOEXEC に libc が `/bin/sh`
      で retry するのは OS/libc 既定動作(glibc `execvpe`
      系・musl は retry しない残留差分)であり cheaper の
      保証対象外」
- [x] **#163 節**: 内容審査 7 項目除去(A8 shell/shim・B6
      script ext・B3 native magic・B4 workspace-resident・
      B5 bare `node`・A10 `argv_shell_re` metachar —
      call-time + task 宣言 argv の両適用点・F3 process
      task の native/shim 検査)。利用者影響: 従来拒否の
      宣言 exec(python/ssh/tar/.sh/workspace-resident
      binary/bare `node`/metachar argv)が受理される挙動
      変化。修復: 拒否近似が必要なら #164 の
      `security.execDenyPatterns` を user `policy.yaml` へ
      宣言(seed は次節)
- [x] **#164 節**: builtin executable deny list 削除 →
      opt-in `security.execDenyPatterns`(deterministic
      executable deny-list・既定 `[]` — ADR D2 命名)。
      利用者影響: 旧 mandatory deny は無指定では効かない。
      修復: seed 例を提示 — `command.md` :78 の旧 regex
      verbatim・:84-98 の分類別 seed・旧 shell 名
      predicate(`sh|bash|…`)は別機構除去のため別途
      pattern 追加が必要な旨の注記(`command.md`:100)
- [x] **#165 節**: `command.grants`/`process.tasks` →
      単一 `exec` grant namespace。`process` 節・
      `tool:"process"` は loader 写像 + alias で存続。
      名衝突(`command.grants` ∩ `process.tasks`)は
      `InvalidRuntimePolicy` + `details.{grant,task}` fail
      closed。`--grant`/`policy grant add` CLI spec の
      `fixed:true`・明示 `scope` は `InvalidRequest` 拒否。
      project task = `scope:"project"`(route 不可視)。
      利用者影響: authoring の section 分離は維持・grant
      名空間は共有。修復: 名衝突時は一方を rename・
      `fixed`/`scope` を含む CLI spec は除去(必要なら
      trusted YAML で直接 authoring)
- [x] **#166 節**: route `grants:[name,…]`(trusted 層
      限定の set 投影)+ `kind:"route"` record の
      `{command, executed}` split。利用者影響:
      `executed.argv` は平文で記録される注意・project
      層 `grants` key presence は `InvalidRuntimePolicy`
      で route load 全滅(record schema は additive —
      旧 reader は `executed` を無視して集計可)。修復:
      project routes.yaml の `grants` key 除去・
      `executed` を含む record を扱う集計系は additive
      field として読み替え
- [x] **#167 節**: route `substitute:{grant,argv}`
      (trusted 層限定)+ exit は「実行した command」の
      exit(substitute mode では親 agent が substitute
      の exit を原 command のものとして解釈 —
      semantic substitution を trusted opt-in で受容)。
      利用者影響: 新機構 — 既存 route 影響なし。修復:
      なし(新機能)。project 層 `substitute` key
      presence は `grants` と同じく
      `InvalidRuntimePolicy` で拒否される旨の注意
- [x] **#168 節**: project 層 routes.yaml の `trusted`
      key presence を `InvalidRuntimePolicy` で拒否
      (`true`/`false`/非 bool 値不問)。利用者影響:
      `trusted` を含む project routes.yaml は **load
      全滅** — strict 経路(`route resolve`/`route run`)
      は exit 3、hook 経路は fail-open でその workspace
      の全層 routing が不活性化。**症状は debug log の
      みに現れ `route resolve` を実行するまで不可視**。
      修復: (a) 当該 key の削除 1 行(shadow を意図して
      いないならこれで完了)、または (b) routes.yaml を
      `<userConfigDir>/` へ vendor して user 層 route 化
      (`trusted:true` は user 層で inert 受理・ゼロ編集
      で shadowing 復活)。hook 経路の fail-open 症状は
      `route resolve --command '…'` で診断
- [x] deny-list 命名規約(ADR D2)を全節へ適用 —「保護」
      「sandbox」「containment」「防御」とは呼ばない

### 2. `docs/README.md` — User guides index row

- [x] User guides リストへ `migration.md` の行を追加 —
      位置は `managed-process.md`・`command.md` の後・
      `workflow-usage.md` の前あたり。内容説明は
      「#163–#168 の破壊的変更・症状別修復手順」系の
      1-2 行(手動 index 規約 — `release.md` 先例)

### 3. `docs/designs/README.md` — status 整合確認のみ

- [x] 本 design row(:42・DESIGN-034Lrgl6KUIjrLaU6BBL2U)
      が front-matter status(`approved`)と整合している
      ことを確認 — 編集不要なら無改訂(row 追加・status
      追随済みのため)

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (PLAN-034L7LkYl6qzs5eZ9mEJZL — T4)
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U — §(f) 構成契約・収録項
  表が AUTHORITATIVE)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT — R7/R8・AC5)
- `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH — D1 canonical 文言正本・
  D4 置き場所・D2 命名)
- `docs/test-specs/documentation-honesty-sweep-and-migration-guide.md`
  (TSPEC-034L7LgKjsa5dwb8PstQX9 — migration guide 構成
  Rule)
- 破壊的変更の実装側 ADR(告知材料):
  `docs/adr/exec-contract-content-audit-removal.md`(#163)・
  `docs/adr/opt-in-exec-deny-patterns.md`(#164)・
  `docs/adr/unified-exec-grant-model.md`(#165)・
  `docs/adr/route-grant-set-projection.md`(#166)・
  `docs/adr/substitute-exec-route-mode.md`(#167)・
  `docs/adr/trusted-route-flag-issuer-redesign.md`(#168)

## Dependencies

- なし(T1/T2/T3 と file set が disjoint — 並列可能。
  migration.md の内容は §(f) で固定済みで sweep 訂正の
  結果に依存しない。index row は migration.md と同 commit)

## Affected Files

- `docs/guides/migration.md`(add — §(f) 構成)
- `docs/README.md`(modify — User guides index row 1 行)
- `docs/designs/README.md`(確認のみ — 整合なら無改訂)

## Done When

- [x] `docs/guides/migration.md` が存在し front matter
      無し・preamble(canonical disclaimer 全 3 項目)+
      `#163`〜`#168` の 6 節 × 「変更内容/利用者影響/
      修復・移行手順」3 小節の構成である
- [x] 各節が design §(f) 収録項表の必須項を含む(#164
      seed 例・#165 名衝突/`fixed`/`scope` 拒否・#166
      `executed.argv` 平文注意 + additive・#168 load
      全滅 + hook fail-open 不可視症状 + 修復 2 経路)
- [x] `docs/README.md` User guides に `migration.md`
      の行がある
- [x] `docs/designs/README.md` の本 design row が
      status `approved` と整合(編集不要なら無改訂)
- [x] `node scripts/check-doc-ids.mjs` exits 0

## Out of Scope

- guides・skill の stale 訂正(T1)・ADR/design/spec/
  test-spec への訂正反映(T2/T3)
- version 番号・release への紐付け(issue 番号起点 —
  release 時の責務)
- CHANGELOG.md の新設(ADR O8 不採用)
- migration guide の継続運用方式の決定(次の破壊的変更
  発生時点 — spec Deferred)
- code・fixture・test の変更(spec R8)

## Verification

- [x] `ls docs/guides/migration.md` が存在を返す +
      `grep -c "^## #" docs/guides/migration.md` が 6
      節を返す(実測を記録)
- [x] `grep -n "OS sandbox\|ENOEXEC\|execDenyPatterns"
      docs/guides/migration.md` が前提節・#164 節で hit
      (実測を記録)
- [x] `grep -n "migration" docs/README.md` で index row
      を確認(実測を記録)
- [x] `node scripts/check-doc-ids.mjs` exits 0(実測を記録)

## 実施記録（issue #169 sweep 実施時）

- `docs/guides/migration.md` 新設 — `# Migration guide` + 2 行導入（issue 番号起点・現在の main に適用される変更群）+「前提: cheaper が強制するもの・しないもの」節（canonical disclaimer 全 3 項目:7-9）+ #163–#168 の 6 issue 節・各 3 小節構成。front matter 無し（release.md・distribution.md 同形）。
- #163 節: 内容審査 7 項目除去（A8/B6/B3/B4/B5/A10 両適用点/F3）+ spawn viability のみの現行解決 + 既定では拒否不発の挙動変化 + #164 への pointer。
- #164 節: builtin deny 削除 → opt-in `security.execDenyPatterns`（deterministic executable deny-list・既定 `[]` — ADR D2 命名、「保護」「sandbox」「containment」「防御」の語は不使用）。`command.md` より旧 regex verbatim 単一行 + 分類別 seed を転記、旧 shell 名 predicate（`sh|bash|…`）は別機構除去のため別途 pattern 追加が必要な注記も転記。
- #165 節: unified `exec` grant namespace・loader 写像存続・名衝突 `InvalidRuntimePolicy` + `details.{grant,task}`・CLI spec の `fixed`/`scope` は `InvalidRequest`・project task = `scope:"project"`。修復: rename + `fixed`/`scope` key 除去。
- #166 節: trusted 層限定 `grants:[name,…]` set 投影 + `{command, executed}` record split・`executed.argv` 平文注意・project `grants` presence は load 全滅・additive field の読み替え案内。
- #167 節: `substitute:{grant,argv}` trusted 層限定・exit mirror・semantic substitution 受容。新機構のため修復なし + project `substitute` presence 拒否の注意。
- #168 節: project 層 `trusted` presence 値不問拒否（`{label}.trusted: not declarable in the project layer`）・load 全滅（strict exit 3 / hook fail-open）・debug log のみの不可視症状・2 修復経路（key 削除 / user 層 vendor）・`route resolve --command '…'` 診断。
- `docs/README.md` User guides index: :27-28 に `migration.md` row 追加（`command.md` 直後・`workflow-usage.md` 直前 — 指定位置どおり）。
- `docs/designs/README.md` :42 の本 design row は `approved` で front-matter status と整合 — 無改訂（確認のみの項目）。

### 検証（実測）

- `ls docs/guides/migration.md` → 存在。
- `grep -c "^## #" docs/guides/migration.md` → 6（#163–#168 節）。
- `grep -c "^### " docs/guides/migration.md` → 18（6 issue × 3 小節）。
- `grep -n "OS sandbox\|ENOEXEC" docs/guides/migration.md` → :7-9 で前提節 3 項目 hit・`execDenyPatterns` は 6 hit。
- `grep -n "migration" docs/README.md` → :27 で index row 確認。
- `node scripts/check-doc-ids.mjs` → exits 0（doc id check OK: 1191 markdown files scanned）。
