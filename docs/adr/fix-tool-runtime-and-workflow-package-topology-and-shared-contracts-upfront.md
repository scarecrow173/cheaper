---
id: "ADR-034zGZpjoQlppwkkOs5gur"
type: "adr"
status: "accepted"
title: "Tool Runtime / Workflow Engine epic の package 配置と共有契約を先行固定する"
created: "2026-10-04"
updated: "2026-10-04"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/68"
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
  related:
    - "../designs/pi-execution-engine.md"
    - "./workspace-filesystem-boundary-cheaper-sandboxed-tool.md"
    - "./pi-sibling-tool-call-parallelism-via-runtime-output-contract-guidance.md"
  refines:
    - "./worker-definition-runtime-policy-runtime-invariants-capability.md"
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "https://github.com/scarecrow173/cheaper/issues/56"
    - "https://github.com/scarecrow173/cheaper/issues/57"
    - "https://github.com/scarecrow173/cheaper/issues/58"
    - "https://github.com/scarecrow173/cheaper/issues/59"
    - "https://github.com/scarecrow173/cheaper/issues/60"
    - "https://github.com/scarecrow173/cheaper/issues/61"
    - "https://github.com/scarecrow173/cheaper/issues/62"
    - "https://github.com/scarecrow173/cheaper/issues/63"
    - "https://github.com/scarecrow173/cheaper/issues/64"
    - "https://github.com/scarecrow173/cheaper/issues/65"
    - "https://github.com/scarecrow173/cheaper/issues/66"
    - "https://github.com/scarecrow173/cheaper/issues/67"
    - "https://github.com/scarecrow173/cheaper/issues/69"
    - "https://github.com/scarecrow173/cheaper/issues/70"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Tool Runtime / Workflow Engine epic の package 配置と共有契約を先行固定する

## Context and Problem Statement

epic #68 の子 issue（#56–#67, #69, #70）は複数 worktree で並列実装される。各 issue が独立に package 配置・policy schema・metrics 型・resource 型・workflow JSON・CLI surface を決めると drift と merge 衝突が起きる。並列着手前に、子 issue が共有する契約を 1 箇所で固定する必要がある。現状（main @ e51e236）: runtime src に workflow / subprocess / artifact / scheduler は存在せず、tool は `packages/runtime-pi/src/tools/*` で Pi `defineTool` / Pi helper に依存している。`RuntimePolicy.execution` は `{timeoutMs, maxTurns, maxToolCalls}` のみで `mergePolicyLayers` は last-wins。

## Decision Drivers

- 依存方向: Workflow 層が Pi に依存しない（#63 AC）。
- less-trusted policy layer が上限を緩和できない（#57/#61 AC）。
- 並列 worktree 間の hot file 衝突を最小化する（root tsconfig / package manifests / policy schema / cli.ts）。
- YAGNI/KISS: 現在の子 issue が必要とする範囲のみ固定する。

## Considered Options

1. 契約を固定せず各 issue に委ねる（却下: drift / 衝突）。
2. core に全て入れる（却下: core が policy と execution を混在、Pi helper 注入の置き場がない）。
3. runtime-pi 内に全て入れる（却下: Workflow が Pi 依存になる）。
4. **採用**: `@cheaper/tool-runtime` と `@cheaper/workflow` を新設し、下記契約を固定する。

## Decision Outcome

Chosen option: 4.

### D1. Package 配置と依存方向

- 新設 `packages/tool-runtime` (`@cheaper/tool-runtime`): canonical tool handler / ToolCatalog / OperationManager / ToolScheduler / ResourceManager / MutationJournal（journal は #63 で移動し、`runtime-pi/src/journal.ts` と public index は互換 re-export を残す）。
- 新設 `packages/workflow` (`@cheaper/workflow`): Workflow schema / compiler / ResourceManifest 推論 / Artifact Store / Workflow engine。
- 依存方向（固定）: `tool-runtime → core`、`runtime-pi → tool-runtime + core`、`workflow → tool-runtime + core`、`cli → workflow + runtime-pi + tool-runtime + core`。
- `workflow` は `WorkerExecutor` interface を受け取り（composition root = cli で runtime-pi 実装を注入）、`runtime-pi` を import しない。
- dependency guard: `tool-runtime` / `workflow` の package.json **と src の static import / export / literal dynamic import** を検査し、`@earendil-works/*` と `@cheaper/runtime-pi` を禁止する（`workflow` は加えて `@cheaper/runtime-pi` と exact package name `cheaper`（cli package。prefix ではなく完全一致）を、`tool-runtime` は `@cheaper/workflow` を禁止）。core は上位 package を import しない。
- core に残るもの: policy / vocabulary / capability / workspace path identity / worker definition / token counter / `ResourceClaim` `ResourceManifest` 型（D4）。
- tool metadata の正本（`@cheaper/tool-runtime` が公開。#64 compiler はこれだけを注入して参照し handler 実装を参照しない）:

```ts
type JsonValue = null | boolean | number | string | JsonValue[] | { [k: string]: JsonValue };
interface ToolMetadata { name: ToolName; inputSchema: Record<string, unknown>; outputSchema: Record<string, unknown> }
interface ToolCatalog { get(name: string): ToolMetadata | undefined; list(): readonly ToolMetadata[] }  // 副作用なし。Pi session / model 初期化をしない
interface CanonicalToolResult { content: Array<{ type: "text"; text: string }>; details?: JsonValue }
```

- handler の canonical result は `CanonicalToolResult` とし、Workflow の tool job result もこの形。Pi adapter は text/details を保持し、独自再解釈や二重 accounting をしない。`count_tokens` 等の構造化情報は `details` にも保持する。
- input validation は canonical runtime の入口で一度行い、direct 呼出しが Pi の引数 validation を迂回しても同じ拒否になる。handler の throw は失敗であり、error text を成功 result に変換しない（Pi の toolResult への失敗変換は adapter の責務）。
- write/edit/ls の Pi helper-backed handler は composition root または runtime-pi adapter が注入する。pure registry は Pi helper を自動 import せず、未登録 handler は明示的に拒否する。
- root `tsconfig.json` references、各 `package.json` deps / lockfile、`scripts/check-version.mjs` / `set-version.mjs` の更新は coordinator の scaffolding commit で行い、各 worker は触らない（`diff` と `typebox` は tool-runtime の dependency として先行追加済み）。

### D2. RuntimePolicy `execution.concurrency`

```yaml
execution:
  concurrency:
    maxOperations: 8          # 正整数。invocation 全体（Worker ごとではない）の同時 operation 上限
    mutations: serialized     # "serialized" | "per-resource"
```

- 既定 `maxOperations: 8`, `mutations: serialized`。
- 合成は二段階。**builtin → user は trusted 設定段階**で、既存の deep merge / scalar replace を使う（user は `maxOperations` を増減でき、`mutations: per-resource` を指定できる。省略は builtin/default を継承）。**続く project は restriction 段階**で、trusted 設定を緩和できない。CLI は effective policy を narrow するだけ。
- 実装は `mergePolicyLayers(base, override, options?: { mode: "replace" | "restrict-concurrency" })` の 1 箇所。省略時は従来の replace。`loadPolicy` は user に replace、project に restrict-concurrency を明示する。restriction の対象 key は `execution.concurrency.maxOperations`、導入後の `workflow.maxJobs`、`process.maxConcurrent` のみ（両値がある場合 min、省略は inherit）。`mutations` は restriction 段階のみ `serialized` 優先（trusted per-resource + project 省略/per-resource → per-resource、trusted per-resource + project serialized → serialized、trusted serialized + project per-resource → serialized）。他の既存 scalar/array 項目の合成は変更しない。
- 例: builtin 8/serialized → user 16/per-resource → project 32/per-resource = 16/per-resource。project 4/serialized なら 4/serialized。CLI 20 は拒否。user が無いとき project 16/per-resource は 8/serialized のまま。
- legacy complete policy に `concurrency` が無い場合は許容する。builtin/user の trusted merge 後・project restriction 前に raw policy へ既定 `{maxOperations: 8, mutations: "serialized"}` の欠けた field を補完し、その後 project merge → effective complete validation → normalize。partial schema に既定値を入れて「省略」を「実指定」に変えない。
- CLI concurrency option を追加する場合、effective cap を超える要求と serialized→per-resource の緩和要求は `POLICY_VIOLATION`（`restrict.ts` の timeout と同じ扱い）。CLI option 自体の追加は #57 の必須作業ではない。
- `per-resource` の実行には #61 の validated manifest が必要で、不明 access は broad claim に縮退する。
- `workflow.maxJobs` は別 limit（既定 1、明示有効化で増加）。`process.maxConcurrent` は `maxOperations` の sublimit。
- 本 D2 は accepted ADR「Worker Definition / Runtime Policy / Runtime Invariants / Capability」の scalar replace を `execution.concurrency`（および上記 2 key）に限って refine する。他の permission 項目（`tools.allow` 等）が安全な上限合成になったとは主張しない。

### D3. `meta.execution` metrics（#58）

```ts
execution: {
  operations: number;                       // admit された operation 数（開始前 cancel を含む）。入場拒否は含めず rejectedToolCalls のまま
  byState: Record<OperationState, number>;  // queued|running|completed|failed|cancelled|timed_out。全 state の key を 0 を含め常に持つ
  byTool: Record<string, number>;
  maxConcurrency: number;                   // 観測された最大同時 running 数
  operationMs: number;                      // 開始済み operation の running 区間合計
  activeMs: number;                         // running 区間の union
  overlapMs: number;                        // operationMs - activeMs
  queueMs: number;                          // queued 区間合計（cancel された待ちを含む）
}
```

- 不変条件: `operations = sum(byState) = sum(byTool)`。queued 区間 = admit → start（または queued のまま terminal/snapshot）。running 区間 = start → 実行 settle（または snapshot）。cancel 要求だけでは running 区間を終了しない。terminal state は cleanup / 実行 settle の観測後に確定する。
- monotonic clock。snapshot 時に進行中区間を snapshot 時刻で clamp。
- `WorkerRunMeta.execution` は **ownerId で絞った** operation 集合の snapshot。Workflow の `meta.execution` は invocation 全体の集合の snapshot。field 定義は同一。sum / union / max はそれぞれその集合から求め、Worker snapshot を足し合わせて Workflow snapshot を作らない。
- success / error 両 envelope に同一型で載せる。`ErrorEnvelopeMeta.execution` は `ExecutionMetrics | null`（service 未生成の error は `null`、生成済みで operation 0 件はゼロ snapshot）。
- `criticalPathMs` / `parallelismSavedMs` は採用しない。usage.jsonl への追加は本 epic の範囲外。

### D4. `ResourceClaim` / `ResourceManifest`（#65-A, core）

```ts
type ResourceScope = "exact" | "subtree";
type ResourceClaim = {
  resource: { kind: "workspace"; path: string; scope: ResourceScope };  // path = canonical 絶対 real path
  mode: "shared" | "exclusive";
};
type ResourceManifest = { reads: ResourceClaim[]; writes: ResourceClaim[]; produces: ResourceClaim[] };
```

- `path` は canonical workspace 内の **絶対 real-path identity**（`packages/core/src/workspace/paths.ts` の real 解決。存在しない tail を含む）。入力の workspace-relative path / display path とは区別し、display path は診断専用に保持する。比較 key は Windows では case-insensitive、POSIX では case-sensitive。
- `exact` は当該 path のみ、`subtree` は当該 path 自体と全 descendants（不存在を含む）。境界比較は separator / component 単位で行い、文字列 prefix で兄弟名（`src/a` と `src/ab`）を重ねない。
- reads = shared、writes / produces = exclusive。配列と mode の不整合は reject。produces は「file が新規か」ではなく **その workflow 内の producer ownership 宣言**。future file output は `exact`、directory output は `subtree` を producer が明示し、存在 / 不在から file / directory を推測しない。同一 / subtree-overlap output に複数 producer がある場合は compile error。
- glob を ancestor `subtree` に近似してよいのは safety lock としての reads / writes のみ。produces は literal exact path または明示 subtree のみ（`dir/**` は dir 全体の ownership、`dir/*.json` のような部分集合 producer は拒否して列挙か directory ownership を要求）。推論は宣言の ownership を根拠にし、filesystem の現存一覧から出力を決めない。
- conservative fallback: read 不明 → workspace root `subtree` shared、mutation 不明 → workspace root `subtree` exclusive。
- conflict 時の挙動: 取得できない claim は **待機**（queue、abort / invocation timeout で cancel）。fail ではない。複数 claim は all-or-none・安定順取得（hold-and-wait 回避）。
- job lease の所有は親 job（lease token を保持）。子 operation は覆われた範囲を borrow し、親 lease を release しない。同一 parent 内の operation 同士の conflict は別に検査する。
- artifact identity は別 namespace（`kind: "artifact"` を #66 で追加可。workspace path と混在させない）。
- 型と正規化・containment / conflict predicate は core に 1 箇所（`packages/core/src/resources/`）。lease 実行は tool-runtime、推論は workflow。hard-link alias や外部 process の symlink 差替えの解決は主張しない（現行 workspace boundary を継承）。

### D5. Workflow JSON v1（#64）

```json
{
  "schemaVersion": 1,
  "inputs":  { "<name>": <JSONSchema> },
  "jobs":    { "<id>": { "kind": "tool", "tool": "...", "input": {...}, "needs": [], "resources": {...} }
             | { "kind": "worker", "worker": "...", "input": {...}, "needs": [], "resources": {...} } },
  "outputs": { "<name>": { "schema": <JSONSchema>, "value": <Binding> } }
}
```

- Binding は exact-key object の 3 種: `{ "$input": "<name>" }`、`{ "$from": "<jobId>", "path": "<JSON Pointer>" }`、`{ "$literal": <JsonValue> }`。`$from` の pointer root は upstream job の **result 自体**（`CanonicalToolResult` または Worker result）。空文字は result 全体、`/details/tokens` は `result.details.tokens`。dotted path / alias は受理しない。RFC 6901 の `~0` / `~1` を扱い、存在しない key / index は明示的に失敗する。
- `$literal` の配下は reference scanner / materializer が再帰解釈しない。`$input` / `$from` の解決で挿入した値も再 scan しない。literal 内や input default 内の `$from` は edge を作らない。reserved key の不正 wrapper や余分 field は definition error。
- `resources` は optional で、D4 の `ResourceManifest` の **JSON 入力表現** `{ "reads"?: ScopeDeclaration[], "writes"?: ScopeDeclaration[], "produces"?: ScopeDeclaration[] }`、`ScopeDeclaration = { "path": <workspace-relative string>, "scope": "exact" | "subtree" }`。mode は配列種別から導出。path は literal または public `$input` の解決値（`$from` は v1 で不許可）。`ResourceClaim[]` の裸配列や別 key 名は使わない。省略 / 空は D4 の conservative fallback。既存 Worker の writeScope / capability を拡張する宣言ではなく、宣言が authoritative writeScope と矛盾する場合は拒否。
- public input / output schema は `packages/core/src/worker/schema.ts` の `compileSchema`（Ajv JSON Schema 2020-12）をそのまま使う。v1 で第二の keyword subset を作らず、inline schema とローカル `$defs` のみ、外部 `$ref` の fetch / 自動ロードはしない。各 input の root `default` のみ clone して省略値へ適用し、nested default の自動補完はしない。default 自体も schema validate する。default のない public input は required、nullable optional は `default: null`。
- validation 段階: (1) definition validation は invocation 値なしで行え、`interface` が使う。(2) invocation validation は defaults → input validation → `$input` materialization → static DAG compilation。(3) `$from` 値が未生成の job input は構造検査のみで compile し、依存完了後の materialization 直後かつ job 開始前に Worker / tool の input schema で validate する（skip しない）。(4) output binding は全 job terminal 後に materialize して宣言 output schema へ validate。動的 graph refinement は v1 に入れない。
- `needs` は control dependency、`$from` / artifact 参照 / produces∩reads は data dependency（後者 2 つの推論は #65-B）。explicit `needs` と `$from` は dedupe して DAG edge とし、self-edge / unknown job / cycle は definition error。同時 runnable な job の tie-break と envelope `jobs` の並びは job id の昇順（safety scheduling の順序であり semantic edge ではない）。
- `process` は `kind: "tool", tool: "process"` の **固定 executor** で、policy に宣言された named task の argv を shell 解釈なしで直接 spawn する（glob 展開 / pipe / metachar なし、cwd = workspace root または validated workspace-relative dir、path は workspace boundary 適用）。任意 command 文字列の実行は提供しない。Worker の `tools.allow` capability tool として露出する場合も同じ executor を通るため、Runtime Invariant「shell なし」と衝突しない。

### D6. CLI surface

- v1 の CLI surface は 2 つ: `cheaper workflow run <path> --input <file>|-`（stored definition。`--input` 省略時の invocation 値は `{}`、`--input -` は stdin）と `cheaper workflow interface <path>`（utility envelope `worker: null`。public inputs / defaults / requiredness / output schema のみ）。bare `cheaper workflow`（stdin definition）と `--values` は後続に defer する。親が動的生成した workflow は一時 JSON file を同じ loader / compiler に渡して実行する（parent 側 overhead は #67 の dynamic-vs-stored 比較に明記）。
- 既存の `cheaper run` / worker alias と `--input` の意味（file path または `-`）は不変。workflow file は canonical `--cwd` workspace 相対で解決し、boundary / deny policy を適用、file から capability を付与しない。
- Workflow envelope は新 variant として `packages/cli/src/envelope.ts` に 1 箇所で定義。`result` = 宣言 outputs、`jobs` = job id 昇順の status / meta / bounded diagnostics（private result 本文を含めない）。v1 は job の optional / required flag を持たず、いずれかの job の失敗・`dependency_failed` skip は overall `ok: false` + 非 0 exit（独立成功と materialize 可能な outputs は保持）。aggregate usage を単一 Worker model として偽装しない。
- exit / error code: definition / invocation error は既存 `INVALID_INPUT` / `INVALID_REQUEST`、外側 timeout は `TIMEOUT`、外側 signal は `ABORTED`、job failure の集約は `TOOL_EXECUTION_FAILURE`（各 job error に元 code を保持）、output materialization / schema failure は `INVALID_OUTPUT`。専用 Workflow error code を後続で設ける場合は integrator が 1 箇所で更新する。
- `args.ts` / `cli.ts` への workflow command 追加は 1 人の integrator（#69-B → #70 → #62-B を順に担当）が行う。

### D7. 実行 context の単一性と `runWorker` seam

- OperationManager / ToolScheduler / ResourceManager / Artifact Store は invocation につき 1 つ。`runWorker(deps: RunWorkerDeps)` の一引数 signature と戻り値は変更せず、`RunWorkerDeps` に optional field を追加する:

```ts
// @cheaper/tool-runtime が公開。ResourceLease は #61 で具体化する。
interface WorkerExecutionBinding {
  runtime: InvocationToolRuntime;   // 公開 facade。内部 manager 群を deps bag に露出しない
  ownerId: string;                  // invocation 内で一意。workflow では job id
  parentLease?: ResourceLease;
}
interface RunWorkerDeps { /* existing fields unchanged */ execution?: WorkerExecutionBinding }
```

- `execution` が無い single-Worker 呼出しでは `runWorker` が invocation tool runtime を 1 つ作って所有し、finally で abort / drain / dispose する。`execution` がある場合は借りた runtime 上に `ownerId` の child scope を作り、その scope だけを終了する（shared runtime / 他 job の scope / 親 lease を dispose・cancel・release しない）。root と実効 concurrency policy は runtime 作成時に固定し、不整合な binding は fail closed。
- `runAbort`（Worker timeout / signal / budget）は child scope に渡す。root invocation signal は全 scope を cancel できるが、1 つの Worker の maxTurns / timeout は他 job を abort しない。Worker の budget / meter / journal は child scope ごとに保持し、shared scheduler / ResourceManager は scope に注入する（Worker ごとに新しい semaphore を作らない）。
- Workflow composition root（cli）は invocation tool runtime を 1 つ生成し、tool job と Worker job に同じ実体を渡す。engine が取得した job lease は `parentLease` として渡し、job の child operation が settle した後に engine が release する。Artifact Store は workflow invocation に必要なときだけ 1 つ作り、tool-runtime の `ArtifactReader` port へ注入する。runtime-pi / tool-runtime が workflow package を import して store を取得することは禁止。
- 具体的な operation service と lease は #56 / #57 / #61 で実装する。#63 に未実装 scheduler の no-op を作らせない。

### Consequences

- Good: 子 issue が独立 worktree で並列実装でき、統合点（integrator が触る hot file）が明確になる。
- Bad: 2 package 追加により build / references / version check の対象が増える。
- 本 ADR は既存 capability ADR の concurrency 合成を限定的に refine する。契約と矛盾する変更は superseding ADR を要する。互換的な細化や implementor acceptance check の追加は本 ADR への追記でよい。
- 本 ADR は実装を含まない（scaffolding と shared type の宣言のみ）。各子 issue の spec / design は本 ADR を `references` に持つ。

### Confirmation

- scaffolding commit 後、`mise run check` が pass する（dependency guard test を含む）。
- 各子 issue の design doc が本 ADR の該当 D 番号を引用し、D2 layer table・D4 output identity・D5 reference / default / interface semantics・D7 borrowed / owned lifetime の期待結果が contract test で確認される（review-protocol で確認）。

## More Information

- epic issue #68（一次情報）:
  https://github.com/scarecrow173/cheaper/issues/68
