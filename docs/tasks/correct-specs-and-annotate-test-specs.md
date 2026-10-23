---
id: "TASK-034L7LJt4ZNXV0S6zO47ND"
type: "task"
status: "done"
title: "documentation honesty sweep — spec in-place 訂正 + test-spec annotation (12 doc)"
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
        path: "docs/specs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/specs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/specs/managed-process-canonical-tool.md"
      - type: "file"
        path: "docs/specs/workflow-resource-inference.md"
      - type: "file"
        path: "docs/specs/cheaper-v1-worker-runtime-cli.md"
      - type: "file"
        path: "docs/test-specs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/test-specs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/test-specs/managed-process-task-authorization.md"
      - type: "file"
        path: "docs/test-specs/route-resolution.md"
      - type: "file"
        path: "docs/test-specs/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/test-specs/route-grant-set-projection.md"
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
    - "docs/impl/ir/trusted-route-flag-issuer-redesign.md"
    - "docs/guides/workers.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep — spec in-place 訂正 + test-spec annotation (12 doc)

## Work

plan `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
(PLAN-034L7LkYl6qzs5eZ9mEJZL)の T3。design
(DESIGN-034Lrgl6KUIjrLaU6BBL2U)§(d) spec/design 9 件 per-doc
訂正表(うち spec 5 件)+ §(d-2) off-list(cheaper-v1 spec +
test-spec 6 件・placement convention)に従い、12 doc へ
living-contract 側の訂正を適用する。

**共通編集契約**:

- **spec(6 doc)**: living contract として要件本文を
  **in-place** で現在の真実へ訂正し、訂正行へ `（issue
  #16X — …）` の inline 注釈を付す(route spec R1/R12/R15/
  R34 の #166–#168 先例)。design 表の「訂正方向」列を転記 —
  「維持する項目」を削減しない。
- **test-spec(6 doc)**: Rule/Example/Guarantee 本文は保持し、
  該当 bullet/Example 行の直下へ `> （issue #169 注記 —
  <現行の真実>）` の blockquote を挿入する in-place
  annotation(**file-end `## Refine 注記` 節は設けない** —
  §(d-2) placement convention)。同一 falsified 機構を共有
  する連続 bullet は cluster 末尾へ 1 本に集約可(TS-5 のみ
  実用)。
- **backlink**: 全 12 doc の `relations.related` へ 4 件追加 —
  `./documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (specs 配下)または `../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (test-specs 配下)・`../adr/documentation-honesty-sweep-and-migration-guide.md`・
  `../guides/migration.md`・`../designs/documentation-honesty-sweep-and-migration-guide.md`。
  `refines`/`refined-by`/`supersedes`/`superseded-by` は新規に
  一切張らない。
- deny-list 命名(ADR D2)を全編集へ適用。

### 1. spec 6 件 — in-place 訂正(§(d)/(d-2) の訂正方向を転記)

- [x] **S-1** `docs/specs/policy-gated-command-tool.md`
      — :60(executable 解説 + argv pattern + cwd/env —
      内容審査なし・opt-in deny のみ)・:78(native-magic/
      shell-shim/interpreter/bare-node deny 除去 — pin は
      維持・opt-in `execDenyPatterns` のみ)・:79-:80
      (metachar 拒否除去 — literal argv・残存 bounds は
      `INVALID_INPUT`)・:94 AC4(現行契約へ — opt-in
      前提へ降格)・**:82/:104 Deferred→決定済み訂正**
      (writeScope 宣言 worker が `command`/`!fixed` grant
      surface を expose すると scope 生成が
      `POLICY_VIOLATION` fail-closed — `registry.rs:1167`・
      workers.md:54-67 と整合。`process` task の outputs ⊆
      writeScope coverage tri-state も workers.md:61-67
      の規約を写す)。末尾 `## Refine 注記` 併置推奨
      (差分が大きい)。trusted boundary・pin・argv
      pattern・cwd/env 境界・lifecycle 要件は**維持**
      (:81 OS sandbox 非該当記述は既に正直 — 保持)
- [x] **S-2** `docs/specs/invocation-scoped-command-grants.md`
      — :60/:85/:93/:105/:118/:133/:138(deny 参照を
      `security.execDenyPatterns` 宣言時の条件付き検査へ
      条件化 — 機構は実在・既定 `[]` では非活性・存在
      抹消ではない)・:64(trusted 供給元を builtin +
      user-scope + 合成 invocation layer `--grant` の
      3 源へ — issue #105・ADR-034FlTS3BaiV3WKz8bX1Xv
      注釈)・:70(deny 検査を宣言時のみへ)・:150
      AC3・:153 AC6(同上条件付記)・**:115/:173 rename
      residual**(`RuntimePolicy.command`→`RuntimePolicy.exec`
      — `schema.rs:134`・`inner.command`→`inner.exec`・
      `command: None`→`exec: None` — `workflow_run.rs:
      500-503`・`invocation.rs:277-286`)。file-level
      `command.grants`・`command` section・`command:`
      layer key・invocation layer shape への言及は**訂正
      しない**(表現 drift 境界 — rename は resolved
      struct のみ)
- [x] **S-3** `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
      — R1/R7/R12/R15/R34 は #166–#168 で更新済みのため
      `related` backlink のみ + 残差分確認。:303-311
      (R8 rewrite-safe gate)・:306-307 の `unsafe_token`
      分類は routing candidacy 層の保守的 gate で依然
      強制中(`hook/routing_policy.rs` `tokenize_shell`)
      — **本文訂正不要**。編集時に exec 時 metachar
      拒否を残す記述が見つかった場合のみ R8 との層
      区別を 1 行補記(実施者の確認義務)
- [x] **S-4** `docs/specs/managed-process-canonical-tool.md`
      — :62(入力は task 名のみ — model は command/argv
      を指定できない。task の `command` 配列は
      `fixed:true` grant の literal argv として内容審査
      なしに spawn — issue #163/#165)・:72 AC(unknown
      task・`task` 名以外の input key = `INVALID_INPUT`・
      cwd escape・undeclared output scope を拒否 — 宣言
      `command` 配列の内容審査は行わない)
- [x] **S-5** `docs/specs/workflow-resource-inference.md`
      — :74 item 9(`tool:process` declaration seam を
      unified grant の resource 宣言 — `fixed:true`
      grant の `inputs`/`outputs`・`process.tasks` は
      loader 写像で存続 — へ再配線、「undeclared
      process は INVALID_REQUEST」を fixed grant 前提で
      再記述 — issue #165)
- [x] **off-list** `docs/specs/cheaper-v1-worker-runtime-cli.md`
      — :71/:75/:105/:115 を canonical file tool surface
      (write/edit/ls/read/find/grep の workspace
      boundary・denyPaths・writeScope)へ限定し、
      `command`/`process` の spawn 子プロセスの fs
      副作用は OS sandbox 無しでは強制されない旨を明記
      (disclaimer 項目 1・2 — issue #169)。末尾
      `## Refine 注記` 併置可

### 2. test-spec 6 件 — in-place annotation note(§(d-2))

- [x] **TS-1** `docs/test-specs/policy-gated-command-tool.md`
      — anchor :52/:53/:62/:68。注記: #163 で builtin
      deny set(interpreter/shell/exec-wrapper/loader・
      native magic・workspace-resident・metachar)が
      mandatory 層から除去、#164 で opt-in
      `security.execDenyPatterns` へ再導入。fail-closed
      gate は空 grants・pin 不能で維持、deny 該当は
      宣言時のみ。3 適用点構造(policy resolve・pin・
      spawn 前再検証)は opt-in deny の適用点として
      存続。`node` 例は既定では PATH 解決・受理
- [x] **TS-2** `docs/test-specs/invocation-scoped-command-grants.md`
      — anchor :47/:51/:59/:74/:82/:95。注記: deny
      検査は全 anchor で「`security.execDenyPatterns`
      宣言時のみ(既定 `[]` では非活性 — issue #164)」
      へ条件化。`RuntimePolicy.command`→`.exec` rename
      (#165・`schema.rs:134`)。layer ordering・
      collision・attribution 規約は維持
- [x] **TS-3** `docs/test-specs/managed-process-task-authorization.md`
      — anchor :50。注記: input-level の raw command
      拒否(呼出し input は `task` 名のみ)は維持。
      task 宣言 `command` 配列の内容審査(argv 要素の
      shell 名・metachar・script ext 拒否)は #163 で
      除去 — 宣言要素は literal argv として無審査で渡る
- [x] **TS-4** `docs/test-specs/route-resolution.md`
      — anchor :52-56/:73-74。注記: #168 で
      `Route.trusted` field 削除・merge 無条件化 —
      project 層 route は下位層を **shadow せず**同名
      衝突は常に project 側が負けて `suppressed`
      provenance へ記録(builtin/user 層間 shadow は
      同一 trusted 層の宣言 override として有効)。
      project 層 `trusted` key presence は load 時に
      `InvalidRuntimePolicy` 拒否(`{label}.trusted:
      not declarable in the project layer` — 値不問・
      strict 経路は error・hook 経路は fail-open で
      route arm 不活性)。builtin/user 層の `trusted:`
      は受理されるが inert
- [x] **TS-5** `docs/test-specs/substitute-exec-route-mode.md`
      — anchor :124-126/:174-175/:337-361/:374-378/
      :406-407/:473-478/:513-516/:542-543/:566-571。
      注記: TS-4 と同じ #168 の真実(`Route.trusted`
      削除・presence 拒否・merge 無条件化・
      `suppressed` は「衝突で負けた project route」
      のみを運ぶ semantics へ狭義化・builtin/user
      `trusted:` は inert 受理)。**cluster 化** —
      同一 falsified 機構のため merge 節・Guarantees・
      Risk 各 cluster 末尾へ 1 本ずつでよい
- [x] **TS-6** `docs/test-specs/route-grant-set-projection.md`
      — anchor :150-155。注記: project 層 `trusted`
      key presence は「判別に使わない」ではなく load
      時拒否(#168)・`Route.trusted` 削除・merge
      無条件化。`grants` presence 拒否・宣言層制約の
      他の記述は維持

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (PLAN-034L7LkYl6qzs5eZ9mEJZL — T3)
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U — §(d)/(d-2) が
  AUTHORITATIVE: per-doc anchor・訂正方向・placement
  convention)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT — R5/R6・AC2/AC4)
- `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH — D3 spec in-place 許容・
  D2 deny-list 命名)
- `docs/test-specs/documentation-honesty-sweep-and-migration-guide.md`
  (TSPEC-034L7LgKjsa5dwb8PstQX9 — in-place 訂正・
  annotation・backlink Rules)
- `docs/impl/ir/trusted-route-flag-issuer-redesign.md`
  (#168 IR — TS-4/5/6 の charter 根拠: residual
  follow-up 節が #169 へ defer)

## Dependencies

- なし(T1/T2/T4 と file set が disjoint — 並列可能)

## Affected Files

- `docs/specs/` 6 file(modify — 要件本文 in-place +
  `related` backlink。S-3 は backlink のみ + 確認)
- `docs/test-specs/` 6 file(modify — bullet 直下
  blockquote annotation + `related` backlink。本文保持)

## Done When

- [x] spec 6 doc が「訂正方向」列どおり in-place 訂正
      され、`（issue #16X — …）` inline 注釈が付く
      (S-1 は :82/:104 が Deferred 体裁を脱し
      `registry.rs:1167` fail-closed 記述・末尾注記
      併置。S-3 は backlink + 残差分確認済み)
- [x] S-2 の :115/:173 が `RuntimePolicy.exec`・
      `inner.exec`/`exec: None` へ rename され、
      `RuntimePolicy.command`・`inner.command` の残存
      がない(file-level `command.grants` shape 言及は
      残ってよい)
- [x] test-spec 6 doc の該当 bullet 直下に `> （issue
      #169 注記 — …）` blockquote があり、file-end
      `## Refine 注記` 節が**ない**(`grep -l "issue
      #169" <6 files>` が 6 件 — placement も確認)
- [x] TS-5 の注記が merge 節・Guarantees・Risk の各
      cluster 末尾へ集約されている
- [x] 12 doc 全ての front-matter `related` に backlink
      4 件があり path が §(b) 規約
- [x] 維持要件(S-1 の trusted boundary・pin・argv
      pattern・lifecycle、S-2 の layer ordering・shape、
      spec の AC 構造)が差分で削減されていない
      (`git diff` 行単位確認)
- [x] 新規 `refines:`/`refined-by:`/`supersedes:`/
      `superseded-by:` が差分内に 0 件
- [x] `node scripts/check-doc-ids.mjs` exits 0

## Out of Scope

- spec 要件本文の「訂正方向」列を超えた書換(維持要件の
  削減・新規要件の追加)
- test-spec への file-end `## Refine 注記` 節・本文
  Rule/Example の書換(履歴非改竄 — annotation のみ)
- ADR/design への注記追記(T2)・guides 訂正(T1)・
  migration guide(T4)
- code・fixture・test の変更(spec R8)
- 一覧外の新規 stale 発見への対応(継続 sweep)

## Verification

- [x] §1 の spec 6 doc を編集し `git diff` が訂正対象行
      + `related` 行のみであること(実測を記録)
- [x] §2 の test-spec 6 doc を編集し `grep -l "issue
      #169"` が 6 件・blockquote 配置であること(実測)
- [x] `git diff` で `refines:`/`refined-by:`/`supersedes:`/
      `superseded-by:` の新規出現が 0(実測を記録)
- [x] `node scripts/check-doc-ids.mjs` exits 0(実測を記録)

## 実施記録（issue #169 sweep 実施時）

### spec 6 doc — in-place 訂正

- S-1 `docs/specs/policy-gated-command-tool.md` — :60 実行前検証列挙・:78 pin 記述・:79-:80 argv 記述・:82 writeScope 決定記述・:94 AC4・:104 Deferred 記述を全て訂正方向列どおり in-place 更新。末尾に `## Refine 注記（issue #169）` を併置（design 推奨 — 差分が大きいため）。trusted boundary・pin・argv pattern・cwd/env 境界・lifecycle 要件は維持（:81 OS sandbox 非該当記述も保持）。
- S-2 `docs/specs/invocation-scoped-command-grants.md` — deny 参照を全 anchor（:60/:70/:85/:93/:105/:118/:133/:138/AC3:150/AC6:153）で `security.execDenyPatterns` 宣言時の条件付き検査へ条件化。:64 の trusted 供給元を builtin + user-scope + 合成 invocation layer の 3 源へ訂正。:115 `RuntimePolicy.command`→`RuntimePolicy.exec`・:173 `command: None`→`exec: None`・`inner.command`→`inner.exec` へ rename。file-level `command.grants`・`command` section・invocation layer shape の言及は訂正していない（表現 drift 境界）。
- S-3 `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` — `related` backlink のみ。残差分確認: metachar 言及は :204（複合 shell command routing gate）・:307（`unsafe_token` 分類）・:580（rewrite-safe gate AC）のいずれも routing candidacy 層（R8）で、exec 時 metachar 拒否を残す記述は無し — 本文訂正不要の判断を確認。
- S-4 `docs/specs/managed-process-canonical-tool.md` — :62 要件・:72 AC を訂正方向列どおり in-place 更新（task 名のみ入力・`fixed:true` grant の literal argv・内容審査なし）。
- S-5 `docs/specs/workflow-resource-inference.md` — :74 item 9 を unified grant の resource 宣言（`fixed:true` grant `inputs`/`outputs`・`process.tasks` は loader 写像で存続）へ再配線し issue #165 注釈を付記。
- off-list `docs/specs/cheaper-v1-worker-runtime-cli.md` — :71（In Scope の filesystem boundary 記述）・:75（Runtime Invariants 列挙）へ inline 注釈で canonical file tool surface への限定を明記。:105/:115 の AC 表現は file tool 面では現行とも整合するため本文を保持し、末尾 `## Refine 注記（issue #169）` で 4 anchor 全ての scope 限定を記述（併置可の規約に基づく）。

### test-spec 6 doc — in-place annotation note

- TS-1 `docs/test-specs/policy-gated-command-tool.md` — 3 本（:52-53 cluster 末尾・:62・:68）。
- TS-2 `docs/test-specs/invocation-scoped-command-grants.md` — 12 本（design anchor :47/:51/:59/:74/:82/:95 + 同一 falsified 機構の residual :64/:69/:71/:75/:103/:118）。
- TS-3 `docs/test-specs/managed-process-task-authorization.md` — 1 本（Rule bullet — design anchor :50、実測は :54）。
- TS-4 `docs/test-specs/route-resolution.md` — 2 本（:52-56 cluster 末尾・:73-74）。
- TS-5 `docs/test-specs/substitute-exec-route-mode.md` — 9 本（design anchor 全 range: Feature :124-126・Rule :174-175・merge suppressed cluster :337-361・:374-378・:406-407・Example :473-478・Guarantee :513-516・Non-goal :542-543・Risk :566-571 — cluster 規約で 3 本への集約も許容されたが、全 anchor range への直下配置を選択）。
- TS-6 `docs/test-specs/route-grant-set-projection.md` — 1 本（:150-155 cluster 末尾）。
- 全 28 本が `> （issue #169 注記 — …）` blockquote 形。file-end `## Refine 注記` 節は test-spec には一切無し（`grep -n "^## Refine" docs/test-specs/*.md` → 0 hit）。

### 検証（実測）

- `grep -c "issue #169 注記" docs/test-specs/<6 files>` → 3/12/1/2/9/1（全 6 file で注記存在）。
- `grep -n "RuntimePolicy\.command\|inner\.command\|command: None" docs/specs/*.md` → rename 残余 0 hit（file-level `command.grants` 等の有効表現を除く）。
- `git diff -- docs/specs docs/test-specs | grep` 新規 `refines:`/`refined-by:`/`supersedes:`/`superseded-by:` 非空値 → 0 件。
- `node scripts/check-doc-ids.mjs` → exits 0（doc id check OK: 1191 markdown files scanned）。
- `git status --short` → 変更は対象 12 file のみ（spec 6 + test-spec 6）。

### 偏差・判断記録

- TS-2: design anchor 6 件に加え、同一 falsified 機構（deny 無条件列挙・#165 rename 残余）を持つ residual 6 箇所（:64/:69/:71/:75/:103/:118）へ同形注記を追加 — spec Out of Scope 既定機構（一覧外の同型 stale への軽微注記）に基づく。
- TS-3: design anchor `:50` は drift — 実測では対象 Rule bullet は :54 に位置。claim text で特定して注記を配置。
- TS-5: cluster 規約は「merge 節・Guarantees・Risk 各 cluster 末尾へ 1 本ずつでよい」と最小 3 本を許容したが、anchor 網羅のため全 9 range へ直下配置した（過剰側の偏差 — bookkeeping sweep の介入最小性との trade-off で網羅性を優先）。
- S-5: design anchor は item 9 のみだが、同一 declaration seam を参照する AC9 にも `（fixed:true grant の inputs/outputs — issue #165）` の軽微な注釈を付記。
- cheaper-v1 spec: design は :71/:75/:105/:115 の in-place 訂正を指定。:105/:115 の AC 文は file tool 面の真実として維持できるため inline 書換を最小化し、末尾注記で 4 anchor の scope 限定を一括記述（併置可の規約内の判断）。
