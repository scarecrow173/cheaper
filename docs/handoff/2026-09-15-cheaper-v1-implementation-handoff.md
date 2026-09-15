# cheaper — Coding Agent Implementation Handoff

> 一次ソース: 2026-09-15 にユーザーから提示された実装引継ぎ文書を原文のまま保存したもの。
> 本文書は編集しない。派生した spec / ADR / design は `relations.source` からここを参照する。

## 0. この文書の目的

この文書は、`cheaper` プロジェクトの現時点の設計をコーディングエージェントへ引き継ぐための実装指示書である。
以下に記載されている事項は、これまでの設計検討で確定したV1方針である。
実装時は、既存の設計判断を独断で置き換えないこと。
特に以下を守ること。

- 不要なインフラを追加しない
- Pi SDKの上に独自Agent Runtimeを再実装しない
- Worker固有ロジックをRuntimeへハードコードしない
- 親Coding Agent固有の機能へ依存しない
- security boundaryをLLM promptだけに依存させない
- deterministicな情報をLLMに自己申告させない
- V1のスコープを勝手に拡大しない

## 1. プロジェクト概要

プロジェクト名： cheaper

`cheaper` は、Claude Code / OpenAI Codex / GitHub Copilot / Cursorその他の親Coding Agentから、一部の処理を安価なモデルへ委譲するための汎用Worker Runtimeである。
目的は、frontier modelへ巨大なcontextや機械的コード生成を集中させず、安価なモデルへ処理を逃がすことである。

基本構造：

```
Parent Coding Agent
Claude / Codex / Copilot / Cursor / ...
              │
              ▼
        APM Hook / Skill
              │
              ▼
          cheaper CLI
              │
              ▼
       Worker Resolver
              │
              ▼
      Worker Definition
              │
       ┌──────┴──────┐
       ▼             ▼
 portable tools   model tier
       │             │
       ▼             ▼
 Runtime Policy   Model Resolver
       │             │
       └──────┬──────┘
              ▼
           Pi SDK
              │
       ┌──────┴──────┐
       ▼             ▼
   Pi tools      Pi models/auth
       │             │
       └──────┬──────┘
              ▼
          Workspace
              │
              ▼
      Structured Result
              │
              ▼
         Parent Agent
```

## 2. 基本設計思想

### 2.1 親Agent非依存

`cheaper`内部へ、

- Claude Code固有subagent
- Codex固有subagent
- Copilot固有Agent
- Cursor固有Agent

などを持ち込まない。
親Agentが認識するものは原則CLI contractだけである。

```
cheaper bulk-read
cheaper cheap-write
cheaper run <worker>
```

### 2.2 RuntimeはPi SDKに固定

複数Runtime Adapterを切り替える構成にはしない。
RuntimeはPi SDKを利用する。
Piが提供する、

- provider abstraction
- OAuth / API authentication
- subscription access
- model registry
- agent loop
- tool calling
- coding tools
- usage information
- abort

を利用する。
`cheaper`独自のAgent Loopは原則実装しない。

### 2.3 Workerは宣言的定義

WorkerをTypeScriptコード等へハードコードしない。
Workerは、

```
YAML frontmatter
+
Markdown instructions
```

からなるMarkdownファイルとして定義する。
built-in Workerもuser-defined Workerも同じ仕組みを使う。

### 2.4 WorkerとRuntime Policyを分離

Worker Definition： Workerが何をしたいか
Runtime Policy： cheaper全体としてどこまで許すか

を表す。

```
Worker requested capabilities
            ∩
Runtime Policy
            ↓
Actual Runtime Capabilities
```

### 2.5 Runtime Policyより強いRuntime Invariantsを持つ

Runtime Policy自体も変更できない制約が存在する。
例：

- workspace外filesystem access禁止
- symlink escape禁止
- shellなし
- network toolなし
- unknown tool禁止
- output schema validation必須
- CLIによる権限拡張禁止

### 2.6 deterministicな情報はwrapperが取得

以下をLLMに自己申告させない。

- changed files
- diff
- added/deleted lines
- exit status
- duration
- provider/model
- token usage
- tool count
- turn count

filesystemやGit等から機械的に取得する。

## 3. V1 Worker

V1 built-in Workerは2つ。

### 3.1 bulk-reader

大量・巨大なコードをcheap modelへ読ませる。

目的：

```
large source context
        ↓
bulk-reader
        ↓
cheap model
        ↓
compressed findings
        ↓
parent frontier model
```

基本tools： read / grep / find / ls

用途：

- repository investigation
- call flow investigation
- dependency tracing
- relevant files/symbol discovery
- large-context summarization

ファイル変更は行わない。

### 3.2 cheap-writer

機械的・パターンベースのコード生成をcheap modelへ委譲する。

用途：

- tests
- DTO
- types
- configs
- boilerplate
- repetitive adapters
- simple mechanical refactoring

対象外：

- architecture decisions
- security-sensitive logic
- concurrency
- subtle debugging
- ambiguous business rules

基本tools： read / grep / find / ls / write / edit

生成コード全文を親Agentへ返すことを基本とせず、workspaceを直接変更する。

## 4. Process Model

V1はone-shot process。
daemon/server化しない。

```
request
 ↓
Worker resolve
 ↓
Worker validation
 ↓
Runtime Policy resolve
 ↓
model resolve
 ↓
Pi session creation
 ↓
Agent execution
 ↓
output validation / repair
 ↓
filesystem / usage measurement
 ↓
structured result
 ↓
session disposal
 ↓
process exit
```

以下はV1では作らない。

- REST server
- FastAPI
- MCP server
- daemon
- Redis
- PostgreSQL
- Queue
- Kubernetes
- distributed workers
- central telemetry server
- vector DB
- semantic cache
- multi-agent orchestration
- Web UI

## 5. CLI Contract

JSON-first CLIとする。

主要command：

```
cheaper bulk-read
cheaper cheap-write
cheaper run <worker>
```

aliases：

```
cheaper bulk-read   = cheaper run bulk-reader
cheaper cheap-write = cheaper run cheap-writer
```

task inputはJSON。

Canonical interface：

```
cat request.json | cheaper bulk-read --cwd /repo
```

file input：

```
cheaper bulk-read \
  --input request.json \
  --cwd /repo
```

共通CLI control：

```
--input <file|->
--cwd <path>
--timeout <duration>
--format <json|text>
--quiet
--help
--version
```

原則：

```
task data                  → JSON
runtime invocation control → CLI
worker behavior            → Worker Definition
global execution policy    → Runtime Policy
```

## 6. stdout / stderr

stdoutは最終result専用。
defaultはJSON。
Pi event、debug log、tool log等を混ぜない。

概念：

```json
{
  "schemaVersion": 1,
  "ok": true,
  "worker": "bulk-reader",
  "result": {},
  "meta": {}
}
```

人間向け表示： `--format text`
TTY検出による自動形式変更はしない。

stderr：

- warnings
- diagnostics
- runtime errors
- optional progress

に使用する。

## 7. cwd / Workspace

`cwd`は単なるprocess working directoryではない。

```
cwd
=
workspace root
=
filesystem security boundary
=
project configuration scope
```

未指定なら `process.cwd()` を使う。
必ずcanonicalizeする。
parent directoryからproject rootを自動探索しない。

## 8. Worker Definition

形式：

```markdown
---
name: bulk-reader
description: Analyze large source trees and return compressed findings.

tools:
  - read
  - grep
  - find
  - ls

cheaper:
  model:
    tier: low

  output:
    schema:
      file: ./schemas/bulk-reader.schema.json
---

You are a precise code analyst.

...
```

必須：

- name
- description
- tools
- cheaper.model.tier
- cheaper.output.schema
- Markdown body

すべてnon-empty。

## 9. Worker Name

lower-kebab-caseのみ。

```
^[a-z0-9][a-z0-9-]*$
```

例： bulk-reader / cheap-writer / unit-test-writer

ファイル名と`name`は一致必須。
`bulk-reader.md` なら `name: bulk-reader`。

## 10. Portable Tool Vocabulary

V1： read / grep / find / ls / write / edit

これはcheaperの公開tool vocabulary。
Pi内部tool名に依存しているものとして扱わない。
runtimeがPi toolへ変換する。

安全なcompatibility aliasのみ許可：

```
Read   → read
Grep   → grep
Glob   → find
Write  → write
Edit   → edit
```

以下などは推測変換禁止： Bash / Shell / Execute / WebFetch / Agent / Search

unknown toolはfail closed。

## 11. Worker toolsはpermissionではない

Workerの `tools: [write]` は「write capabilityを要求する」という意味。
permission grantではない。

Runtime Policy側でwriteが許可されていなければ `POLICY_VIOLATION` とする。
silentにwriteだけ削除して実行しない。

## 12. cheaper Namespace

`cheaper:`内部はstrict schema。

V1で許可： `cheaper.model` / `cheaper.output` のみ。

Worker Definitionへ以下を置かない。

- timeout
- maxTurns
- retry
- repair
- filesystem policy
- concrete model
- provider
- authentication
- network permission
- shell permission

これらはRuntime側の責務。

## 13. Model Tier

Workerは具体モデル名を持たない。

```yaml
cheaper:
  model:
    tier: low
```

allowed： low / medium / high

これはドル価格そのものではなくrelative resource tier。

```
low     cheap / fast / quota-friendly
medium  general-purpose
high    expensive / scarce / frontier-class
```

## 14. Model Resolution

RuntimeはPi SDK固定。
Piで現在利用可能なmodel一覧を取得する。
`cheaper`が自動tier分類する。

```
Pi available models
        ↓
Model Classifier
        ↓
low / medium / high
        ↓
Worker requested tier
        ↓
preference ordering
        ↓
Concrete Pi Model
```

classification priority：

1. user override
2. cheaper built-in classification
3. Pi metadata heuristic
4. unknown → medium

未知modelを`low`にはしない。
具体model listをWorker Definitionへ書かない。
同一tierのprovider/model preferenceはcheaper config側で定義する。

## 15. Output Schema

Worker Definitionはstructured output schemaを必ず持つ。
以下のexactly one： `file` / `inline`

external：

```yaml
cheaper:
  output:
    schema:
      file: ./schemas/bulk-reader.schema.json
```

inline：

```yaml
cheaper:
  output:
    schema:
      inline:
        type: object
        required:
          - answer
        properties:
          answer:
            type: string
```

external schemaの相対pathは、`cwd` ではなく、Worker Definition file directory 基準。

対応format： `.json` / `.yaml` / `.yml`

remote schemaはV1対象外。

## 16. Worker Top-Level Compatibility

cheaper標準field： name / description / tools / cheaper

`cheaper:`内部はstrict。
一方、トップレベルの未知fieldは他Coding Agentとの共存のため許容し、ignoreする。

例： `model: ...` / `permissionMode: ...` があっても、それをcheaper設定とは解釈しない。

## 17. Worker Discovery

named Worker： `cheaper run bulk-reader`

探索順：

1. `<cwd>/.cheaper/workers/bulk-reader.md`
2. `<user-config>/cheaper/workers/bulk-reader.md`
3. `built-in/bulk-reader.md`

priority： project > user > built-in

whole-file shadowing。mergeしない。

## 18. Worker Resolution Rules

parent directoryは探索しない。
`<cwd>/.cheaper/workers` だけを見る。

invalidなproject Workerが存在する場合： project invalid → ERROR
user/built-inへfallbackしない。
設定ミスを隠さない。

## 19. Explicit Worker File

直接指定可能： `cheaper run --worker-file ./workers/foo.md`

この場合named resolutionを完全にbypass。
relative pathは`cwd`基準。absolute pathも許可。
Worker file自体はworkspace外でもよい。

以下は許可しない： `cheaper run ./foo.md`
nameとpathを曖昧にしない。

## 20. Runtime Policy

Runtime Policyは全Worker共通。
Worker個別policyはV1では存在しない。

Canonical built-in policy：

```yaml
version: 1

execution:
  timeout: 120s
  maxTurns: 8
  maxToolCalls: 64

tools:
  allow:
    - read
    - grep
    - find
    - ls
    - write
    - edit

filesystem:
  symlinks: within-workspace

model:
  maxTier: medium
  tierFallback: none

output:
  maxTokens: 2000
  repairAttempts: 1

retry:
  transientAttempts: 1
```

## 21. Runtime Policy Sources

```
built-in
    ↓
user
    ↓
project
    ↓
CLI restrictions
    ↓
Runtime Invariants
```

locations：

```
built-in: <package>/policy.yaml
user:     <user-config>/cheaper/policy.yaml
project:  <cwd>/.cheaper/policy.yaml
```

project/userはpartial overrideを許可する。
ただし`version: 1`は各policy fileで必須。

## 22. Runtime Policy Merge

objectはdeep merge。scalarは後段replace。arrayはunionせずarray全体replace。

例：

built-in：

```yaml
execution:
  timeout: 120s
  maxTurns: 8
```

user：

```yaml
execution:
  timeout: 180s
```

project：

```yaml
execution:
  maxTurns: 12
```

effective：

```yaml
execution:
  timeout: 180s
  maxTurns: 12
```

## 23. Policy Validation

順序：

```
load built-in
↓
validate complete schema
↓
load user
↓
validate partial schema
↓
merge
↓
load project
↓
validate partial schema
↓
merge
↓
validate complete effective policy
```

invalid user/project policyをsilent ignoreしない。

## 24. execution Policy

```yaml
execution:
  timeout: 120s
  maxTurns: 8
  maxToolCalls: 64
```

`timeout`： wall-clock timeout。 syntax： `^[1-9][0-9]*(ms|s|m|h)$`
`maxTurns`： positive integer。
`maxToolCalls`： positive integer。

timeoutには以下も含む。

- provider retry
- model calls
- tool execution
- output validation
- repair

## 25. tools Policy

```yaml
tools:
  allow:
    - read
    - grep
    - find
    - ls
    - write
    - edit
```

allowed vocabularyはWorker Definitionと同じ。
空配列もvalid。

effective tool behavior：

```
Worker requested tools
      ∩
Policy allowed tools
```

ただし1つでもrequested toolが許可されていなければWorker全体を拒否。
部分実行しない。

## 26. filesystem Policy

V1：

```yaml
filesystem:
  symlinks: within-workspace
```

allowed： `deny` / `within-workspace`

`within-workspace`： symlink targetをrealpath/canonicalizeして、最終targetがworkspace内なら許可。workspace外なら拒否。

workspace rootそのものを変更するpolicy fieldは存在しない。

## 27. model Policy

```yaml
model:
  maxTier: medium
  tierFallback: none
```

`maxTier`： low / medium / high。 order： low < medium < high

Worker requirementが`maxTier`を超える場合： `POLICY_VIOLATION`

`tierFallback`： `none` / `higher`

`none`： 要求tierが無ければ `MODEL_TIER_UNAVAILABLE`
`higher`： low → medium → high のように上位だけ探索。ただし`maxTier`を超えない。
lower fallbackはしない。

## 28. output Runtime Policy

```yaml
output:
  maxTokens: 2000
  repairAttempts: 1
```

`maxTokens`： positive integer。 effective： `min(policy maxTokens, selected model maximum)`
`repairAttempts`： non-negative integer。

初回resultがinvalid JSONまたはschema mismatchならrepairできる。

重要： repair = output-only operation。
repair時はtoolsを無効化する。repairによるfilesystem再変更を防ぐ。

schema validation自体はRuntime Invariantとして必須。

## 29. retry Policy

```yaml
retry:
  transientAttempts: 1
```

non-negative integer。initial callを含まない追加retry回数。

retry対象：

- temporary provider failure
- rate limit
- connection reset
- temporary unavailable

対象外：

- authentication failure
- invalid request
- unsupported model
- policy violation
- schema error
- tool error

Worker全体を最初から自動replayしない。tool callも自動replayしない。

## 30. CLI Restrictions

CLIはRuntime Policyを狭めることだけ可能。

例えばpolicy `execution.timeout: 120s` なら `--timeout 60s` はvalid。
`--timeout 300s` はpolicy expansionなので `POLICY_VIOLATION`。
silent clampはしない。

## 31. Runtime Invariants

Policy fileから変更不能。

```
workspace root = canonicalized cwd
filesystem access remains inside workspace
symlink escape prohibited
unknown tools prohibited
shell unavailable
network tool unavailable
Worker output must match declared schema
Worker Definition cannot elevate Runtime Policy
CLI cannot elevate Runtime Policy
```

## 32. Error Model

現時点で想定される主なerror code：

```
WORKER_NOT_FOUND
INVALID_WORKER_DEFINITION
INVALID_WORKER_NAME
WORKER_NAME_MISMATCH
UNKNOWN_TOOL

INVALID_RUNTIME_POLICY
POLICY_VIOLATION

OUTPUT_SCHEMA_NOT_FOUND
INVALID_OUTPUT_SCHEMA

MODEL_TIER_UNAVAILABLE

TIMEOUT
ABORTED

PROVIDER_FAILURE
TOOL_EXECUTION_FAILURE
PARTIAL_MUTATION
INTERNAL_ERROR
```

正式なJSON Error Contractは今後詰める。

## 33. Exit Codes

現時点の分類：

```
0   success
2   invalid request / CLI usage
3   configuration error
4   policy / filesystem violation
5   provider / model failure
6   worker / tool execution failure
7   timeout
8   aborted
9   partial mutation / unsafe completion
70  internal error
```

詳細分類はJSON error code側で行う。

## 34. Abort

対象： SIGINT / SIGTERM / timeout

Pi/model requestをabortする。
cheap-writerでabortしてもautomatic rollbackしない。
Git checkout等で勝手に戻さない。
filesystem mutation済みの可能性があればstructured result/errorで明示する。

## 35. Usage / Telemetry

最低限取得する予定：

- provider
- model
- input tokens
- output tokens
- cached tokens
- duration
- turn count
- tool count

central telemetry backendは作らない。
local JSON/stdout/stderr等で十分。
詳細形式はまだ未確定。

## 36. APMとの責務分離

APM： hooks / skills / instructions / routing / distribution
cheaper： Worker resolution / Worker execution / Runtime Policy / model selection / Pi execution / filesystem isolation / structured output / usage collection

Hook自身からPiを直接起動しない。

```
Parent Agent
 ↓
Hook detects case
 ↓
Skill instructs parent
 ↓
parent invokes cheaper
 ↓
cheaper invokes Pi
```

## 37. bulk-reader Hook想定

```
Parent Agent
 ↓
Read
 ↓
Hook
 ↓
large?
 ├─ no → normal Read
 └─ yes
      ↓
   deny / redirect
      ↓
   bulk-read Skill
      ↓
   cheaper bulk-read
```

Hookは小さく保つ。hidden background Workerを起動しない。

## 38. V1成功条件

V1完成条件：

- `cheaper bulk-read`がPiを利用して任意repoをread-only分析できる
- 親Agentへ短いstructured resultだけ返せる
- Worker DefinitionをMarkdownファイルとして独立定義できる
- project/user/built-in Worker resolutionが動く
- Pi available modelからtier分類・選択できる
- model classificationをuser overrideできる
- `cheap-write`でworkspaceを変更できる
- changed files/diff統計をwrapperが実測できる
- Runtime Policyをbuilt-in/user/projectでmergeできる
- workspace filesystem boundaryを強制できる
- invalid outputをschema validationできる
- failure時にnon-zero exitとstructured errorを返せる
- 親Coding Agent固有機能へ依存しない
- Pi implementation detailsを親Agentへ露出しない

## 39. V1 Non-goals

実装しない。

```
daemon
Web UI
REST API
MCP Server
Redis
PostgreSQL
Queue
Kubernetes
distributed workers
central telemetry
vector DB
semantic cache
multi-agent orchestration

Worker inheritance
Worker composition
Worker merge
prompt imports
remote Workers
remote schemas

Worker-specific Runtime Policy
Worker-defined timeout
Worker-defined maxTurns
Worker-defined concrete model
Worker-defined provider
Worker-defined credentials
Worker-defined shell
Worker-defined network permissions

shell tool
network tool
automatic Git rollback
whole-Worker automatic retry
```

## 40. 現時点で未確定の設計

以下はまだ実装前に詳細設計が必要。

bulk-reader contract

- input JSON Schema
- output JSON Schema
- path semantics
- directory/glob handling
- relevant symbol representation
- suggested reads representation
- source excerpt policy

cheap-writer contract

- input JSON Schema
- output JSON Schema
- reference files
- target / multiple targets
- partial writes
- changed files calculation
- diff statistics

Pi SDK integration

- exact session creation API
- model registry interaction
- AuthStorage
- tool factory
- event handling
- usage extraction
- abort controller
- repair model call

Model Resolver

- built-in model family classification
- metadata heuristic
- user override config schema
- same-tier preference config

Filesystem security

- exact path normalization algorithm
- symlink handling implementation
- TOCTOU considerations
- write atomicity
- secret/environment exposure

Error Contract

- stdout error envelope
- error detail fields
- retryable semantics
- partial mutation representation
- exit code mapping

Usage telemetry

- local record format
- stderr/stdout exposure

APM integration

- exact Hook contract
- bulk-read Skill
- cheap-write Skill

## 41. 実装時の優先順位

実装を始める場合、いきなりPi integrationから書き始めないこと。
以下の順序を推奨する。

1. Core types / schemas
2. Worker Definition parser + validator
3. Worker Resolver
4. Runtime Policy parser + validator + merger
5. Portable tool model
6. Workspace / path security primitives
7. Model Resolver abstraction
8. Pi SDK integration
9. bulk-reader contract + execution
10. cheap-writer contract + execution
11. deterministic filesystem/diff measurement
12. CLI envelopes / errors / exit codes
13. integration tests
14. APM integration

## 42. テスト方針

少なくとも以下をtestする。

Worker resolution

- project shadows user
- user shadows built-in
- invalid project does not fallback
- explicit file bypasses resolution
- name mismatch fails
- parent directory not searched

Worker validation

- missing required field
- empty field
- unknown cheaper field
- unknown tool
- alias normalization
- file schema
- inline schema
- invalid JSON Schema

Runtime Policy

- built-in complete policy
- user partial merge
- project partial merge
- array replacement
- unknown field rejected
- invalid type rejected
- CLI can narrow
- CLI cannot expand

Security

- ../ escape
- absolute-path escape
- symlink outside workspace
- symlink inside workspace
- write outside workspace
- read outside workspace
- unknown tools

Execution

- timeout
- abort
- maxTurns
- maxToolCalls
- tier unavailable
- tier fallback
- provider retry
- schema repair
- repair has no tools

cheap-writer

- actual filesystem diff
- partial mutation
- abort after mutation
- no automatic rollback

## 43. 実装原則

YAGNI： 将来必要かもしれない機能を先回りして作らない。特に daemon / remote service / model gateway / policy language / dynamic plugin system / Worker inheritance / distributed execution を追加しない。

Fail closed： 不明な状態で推測して実行しない。unknown tool / unknown policy field / invalid Worker / invalid schema / workspace escape / unsupported model tier は明示的にfailする。

No silent fallback： 設定ミスを隠さない。project Workerが壊れているならbuilt-in Workerへfallbackしない。project policyが壊れているならuser policyだけで実行しない。

No silent privilege reduction： Workerがwriteを要求しているがpolicyがread-onlyなら、readだけ渡して実行しない。Worker要求と実行環境が一致しないためfailする。

No silent privilege expansion： CLI、Worker、project configからRuntime Invariantsを越えさせない。

Single source of truth： 同じ設定を複数レイヤーへ重複させない。

```
Worker behavior            → Worker Definition
global runtime limits      → Runtime Policy
hard security constraints  → Runtime Invariants
concrete model selection   → Model Resolver
provider/auth/agent execution → Pi
```

## 44. 目標とする最終的な性質

`cheaper`は最終的に、「安価なモデルを使うための専用Agentを2つハードコードしたツール」ではなく、「Pi上で宣言的Workerを安全かつ決定論的に実行する、小さな汎用Worker Runtime」として実装する。

`bulk-reader`と`cheap-writer`はその最初のbuilt-in Workerにすぎない。

したがって、新しいWorkerを追加するときに、cheaper Runtime source code を変更せず、Worker Definition + Output Schema を追加するだけで原則動作する構造を維持する。

## 45. 最重要アーキテクチャ境界

実装中に迷った場合は、この境界へ戻ること。

```
APM                   WHEN to delegate
Worker Definition     WHAT the Worker is
Runtime Policy        HOW MUCH it may consume / use
Runtime Invariants    WHAT MUST NEVER be violated
Model Resolver        WHICH available model to use
Pi SDK                HOW the Agent actually runs
Parent Coding Agent   WHY the work is needed
```

この責務境界を崩さないこと。
