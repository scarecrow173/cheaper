---
id: "DISC-034zIKizOrTom96Q19mbYE"
type: "discovery"
status: "resolved"
title: "Workflow contract compiler の責務と検証境界"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/64"]
  derives-from: ["../ideas/compile-workflow-contracts.md"]
  derived-by: ["../specs/workflow-contract-compiler.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Workflow contract compiler の責務と検証境界

## Exploration Goal

ADR-034zGZpjoQlppwkkOs5gur D1/D5 の compiler と #62 実行境界を整理する。
Entry Decision: A-4。issue Triage と accepted ADR に目的・契約・acceptance が揃っている。
Dispatch-specific override: 明示された文書 chain と編集範囲を優先し、briefing-flow の SUBAGENT-STOP に従い meta profile の新規編集・sub-worker 派遣は行わない。
Active stack: idea-doc → discovery-doc → spec-doc → design-doc → planning-flow → implementation-flow。

## Key Issues and Assumptions

- `compileSchema` と `validateWorkerInput` は core public export 済み。Ajv strict 2020-12 を再利用できる。
- ToolCatalog の get/list は metadata port。handler/Pi/process は参照しない。
- default は各 public schema root のみ、required は default の有無で決まる。
- `$literal` と挿入値の再走査は禁止。依存辺は元 binding から採取する。
- `$from` 未生成時の完全 input schema validation は不可能。materialization と validation を一つの公開 hook にする。
- resource 宣言の構造はここで検査できるが canonical identity・writeScope・producer 推論は #65-B の責任。

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| placeholders を Worker schema に許す | compile 時に一括検証できる | Worker contract を弱める | 却下 |
| compiler が実行まで所有 | validation を強制できる | engine/Pi 境界と scope 違反 | 却下 |
| definition/invocation/execution hook を分離 | interface と engine が同一規則を使う | engine が hook を呼ぶ契約が必要 | 推奨 |

## Tentative Conclusions

definition は invocation 値なしで検査する。compiler は defaults → validation → `$input` → DAG の順で処理する。job の `$from` を解決する公開 hook は常に最後に Worker/tool input validation を行う。output hook は全 job の terminal 証跡を要求する。

## Open Questions

resource 宣言付き compile は inference port が未注入なら明示的エラーにする案を spec/design でレビューする。宣言なしの workflow は resource 推論未実装を主張せず compile 可能とする。

## Promotion Candidates

- spec: public contract、validation 段階、投影、検証可能な acceptance。
- ADR: 既存 accepted ADR D1/D5 を適用し新しい architecture 決定は追加しない。

## Evidence

`packages/core/src/worker/schema.ts`、`worker/validate.ts`、`packages/tool-runtime/src/catalog.ts`、issue 末尾 Triage を現在の checkout で確認した。
