---
id: "DISC-034EM8dPyCYPdx7fqHlaBU"
type: "discovery"
status: "resolved"
title: "command tool の policy grant と信頼境界を整理する"
created: "2026-10-12"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/101"
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
    - "./managed-process-canonical-tool.md"
    - "./managed-process-lifecycle-hardening.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/policy-gated-command-tool.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
  verifies: []
  verified-by: []
  references:
    - "../guides/managed-process.md"
  defers: []
  deferred-by: []
---

# command tool の policy grant と信頼境界を整理する

## Exploration Goal

issue #101 は、canonical Tool Runtime に policy-gated な動的コマンド実行 tool（working name `command`）を追加する。既存の `process` tool は `policy.yaml > process.tasks` に事前登録された named task しか実行できない。新 tool は、trusted な USER scope policy で宣言された grant を持つ worker に限り、program + 動的 argv の実行を許可する。default deny。

この discovery が答える問い: `command` を既存の managed-process lifecycle に載せるためにどこを触るか、そして「grant は trusted layer（user / builtin）でのみ宣言可能で project policy は narrow のみ」という信頼境界を現在の policy merge 機構でどう実現するか。

## Constraints

issue #101 由来の要求:

- shell 評価を一切行わない（`sh -c` / `cmd /c` / script shim / interpreter bypass 禁止）
- 子プロセスの env は default で sanitize する
- cwd / path は検証前に normalize する
- argv は token 単位で検証する
- 既存 managed-process lifecycle（spawn / timeout / abort / tree-kill / bounded output）を再利用する
- `meta.changes` は未観測の子プロセス mutation を主張してはならない
- project policy は grant を narrow できるが extend してはならない
  - （2026-10-13 追記）accepted ADR `../adr/command-grant-trust-boundary-and-managed-process-reuse.md` が issue #101 の narrow-only 要求を supersede した: 採用された Option A は本 iteration では project 側 narrowing 機構を一切提供せず、project grant key は load 時 reject となる。
- Worker / Workflow は同一の canonical 実装を共有する

## Key Issues and Assumptions

- 確認済み: tool vocabulary は `crates/cheaper-core/src/tools/vocabulary.rs` の closed set（8 個）。`command` 追加は `TOOL_NAMES`・`ToolName`・`from_canonical`・`is_mutating_tool`（`crates/cheaper-core/src/worker/validate.rs`）・`crates/cheaper-tools/src/catalog.rs` の NAMES + schema dispatch・`crates/cheaper-tools/src/registry.rs` の match arms に波及する。compile error が全 touch point を導く。
- 確認済み: policy load 順序（`crates/cheaper-core/src/policy/load.rs`）は builtin（complete schema）→ user `<userConfigDir>/policy.yaml`（partial、`MergeMode::Replace`）→ project `<cwd>/.cheaper/policy.yaml`（partial、`MergeMode::RestrictConcurrency`）。
- 確認済み（critical gap）: 現行には "trusted-layer-only" の仕組みが存在しない。`RestrictConcurrency` は numeric / duration field の小さな allowlist を clamp するだけで、object は deep-merge・array は replace されるため、default ルールでは project layer が grant を widen できてしまう。
- 確認済み: `process.tasks` は現状 project policy で extend 可能であり、`docs/guides/managed-process.md` で intentional として文書化済み。`command` の grant は意図的にこれより厳しい境界を要求する。
- 確認済み: input schema は grant selector + args（例: `{name, args: string[]}`）とし、model は executable / cwd / env を直接供給しない。`process` の `{task}` shape を mirror する。
- 確認済み: error code は既存の `POLICY_VIOLATION`・`INVALID_INPUT`・`TOOL_EXECUTION_FAILURE`・`TIMEOUT`・`ABORTED` で足りる。
- 仮定: grant の宣言 shape は `{executable, argv?, cwd?, env?, timeout?, killGrace?, maxOutputBytes?}` を候補とし、design phase で確定する。

## Reusable Machinery

探索で特定した再利用可能な既存機構（`crates/cheaper-tools/src/tools/process/`）:

- `executable.rs`（pub）: native magic check（MZ / ELF / Mach-O）、shell / shim 拒否（bare `node` reject を含む）、Windows `.exe` / `.com`、POSIX `X_OK`、PATH search。
- `tree.rs`（pub）: POSIX `kill(-pgid)`、Windows CIM-snapshot + `taskkill /T`（PID-reuse protection 付き）。
- `spawn.rs`（private）: `env_clear` + explicit envs only、`CREATE_NO_WINDOW`、`process_group(0)`。
- `monitor.rs`: 結合 stdout+stderr byte cap、heartbeat、timeout / abort drive。
- `prepare.rs`: `minimal_env`、`resolve_workspace_path` + denylist + writeScope coverage。
- `cleanup.rs`: 後始末。
- `crates/cheaper-core/src/policy/process.rs` の `shell_executable_re`（:114）/ `script_ext_re`（:124）/ `argv_shell_re`（:129）regex — 現状は module-private の `fn` であり、公開済みなのは wrapper の `is_shell_process_executable`（`pub`、:136）のみ。複製せず再利用する（公開 wrapper または `pub(crate)` 化が必要）。

その他の接続点:

- `OperationDeclaration.pool` + `ResourceAccess` claims による resource / scheduling。未知の副作用には保守的に `Exclusive` access を取る。
- managed tracking（`crates/cheaper-tools/src/registry.rs` の `is_process` flag）に `command` を含め、`drain()` が spawn 済み子プロセスを待つようにする。
- invocation parity: `create_worker_scope` は `options.process` を再解決して不一致を reject する（`crates/cheaper-tools/src/invocation.rs:210-216`）。より広い grant set の密輸を防ぐため、grant にも同じ consistency check が必要。

## Alternatives and Comparison

信頼境界（project policy が grant を extend できないこと）の実現方式。

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A: project partial schema から grant field を除外 | schema-level reject で機構が単純。「invalid policy は silently ignore しない」既存方針と整合 | project で narrow すら表現できない（allowlist 縮小も不可）| 採用候補 |
| B: merge.rs に専用の grant-narrowing pass を追加 | project が grant 削除や argv / cwd / env subfield の tightening を表現できる | grant ⊆ grant の comparator が必要で merge 層が複雑化 | maybe |
| C: merge 後 validation（restrict.rs 的）で effective grant が user / builtin layer に存在しないものを reject | merge 層を触らず validation に閉じる | layer 由来の provenance tracking が別途必要 | maybe |

## Tentative Conclusions and Hypotheses

- 仮説: executable は既存の resolution + shell/shim/native check で canonical path に解決し、argv pattern は join した文字列ではなく token ARRAY に対して match する（`argv_shell_re` の metachar 拒否後）。env は `minimal_env` semantics を使う。
- 仮説: touch point は vocabulary → worker validate → catalog → registry → policy schema/merge の順に辿れ、reuse 可能な process 基盤により新規実装面積は主に policy layer に集中する。
- 確認済み: 既存 `process` の contract は不変とし、`command` は grant 不在時 default-disabled とする。

## Open Questions

- project policy から `process.tasks` の extend 権限も剥奪すべきか（現状は許可済み）。推奨: `process` の挙動を silent に変えず、divergence として明示文書化する。
- ~~Option A/B/C のどれを信頼境界に採用するか（spec / ADR で確定）。~~ **resolved（2026-10-13）**: accepted ADR `../adr/command-grant-trust-boundary-and-managed-process-reuse.md` で Option A（project partial schema から `command` の grant authoring key を全て除外し、含む project policy は load 時に `InvalidRuntimePolicy` で reject）を採用。
- grant shape の最終形（`timeout`・`killGrace`・`maxOutputBytes` の override 範囲を grant 側と tool 側でどう配分するか）。

## Scope

### In Scope

- Worker と Workflow 両方からの trusted grant 経由 dynamic executable + argv 実行。
- forbidden executable / argv / shell injection / cwd escape / project-policy grant escalation の実行前 reject。
- timeout / abort / nonzero / output-cap / tree-cleanup と Windows / POSIX 両方の test。
- global budget / resource scheduling / usage-savings 統合（未観測の変更を報告しない）。
- trust / sandbox 限界の文書化。

### Out of Scope

- 無制限 shell、interactive TTY、daemon。
- OS sandbox 無しの read-only / containment 保証。
- tool-operator worker（issue #102）。
- artifact-store への log 永続化。

## Acceptance Criteria (issue #101 転記)

- 既存 `process` contract は不変。
- `command` は grant 無しでは default-disabled。
- trusted grant 経由で Worker / Workflow 両方から dynamic executable + argv を実行できる。
- forbidden executable / argv / shell injection / cwd escape / project-policy grant escalation が実行前に reject される。
- timeout / abort / nonzero / output-cap / tree-cleanup が Windows / POSIX の test で検証される。
- global budget / resource scheduling / usage-savings と統合され、未観測の変更を報告しない。
- trust / sandbox 限界が文書化される。
- `cargo fmt` / `clippy` / `test` が pass する。

## Promotion Candidates

- [x] spec-doc needed: `command` tool の input schema、grant shape、trust boundary、acceptance criteria の製品要件。→ `../specs/policy-gated-command-tool.md`
- [x] adr-doc needed: project policy が grant を extend できない信頼境界の実現方式（Option A/B/C の決定）。→ `../adr/command-grant-trust-boundary-and-managed-process-reuse.md`

## Sources and Relations

source は issue #101。既存 process tool の認可・lifecycle 背景は `managed-process-canonical-tool` / `managed-process-lifecycle-hardening` の各 discovery と `docs/guides/managed-process.md` を参照。
