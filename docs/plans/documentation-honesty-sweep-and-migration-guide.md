---
id: "PLAN-034L7LkYl6qzs5eZ9mEJZL"
type: "plan"
status: "completed"
title: "documentation honesty sweep + migration guide — per-doc 訂正・refine 注記・migration guide 適用計画"
created: "2026-10-24"
updated: "2026-10-24"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/169"
    - "../discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  changes:
    added:
      - type: "file"
        path: "docs/guides/migration.md"
    modified:
      - type: "file"
        path: "docs/README.md"
      - type: "file"
        path: "docs/designs/README.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/managed-process.md"
      - type: "file"
        path: "docs/guides/workers.md"
      - type: "file"
        path: "docs/guides/usage.md"
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
        path: "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
      - type: "file"
        path: "packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
  implemented-by:
    - "docs/tasks/correct-guides-and-skills-honesty-sweep.md"
    - "docs/tasks/append-refine-notes-to-adr-and-design-docs.md"
    - "docs/tasks/correct-specs-and-annotate-test-specs.md"
    - "docs/tasks/create-migration-guide-and-index.md"
    - "docs/tasks/verify-documentation-honesty-sweep.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/designs/trusted-route-flag-issuer-redesign.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "docs/designs/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/adr/documentation-honesty-sweep-and-migration-guide.md"
    - "docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md"
  derived-by:
    - "docs/tasks/correct-guides-and-skills-honesty-sweep.md"
    - "docs/tasks/append-refine-notes-to-adr-and-design-docs.md"
    - "docs/tasks/correct-specs-and-annotate-test-specs.md"
    - "docs/tasks/create-migration-guide-and-index.md"
    - "docs/tasks/verify-documentation-honesty-sweep.md"
  verifies: []
  verified-by:
    - "docs/test-specs/documentation-honesty-sweep-and-migration-guide.md"
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
    - "docs/guides/command.md"
    - "docs/guides/managed-process.md"
    - "docs/guides/workers.md"
    - "docs/guides/usage.md"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/configuration.md"
    - "docs/guides/release.md"
    - "docs/guides/distribution.md"
    - "docs/README.md"
    - "docs/tasks/file-issue-docs-honesty-and-migration-guide.md"
  defers: []
  deferred-by: []
---

# documentation honesty sweep + migration guide — per-doc 訂正・refine 注記・migration guide 適用計画 Implementation Plan

## Goal

DESIGN-034Lrgl6KUIjrLaU6BBL2U の per-doc 訂正契約に従い、権限モデル
再設計(#151)分解の最終 slice(7/7)として **claim ≤ 実強制** の文書
正直性 sweep を適用する。構成は 4 面 + 検証: (1) guides + 配布 skill
の stale 記述を in-place 訂正(G-1〜G-8 + off-list 2 件)し canonical
disclaimer(ADR D1)を所定節へ配置、(2) refine 対象 ADR 7 + off-list
ADR 3・design 4 + off-list design 4 の計 18 doc へ file-end
`## Refine 注記（issue #169）` 節追記 + `related` backlink、
(3) spec 6 doc の要件本文 in-place 訂正 + test-spec 6 doc への
in-place annotation note、(4) `docs/guides/migration.md` 新設
(#163–#168 破壊的変更の issue 別「変更内容→利用者影響→修復手順」)
+ `docs/README.md` index、(5) grep 検証 + doc-id/audit/full check。
全 anchor・訂正方向・注記内容は design §(c)/(d)/(d-2)/(e)/(f) が
file:line 精度で固定済み — 本 plan・task は design の行指定を転記する
だけで、文言・配置・訂正方式を再解釈しない。

## Scope

単一の implementation stream(文書のみ — code・fixture・test 変更
ゼロ、spec R8)。「文書の主張を実強制範囲へ揃える」1 つの編集意図に、
38 file 修正 + 1 file 新設(migration.md)が横断的に付随する構成で
あり、per-doc anchor・旧/新記述・訂正方式は design が固定済み —
分割するのは commit の論理境界(design §Data and Control Flow の
3 塊: guides 訂正 → refine 注記 sweep → migration guide + index)を
訂正方式別に細分した task 境界のみで、plan は 1 つに収める。

| In | Out |
|---|---|
| guides + skills の in-place 訂正 — G-1(`command.md:44`)・G-2(`command.md:133`)・G-3(`managed-process.md:23`)・G-4(`managed-process.md:27`)・G-7(`workers.md:138-139`)・G-8(`cheaper-usage/SKILL.md:353`)・off-list `usage.md:207`・`cheaper-tool-operate/SKILL.md:82-85` | code・fixture・test・`.agents/`・`apm_modules/` 生成物への一切の変更(spec R8 — 文書のみの差分) |
| canonical disclaimer 配置 — `command.md` trust/sandbox 限界節(:141-153)・`managed-process.md` caveat 節(:47-49)・migration.md 前提節(§(a) 全 3 項目) | 既存の正直な記述の書換・削除(`command.md:63/:148`・`managed-process.md:47/:49`・`parent-agent-integration.md:235-258` 等 — 維持・再利用が規約) |
| ADR 10 件(ADR1-7 + off-list 3)・design 8 件(D-1〜D-4 + off-list 4)への file-end `## Refine 注記（issue #169）` 追記 + `related` backlink | ADR/design 本文の書換(履歴非改竄 — 決定時点の記録を保持し注記で補正する方式のみ) |
| spec 6 件(S-1〜S-5 + off-list cheaper-v1)の要件本文 in-place 訂正 + `（issue #16X — …）` inline 注釈 + test-spec 6 件(TS-1〜TS-6)の in-place annotation blockquote | spec の維持要件(trusted boundary・pin・argv pattern・cwd/env 境界・lifecycle)の削減 — 「訂正方向」列の範囲のみ |
| `related` backlink 4 件/spec・ADR・migration guide・本 design への link — §(b) path 規約) | 新規 `refines`/`refined-by`/`supersedes`/`superseded-by` edge・superseded status 変更(graph lineage 保護 — commit 9d76d40 規約) |
| `docs/guides/migration.md` 新設(preamble + #163–#168 6 節 × 3 小節)+ `docs/README.md` User guides row | migration guide の version 番号紐付け(issue 番号起点 — release 時の責務)・CHANGELOG 方式 |
| deny-list 命名規約の sweep 全編集への適用(ADR D2 —「deterministic executable deny-list」・「保護」「sandbox」「containment」「防御」と呼ばない) | 16 対象一覧外 doc の総書換え・本 sweep で未捕捉の一覧外 stale(発見次第の継続 sweep — spec Deferred) |
| | Option E exec-less route・OS-sandbox backed mode(親 chain open/deferred のまま)・release/version bump |

## Upstream Documents

- `docs/specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md`
  (SPEC-034L6i3XmfCaWRHAiw8TTT、proposed — R1-R8・Acceptance
  Criteria AC1-AC5。status 昇格は spec chain 側の管理 — T5 で追随)
- `docs/adr/documentation-honesty-sweep-and-migration-guide.md`
  (ADR-034L6ihibvbjmSnKmSffWH、proposed — D1 canonical disclaimer
  文言正本・D2 deny-list 命名・D3 refine-only 訂正方式・D4
  migration guide 置き場所)
- `docs/discovery/documentation-honesty-sweep-and-breaking-change-migration-guide-issue-169.md`
  (DISC-034L67agpaqLudLiaNI822、resolved — claim vs 実強制の実測
  inventory・掃き出し表現一覧・migration guide 収録項の正本)

## Design Inputs

- `docs/designs/documentation-honesty-sweep-and-migration-guide.md`
  (DESIGN-034Lrgl6KUIjrLaU6BBL2U、approved — AUTHORITATIVE な
  HOW: §(a) canonical disclaimer 文言・配置表、§(b) refine 注記
  canonical 形・配置規則・`related` backlink path 規約、
  §(c) ADR 7 件 per-doc 訂正表、§(d) spec/design 9 件 per-doc
  訂正表、§(d-2) off-list opportunistic 9 件 + test-spec 6 件
  (placement convention — in-place annotation blockquote)、
  §(e) guides 訂正文 shape(G-1〜G-9)、§(f) migration.md 構成
  契約、§(g) churn inventory、Verification Notes(grep 検証
  可能形))。本 plan・task は design の行指定を転記するだけで、
  文言・配置・訂正方式を再解釈しない。

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `docs/guides/command.md` | G-1(:44)executable 受理形訂正(workspace 相対 path 拒否 clause 除去・3 形態受理・内容審査なし・唯一の executable 審査 = opt-in `security.execDenyPatterns`)・G-2(:133)metachar 拒否除去(input schema `INVALID_INPUT` bound・literal argv)・G-5(:141-153)trust/sandbox 限界節の canonical 化(節冒頭 1 文強化 + ENOEXEC 区別 bullet) | T1。deny-list 命名(ADR D2)適用・:63/:148 の既存但書は維持 |
| `docs/guides/managed-process.md` | G-3(:23)宣言 argv 内容審査なし化・G-4(:27)native PE/ELF/Mach-O 限定除去 + ENOEXEC 注記・G-6(:47-49)caveat 節の canonical 化 + :27 への cross-reference | T1 |
| `docs/guides/workers.md` | G-7(:138-139)writeScope 但書(「canonical file tool の write に限る」) | T1。:54-65 tri-state 記述は整合済みで保持 |
| `docs/guides/usage.md` | (d-2) off-list :207「deny list」へ `(opt-in — security.execDenyPatterns 宣言時のみ)` 条件付記 | T1。G-9 optional 項目の採用分 |
| `packages/apm/.apm/skills/cheaper-usage/SKILL.md` | G-8(:353)deny list 条件付記 | T1。配布物 — front matter 無し・backlink 不要 |
| `packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md` | (d-2) off-list :82-85 deny-list 無条件前提を条件付記へ in-place | T1。配布物 |
| `docs/adr/command-grant-trust-boundary-and-managed-process-reuse.md` | ADR1 — :60/:69/:90/:116/:135 anchor の mandatory 内容審査前提記述へ file-end `## Refine 注記`(#163 除去・#164 opt-in 降格・Option A 境界は維持)+ `related` backlink | T2 |
| `docs/adr/worker-definition-runtime-policy-runtime-invariants-capability.md` | ADR2 — :96 invariants 適用範囲の限定注記(canonical surface + 実行契約・3 層モデル維持) | T2 |
| `docs/adr/workspace-filesystem-boundary-cheaper-sandboxed-tool.md` | ADR3 — title・:51・:55 の「sandboxed tool」/security boundary 表現へ canonical file tool 限定注記 | T2 |
| `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md` | ADR4 — :181-190/:280-317/:441/:450/:465/:589-591/:635/:671(grant-set 投影一般化・`Route.trusted` 削除・宣言層 provenance)+ :709-710 cross-chain 規約継続 | T2 |
| `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md` | ADR5 — 軽微 1-2 行注記(approvalBypass gating は整合・周辺表現のみ変化) | T2 |
| `docs/adr/defer-non-exec-routing-to-permission-model-redesign.md` | ADR6 — :175-209(deferred sketch 採否確定)+ :225-229(deny-list 恒久 unroutable 主張の falsify 済み訂正) | T2 |
| `docs/adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md` | ADR7 — :185(unified grant namespace・literal argv・ENOEXEC 区別 — disclaimer 項目 3) | T2 |
| `docs/adr/managed-process-named-task-lifecycle.md` | (d-2) off-list — :59(内容審査・shim deny・execPath fast-path 前提の注記) | T2 |
| `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md` | (d-2) off-list — :62/:83/:89/:92(deny 無条件列挙 + `RuntimePolicy.command` rename 残余) | T2 |
| `docs/adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md` | (d-2) off-list — :95(mandatory 内容審査前提の軽微注記 — 既存 opportunistic 項目の正式化) | T2 |
| `docs/designs/policy-gated-command-tool.md` | D-1 — :104/:113/:119-123/:139-158/:209/:259/:300(deny list 部分のみ注記 — pin B1/B2・argv A6・C7 manifest は authoritative 継続) | T2 |
| `docs/designs/workflow-resource-inference.md` | D-2 — :57/:65/:89(unified grant resource 宣言への再配線注記 — `## Coordinator review amendments` の**後**に配置) | T2 |
| `docs/designs/rust-rig-runtime-migration-architecture.md` | D-3 — :291-293(B5 bare `node` reject 除去 — PATH 解決 + spawn viability 受理) | T2 |
| `docs/designs/pretooluse-route-resolver-and-delegated-workflow-execution.md` | D-4 — :301-310/:398-429/:609/:1034-1036(viability 条件付き弱化・deny 除去・`Route.trusted` 削除・provenance trust) | T2 |
| `docs/designs/managed-process-canonical-tool.md` | (d-2) off-list — :57/:85/:88(task argv 内容審査前提 — literal argv 化・win32 shim は viability 制約) | T2 |
| `docs/designs/invocation-scoped-command-grants.md` | (d-2) off-list — :74/:99/:173/:229/:327/:341(deny 無条件列挙 + stale コード参照 + rename 残余) | T2 |
| `docs/designs/workspace-filesystem-boundary-sandboxed-tools.md` | (d-2) off-list — :44-56/:85-91(「sandboxed tools」命名・exec surface 不在前提 — canonical file tool 限定) | T2 |
| `docs/designs/overview.md` | (d-2) off-list — :54-67/:79-88(「sandboxed tools」・invariant 行を file tool surface 限定 — `packages/*` drift は最小言及のみ) | T2 |
| `docs/specs/policy-gated-command-tool.md` | S-1 — :60/:78/:79-80/:94 in-place(内容審査除去・opt-in deny)+ :82/:104 Deferred→決定済み訂正(writeScope+command fail-closed・`registry.rs:1167`)+ 末尾 `## Refine 注記` 併置推奨 | T3。trusted boundary・pin・argv pattern・cwd/env・lifecycle 要件は維持 |
| `docs/specs/invocation-scoped-command-grants.md` | S-2 — :60/:64/:70/:85/:93/:105/:118/:133/:138/:150/:153(deny 条件化・3 源 trusted)+ :115/:173 `RuntimePolicy.exec`/`inner.exec`/`exec: None` rename | T3。file-level `command.grants` shape の言及は訂正しない(表現 drift 境界) |
| `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md` | S-3 — `related` backlink のみ + 残差分確認(R8 routing candidacy 層は訂正不要・見つかれば層区別 1 行補記) | T3 |
| `docs/specs/managed-process-canonical-tool.md` | S-4 — :62/:72 in-place(task 名のみ・`fixed:true` grant literal argv・内容審査なし) | T3 |
| `docs/specs/workflow-resource-inference.md` | S-5 — :74 item 9 in-place(unified grant resource 宣言へ再配線・INVALID_REQUEST を fixed grant 前提で再記述) | T3 |
| `docs/specs/cheaper-v1-worker-runtime-cli.md` | (d-2) off-list — :71/:75/:105/:115 in-place(boundary/AC 主張を canonical file tool surface へ限定 + 末尾注記併置可) | T3 |
| `docs/test-specs/policy-gated-command-tool.md` | TS-1 — :52/:53/:62/:68 in-place annotation note(builtin deny 除去・opt-in `execDenyPatterns`・3 適用点存続) | T3。file-end 節は設けない(§(d-2) placement) |
| `docs/test-specs/invocation-scoped-command-grants.md` | TS-2 — :47/:51/:59/:74/:82/:95 同上(deny 条件化 + `RuntimePolicy.exec` rename) | T3 |
| `docs/test-specs/managed-process-task-authorization.md` | TS-3 — :50 同上(input-level 拒否維持・宣言 argv 内容審査は除去) | T3 |
| `docs/test-specs/route-resolution.md` | TS-4 — :52-56/:73-74 同上(#168 — `Route.trusted` 削除・merge 無条件化・presence 拒否・`suppressed` 狭義化) | T3。charter 内(#168 IR deferral) |
| `docs/test-specs/substitute-exec-route-mode.md` | TS-5 — :124-126/:174-175/:337-361/:374-378/:406-407/:473-478/:513-516/:542-543/:566-571 同上(cluster 化可 — merge 節・Guarantees・Risk 各末尾へ 1 本ずつ) | T3。charter 内 |
| `docs/test-specs/route-grant-set-projection.md` | TS-6 — :150-155 同上(project `trusted` presence 拒否の未反映分) | T3。charter 内 |
| `docs/guides/migration.md` | §(f) 構成 — front matter 無し・preamble(canonical disclaimer 全 3 項目)+ #163–#168 6 節 × 「変更内容/利用者影響/修復・移行手順」3 小節(§(f) 収録項表を転記) | T4。新設 |
| `docs/README.md` | User guides リストへ `migration.md` row 追加(`managed-process.md`・`command.md` の後・`workflow-usage.md` の前) | T4。手動 index 規約(`release.md` 先例) |
| `docs/designs/README.md` | status 整合確認のみ — 本 design row(:42・approved)は追加済み | T4/T5。編集不要なら無改訂 |
| `docs/specs/README.md`・`docs/adr/README.md`・`docs/discovery/README.md`・`docs/plans/README.md`・`docs/test-specs/README.md`・`docs/tasks/README.md` | 本 chain 行の status/title 整合確認 | T5。spec/ADR status 追随時のみ差分 |

## Implementation Sequence

- [ ] TASK-034L7LD2Gc1Num3wT45UMY (T1): guides + skills
      in-place 訂正 — `command.md` G-1(:44)・G-2(:133)・
      G-5 disclaimer(:141-153)、`managed-process.md`
      G-3(:23)・G-4(:27)・G-6 caveat(:47-49)、`workers.md`
      G-7(:138-139)、`usage.md` :207、cheaper-usage SKILL
      :353、cheaper-tool-operate SKILL :82-85。design §(e)
      の新旧 shape を転記 — commit `docs(guides):` で閉じる
- [ ] TASK-034L7LDZKCG5pF2MsoQOyC (T2): ADR + design refine
      注記 sweep — ADR1-7 + off-list ADR 3 件、D-1〜D-4 +
      off-list design 4 件の計 18 doc へ file-end `## Refine
      注記（issue #169）` 節(§(b) canonical 形 — 節は 1 つ・
      anchor ごとの bullet・governing link)+ `related`
      backlink 4 件(§(b) path 規約)。本文は保持
      — commit `docs(adr,design):` で閉じる
- [ ] TASK-034L7LJt4ZNXV0S6zO47ND (T3): spec in-place 訂正 +
      test-spec annotation — S-1〜S-5 + off-list cheaper-v1
      spec の要件本文を「訂正方向」列どおり書換
      (`（issue #16X — …）` inline 注釈付き・S-1 は末尾注記
      併置)+ TS-1〜TS-6 の該当 bullet 直下へ
      `> （issue #169 注記 — <現行の真実>）` blockquote
      (TS-5 は cluster 化)+ 全 12 doc へ `related`
      backlink — commit `docs(specs,test-specs):` で閉じる
- [ ] TASK-034L7LKoGPWs0X67ASTfsm (T4): `docs/guides/
      migration.md` 新設(§(f) 構成 — preamble + 6 issue 節)+
      `docs/README.md` User guides row + `docs/designs/
      README.md` status 整合確認
      — commit `docs(guides):` で閉じる
- [ ] TASK-034L7LPMgAWmjPhefil8Dn (T5): 最終検証 —
      design Verification Notes の grep 検証全項目・
      spec AC ↔ TSPEC Rule 対応確認・差分 docs-only 確認・
      `node scripts/check-doc-ids.mjs`・`audit_docs`・
      `mise run check`・status 更新

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 guides + skills in-place 訂正 | — | T5 |
| T2 ADR + design refine 注記 | — | T5 |
| T3 spec in-place + test-spec annotation | — | T5 |
| T4 migration.md + index | — | T5 |
| T5 最終検証 | T1, T2, T3, T4 | — |

critical path は (T1 ∥ T2 ∥ T3 ∥ T4) → T5。全 task の file set
は完全に disjoint(guides/skills ↔ adr/designs ↔ specs/test-specs ↔
guides 新設+README)のため 4 task 全て並列可能 — design §Data and
Control Flow の推奨順序(guides → spec → ADR/design → test-spec →
migration → index → verify)は依存ではなく便宜上のものであり、
本 DAG はそのまま訂正方式別の file 分割へ置き換えた。

task 境界と commit 境界の対応: design が提案する 3 塊 commit
(guides 訂正・refine 注記 sweep・migration + index)に対し、本
plan は refine 注記 sweep を訂正方式で 2 分割(T2 = file-end 注記
追記型 / T3 = in-place 訂正・annotation 型)した 4 塊とする —
1 task = 1 commit の粒度で、design の 3 塊より細かいが同じ論理
境界を外側に広げたものではない(後方互換の分割)。

既知の編集上の注意(transient ではなく契約):

- **refine 注記の節位置** — 「最終節の**後**」が規則であって
  特定節名の後ではない。D-2 `workflow-resource-inference.md`
  は `## Coordinator review amendments` が最終節 — その後に置く。
  ADR7 の注記本文には canonical disclaimer 項目 3(ENOEXEC)を
  含める(§(a) 配置表の必須項)。
- **test-spec は file-end 節を設けない** — §(d-2) placement
  convention: Rule/Example bullet 直下の `> （issue #169 注記
  — …）` blockquote。同一 falsified 機構の連続 bullet は
  cluster 末尾 1 本へ集約可(TS-5 のみ)。
- **S-2 rename 境界** — `RuntimePolicy.command` → `.exec` は
  resolved struct のみ。file-level `command.grants`・`command`
  section・`command:` layer key への言及は有効のまま訂正しない
  (表現 drift 回避)。
- **S-1 :82/:104** — Deferred 体裁を「決定・強制済み」へ訂正
  (`registry.rs:1167` fail-closed・workers.md:54-67 と整合)。
- **code 側不整合の flag 機構** — 「doc が正しく実装が誤り」を
  発見した場合は当該 doc を訂正せず issue へ follow-up 記録
  (design §Data and Control Flow — doc を実装へ合わせる訂正と
  混同しない)。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 注記品質のばらつき — 18 + 12 doc の注記/訂正で作業者の判断幅が残る | med | med | design §(c)/(d)/(d-2)/(e) の per-doc 表が anchor・旧記述・訂正方向・注記要点まで固定 — 行指定の転記のみで再解釈禁止を task Done When に明記 |
| 2 | in-place 訂正の過剰書換 — spec 要件本文を現行真実へ書き換える際、維持要件(trusted boundary・pin・argv pattern・lifecycle)まで消す | med | high | per-doc 表の「維持する項目」を task へ転記(S-1 明示・S-2 file-level shape 境界)・`git diff` で訂正対象行のみ差分であることを確認 |
| 3 | `related` ではなく `refines`/`refined-by`/`supersedes` を誤って張る graph pollution 再発(9d76d40 復旧事案の反復) | low | high | T5 で `git diff` + grep(`refines:`/`refined-by:`/`supersedes:`/`superseded-by:` の新規出現 0)を機械検証 — spec R6・ADR D3 |
| 4 | test-spec に file-end `## Refine 注記` 節を設けてしまう誤配置 | low | med | T3 task に §(d-2) placement convention を転記(bullet 直下 blockquote・file-end 節なし)— T5 grep で `issue #169` blockquote 存在を検証 |
| 5 | 既存の正直な記述を壊す均質化ミス — `command.md:63/:148`・`managed-process.md:47/:49`・`parent-agent-integration.md` の整合済み記述を canonical 文言と重複・矛盾させる | med | med | design G-9 の「変更不要」一覧を task へ転記 — 対象外行の差分を `git diff` で確認 |
| 6 | code 側不整合の混同 — 「doc が正しい・実装が誤り」を doc 訂正で隠す | low | med | flag → follow-up issue 機構(design §Data and Control Flow)を各 task の Out of Scope に明記 |
| 7 | migration guide の収録項欠落 — #163–#168 各節の「変更内容/利用者影響/修復」が design §(f) 表の必須項を欠く | med | med | §(f) 収録項表を task T4 へ転記 — T5 で 6 節 × 3 小節存在 grep |
| 8 | spec(SPEC-034L6i3XmfCaWRHAiw8TTT)・ADR(ADR-034L6ihibvbjmSnKmSffWH)が proposed のまま — status 昇格未処理 | low | low | design(approved)が file:line 契約を固定済みのため plan 実効性に影なし。status 追随は T5 で spec chain 慣行どおり処理(#168 先例 — spec=approved・ADR=accepted) |
