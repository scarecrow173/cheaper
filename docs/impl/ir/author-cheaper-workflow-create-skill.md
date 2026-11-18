---
id: "IMPL-034ZusOsqpn5CG7IWNftoR"
type: "impl"
status: "completed"
title: "Author cheaper-workflow-create skill"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"doc","path":"packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md"}
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/author-cheaper-workflow-create-skill.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-create-skill-verification.md"
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Author cheaper-workflow-create skill

## Summary

`packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` を新設した。
親 Agent が Workflow JSON v1 定義を設計・`workflow interface` で静的
検証・catalog へ登録できるようにする skill で、定義保存と実行を
分離し、`workflow run`・grant/policy/Route 変更を行わない承認境界を
明文化した。

## Context

- task: `../../tasks/author-cheaper-workflow-create-skill.md`
- plan: `../../plans/implement-workflow-create-skill.md`
- design: `../../designs/workflow-create-skill.md`
- spec: `../../specs/workflow-create-skill-issue-204.md`
- ADR: `../../adr/workflow-create-skill-uses-interface-validation-and-reviewable-catalog-diff.md`
- issue: #204(Epic #200 の子)

## Implementation

### Changes

- `packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md` (new)
  - front matter(`name`/`description`)、When to use(明示的作成依頼
    または discover 提案の承認のみで発動、候補検出だけでは永続化
    しない)、Approval boundary(定義保存と実行の分離、grant/policy/
    Route 非改変、上書き前確認)、Authoring procedure(9 手順)、
    Workflow JSON v1 anatomy(必須 top-level 4 key・inputs defaults・
    job kind・binding 3 wrapper・RFC 6901 pointer・`when` skip
    semantics・resources 宣言)、検証済み template 5 形態、Validate
    (error 対処表)、Register in the catalog、Report back、
    Good vs bad、Related skills の節構成。

### Decisions During Implementation

- 検証 authority は `cheaper workflow interface` のみとし、skill 内に
  並行 schema を持たない(ADR の決定どおり compiler が source of
  truth で drift を防ぐ)。
- template は実測 pass した形のみ採用した。template 5(条件付き
  `when`)は skip 耐性を示すため `{"type": ["string","null"]}` の
  nullable output schema を持つ。
- catalog 登録は reviewable な manifest 手追記 + `workflow list`
  確認(#201 で write CLI が deferred のため)。`workflow list` の
  `duplicate`/`missing`/`invalid` 状態と diagnostics 語彙をそのまま
  記載した。
- `--input` は file path または `-` (stdin) のみ。inline JSON
  `--input '{...}'` は Good vs bad の反例としてのみ記載。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `workflow interface` 実測: 5 template 相当の fixture
  (minimal-worker・tool-worker-when・fan-in・process-chain・
  saved-workflow・routed-exec) が `ok:true` exit 0。skill 内 code
  block を抽出して fixture と照合し、fan-in は整形差のみで
  interface pass を再確認した。
- invalid case 実測: unknown worker・unknown input ref・unknown job
  ref・unknown dependency・invalid wrapper・invalid pointer・
  primitive child pointer・invalid default・input schema 不正・
  cycle・追加 top-level property・malformed JSON・version 非1 が
  すべて `ok:false` exit 2 で、対処表の message/code と一致。
- `workflow list` 実測: scratch workspace の catalog で ok/missing/
  duplicate の各 status と diagnostics を確認。
- `git diff --check` clean、`doc_status audit --blocking` blocking 0、
  `mise run check` exit 0(guide 側 record と同一の最終検証)。

## Risks

- skill 内の tool/worker 名・pointer 例は現行 catalog/embedded
  resource に依存する。tool 追加・worker schema 変更時に例が古く
  なり得るが、skill 自体が `workflow interface` 実測を必須手順に
  しているため drift は利用時に検出される。

## Follow-ups

- `cheaper-workflow-discover`(#203) 実装後、提案→承認→create の
  導線を相互参照する。
- catalog write CLI が将来追加された場合、手追記手順を CLI 手順へ
  更新する。
