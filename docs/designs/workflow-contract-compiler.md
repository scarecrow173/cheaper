---
id: "DESIGN-034zJ38fX1W3g0Zrceda1E"
type: "design"
status: "draft"
title: "WorkflowCompiler の validation と materialization 境界"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/64"]
  derives-from: ["../specs/workflow-contract-compiler.md", "../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
  references: ["../adr/fix-tool-runtime-and-workflow-package-topology-and-shared-contracts-upfront.md"]
---

# WorkflowCompiler の validation と materialization 境界

## Context and Boundaries

ADR-034zGZpjoQlppwkkOs5gur D1/D5 を実装し、D4/D6 の consumer seam を保つ。overview を読み、依存方向の正本は新しい accepted ADR とする。hot file の overview は編集しない。
handler、WorkerExecutor、filesystem、process、engine は compiler に含めない。

## Components

| File | Responsibility |
| --- | --- |
| `src/types.ts` | Workflow JSON、ports、compiled interface、projection の型 |
| `src/json.ts` | JSON 境界検査、key 安定順 clone、安全な object 構築 |
| `src/bindings.ts` | exact wrapper parser/scanner/materializer と result-root pointer |
| `src/schema.ts` | strict definition と public/job/output metadata/schema 検査 |
| `src/dag.ts` | edge dedupe、id 昇順 Kahn order、cycle/self/unknown 診断 |
| `src/compiler.ts` | defaults/input、compile、job/output hooks、resource port |
| `src/interface.ts` | invocation 無しの public projection |
| `src/index.ts` | public export のみ |

## Public Interfaces

```ts
interface WorkerCatalog { get(name: string): WorkerDefinition | undefined }
interface WorkflowCompilerPorts {
  tools: ToolCatalog;
  workers: WorkerCatalog;
  resources?: ResourceInference;
}
class WorkflowCompiler {
  constructor(ports: WorkflowCompilerPorts);
  validateDefinition(raw: unknown): WorkflowDefinition;
  compile(raw: unknown, inputs?: unknown): Promise<CompiledWorkflow>;
  projectInterface(raw: unknown): WorkflowInterface;
}
interface CompiledWorkflow {
  schemaVersion: 1;
  inputs: Record<string, JsonValue>;
  jobs: Record<string, CompiledJob>;
  order: readonly string[];
  edges: readonly DependencyEdge[]; // {from: producer, to: consumer}
  outputs: Record<string, WorkflowOutput>;
  materializeJobInput(jobId: string, results: ReadonlyMap<string, JsonValue>): JsonValue;
  materializeOutput(name: string, states: ReadonlyMap<string, WorkflowJobState>): JsonValue;
  materializeOutputs(states: ReadonlyMap<string, WorkflowJobState>): Record<string, JsonValue>;
}
```

`WorkflowJobState` は `{terminal:boolean,result?:JsonValue}`。status 名・error 集約を発明せず #62 に委ね、output hook は全定義 job に terminal:true を要求する。failed job が result を持たなくても、参照しない output は materialize できる。参照 missing は `INVALID_OUTPUT`。

単一 output hook を public にし、#62 は output ごとに検証して独立した成功 outputs を保持できる。batch hook は全 output 成功を要求する convenience であり、partial output 集約は engine が単一 hook を使って行う（D6）。どちらも全 job terminal の前提を検査する。

内部 compiled class の private fields に元 definition、入力 snapshot、schema validators、metadata を保持する。公開 fields は sorted JSON、hooks は prototype methods とし JSON 表現に混ぜない。元定義・invocation・compiled 表示 field の後続 mutation は validation 用 snapshot を変えない。

## Definition Validation

外部境界で finite number、JSON-compatible plain object/array、cycle/undefined 拒否を行う。reserved binding keys は exact-key の `$input`、`$from`+`path`、`$literal` のみ。output.value はこの Binding のいずれかであり一般 object は不可。

`WORKFLOW_JSON_SCHEMA` を公開し、top-level/job/resource/output wrapper の構造も core `compileSchema`（2020-12）で検査する。job.input 内部の reserved wrapper 検査は binding parser が担当し、`$literal` を再帰検査しない規則を一箇所に保つ。

各 public schema と catalog tool schemas は core `compileSchema` を利用。root default は同じ validator で検査する。schema の keyword subset は増やさず、外部 `$ref` を fetch/load しない core contract を継承する。WorkerCatalog は既存 loader が検証した WorkerDefinition を返す read-only metadata port とする。

tool 名の存在は ToolCatalog.get で確認し、Worker 名は WorkerCatalog.get で確認する。Worker.tools の各 capability tool も ToolCatalog に存在することを確認する。runtime policy による許可・拒否はここで拡張せず実行境界に残す。tool process も catalog の metadata として同じ経路で扱う。

needs/from/input reference と pointer syntax を検査する。schema から明確に不可能と判定できる path（閉じた object の存在しない key、primitive の子、明確な array index 不正）は definition error。union/ref/open schema 等不確実な path は推測で拒否せず実行時 pointer 検査に残す。

## Invocation and Binding Flow

1. definition 検証（interface と同じ）。
2. 省略 input に root default の deep clone を適用。unknown key と missing required を拒否。
3. public input 全体を各 schema で検査する。nested default は補完しない。
4. 元 binding を一度だけ走査し `$input` を clone で挿入。`$literal` 配下と挿入済み値は再 scan しない。元 binding から `$from` edge を得る。
5. `$from` 無しの input は compile 時にも最終 schema で検証。含む input は構造検査のみ。
6. resource port の結果も含め DAG を確定する。

job hook は元 binding に public inputs と upstream results を渡して一度で materialize し、常に最後に schema を検証する。Worker は core `validateWorkerInput` を直接使用し Worker schema を変更しない。tool は catalog inputSchema の compiled validator を使う。compile 済み表示 input の再走査は行わない。

pointer は空文字で result 全体を返し、`/` 分割と `~1`/`~0` decoding を行う。array は canonical 非負 index のみ（`01`、`-`、範囲外は拒否）。own property だけを参照する。`null` は値、missing と区別する。

## DAG

job ids と edges を code-unit 昇順で並べる（locale 非依存）。needs/from/resource edges を Set で dedupe。self/unknown は追加境界で拒否し、Kahn frontier は毎回 id 昇順の最小を選ぶ。残った job ids を cycle 診断に添える。job 表示順は id 昇順、実行依存 order は topological order。

## Resource Port and Responsibility

definition では `{reads?,writes?,produces?}` と各 `{path,scope}` を strict に検査し path は literal string または `$input` のみ。invocation で string に解決する。省略と `{}` は fallback として保持し、各 `[]` は明示的な access なしなので key を削除しない。

```ts
interface ResourceInferenceJob {
  job: WorkflowJob; // public input 解決後、from は未生成
  worker?: WorkerDefinition;
}
interface ResourceInferenceResult {
  manifests: Readonly<Record<string, ResourceManifest>>;
  edges: readonly DependencyEdge[];
}
interface ResourceInference {
  infer(jobs: Readonly<Record<string, ResourceInferenceJob>>):
    ResourceInferenceResult | Promise<ResourceInferenceResult>;
}
```

空でない resources object（`{reads:[]}` を含む）があるなら port を必須とする。port は全 job を受け、D4 canonical identity/fallback と producer ownership を扱い、WorkerDefinition の authoritative writeScope/capability と矛盾する宣言を拒否する責任を持つ。#65-A の normalize/predicate API は merge 前に import せず port 内で将来利用する。戻り manifest は全 job 分を要求し欠落は失敗。追加辺も通常の unknown/self/cycle 検査に通す。port の throw は成功に変換しない。未注入を empty inferred edges の成功に変えない。

core の real-path normalization が非同期でも接続できるよう infer は Promise を許容し、compile は常に Promise を返す。definition validation/projection は同期で、resource port を呼ばない。

resources が全て省略/{} の場合、正規化・producer 推論を実行したと主張せず compile し、runtime の conservative fallback を維持する。interface は port を呼ばず宣言の構造だけを検査する。

## Public Projection

`WorkflowInterface = {schemaVersion:1,inputs:{name:{schema,required}},outputs:{name:{schema}}}`。
default の正本は input schema root の `default` であり投影に二重の default map を作らない。required は root default の有無から算出。jobs/bindings/resources/Worker instructions は返さない。catalog get 以外の execution 副作用はない。

## Errors and Verification

definition/schema/catalog/reference/DAG/resource 未実装は `INVALID_REQUEST`、invocation/job input は `INVALID_INPUT`、output readiness/binding/schema は `INVALID_OUTPUT`（D6）。core schema compile error は definition 境界で cause を保ち wrap する。

fake ToolCatalog/WorkerCatalog/ResourceInference を使い spec AC1–AC7 を検証する。実 provider proof や engine 完成は主張しない。documentation に compact fixture と token-efficiency guidance を含める。

## Approval and Current State

2026-10-04: coordinator から `APPROVED: spec` を受信し spec を approved にした。design gate の質問 `msg_2b401cfb9478` は未回答であり、この design は draft のまま。graph の共有 ADR lineage による `focus-required` に対する運用判断も未受領。

coordinator terminal の read-only tail に `Quota exhausted` / `Your daily usage quota has been exhausted` を確認した。process exit は確認していない。dispatch の self-approval 禁止を守り、plan/task/impl と product code は未着手。再開には coordinator の quota 回復と design/plan 承認が必要。
