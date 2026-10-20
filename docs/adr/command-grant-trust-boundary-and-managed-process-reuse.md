---
id: "ADR-034EMgaatFhsXjR97kH14c"
type: "adr"
status: "accepted"
title: "command grant の信頼境界を trusted layer のみに固定し managed-process 基盤を再利用する"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/101"]
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
    - "../specs/policy-gated-command-tool.md"
    - "../specs/tool-operator-built-in-worker.md"
    - "../specs/invocation-scoped-command-grants.md"
    - "./managed-process-named-task-lifecycle.md"
    - "./managed-process-bounded-lifecycle-policy.md"
    - "./opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "./invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from: ["../discovery/policy-gated-command-tool.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: ["../guides/managed-process.md"]
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Command grant trust boundary and managed-process reuse

## Context and Problem Statement

issue #101 は canonical tool vocabulary に policy-gated な動的コマンド実行 tool `command` を追加する。spec `../specs/policy-gated-command-tool.md` が「grant は trusted layer（builtin + user-scope `<userConfigDir>/policy.yaml`）でのみ宣言可能で、project layer からは authoring 不能」「model は executable / cwd / env を直接供給しない」という製品要件を固定し、信頼境界の実現方式（Option A/B/C）を本 ADR へ defer した。

現行の policy load 順序（`crates/cheaper-core/src/policy/load.rs`）は builtin（complete schema）→ user `<userConfigDir>/policy.yaml`（partial、`MergeMode::Replace`）→ project `<cwd>/.cheaper/policy.yaml`（partial、`MergeMode::RestrictConcurrency`）である。`RestrictConcurrency` は numeric / duration field の小さな allowlistを min-clamp するだけで、object は deep-merge・array は replace される。つまり **trusted-layer-only の仕組みは現存せず**、default の merge 規則のままでは project layer が grant を新規追加・widen できてしまう。

さらに `process.tasks` は現状 project policy でも extend 可能であり、`docs/guides/managed-process.md` で intentional として文書化済み。`command` の grant は任意 executable の事前認可であるため、この緩い境界を継承できず、意図的に厳しい divergence を取る。

もう一つの設計上の問いは、`command` が既存 managed-process 機構（`crates/cheaper-tools/src/tools/process/`）をどう再利用するかである。`executable.rs`（native magic MZ / ELF / Mach-O check、shell / shim / bare-node 拒否、PATH search、Windows `.exe` / `.com`、POSIX `X_OK`）と `tree.rs` は `pub`、一方 `spawn.rs` / `monitor.rs` / `prepare.rs` / `cleanup.rs` は `tools::process` に private な `mod` 宣言であり、その内部 helper は `pub(super)` で sibling module から到達できない（`mod.rs:52` が `pub use monitor::StreamCaptureResult` として型のみ re-export するが、lifecycle 関数群は公開されない）。`crates/cheaper-core/src/policy/process.rs` の `argv_shell_re` / `shell_executable_re` / `script_ext_re` は module-private の `fn` であり、公開済みなのは wrapper の `is_shell_process_executable`（`pub`、process.rs:136）のみである。

加えて、これらの module は可視性の問題だけではなく `process` 固有の型・policy に結合している。`prepare()` は signature が `ProcessTask` を取り、`process_task_resources(name ∈ policy.tasks)` を呼ぶため `tasks` 宣言に依存し、`monitor.rs` / execute 経路は `options.process.{timeout, kill_grace, heartbeat, max_output_bytes}` を直接読む。そのため再利用には可視性変更だけでなく、grant 用の generic 宣言型への refactoring が必要であり、grant ごとの timeout / killGrace / maxOutputBytes override を認めるなら新たな plumbing が別途必要になる。

## Decision Drivers

* grant 認可の fail-closed 性: project layer が信頼境界を迂回する経路を merge semantics の「正しい適用」に依存させない。境界は schema 層で構造的に閉じる。
* 既存方針との整合: invalid policy は silently ignore しない（`validate_policy_schema` が schema 違反を error にする現行挙動）。
* KISS / YAGNI: 信頼境界を実現する最小の機構を選び、semantic 比較や provenance tracking のような重い機構は要求しない。
* DRY: spawn / monitor / cleanup の lifecycle 知識を `command` 側へ複製しない。表現 drift（異なる native check・env sanitize 規則の分裂）を防ぐ。
* 信頼しない入力への policy 委譲禁止: tool input（model 供給）は grant selector に限定し、executable / cwd / env の policy 判断を untrusted LLM input に載せない。

## Considered Options

* Option A: project partial schema から grant authoring field を除外する。grant 宣言 key を含む project policy は schema validation で outright に fail し、load 全体が拒否される。project 側からの narrowing 手段は提供しない。
* Option B: grants を shared partial schema に残したまま `merge.rs` に grant ⊆ grant の semantic narrowing pass を追加し、project が grant 削除や argv / cwd / env subfield の tightening を行えるようにする。
* Option C: `restrict.rs` と同型の post-merge validation pass を追加し、effective grant のうち user / builtin layer に存在しないものを reject する。

## Decision Outcome

Chosen option: **Option A** — grant authoring field を project partial schema から全て除外し、project layer が grant を新規追加・widen する経路を schema 層で構造的に閉じる。grant 宣言 key を含む project policy は per-call ではなく load 時に `InvalidRuntimePolicy`（schema validation error）で fail し、load 全体が拒否される。

理由: Option A は最も単純な fail-closed 境界であり、merge semantics の正しさに依存せず schema validation という既存の fail-fast 境界だけで強制できる。merge 層を一切変更せず、error は policy load 時に project file 由来として明確に報告される。Option B は表現力が高い（project が per-subfield の tightening を宣言できる）が、grant ⊆ grant の semantic comparator が必要で、pattern 言語（argv pattern、cwd、env）の包含判定は脆弱で誤判定が信頼境界の穴になる。Option C は merge 層を触らずに済むが、検出が遅く（merge 後）、どの layer が grant を持ち込んだかの provenance tracking を別途要求し、error reporting を複雑にする。

なお project 側の narrowing field（`command.grantDeny: string[]` のような grant 削除専用 key、または tighten-only subfield のみを持つ constrained override map）も検討したが却下する。default の merge 規則では array は wholesale replace され、`command.*` field は `RESTRICT_*` allowlist に含まれないため、project 側 field は widen 可能であり構造的に narrow-only を保証できない。また project 専用 field は merge 後の Complete schema validation（`additionalProperties: false`）と衝突し、pipeline 途中で strip する処理を別途要する。本 iteration では project 側 narrowing を提供せず、将来の明示的な workspace trust 設計に defer する。

### 副次的決定

* tool input は grant selector + args 配列（`{name, args: string[]}`）とし、model は executable / cwd / env を供給しない。free な `{program, argv}` input は policy 決定を untrusted LLM input に委ねるため却下。`process` の `{task}` shape を mirror する。
* managed-process 機構は複製せず、`spawn.rs` / `monitor.rs` / `prepare.rs` / `cleanup.rs` の lifecycle 実装を `command` から再利用する。複製は DRY 違反かつ drift リスクのため却下。ただし再利用は可視性変更だけでは済まない: これらの module は private `mod` + `pub(super)` helper で閉じられており（`mod.rs:52` の re-export は `StreamCaptureResult` 型のみ）、さらに `prepare()` は `ProcessTask` を引数に取り `process_task_resources(name ∈ tasks)` を呼び、`monitor.rs` / execute 経路は `options.process.{timeout, kill_grace, heartbeat, max_output_bytes}` を直接読む。`command` からの再利用には task 固有の結合を剥がす generic 宣言型への refactoring が必要であり、grant ごとの timeout / killGrace / maxOutputBytes override を認める場合は新たな plumbing が別途必要になる。
* grant の executable 解決は `resolve_process_executable`（`tools/process/executable.rs`、`pub`）と native magic check、shell / shim 拒否規則を再利用する。`argv_shell_re` / `shell_executable_re` / `script_ext_re`（現 module-private）は `pub(crate)` 化または `is_shell_process_executable` と同型の公開 wrapper で提供し、別実装は作らない。
* tool registration は fail-closed とし、`registry.rs` の "process exposure requires nonempty authorized tasks"（registry.rs:1020）を mirror して "command exposure requires nonempty grants" を実装する。grant が一つも無い環境では `command` は tools.allow 宣言だけでは到達不能。
* `create_worker_scope` が `options.process` を再解決して不一致を reject する invocation parity（`invocation.rs`）を grant にも適用し、scope 生成後により広い grant set を密輸できないようにする。
* `process.tasks` の project extend 権限は本決定では変更しない。`command` の厳しい境界との divergence として docs に明記する（変更は別 spec / ADR の関心事）。

### Consequences

* Good, because 信頼境界が schema validation という単一の fail-fast 層に閉じ、merge 層・runtime 層に防御を分散させなくて済む。error は project policy file 由来として load 時に報告され、silent ignore が原理的に起きない。
* Good, because `process` tool の既存 contract と `process.tasks` の project extend 許可は不変で、後方互換を壊さない。
* Bad, because project admin は dynamic command を authorize できなくなる（`process.tasks` で固定 argv task は宣言できるが `command` grant は宣言できない）。これは意図した制約であり bug ではない — 任意 executable の事前認可は trusted user の判断とする。docs（managed-process guide または新規 command guide）にこの divergence を明記する。
* Bad, because trusted layer の grant（実質 user-scope）は高権限である: 子プロセスは OS sandbox 無しで動作し、grant で宣言した executable は filesystem / network への制御不能な副作用を持ち得る。task 宣言と `.cheaper/**` deny は containment を提供しない。この限界は docs に明記しなければならない（issue #101 acceptance criteria の必須項）。
* Bad, because Option B のような fine-grained な project narrowing（argv pattern の tightening 等）は表現できず、本 iteration では project 側の narrowing 手段を一切提供しない。要求が強まれば、将来の明示的な workspace trust 設計として別 ADR で revisit する。
* Neutral, because builtin policy は `command.grants: {}` を同梱するため、技術的には builtin も grant の authoring layer である。これは意図した構成であり、builtin は trusted layer とみなす。実質の authoring layer は user-scope `<userConfigDir>/policy.yaml` となる。
* 実装上の注意: 現行 `PolicySchemaMode` は `Complete` と `Partial` の二値で、user partial と project partial は同一 partial schema を共有している（`load.rs` の両 call site が `PolicySchemaMode::Partial`）。Option A の実現には user partial と project partial を区別する schema mode（例: `Partial` → `UserPartial` / `ProjectPartial`、または project 用フラグ）の導入が必要である。

### Confirmation

* grant authoring key を含む project policy が `InvalidRuntimePolicy`（schema validation error）で load 時に reject される test。
* grant ゼロ環境で `command` が `POLICY_VIOLATION` で registration に失敗することの test。
* `cargo test --workspace`（dev-seam feature 付き canonical check）の pass。

## Implementation Plan

* Affected paths:
  * `crates/cheaper-core/src/policy/schema.rs`: partial schema を user / project で分岐し、project partial から `command` 以下の grant authoring field を全て除外する。
  * `crates/cheaper-core/src/policy/load.rs`: user layer と project layer で異なる schema mode を渡す。
  * `crates/cheaper-core/src/policy/process.rs`（または新規 `command.rs`）: grant declaration shape の resolve / validation、`argv_shell_re` 等の regex の `pub(crate)` 化または公開 wrapper。
  * `crates/cheaper-tools/src/tools/process/mod.rs`: `spawn` / `monitor` / `prepare` / `cleanup` の可視性拡大または shared module への抽出。加えて `prepare()` の `ProcessTask` / `process_task_resources` 結合と `options.process.*` の直接参照を剥がす generic 宣言型への refactoring。
  * `crates/cheaper-tools/src/tools/command/`（新規）: `{name, args}` input schema、grant selector 解決、executable 解決 + 実行前検証、共用 executor への委譲。
  * `crates/cheaper-core/src/tools/vocabulary.rs`、`crates/cheaper-core/src/worker/validate.rs`、`crates/cheaper-tools/src/catalog.rs`、`crates/cheaper-tools/src/registry.rs`: `command` の closed-set vocabulary 追加、mutating tool 登録、schema dispatch、nonempty-grants fail-closed gate。
  * `crates/cheaper-tools/src/registry.rs`: managed-subprocess tracking に `command` を含める。`is_process` flag（registry.rs:845 付近、`managed.track()`）を `process` / `command` 両方で true にし、`drain()` / `wait_idle`（registry.rs:944-951 付近）が runtime shutdown で `command` の spawn 済み子 process も待つようにする。
  * `crates/cheaper-tools/src/catalog.rs`: `resources()` arm に `command` の分岐を追加し、`process_operation_declaration` に相当する operation declaration で workspace root claim に対する保守的な `Exclusive` access を宣言する（副作用が未知のため）。
  * `crates/cheaper-workflow/src/resource_inference.rs`: `kind: tool` job の resource 推論経路に `command` の宣言を通す touch point。
  * `crates/cheaper-tools/src/invocation.rs`: grant の re-resolve consistency check。
  * builtin policy（`crates/cheaper-cli/resources/` 側の既定 policy）: `command` セクションの既定（`grants: {}`）。
  * `docs/guides/`（managed-process guide 拡張または新規 command guide）: trust / sandbox 限界と `process.tasks` との divergence の文書化。
* Patterns to follow: `PolicySchemaMode` による load 時 fail-fast 検証、`resolve_process_executable` + `is_shell_process_executable` の executable 規則、`minimal_env` semantics、`registry.rs:1020` の nonempty-gate、`invocation.rs` の policy re-resolve parity、`RestrictConcurrency` mode の「project は trusted cap を緩められない」という方向性。
* Patterns to avoid: spawn / monitor / cleanup / tree-kill の複製、free `{program, argv}` tool input、merge.rs への semantic comparator 追加、invalid project policy の silent ignore、`command` grant の `.cheaper/policy.yaml` での authoring 許可。
* Dependencies or configuration: 新規依存なし。grant shape の最終形（`{executable, argv?, cwd?, env?, timeout?, killGrace?, maxOutputBytes?}` の override 配分）は design phase で確定するが、「project partial で authoring 不能」「model input に executable / cwd / env を載せない」という本 ADR の境界を超えない。

## Verification

* [ ] grant authoring key を含む project `policy.yaml` が schema validation error で load 時に reject され、user `policy.yaml` の同内容は受理される focused test が通る。
* [ ] grant 不在環境で `command` tool が POLICY_VIOLATION で fail-closed し、tools.allow のみでは露出しない test が通る。
* [ ] `{name, args}` 以外の executable / cwd / env key を持つ tool input が INVALID_INPUT で reject される test が通る。
* [ ] `command` が `resolve_process_executable` の native magic / shell / shim 拒否、`argv_shell_re` の metachar 拒否、`minimal_env` の sanitize を `process` と同一実装で通る test が通る。
* [ ] spawn / monitor / cleanup の実装が `process` module と共有され、`command` 側に複製がない構造確認。
* [ ] `create_worker_scope` の grant re-resolve parity test が通る。
* [ ] trust / sandbox 限界（no OS sandbox、project admin は grant を authoring できない、`process.tasks` との divergence）が docs に文書化されている。
* [ ] `mise run check`（biome / fmt / clippy / version・doc-id check / `cargo test --workspace` / node tests / lockfile・payload check）が pass する。

## Pros and Cons

| Option | Pros | Cons |
| --- | --- | --- |
| A: project partial schema から grant authoring field を除外 | schema-level reject で機構が単純。merge semantics を迂回不能。「invalid policy は silently ignore しない」方針と整合。error が load 時・project file 由来で明確 | merge 層を通さないので user / project partial の schema mode 分離が必要。fine-grained narrowing は別 field / pass の設計を要求 |
| B: merge.rs に grant ⊆ grant narrowing pass | project が grant 削除や argv / cwd / env subfield の tightening を直接表現できる | semantic comparator が脆弱で、包含判定ミスが信頼境界の穴になる。merge 層が複雑化し他 policy key の merge semantics に影響し得る |
| C: post-merge validation（restrict.rs 的）で effective grant を検査 | merge 層を触らず validation に閉じる | 検出が遅く、layer provenance tracking が別途必要で error reporting を複雑にする。schema 層で閉じられる問題を後段へ先送りする |

## More Information

* source: GitHub issue #101（`command` tool の要求と acceptance criteria の正本）。
* derives-from: `../discovery/policy-gated-command-tool.md`（DISC-034EM8dPyCYPdx7fqHlaBU。touch point 調査、reusable machinery、Option A/B/C の比較）。
* related: `../specs/policy-gated-command-tool.md`（製品要件の正本。信頼境界の実現方式を本 ADR へ defer）、`./managed-process-named-task-lifecycle.md`（named task 認可と shell-free lifecycle の正本）、`./managed-process-bounded-lifecycle-policy.md`（再利用する lifecycle policy の正本）。
* references: `../guides/managed-process.md`（managed process の認可・limit と `process.tasks` project extend 許可の文書）。
