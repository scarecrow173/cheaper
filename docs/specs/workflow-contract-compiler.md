---
id: "SPEC-034zIKj7DJcwZFr3FvjLrP"
type: "spec"
status: "approved"
title: "Workflow JSON v1 を検証済み DAG へ compile する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/64"]
  derives-from: ["../discovery/workflow-compiler-boundaries.md"]
  derived-by: ["../designs/workflow-contract-compiler.md"]
  refines: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# Workflow JSON v1 を検証済み DAG へ compile する

## What Should Be Built

`@cheaper/workflow` に ADR-034zGZpjoQlppwkkOs5gur D1/D5 の Workflow JSON v1、binding、WorkflowCompiler、public interface projection を実装する。

## Why Now

#62 engine と #70 interface が同一の definition/public schema 検査を再利用し、親 agent に private DAG と中間 result を読み直させないため。

## Users and Value

親 agent は typed inputs/defaults/requiredness/output schema のみで呼出しを準備できる。engine は決定的な DAG と実行直前の検証 hook を使える。

## Scope

### In Scope

- strict definition、JSON 互換値、default/requiredness、binding、RFC 6901 pointer、DAG。
- Worker/tool metadata 注入と input/output materialization validation hooks。
- resource 宣言の構造検査と #65-B inference port、interface projection、contract tests。

### Out of Scope

- engine、handler、process 実行、Artifact Store、CLI、外部 schema fetch。
- resource canonicalization/producer 推論そのもの、policy/capability 拡張、nested workflow。

## Requirements

1. unknown definition fields、invalid reserved wrappers、unknown references、self-edge/cycle は definition error（`INVALID_REQUEST`）。unknown/invalid/missing invocation inputs は `INVALID_INPUT`。
2. input root default を deep clone して省略値だけに適用し schema 検査する。nested defaults を適用しない。default 無しは required、`default:null` は nullable optional。
3. `$input` は JSON 型を保持する。`$literal` 以下と挿入済み値は再 scan しない。`$from` path は result-root RFC 6901 pointer、missing key/index は明示エラー。
4. `needs` と元 `$from` の辺を dedupe し、job id 昇順の tie-break で topological order を確定する。定義 object の insertion order に依存しない。
5. Worker は注入された validated WorkerDefinition で存在と既存 capability/schema を確認する。tool は D1 ToolCatalog の metadata を参照する。未生成 `$from` を含む input は構造検査だけで compile し、公開 materialization hook が解決直後かつ開始前の `validateWorkerInput`/tool schema validation を必ず行う。
6. output hook は全 job terminal を確認して binding を解決し、各 output schema で validate する。missing result/schema failure は `INVALID_OUTPUT`。成功と失敗 job の status 集約は #62 が行う。
7. definition validation と interface projection は invocation 値なしで動き、catalog metadata lookup 以外の execution 副作用を持たない。投影は input schemas/defaults/requiredness と output schemas のみ。
8. resources path は literal string または `$input`、`$from` 不可。空でない resources object 付き compile は #65-B port 未注入なら `INVALID_REQUEST` で拒否し推論を成功扱いしない。resources 省略・`{}` は port 不要（D4 fallback の実行責任は engine/runtime）。各配列の空配列は明示的な「access なし」であり fallback ではない。port は WorkerDefinition を受け取り authoritative writeScope と宣言の矛盾を拒否し、canonical ResourceManifest と inferred edges を返す。compiler は追加辺を同じ DAG 検査に通す。

## Acceptance Criteria

- AC1: default/unknown key/invalid default/missing required/JSON 型保持、non-mutation、invocation 間 clone isolation の test が pass。
- AC2: 空 pointer、escape、array、missing、literal non-recursion、挿入値 non-scan、invalid/extra wrapper の test が pass。
- AC3: edge dedupe/self/unknown/cycle/id tie-break/deterministic compiled representation の test が pass。
- AC4: unresolved `$from` compile と materialization 後の Worker/tool validation、全 terminal 後の output schema validation の test が pass。
- AC5: invocation 無しの projection に private jobs/bindings/resources/prompt が無く、execution 副作用が無い test が pass。
- AC6: workflow dependency guard と `mise run check` が pass（既知 CLI timeout flake は単独再実行の証跡を添付）。
- AC7: compact な実行可能 contract fixture と、単独 Worker は直接呼び出し、fan-out/fan-in は Workflow にする token-efficiency guidance を documentation に記録する。

## Sources

issue #64 本文末尾 Triage 2026-10-04 と accepted epic ADR D1/D4/D5/D6 を適用する。
