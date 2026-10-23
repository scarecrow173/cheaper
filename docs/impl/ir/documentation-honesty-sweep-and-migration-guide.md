---
id: "IMPL-034L8CAW785uSRtJvzJh15"
type: "impl"
status: "completed"
title: "documentation honesty sweep and migration guide"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
  changes:
    added:
      - type: "file"
        path: "docs/guides/migration.md"
        note: "#163-#168 破壊的変更の issue 別 migration guide 新設 — preamble(canonical disclaimer 全 3 項目)+ 6 節 × 「変更内容/利用者影響/修復・移行手順」3 小節(design §(f) 構成契約どおり)"
    modified:
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
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
      - type: "file"
        path: "docs/README.md"
      - type: "file"
        path: "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
      - type: "file"
        path: "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
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
        path: "docs/adr/README.md"
      - type: "file"
        path: "docs/plans/README.md"
      - type: "file"
        path: "docs/specs/README.md"
      - type: "file"
        path: "docs/tasks/README.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/correct-guides-and-skills-honesty-sweep.md"
    - "docs/tasks/append-refine-notes-to-adr-and-design-docs.md"
    - "docs/tasks/correct-specs-and-annotate-test-specs.md"
    - "docs/tasks/create-migration-guide-and-index.md"
    - "docs/tasks/verify-documentation-honesty-sweep.md"
  implemented-by: []
  depends-on:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "docs/impl/ir/opt-in-exec-deny-patterns-issue-164.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/route-grant-set-projection.md"
    - "docs/impl/ir/substitute-exec-route-mode.md"
    - "docs/impl/ir/trusted-route-flag-issuer-redesign.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/documentation-honesty-sweep-and-migration-guide.md"
  references:
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/guides/migration.md"
    - "docs/guides/command.md"
    - "docs/guides/managed-process.md"
    - "docs/guides/workers.md"
    - "docs/guides/usage.md"
    - "docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md"
    - "docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "docs/adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
    - "docs/adr/managed-process-named-task-lifecycle.md"
    - "docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "docs/designs/policy-gated-command-tool.md"
    - "docs/designs/workflow-resource-inference.md"
    - "docs/designs/rust-rig-runtime-migration-architecture.md"
    - "docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "docs/designs/managed-process-canonical-tool.md"
    - "docs/designs/invocation-scoped-command-grants.md"
    - "docs/designs/workspace-filesystem-boundary-sandboxed-tools.md"
    - "docs/designs/overview.md"
    - "docs/specs/policy-gated-command-tool.md"
    - "docs/specs/invocation-scoped-command-grants.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/specs/managed-process-canonical-tool.md"
    - "docs/specs/workflow-resource-inference.md"
    - "docs/specs/cheaper-v1-worker-runtime-cli.md"
    - "docs/test-specs/policy-gated-command-tool.md"
    - "docs/test-specs/invocation-scoped-command-grants.md"
    - "docs/test-specs/managed-process-task-authorization.md"
    - "docs/test-specs/route-resolution.md"
    - "docs/test-specs/substitute-exec-route-mode.md"
    - "docs/test-specs/route-grant-set-projection.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# documentation honesty sweep and migration guide

## Summary

issue #169(#151 実装分解の最終 slice 7/7)—「claim ≤ 実強制」の
documentation honesty sweep + 破壊的変更 migration guide を docs
のみの差分で実装した(50 files・code/fixture/test 変更 0 —
SPEC-034L6i3XmfCaWRHAiw8TTT R8・ADR-034L6ihibvbjmSnKmSffWH
D3 refine-only 訂正方式)。#163-#168 の exec-contract 再設計で
陳腐化した文書主張を、canonical disclaimer・deny-list 命名規約・
refine 注記規約(DESIGN-034Lrgl6KUIjrLaU6BBL2U §(a)/(b) が固定)へ
揃え、#163-#168 の利用者向け移行手順を
`docs/guides/migration.md` へ集約した。実装内容:

1. guides + 配布 skills の in-place 訂正(T1・`04fca29`): G-1〜G-8 +
   off-list 2 件 — `command.md` workspace 相対 executable 受理
   (resolve_workspace_path + pin)の現行真実への訂正(P0 falsified
   clause 除去)・metachar 拒否除去・trust/sandbox 限界節 canonical
   化・`managed-process.md` 宣言 argv 内容審査なし化・native
   PE/ELF/Mach-O 限定除去 + ENOEXEC 注記・`workers.md` writeScope
   但書・`usage.md` + `cheaper-usage`/`cheaper-tool-operate`
   SKILL.md の deny-list opt-in 条件付記。`execDenyPatterns` を
   「deterministic executable deny-list」命名規約で統一(ADR D2 —
   「保護」「sandbox」「containment」「防御」呼称 0 件)。
2. ADR/design 18 doc への `## Refine 注記（issue #169）` file-end
   追記 + `related` backlink(T2・`684c012`): ADR1-7 + off-list
   ADR 3・design D-1〜D-4 + off-list design 4。履歴非改竄 —
   本文・決定記録は書き換えず注記で補正。
3. spec 6 doc in-place 訂正 + test-spec 6 doc in-place annotation
   blockquote(T3・`1f65f0d`): S-1 writeScope×`command`
   fail-closed「決定済み」訂正・S-2 `RuntimePolicy.exec`/
   `inner.exec` rename 残余解消・S-3/S-4/S-5 軽微訂正・off-list
   `cheaper-v1` canonical file tool surface 限定。TS-1〜TS-6 は
   対象 bullet/Example 直下の blockquote(file-end 節ではない
   placement convention — #168 IR deferral の回収含む)。
4. `docs/guides/migration.md` 新設 + `docs/README.md` index(T4・
   `2ba98b0`): preamble(canonical disclaimer 全 3 項目 — OS
   sandbox 否定・file tool 強制面・spawn 子 process 観測限界 +
   spawn shell-free vs ENOEXEC→`/bin/sh` libc fallback 区別)+
   #163-#168 6 節 × 「変更内容/利用者影響/修復・移行手順」。
5. 最終検証 + status 昇格(T5・`946d555`): spec AC1-AC5 ↔ TSPEC
   Rule 対応・design VN1-VN5 + (d-2)(a)-(g) 全 grep 実測・
   `check-doc-ids` 1192 files・`audit_docs` error 0・
   `mise run check` exit 0・task=done・plan=completed・
   spec=approved・ADR=accepted 昇格 + README index 追随。

## Context

TASK-034L7LD2Gc1Num3wT45UMY(T1)・TASK-034L7LDZKCG5pF2MsoQOyC
(T2)・TASK-034L7LJt4ZNXV0S6zO47ND(T3)・TASK-034L7LKoGPWs0X67ASTfsm
(T4)・TASK-034L7LPMgAWmjPhefil8Dn(T5)。chain は
DISC-034L67agpaqLudLiaNI822(resolved — claim vs 実強制の実測
inventory・掃き出し表現一覧・migration guide 収録項の正本)→
SPEC-034L6i3XmfCaWRHAiw8TTT(approved — R1-R8・AC1-AC5)→
ADR-034L6ihibvbjmSnKmSffWH(accepted — D1 canonical disclaimer
文言正本・D2 deny-list 命名・D3 refine-only 訂正方式・D4
migration guide 置き場所)→ DESIGN-034Lrgl6KUIjrLaU6BBL2U
(approved — per-doc 訂正計画・refine 注記規約・migration guide
構成・Verification Notes)→ PLAN-034L7LkYl6qzs5eZ9mEJZL
(completed)→ TSPEC-034L7LgKjsa5dwb8PstQX9(approved — 検証
契約 Rules)→ 5 task 全 done。

本 IR は `implements` を 5 task、`depends-on`+`derives-from` を
plan/design、`verified-by` を test-spec、`related` を discovery
+ sibling IR(#163-#168 の 6 chain IR — 本 sweep が訂正した主張の
実装側記録)へ閉じ、cross-chain doc(訂正対象の ADR/design/spec/
test-spec・file-issue task・親 chain spec・guides)は
`references` でのみ参照する — lineage edge(refines)は持たない
(sweep 対象 doc への新規 `refines`/`refined-by`/`supersedes`/
`superseded-by` は graph lineage 保護規約で禁止・T5 `git diff`
実測で新規 0 件確認済み)。

実装は design §(c)/(d)/(d-2)/(e)/(f) の per-doc anchor・旧/新
記述・訂正方式の転記(plan 規約 — 文言・配置・訂正方式を再解釈
しない)で、design からの deviation はない。実施差分は
`git diff 9a919eb..946d555` で 50 files(`docs/**/*.md` 48 +
`packages/apm/.apm/skills/*/SKILL.md` 2 — 1 file 新設
`docs/guides/migration.md`、code・fixture・test 0)。

## Implementation

### Changes

- **guides 4 file(T1)**: `docs/guides/command.md` — G-1(:44)
  「workspace 相対 path は拒否」clause を **#163 で falsify 済み
  として除去**し、workspace 相対 executable 受理
  (resolve_workspace_path + pin)・absolute・bare-name 3 形態受理・
  内容審査なし・唯一の executable 審査 = opt-in
  `security.execDenyPatterns` の現行真実へ訂正(P0 blocker —
  初期 design の訂正文自体が stale で adversarial review が検出、
  `2a69389` で修正)。G-2(:133)shell metachar 拒否除去(input
  schema `INVALID_INPUT` bound・literal argv)。G-5(:141-153)
  「trust / sandbox 限界(必読)」節を canonical 化 — 節冒頭の
  「cheaper は OS sandbox ではない」強化(:143)+ `execDenyPatterns`
  deny-list bullet + spawn shell-free / ENOEXEC→`/bin/sh` libc
  fallback 区別 bullet(:152)+ out of scope 行(:154)。
  `docs/guides/managed-process.md` — G-3(:23)宣言 argv 内容審査
  なし化・G-4(:27)native PE/ELF/Mach-O 限定除去 + ENOEXEC 注記・
  G-6(:47-49)caveat 節 canonical 化(「cheaper は OS sandbox では
  ない」:47)+ :27 cross-reference。`docs/guides/workers.md` —
  G-7(:138-139)writeScope 但書を「canonical file tool の write
  に限る」へ限定。`docs/guides/usage.md` — (d-2) :207 deny list
  へ `(opt-in — security.execDenyPatterns 宣言時のみ)` 条件付記。
  既存の正直な記述(`command.md:63/:148`・`managed-process.md:47/
  :49`・`parent-agent-integration.md`・`configuration.md` — G-9
  変更不要一覧)は差分に含めず維持。
- **配布 skills 2 file(T1)**:
  `packages/apm/.apm/skills/cheaper-usage/SKILL.md`(:353 deny-list
  条件付記 — G-8)・`cheaper-tool-operate/SKILL.md`(:82-85 deny-list
  無条件前提の条件付記化 — (d-2) off-list)。front matter 無しの
  配布物のため backlink 規約対象外。
- **ADR 10 file + design 8 file(T2 — file-end `## Refine 注記
  （issue #169）` 1 節追記 + `related` backlink 4 件/doc)**: ADR1
  `command-grant-trust-boundary-and-managed-process-reuse`(#163
  除去・#164 opt-in 降格・Option A 境界維持)・ADR2 `worker-
  definition-runtime-policy-runtime-invariants-capability`(:96
  invariants 適用範囲を canonical surface + 実行契約へ限定)・
  ADR3 `workspace-filesystem-boundary-cheaper-sandboxed-tool`
  (「sandboxed tool」/security boundary を canonical file tool
  限定)・ADR4 `pretooluse-auto-routing-…`(grant-set 投影一般化・
  `Route.trusted` 削除・宣言層 provenance — :181-190/:280-317 等
  8 anchor)・ADR5 `per-target-hook-rewrite-enablement-…`(軽微
  注記)・ADR6 `defer-non-exec-routing-…`(:175-209 deferred
  sketch 採否確定 + **:225-229 deny-list 恒久 unroutable 主張の
  falsify 済み訂正** — builtin deny 除去 → opt-in `execDenyPatterns`
  記述)・ADR7 `fix-tool-runtime-…-upfront`(:185 unified grant
  namespace・literal argv・ENOEXEC 区別)。off-list ADR 3 —
  `managed-process-named-task-lifecycle`(:59)・`invocation-scoped-
  command-grants-as-a-synthetic-trusted-policy-layer`(:62/:83/
  :89/:92 deny 無条件列挙 + rename 残余)・`opt-in-soft-gating-…`
  (:95)。design D-1 `policy-gated-command-tool`(:104/:113/:119-123
  等 deny list 部分のみ注記 — pin B1/B2・argv A6・C7 manifest は
  authoritative 継続)・D-2 `workflow-resource-inference`(:57/:65/
  :89 — `## Coordinator review amendments` の**後**配置規則の
  唯一の例外ケース)・D-3 `rust-rig-runtime-migration-architecture`
  (:291-293 B5 bare `node` reject 除去)・D-4 `pretooluse-route-
  resolver-…`(viability 条件付き弱化・deny 除去・provenance
  trust)。off-list design 4 — `managed-process-canonical-tool`・
  `invocation-scoped-command-grants`・`workspace-filesystem-
  boundary-sandboxed-tools`・`overview`。
- **spec 6 file(T3 — 要件本文 in-place + `（issue #16X — …）`
  inline 注釈)**: S-1 `policy-gated-command-tool`(:60/:78/:79-80/
  :94 + **:82/:104 Deferred→決定済み訂正** — writeScope×`command`
  fail-closed を `registry.rs:1167` 確定挙動として記述)・S-2
  `invocation-scoped-command-grants`(deny 条件化・3 源 trusted +
  **:115/:173 `RuntimePolicy.exec`/`inner.exec`/`exec: None`
  rename 残余解消** — file-level `command.grants` shape 言及は
  表現 drift 境界として維持)・S-3 `pretooluse-route-resolver-…`
  (backlink のみ + 残差分確認 — R8 routing candidacy 層は訂正
  不要と確定)・S-4 `managed-process-canonical-tool`(:62/:72)・
  S-5 `workflow-resource-inference`(:74 item 9 unified grant
  resource 宣言へ再配線)・off-list `cheaper-v1-worker-runtime-cli`
  (:71/:75/:105/:115 boundary/AC 主張を canonical file tool
  surface へ限定 + 末尾注記)。
- **test-spec 6 file(T3 — 対象 bullet/Example 直下の in-place
  annotation blockquote・file-end 節は設けない)**: TS-1
  `policy-gated-command-tool`(:52/:53/:62/:68 — builtin deny
  除去・opt-in `execDenyPatterns`・3 適用点存続)・TS-2
  `invocation-scoped-command-grants`(:47/:51/:59/:74/:82/:95 —
  deny 条件化 + rename)・TS-3 `managed-process-task-authorization`
  (:50 — input-level 拒否維持・宣言 argv 内容審査除去)・TS-4
  `route-resolution`(:52-56/:73-74 — #168 `Route.trusted` 削除・
  merge 無条件化・presence 拒否・`suppressed` 狭義化 — **#168
  IR「test-spec stale bookkeeping sweep」deferral の回収**)・
  TS-5 `substitute-exec-route-mode`(:124-126/:174-175/:337-361 等
  cluster 化 — merge 節・Guarantees・Risk 各末尾へ 1 本ずつ)・
  TS-6 `route-grant-set-projection`(:150-155 project `trusted`
  presence 拒否の未反映分)。
- **`docs/guides/migration.md` 新設(T4)**: front matter 無し・
  preamble「前提: cheaper が強制するもの・しないもの」(canonical
  disclaimer 全 3 項目 — :7-9)+ #163(内容審査 7 項目の除去 —
  A8/B6/B3/B4/B5/A10/**F3 process task native/shim 検査** の内訳
  ラベル修正済み)・#164(builtin deny list 削除 → opt-in
  `security.execDenyPatterns` + 旧 regex verbatim seed + 分類別
  分解 seed)・#165(unified exec grant model)・#166(route
  grant-set 投影)・#167(substitute-exec route mode)・#168
  (trusted flag issuer redesign — project 層 `trusted` presence
  拒否・zero-edit vendor migration 経路)の 6 節 × 「変更内容/
  利用者影響/修復・移行手順」3 小節。`docs/README.md:27` User
  guides へ index row 追加。
- **status 昇格 + index 追随(T5)**: 5 task=`done`・plan=`completed`・
  spec=`approved`・ADR=`accepted`(#168 chain 慣行 — spec AC
  checkbox・plan task checkbox は spec chain 側管理のため未チェック
  のまま)。`docs/adr|plans|specs|tasks/README.md` index row 追随
  (`docs/designs/README.md` は approved 済みで無改訂・
  `docs/test-specs/README.md`・`docs/discovery/README.md` も整合
  確認のみで差分なし — 差分一覧の 50 file と一致)。

### Decisions During Implementation

- **refine-only 訂正方式(ADR D3 — 履歴非改竄)** — ADR/design/
  test-spec の本文・Rule/Example は決定時点の記録として書き換えず、
  file-end `## Refine 注記（issue #169）` 節・spec 要件本文の
  in-place 訂正 + inline 注釈・test-spec の対象 bullet 直下
  blockquote の 3 方式を doc 種別で使い分け(design §(b) canonical
  形)。注記は「当該 anchor の記述は issue #16X で変更済み」の
  補正であり原文は保持 — audit trail を壊さない。
- **falsified claim は注記ではなく除去(P0 — G-1)** — adversarial
  reviewer が検出した最大 blocker。「workspace 相対 executable は
  拒否」clause は #163 post-実装の現行挙動(resolve_workspace_path
  + pin で受理)と矛盾する falsified claim であり、注記で補足する
  方式では claim ≤ 実強制 の charter 自身を違反するため、当該
  clause を guide 本文から除去して現行真実(3 形態受理・内容審査
  なし)へ in-place 訂正した。design の初版訂正文自体が旧記述を
  引き継いでおり `2a69389` で訂正表を現行真実へ修正済み。
- **sweep inventory の off-list 拡大(P1)** — adversarial が指摘した
  同型 residual を inventory へ formalize: ADR6 :225-229(deny-list
  恒久 unroutable 主張)・S-1 :82/:104(writeScope+command
  fail-closed の Deferred 体裁)・S-2 :115/:173(`RuntimePolicy.
  command`→`.exec` rename 残余)を訂正表へ追加し、§(d-2) として
  off-list 同型 residual 9 件(ADR 3・spec 1・design 4・skill 1)
  + #168 IR deferral の test-spec 3 file + deny 系 test-spec 3
  file を per-doc anchor・訂正方向・配置まで固定。test-spec を
  charter 内として in-place annotation blockquote placement
  convention を明文化。
- **deny-list 命名規約の sweep 全体適用(ADR D2)** —
  `security.execDenyPatterns` は全編集箇所で「deterministic
  executable deny-list」と呼び、「保護」「sandbox」「containment」
  「防御」語彙での呼称は差分内 0 件(T5 grep 実測)。opt-in 条件
  (`宣言時のみ`)の付記を deny-list 言及の全点(`usage.md`・
  2 SKILL.md・migration guide・各注記)へ均一適用。
- **F3 ラベル修正** — migration guide #163 節の F3 を「process
  task の native/shim 検査」へ修正(task-argv metachar は A10 —
  adversarial P2 → `2a69389` で design 側を先に修正、T4 が
  反映済み本文を転記)。
- **`related` backlink のみ・lineage edge 新設なし** — sweep 対象
  front-matter doc への link は `related` への 4 件 backlink
  (sweep spec・sweep ADR・migration guide・sweep design —
  §(b) path 規約: `docs/designs/` 配下は `./`・他は
  `../designs/`)に限定し、`refines`/`refined-by`/`supersedes`/
  `superseded-by` の新規追加は 0(graph lineage 保護 — T5
  `git diff` grep で実測確認)。

## Related Experiments

### Adopted

なし(known-solution task — design が全対象 doc の anchor・旧/新
記述・注記文言・配置規約を file:line 精度で固定済みのため
Experiment Log は未作成)。

### Rejected

なし。

## Validation

文書検証 + full verification pipeline(2026-10-24 実測 — T5 task
§実施記録どおり、全 pass):

- `mise run check`: exit 0 — biome → `cargo fmt --check` →
  clippy `--workspace --all-targets`(dev-seam)→ version 1.0.0 →
  doc-ids(1192 files)→ `cargo test --workspace` → `node --test`
  → lockfile → apm-payload 全段 pass。**docs-only 差分のため
  cargo 側に変更はなく**、canonical pipeline 全体で閉じた(T5
  §Work — 文書のみ差分だが `mise run check` で閉じる規約)。
- `node scripts/check-doc-ids.mjs`: exit 0 — "doc id check OK:
  1192 markdown files scanned"。
- `node .agents/skills/doc-status/scripts/audit_docs.js --type all
  --json`: exit 0 — 1030 file scan・findings 839 件は全て
  `warning`(既存 repo-wide の inconsistent-reciprocal-relation
  系)で `error` 0 件。本 issue 関連 doc の findings は 15 件で
  全て warning(`related`/`derives-from` の非相互 link —
  blocking ではない、sweep で新設した backlink の既知形状)。
- design Verification Notes 全項目 grep/`git diff` 実測(T5 §2):
  VN1 `grep -n "OS sandbox"` が migration.md・command.md・
  managed-process.md の 3 file hit(`migration.md:7-8`・
  `command.md:143`・`:154`・`managed-process.md:47` で
  canonical 項目 1-2 確認)。VN2 stale absence —
  `command.md`/`managed-process.md`/`workers.md`/`usage.md` の
  「native magic」・マジックバイト審査・metachar/shim/
  interpreter 拒否・workspace 相対 executable 拒否・
  `PE/ELF`/`Mach-O` 全 0 hit。VN3 `grep -n "ENOEXEC"` —
  `command.md:152`・`managed-process.md:27`/`:47`・
  `migration.md:9` + ADR 注記側(`documentation-honesty-…md
  :156`・`fix-tool-runtime-…-upfront.md:239`)で「`/bin/sh`
  retry」「OS/libc 既定動作(glibc `execvpe` 系・musl 残留差分)」
  「保証対象外」の 3 要素確認。VN4 `grep -l "Refine 注記"`
  が ADR+design 20 file(対象 18 + 本 sweep 自身の ADR/design
  の format 定義含有)・各 refine 対象 `related` に backlink
  4 件・`git diff` で prohibited lineage edge 新規 0 hit。
  VN(d-2)(a)-(g) — 6 test-spec の `issue #169` blockquote・
  ADR6 `:233`/`:260` falsify 訂正・S-1 `:82`/`:104` 確定記述・
  S-2 `:119`/`RuntimePolicy.exec`・`:177`/`inner.exec`/`exec: None`
  rename 反映(`RuntimePolicy.command`/`inner.command` 残存 0)・
  (e) canonical file
  tool 限定 3 file・(f) opt-in 条件付記 2 file・(g) off-list
  ADR 3 + design 4 の注記節、全確認。VN5 migration.md 6 節 ×
  3 小節(### 18 個)+ `docs/README.md:27` index row。R8 —
  `git diff 9a919eb..HEAD` + 未追跡の和集合が全て
  `docs/**/*.md`・`SKILL.md` で code・fixture・test 0。
- spec AC ↔ issue AC mapping(T5 §1 で対応確認済み):
  - **issue AC1 → spec AC1**: 「cheaper は OS sandbox ではない」
    + 子プロセス fs 副作用の強制不能が migration.md 前提節
    (:7-9)・`command.md` trust/sandbox 限界節(:143・:154)・
    `managed-process.md` authorization boundary 節(:47)に
    canonical 文言で明記。
  - **issue AC2 → spec AC2**: 「sandbox/boundary/protection」
    表現と実強制の整合が guides 4 file・refine 対象 18 doc・
    spec 6 doc・test-spec 6 doc・off-list doc で処理済み —
    掃き出し inventory 全件処理(注記・in-place・annotation の
    doc 種別別方式 + stale absence-grep 0 hit)。
  - **issue AC3 → spec AC3**: spawn shell-free(argv 直接・
    metachar は literal)と ENOEXEC→`/bin/sh` libc fallback
    (OS/libc 既定動作・保証対象外)の区別が `command.md:152`・
    `managed-process.md:27`/`:47`・`migration.md:9`・ADR7
    注記に記述。
  - **issue AC4 → spec AC4**: 16 対象 doc + off-list 追加の
    front-matter doc 全件へ訂正反映(注記節・in-place・
    blockquote)+ `related` backlink 4 件/doc・prohibited
    lineage edge 新規 0 — front-matter 規約(R6)違反なし。
  - **issue AC5 → spec AC5**: `docs/guides/migration.md` が
    破壊的変更一覧(#163-#168 6 節)+ 移行手順(各節 3 小節)+
    `docs/README.md` index row を持つ。
  - **issue AC(文書のみ)→ spec R8**: 差分 50 file 全て
    `docs/**/*.md`・`packages/apm/.apm/skills/*/SKILL.md` —
    code・fixture・test 0 + `check-doc-ids` pass + README
    status 整合。
- review-protocol: design review — cooperative **ACCEPT WITH
  NON-BLOCKING NOTES**、adversarial **BLOCKED → 全 blocker 解消
  後 validator 8/8 PASS**(`2a69389` — P0: G-1 falsified
  clause(初期訂正文自体が stale)、P1: ADR6 :225-229 +
  S-1 :82/:104 + S-2 :115/:173 の同型 residual 未捕捉 +
  inventory の off-list doc + test-spec 未収録、P2: F3 ラベル
  誤り → §(d-2) 新設・test-spec placement convention・
  backlink path 規約・内部行ラベルの `ADR1〜7` 形統一
  (check-doc-ids legacy-id regex 抵触解消)で
  全解消し `5203cc7` で approved 昇格)。

証跡 commit: planning chain `314d041`(seed discovery)→
`97b05a9`(briefing — DISC resolved + spec/ADR)→ `86725e1`
(design)→ `2a69389`(design fix — adversarial blocker 解消:
stale anchor 修正 + off-list/test-spec へ scope 拡大)→ `5203cc7`
(design approved 昇格 + review fold)→ `9a919eb`(planning —
plan + test-spec + 5 task)→ 実装 `04fca29`(T1 guides/skills)→
`684c012`(T2 ADR/design refine 注記)→ `1f65f0d`(T3 spec
in-place + test-spec annotation)→ `2ba98b0`(T4 migration.md +
index)→ `946d555`(T5 検証記録 + status 昇格 — task=done・
plan=completed・spec=approved・ADR=accepted)。本 IR はその後に
追加する。

## Risks

- **文書主張と実装の残余 drift** — 本 sweep は discovery の
  掃き出し inventory + adversarial 拡大(off-list 9 件 + test-spec
  6 file)までを処理したが、一覧対象外の stale 記述が残存し得る
  (spec Deferred — 発見次第の継続 sweep で回収)。test-spec の
  stale window(TS-4〜TS-6 の #168 未反映分を含む)は本 sweep で
  blockquote annotation 済み — 未捕捉分があれば同方式で追加する。
- **注記の有効期限** — `## Refine 注記`・annotation blockquote は
  「当該 anchor の記述は #16X で変更済み」の pointer であり、
  対象 doc が将来 rewrite された際に注記自身が stale になる
  リスクは refine-only 方式の構造的副作用(履歴非改竄の対価 —
  accepted design trade-off)。
- **migration guide の継続運用** — issue 番号起点で version 番号
  紐付けは release 時の責務(design §(f))。次の破壊的変更
  (route schema `version` bump・Option E 等)の際に同形式で節を
  追記する運用契約は release guide 側の責務 — 本 issue では
  format と初回収録のみ提供。
- **docs-only の検証限界** — grep による presence/absence 検証は
  「文言の存在」を pin するが「記述の正しさ」は目視確認依存
  (T5 §3 で注記本文の design 整合・自相矛盾・canonical 等価・
  命名規約を目視確認済み)。code 側の真実が変われば文書は再度
  stale になり得る — claim ≤ 実強制 は継続的責務。
- **`musl` の ENOEXEC retry 残留差分** — disclaimer が明記する
  「musl は `/bin/sh` retry しない」は OS/libc 側の既知差異で
  cheaper の保証対象外として記録済み — 将来 sandboxed mode
  (§Follow-ups Deferred)を入れる場合に再評価。

## Follow-ups

followup-triage 分類: `followup-terminal` — 本 implementation
chain(issue #169)由来の未分類・未処理の follow-up はない。
下記は全て spec/design「Out of Scope」「Deferred」で scope 外
確定・記録済みの defer、親 chain(#151 分解)由来の open 事項、
または本 sweep で処理済みの閉じた項目であり、新規 chain・新規
issue は起こさない。#151 分解 7/7 の最終 slice として chain
内の残件は全て terminal に整理された。

- **closed — test-spec stale window の bookkeeping sweep** —
  #168 IR が #169 へ移管した `route-resolution.md`・
  `substitute-exec-route-mode.md`・`route-grant-set-projection.md`
  の `trusted: true` shadow・displaced 記録の stale window は
  TS-4〜TS-6 として本 sweep で blockquote annotation 済み
  (deny 系 test-spec 3 file も同方式で処理 — T3)。
- **closed — #163-#168 破壊的変更の利用者告知** — spec AC5 /
  issue AC5 の in-scope として `docs/guides/migration.md` 新設
  + index row で実装済み(T4)。
- **repo-wide residual warning(pre-existing pattern)** —
  `audit_docs` findings 839 件は全て `warning` の既存 repo-wide
  inconsistent-reciprocal-relation 系(error 0 — 本 issue 由来の
  新規 error なし)。本 issue 関連 15 件の `related` 非相互 link
  warning は sweep backlink 規約の既知形状で blocking ではなく、
  spec chain 側管理の契約どおり新規 chain は起こさない。
- **documented-deferred — 一覧対象外 doc の継続 stale sweep** —
  discovery inventory + §(d-2) off-list 一覧の対象外で発見される
  stale 記述は spec Deferred の「発見次第の継続 sweep」責務 —
  本 issue では掃き出し inventory の全件処理で閉じ、網羅性は
  継続責務として残置(spec/T5 Out of Scope と同一)。
- **documented-deferred — code 側不整合の修正** — 「doc が
  正しく実装が誤り」のケース発見時は follow-up issue 化が
  spec 規約。本 sweep で該当ケースは検出されなかった(doc が
  stale 側のケースのみ — 全件文書訂正で処理済み)。
- **documented-deferred — Option E(exec-less route)の採否** —
  親 chain(#151 分解)の open 事項として引き続き残置(#168 IR
  と同一 — `validate_routed_convention` 不変・本 issue scope 外)。
- **documented-deferred — OS-sandbox backed mode** —
  disclaimer が「cheaper は OS sandbox ではない」と明記する
  現行の非 sandbox 構成を変える sandboxed mode は spec/T5
  Out of Scope のまま(宣言の正直性を確保した上で、強制面の
  将来拡張は別設計)。
- **documented-deferred — migration guide の version 紐付け・
  CHANGELOG 運用** — issue 番号起点の収録形式は確立済み。
  release 時の version 番号紐付け・継続運用方式は release
  guide 側の責務(design §(f)・plan Out of Scope と同一)。
- **documented-deferred — builtin/user 層 inert `trusted` key
  の schema 完全削除** — #168 IR の documented-deferred を継続
  継承(routes.yaml `version` bump 時に回収 — #169 sweep では
  `execDenyPatterns`・`trusted` 関連の文書主張のみ訂正し、
  schema 変更はしない — docs-only charter)。
