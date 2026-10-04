---
id: "DESIGN-034zT2MINVpakJb3AhS0m5"
type: "design"
status: "approved"
title: "Workflow file loader と interface CLI の接続設計"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: []
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
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "../specs/workflow-file-interface.md"
    - "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow file loader と interface CLI の接続設計

## Context

共有 ADR-034zGZpjoQlppwkkOs5gur D1/D5/D6 を継承する。overview.md を参照したうえで、既存 WorkflowCompiler と projectWorkflowInterface に明示 file/utility CLI を接続する。新しい schema/interface/resource 契約は追加しない。

## Scope

### In Scope

- 明示 file の boundary/deny check、JSON parse、compiler の definition validation。
- invocation input validation の公開、compiler compile の再利用、workflow interface の args/help/envelope 接続。

### Out of Scope

- workflow run、execution engine、resource inference 実装、bundle/package/config の変更。

## Components and Boundaries

- `packages/workflow/src/loader.ts`: `loadWorkflowFile(options, createCompiler)`。options は canonical root/path/symlinkPolicy/denylist。core の resolveWorkspacePath で path を解決し、logical/canonical path とその祖先に deny を適用、通常 file のみ UTF-8 JSON を parse する。
- compiler factory は parsed raw definition を受け、metadata catalog を準備した WorkflowCompiler を返す。CLI の asynchronous resolveWorker と synchronous WorkerCatalog の接続に必要な seam。loader は factory 後に `compiler.validateDefinition(raw)` へ委譲し `{definition, compiler}` を返す。独自 schema validation は持たない。
- `WorkflowCompiler.validateInputs(raw, inputs = {}): Record<string, JsonValue>`: 既存 validateDefinition と invocationInputs を使い defaults/missing/unknown/schema validation を公開する。compile は同じ invocationInputs を使い続ける。loader と LoadedWorkflowFile 型は workflow/src/index.ts から export する。
- `cli.ts`: canonicalizeWorkspaceRoot → resolveConfigDirs/loadPolicy → loader。factory で createToolCatalog と raw jobs に現れる Worker の metadata のみ resolveWorker で準備する。Worker 不在/不正名は空 catalog entry とし canonical compiler の unknown-worker rejection へ渡す。壊れた Worker metadata は error を保存し workers.get から再 throw するため、compiler の structural/schema/default 検証より先に metadata error が出ない。
- interface は `compiler.projectInterface(definition)` のみ。resource inference/compile/execution を呼ばず、result を utilitySuccessEnvelope と既存 formatText/envelopeJson に渡す。args.ts は interface だけを受理し run 等を拒否する。parseArgs の option token を使い explicit --input - も拒否し、許可 option は --cwd/--format/--quiet（global help/version は従来どおり）。cli.ts の HELP と catch の format/quiet kind 判定にも workflow-interface を追加する。

## Data and Control Flow

1. `workflow interface <path> --cwd <root>` の format/path/options を解析する。
2. canonical root と policy を解決する。JSON の file access は capability を付与しない。
3. loader の path 検証 → regular file 読込 → JSON parse → compiler/catalog 構築 → definition validation。
4. #64 projection から schemaVersion/inputs/outputs のみを envelope に格納する。private definition を spread/serialize しない。
5. 将来 run は同じ loader の `{definition, compiler}` と validateInputs/compile を利用する。required inputs の値は interface 取得に不要。

## Non-Functional Constraints

- ADR D6 error table に従い、不在/読込不可/non-file・workspace escape/symlink policy・deny は INVALID_REQUEST、malformed JSON は INVALID_INPUT。両 code の exit は core ERROR_TABLE に従い 2。definition/input error は compiler の code を保持し、errorEnvelope が exit を決める。専用 code は追加しない。
- file と同じ workspace policy を適用。symlink の外部差替え/hard-link alias の追加保証はしない（既存 boundary 契約）。
- JSON は一行 envelope。interface は usage record/model/network/tool handler/Worker execution を生成しない。

## Risks and Trade-offs

- factory の raw jobs scan は untrusted data の型ガードと Worker metadata 収集だけを行う。schema/default/DAG/reference 検証は canonical compiler が所有する。
- file と dynamic definition の invalid schema/default/DAG で同じ code を返すことを契約テストする。

## Deferred Design Concerns

- #62 run composition が ResourceInference を供給する。loader は compiler の注入でそのまま使える。今回 interface のために空 resource inference を発明しない。

## Verification Notes

- [ ] loader 同一 compiler 路、validateInputs default/missing/unknown/schema、file errors/boundary/deny を focused suite で確認。
- [ ] CLI の private 非公開、required 値不要、JSON/text/error/exit、Worker metadata 対応、model 未初期化を単独 test file で確認。
- [ ] pnpm lint/typecheck/build と scoped exit-audit。canonical check deferred to coordinator merge run。

2026-10-04: coordinator により design approval APPROVED。上記 code pin、options、HELP/error-path、export/return type、metadata precedence を反映。共有 ADR に複数既存 plan が接続される router 制約は承認済み運用例外で、spec/design scoped audit 後に今回 plan focus から再開し delegate 順を手動維持する。
