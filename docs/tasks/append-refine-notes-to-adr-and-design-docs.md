---
id: "TASK-034L7LDZKCG5pF2MsoQOyC"
type: "task"
status: "done"
title: "documentation honesty sweep — ADR + design への Refine 注記追記 (18 doc)"
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
        path: "docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md"
      - type: "file"
        path: "docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
      - type: "file"
        path: "docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
      - type: "file"
        path: "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
      - type: "file"
        path: "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
      - type: "file"
        path: "docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
      - type: "file"
        path: "docs/adr/managed-process-named-task-lifecycle.md"
      - type: "file"
        path: "docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
      - type: "file"
        path: "docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
      - type: "file"
        path: "docs/designs/policy-gated-command-tool.md"
      - type: "file"
        path: "docs/designs/workflow-resource-inference.md"
      - type: "file"
        path: "docs/designs/rust-rig-runtime-migration-architecture.md"
      - type: "file"
        path: "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
      - type: "file"
        path: "docs/designs/managed-process-canonical-tool.md"
      - type: "file"
        path: "docs/designs/invocation-scoped-command-grants.md"
      - type: "file"
        path: "docs/designs/workspace-filesystem-boundary-sandboxed-tools.md"
      - type: "file"
        path: "docs/designs/overview.md"
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
  defers: []
  deferred-by: []
---

# documentation honesty sweep — ADR + design への Refine 注記追記 (18 doc)

## Work

plan `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
(PLAN-034L7LkYl6qzs5eZ9mEJZL)の T2。design
(DESIGN-034Lrgl6KUIjrLaU6BBL2U)§(b) refine 注記規約・§(c) ADR
7 件 per-doc 表・§(d) design 4 件 per-doc 表・§(d-2) off-list
表に従い、18 doc へ file-end `## Refine 注記（issue #169）`
節追記 + `related` backlink 4 件を追加する。

**共通編集契約**(全 18 doc):

- **注記配置**: ファイル末尾・既存の最終節の**直後**に
  `## Refine 注記（issue #169）` 節を **1 つ**追記(最終節名は
  doc ごとに異なる — `More Information`・`Verification`・
  `更新履歴`・`Confirmation`・`Coordinator review amendments`
  等。「最終節の後」が規則であって特定節名の後ではない)。
- **注記形状**(§(b) canonical 形): 冒頭に「権限モデル再設計
  (#151 分解 #163–#168)により、本 doc の以下の記述は現行の
  真実と異なる。<ADR/design は決定時点の記録として本文は
  保持する>」+ anchor ごとの箇条書き(`<file:line または節>`
  <旧記述の要旨> → <現行の真実>(#<issue>))+ governing
  link(ADR-034L6ihibvbjmSnKmSffWH・SPEC-034L6i3XmfCaWRHAiw8TTT
  R5・migration guide への link — いずれも各 doc の配置 dir
  からの相対 path で書く(ADR 配下なら `../guides/migration.md`
  等))。複数 anchor でも節は 1 つに集約。
- **本文保持**: ADR/design の本文は一切変更しない(履歴非改竄
  — 注記追記のみ)。supersede・superseded status 変更なし。
- **backlink**: front-matter `relations.related` へ 4 件追加 —
  `../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`・
  `../adr/documentation-honesty-sweep-and-migration-guide.md`・
  `../guides/migration.md`・本 design。本 design への path は
  `docs/adr/` 配下が `../designs/documentation-honesty-sweep-and-migration-guide.md`、
  `docs/designs/` 配下が `./documentation-honesty-sweep-and-migration-guide.md`。
  `refines`/`refined-by`/`supersedes`/`superseded-by` は新規に
  一切張らない(commit 9d76d40 規約・spec R6)。
- deny-list 命名(ADR D2)を注記本文へ適用。

### 1. ADR 7 件(§(c) per-doc 表 — anchor・注記内容を転記)

- [x] **ADR1** `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md`
      — anchor :60/:69/:90/:116/:135。注記: #163 で内容審査
      7 項目(native magic・shell/shim・script ext・workspace-
      resident・bare `node`・call-time/task argv metachar)が
      mandatory 層から除去、#164 で opt-in
      `security.execDenyPatterns`(deterministic executable
      deny-list・既定 `[]`)へ降格。trusted-layer-only の
      grant 境界(Option A)は**維持**・本 ADR の核心的決定と
      して依然 authoritative
- [x] **ADR2** `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md`
      — anchor :96。注記: invariants の適用範囲は cheaper
      所有の canonical surface + 実行契約(file tool の
      workspace boundary・denyPaths・宣言制約・lifecycle)に
      限る。spawn 子プロセスの fs 副作用は OS sandbox 無しで
      は強制不能(disclaimer 項目 1・2)。3 層 capability
      モデルは維持
- [x] **ADR3** `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md`
      — anchor title・:51・:55。注記:「sandboxed tool」は
      canonical file tool の workspace boundary 強制を指し、
      spawn 子プロセスの fs 副作用には及ばない(disclaimer
      項目 1・2 が governing)。決定自体(boundary 強制を
      cheaper 側 tool 実装に置く)は有効
- [x] **ADR4** `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
      — anchor :181-190(D2)・:280-317(D7)・:441/:450/:465・
      :589-591/:635/:671。注記: #166 で C9 single-grant
      投影は route-declared grant-set 投影(`grants:[name,…]`
      — trusted 層限定)へ一般化、#167 で `substitute:
      {grant,argv}` mode 追加。#168 で `Route.trusted` は
      **削除**・trust は宣言層 provenance(builtin/user =
      trusted・project は shadow しない)— project 層
      `trusted` key presence は `InvalidRuntimePolicy` で
      拒否(値不問)。D2(routing ≠ authorization)・層解決順
      は維持。:709-710 の cross-chain `refines` 禁止規約は
      本 sweep でも継続適用
- [x] **ADR5** `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
      — 軽微 1-2 行注記: 決定(approvalBypass trusted
      gating)は #163–#168 と整合。#163 内容審査除去・#168
      `trusted` flag 再設計で周辺表現は変わったが approval
      bypass は trusted 層 opt-in のまま変更なし
- [x] **ADR6** `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md`
      — anchor :175-209・:225-229。注記: 採否は親 design で
      確定済み — grant-set 投影**採用**(#166)・viability
      **条件付き弱化**(#166 — byte-matched argv precheck
      維持・set 内非 exec grant は存在 + pin のみ)・usage
      record `{command, executed}` split **採用**(#166)・
      substitute-exec mode **採用**(#167)・workflow-declared
      grants **不採用**・Option E exec-less は引き続き
      **open**。:225-229 の「deny-list が python/ssh/tar 等の
      grant を恒久拒否し routing mode 不問で `no-grant`
      unroutable」も #163/#164 で falsify 済み — builtin
      deny-list は除去され deny は opt-in 宣言時のみ効くため
      既定ではこれら executable も grant 宣言・pin 可能なら
      route-eligible(`no-grant` は deny 該当ではなく viability
      結果)。`command_denied_executable_re`・`command.rs:
      160-189` 参照自体も機構除去済みの stale コード参照
- [x] **ADR7** `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md`
      — anchor :185。注記: #165 で `process.tasks` は
      unified `exec` grant namespace の `fixed:true` grant
      へ loader 写像・`tool:"process"` は alias 存続
      (`command` grant は `fixed:false`)。「shell 解釈なし
      で直接 spawn」は**維持** — 正確には spawn API 層の
      shell-free で、metachar を含む argv は拒否ではなく
      executable の literal argv(#163)。POSIX +X shebang
      無しの ENOEXEC に libc が `/bin/sh` retry するのは
      OS/libc 既定動作(glibc `execvpe` 系・musl 残留差分)
      で cheaper の保証対象外(disclaimer 項目 3 —
      §(a) 配置表の必須項)

### 2. off-list ADR 3 件(§(d-2) — 同型で file-end 注記 + backlink)

- [x] `docs/adr/managed-process-named-task-lifecycle.md`
      — anchor :59。注記: #163 で内容審査除去(argv
      metachar/shell は literal argv、shim は win32
      viability 制約、bare `node` は PATH 解決)・
      `execPath` fast-path 不実装
- [x] `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
      — anchor :62/:83/:89/:92。注記: deny は
      `security.execDenyPatterns` 宣言時のみ・
      `RuntimePolicy.command`→`.exec` rename(#165 —
      file-level `command.grants` shape は有効のまま)
- [x] `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`
      — anchor :95。注記: mandatory 内容審査前提(native
      magic / shell / shim 拒否・`minimal_env`)の軽微訂正
      — #163 除去・#164 opt-in 降格(既存 opportunistic
      項目の正式化)

### 3. design 4 件(§(d) per-doc 表 — anchor・注記内容を転記)

- [x] **D-1** `docs/designs/policy-gated-command-tool.md`
      — anchor :104/:113(§(a))・:119-123(§(b))・:139-158
      (§(d))・:209(§(j))・:259/:300。注記: deny list 部分
      (native magic・shell/shim・script ext・interpreter
      deny・workspace-resident・metachar check)は #163 で
      mandatory 層から除去・#164 で opt-in
      `execDenyPatterns` へ降格。**supersede ではない** —
      pin 契約(B1/B2)・argv pattern 言語(A6)・Exclusive
      root manifest(C7)・route-context 注入・trusted-layer
      境界は引き続き authoritative
- [x] **D-2** `docs/designs/workflow-resource-inference.md`
      — anchor :57/:65/:89。注記: `process` declaration の
      inference 経路(task inputs/outputs→produces/writes)
      は #165 unified grant resource 宣言(`fixed:true`
      grant)前提へ置き換わる。**注記は `## Coordinator
      review amendments` の後に配置**(当該節が最終節)
- [x] **D-3** `docs/designs/rust-rig-runtime-migration-architecture.md`
      — anchor :291-293。注記: B5(bare `node` reject)は
      #163 で除去。bare `node` は通常の PATH 解決を通り
      spawn viability(realpath・is_file・X_OK)で受理
      (`proc/executable.rs:6-8` 明記)。`execPath` fast-path
      非存在の設計背景は記録として保持
- [x] **D-4** `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md`
      — anchor :301-310・:398-429・:609・:1034-1036。注記:
      #166 で viability は**条件付き弱化**(byte-matched
      argv precheck 維持・grant-set member は存在 +
      eligibility + pin のみ)+ `grants:[name,…]` 投影
      一般化。#167 で substitute mode の検査対象 retarget。
      denied executable・workspace residency・metachar の
      事前 check は #163 で除去(deny は opt-in 宣言時
      のみ)。`unsafe-command` 分類は caller 報告 flag +
      cwd-changing executable の 2 条件のみ。#168 で
      `Route.trusted` 削除・merge 無条件化・trust は宣言層
      provenance — project 層 `trusted` presence は
      `InvalidRuntimePolicy` 拒否

### 4. off-list design 4 件(§(d-2) — 同型で file-end 注記 + backlink)

- [x] `docs/designs/managed-process-canonical-tool.md`
      — anchor :57/:85/:88。注記: #163 で task argv の
      内容審査除去 — 宣言 `command` 配列要素は literal
      argv として無審査で渡る(metachar・shell 名を含ん
      でも拒否されない)。win32 `.cmd`/`.bat`/`.ps1` 非対応
      は CreateProcess viability 制約として残るが deny
      機構ではない
- [x] `docs/designs/invocation-scoped-command-grants.md`
      — anchor :74/:99/:173/:229/:327/:341。注記: deny は
      `execDenyPatterns` 宣言時のみ・code 参照は
      `policy/exec.rs`/`security.rs`・`RuntimePolicy.exec`/
      `inner.exec`/`exec: None` 系へ rename
- [x] `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md`
      — anchor :44-56/:85-91。注記:「sandboxed tool」は
      canonical file tool の workspace boundary 強制のみ。
      #101 以降の `process`・`command` exec surface は
      外部プロセスを spawn し fs 副作用は journal/
      write/edit ops を通らない(全域 snapshot 不要論は
      file tool に限定 — disclaimer 項目 1・2)。file tool
      層の外部バイナリ非依存(rg/fd 回避)は維持
- [x] `docs/designs/overview.md`
      — anchor :54-67/:79-88。注記: (a)「sandboxed
      tools」・「workspace 外アクセス不許可」invariant
      は canonical file tool surface に限定 —
      `command`/`process` 子プロセスの fs 副作用は
      OS sandbox 無しでは強制・完全観測されない
      (#163-168・disclaimer 項目 1・2)。(b)
      `packages/*`→`crates/*` の Rust 移行 drift は
      rust-rig-runtime-migration chain で記録済みの別系統
      — 本 sweep では最小の言及に留める

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/documentation-honesty-sweep-and-migration-guide.md`
  (PLAN-034L7LkYl6qzs5eZ9mEJZL — T2)
- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U — §(b)/(c)/(d)/(d-2) が
  AUTHORITATIVE: canonical 注記形・per-doc anchor・注記内容)
- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT — R5/R6・AC4)
- `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH — D3 refine-only 訂正方式・
  D2 deny-list 命名)
- `docs/test-specs/documentation-honesty-sweep-and-migration-guide.md`
  (TSPEC-034L7LgKjsa5dwb8PstQX9 — refine 反映・backlink
  規約 Rules)

## Dependencies

- なし(T1/T3/T4 と file set が disjoint — 並列可能)

## Affected Files

- `docs/adr/` 10 file(modify — 末尾 `## Refine 注記` 節
  追記 + `related` backlink のみ。本文・既存節は無改訂)
- `docs/designs/` 8 file(同上)

## Done When

- [x] 18 doc 全てに最終節直後の `## Refine 注記（issue
      #169）` 節が 1 つあり、canonical 形の必須要素
      (issue #169・anchor ごとの旧→新 bullet・governing
      link 3 件)を含む(`grep -l "Refine 注記" <18 files>`
      が 18 件)
- [x] 各 doc の注記内容が design §(c)/(d)/(d-2) の注記
      要点どおり(ADR7 は ENOEXEC 区別・ADR6 は :225-229
      falsify・D-2 は `## Coordinator review amendments`
      の後に配置)
- [x] 18 doc 全ての front-matter `related` に backlink
      4 件があり path が §(b) 規約(adr 配下 = `../designs/
      …`・designs 配下 = `./…`)
- [x] 本文・既存節が無改訂(`git diff` で注記節追記 +
      `related` 行追加のみの差分 — 履歴非改竄)
- [x] 新規 `refines:`/`refined-by:`/`supersedes:`/
      `superseded-by:` が差分内に 0 件
- [x] `node scripts/check-doc-ids.mjs` exits 0

## Out of Scope

- 対象 doc の本文・既存節の編集(履歴非改竄 — 注記追記のみ)
- spec in-place 訂正・test-spec annotation(T3)・guides
  訂正(T1)・migration guide(T4)
- supersede・superseded status 変更(ADR D3 — 一切行わない)
- code・fixture・test の変更(spec R8)
- 18 doc 一覧外の新規 stale 発見への対応(継続 sweep)

## Verification

- [x] §1-§4 の全 doc を編集し `grep -l "Refine 注記"` が
      18 件を返す(実測を記録)
- [x] `git diff --stat` で 18 file の差分が注記節追記 +
      `related` 行のみであること(実測を記録)
- [x] `git diff` で `refines:`/`refined-by:`/`supersedes:`/
      `superseded-by:` の新規出現が 0(実測を記録)
- [x] `node scripts/check-doc-ids.mjs` exits 0(実測を記録)

## 実施記録（issue #169 sweep 実施時）

- `grep -l "Refine 注記" <18 files>` → 18 件を返す（10 ADR +
  8 design 全件）。全注記はファイル末尾・既存最終節の直後に配置
  （D-2 `workflow-resource-inference.md` は `## Coordinator review
  amendments` の後 — 実測で末尾節であることを確認）。
- 各注記は canonical 形（issue #169 の前文 + anchor ごとの旧→新
  bullet + governing link 3 件）で、design §(c)/(d)/(d-2) の注記
  要点を転記: ADR7 は ENOEXEC/`/bin/sh` retry の OS/libc 区別を
  含み、ADR6 は :225-229 の `no-grant`/unroutable 主張 falsify を
  注記節内に含む（`grep -n "no-grant\|unroutable"` で注記節 hit）。
- `related` backlink 4 件を全 18 doc へ追加（adr 配下は本 design
  へ `../designs/…`・designs 配下は `./…` の path 規約）。うち
  `managed-process-named-task-lifecycle.md`・`workflow-resource-
  inference.md`・`managed-process-canonical-tool.md` は既存
  `related: []` からの新規 populated list。
- `git diff` で `refines:`/`refined-by:`/`supersedes:`/
  `superseded-by:` の新規出現は 0 件。本文・既存節は無改訂
  （注記節追記 + `related` 行追加のみ）。
- `mise exec -- node scripts/check-doc-ids.mjs` → exits 0。
