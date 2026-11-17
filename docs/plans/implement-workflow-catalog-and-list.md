---
id: "PLAN-034ZiDqt5yJE77dMHMvF3G"
type: "plan"
status: "approved"
title: "Implement workflow catalog and read-only list"
created: "2026-11-17"
updated: "2026-11-17"
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
  implements:
    - "docs/specs/workflow-catalog-issue-201.md"
    - "docs/adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
  implemented-by:
    - "docs/tasks/add-workflow-catalog-manifest-read-and-diagnostics.md"
    - "docs/tasks/add-workflow-list-cli-command.md"
    - "docs/tasks/document-workflow-catalog-contract-and-usage.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/workflow-catalog-issue-201.md"
    - "docs/adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
    - "docs/designs/workflow-catalog-manifest-and-read-only-list.md"
    - "docs/discovery/workflow-catalog-issue-201.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/workflow-catalog-manifest-and-list-contract.md"
  references: []
  defers: []
  deferred-by: []
---

# Implement workflow catalog and read-only list Implementation Plan

## Goal

project scope の Workflow catalog(`.cheaper/workflows/catalog.json`)と
read-only `cheaper workflow list` を実装し、Epic #200 の discover/create
導線が使う bounded な発見用 index を提供する。

## Scope

1 つの implementation stream とする。manifest schema/validation、CLI
grammar/dispatch/envelope、test、user doc は同一の read-only index
contract を共有するため分割しない。

境界:

- `workflow interface`/`workflow run` の runtime 挙動・envelope・exit
  code は変更しない。
- catalog から grant/policy 変更・workflow 実行・named lookup・
  compiler/worker/tool/model の起動を一切導出しない。
- catalog 書込み CLI・自動修復・user-global scope は作らない。

## Upstream Documents

- Approved requirements: `../specs/workflow-catalog-issue-201.md`
- Accepted boundary decision: `../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md`

## Design Inputs

- Approved implementation boundary: `../designs/workflow-catalog-manifest-and-read-only-list.md`
- Source exploration and alternatives: `../discovery/workflow-catalog-issue-201.md`

## File Map

| Files | Owner slice | Responsibility |
| --- | --- | --- |
| `crates/cheaper-workflow/src/catalog.rs` (new), `crates/cheaper-workflow/src/lib.rs` | Catalog schema/boundary | manifest parse・bound 検査・entry 検証・diagnostics |
| `crates/cheaper-workflow/tests/filesystem_contracts/catalog.rs` (new), `crates/cheaper-workflow/tests/filesystem_contracts/main.rs` | Catalog contract tests | manifest/entry/diagnostics/boundary の unit 検証 |
| `crates/cheaper-cli/src/args.rs` | CLI grammar | `WorkflowListCliArgs`、`workflow list` action、option allowlist |
| `crates/cheaper-cli/src/workflow_list.rs` (new), `crates/cheaper-cli/src/lib.rs`, `crates/cheaper-cli/src/cli.rs` | Command/envelope | `execute_list`、dispatch arm、`command_name`、HELP |
| `crates/cheaper-cli/tests/workflow.rs` | CLI integration tests | `workflow list` の envelope/exit/diagnostics/`--limit`/boundary fixture |
| `crates/cheaper-cli/tests/integration/args.rs` | Grammar tests | `workflow list` の受理/拒否 option 組合せ |
| `docs/guides/usage.md`, `docs/guides/parent-agent-integration.md`, `docs/guides/workflow-usage.md` | User docs | `workflow list` 契約、catalog file 契約(#204 向け)、保存場所規約 |

File ownership は task ごとに排他とする。catalog core と CLI surface は
別 task、docs は独立 task とし、code slice 完了後に行う。

## Implementation Sequence

1. `cheaper-workflow/src/catalog.rs` を追加する。manifest 定数
   (`CATALOG_SCHEMA_VERSION`・`CATALOG_RELATIVE_PATH`・bound 群)、
   `CatalogEntry`/`CatalogRead`/`CatalogDiagnostic`/`EntryStatus` 型、
   bounded read、manifest-level fail-closed 検証、per-entry
   resolve/deny/stat 検証、duplicate-id 診断を実装し、`lib.rs` から
   export する。
2. `filesystem_contracts/catalog.rs` test を追加し、absent/malformed/
   version/bound/entry-status/duplicate/escape/deny/not-file の分岐を
   fixture で固定する。
3. `args.rs` に `WorkflowListCliArgs` と `list` action を追加する。
   `workflow` arm で `interface|run` は `positionals.len()==3`、`list` は
   `==2` とし、`list` の allowed option は
   `cwd`/`format`/`quiet`/`log`/`log-format`/`limit` のみ。
4. `workflow_list.rs` に `execute_list` を実装する:
   `resolve_workspace_layout` → `load_policy` →
   `catalog::read_catalog` → `utility_success_envelope` 投影。
   `cli.rs` に dispatch arm・`command_name`・HELP 行を追加する。
5. `tests/workflow.rs` に `workflow list` fixture を追加し、
   no-catalog・正常一覧・diagnostics・`--limit`・error envelope を
   検証する。`tests/integration/args.rs` に grammar 受理/拒否を追加。
6. `docs/guides/` を更新し、`mise run check` を実行する。

## Dependency Graph

- Step 1 → Step 3/4: catalog module が無いと CLI は投影するものが無い。
- Step 2 は Step 1 に、Step 5 は Step 3-4 に、Step 6 は Step 1-5 に
  依存する。
- Step 3 の args 変更と Step 4 の command module は同一 task で行う
  (parse 無しに dispatch できず、逆も同じ)。

## Dependencies and Risks

| Risk | Likelihood | Impact | Mitigation |
| --- | --- | --- | --- |
| diagnostics 専用の path/escape 判定を複製し loader と drift する | Medium | High | `resolve_workspace_path` + `create_path_denylist` + `policy.filesystem` の既存入力だけを使い、独自 escape/symlink 判定を持たない |
| catalog を「権限/実行の根拠」に見せてしまう表現や API が混入する | Medium | Medium | module は read のみ。type 名・doc comment・guide で「発見用 index」語彙を固定し、named lookup/register API を追加しない |
| manifest の読取りが unbounded になり大きな file/entry で context/メモリを浪費する | Low | Medium | 64 KiB/256 entries/field 長 cap を parse 前に適用し、超過は error envelope |
| workflow arm の `interface|run` grammar を壊す | Low | High | `list` は `positionals.len()==2` の独立 action として追加し、既存 shape check を変えない。args 単体 test で両 action を固定 |
| private jobs/prompt/binding が list 出力に混入する | Low | High | 出力は manifest field + status のみ。entry path の file 内容は読まない |

## Task Breakdown

plan 承認後、owner slice ごとに task document を作成する:
(a) catalog manifest read/diagnostics(cheaper-workflow + unit test)、
(b) `workflow list` CLI(args + command + integration test)、
(c) catalog file 契約と guide 文書化。
各 task は対象 fixture を owner code 変更前に追加し、named focused
command を実行する。canonical `mise run check` は integration 完了時に
1 度だけ実行する。

## Verification

| Slice | Sequence step | Command or check | Pass condition |
| --- | --- | --- | --- |
| Catalog manifest/read | 1-2 | `mise exec -- cargo test -p cheaper-workflow --test filesystem_contracts catalog::` | absent→empty、malformed/version/bounds→error、missing-path/escape/deny/not-file→status+diagnostics、duplicate-id→両 entry 診断、compiler 非起動 |
| CLI grammar | 3 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam args::` + `integration args::` | `workflow list` が受理、`interface|run` が従来通り、`list` への他 command option が INVALID_REQUEST |
| Command/envelope | 4-5 | `mise exec -- cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test workflow` | utility envelope `{catalog, workflows, diagnostics, truncated}`、exit 0/error envelope、`--limit` 打ち切り、no-catalog 空一覧、private field 非出力 |
| Docs | 6 | doc_status audit + guide 目視 | `workflow list` と catalog file 契約が usage/parent-agent-integration/workflow-usage に記載、blocking finding なし |
| Full canonical | 6 | `mise run check` | exit code 0 |

## Rollback Strategy

- Trigger: `workflow interface`/`workflow run` の observable behavior
  が変わった場合、catalog read が workflow file/private data を読んだ
  場合に rollback する。
- feature branch の #201 実装 commit のみを revert する。read-only
  機能で永続 data を生成しないため cleanup 対象は無い。
- `git status --short`、workflow integration test、`mise run check`
  で rollback を検証する。

## Open Questions / Gaps

- blocking な gap は無い。diagnostic `kind` の個別名・bound 具体値
  (64 KiB/256/64/512/500/16)・`--limit` default は approved design の
  data model の範囲内で task が確定する。

## Self-Review

- [x] Every spec requirement maps to a task.
- [x] No placeholders remain.
- [x] File paths, relations, status values, and terminology are consistent.

## Implementation Handoff

- Recommended mode: delegated/subagent-capable implementation。task (a)
  と (b) は file set が disjoint であり、(a) の型契約確定後に並行可能。
  task (c) は code slice 完了後に行う。
- Independent work streams: task (a) catalog core と task (b) CLI
  surface は、catalog の pub 型契約が確定すれば並行して進められる。
  実装上は単一 owner が (a) → (b) → (c) の順で行う方が interface
  drift を避けられるため、subagent は 1 本に絞る。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation capabilities in the current environment, then execute task-by-task with review checkpoints.
- If declined or unavailable: execute inline using the same dependency graph and verification matrix.

## Review Handoff

- Assumptions: 新規 dependency は追加しない。`policy.filesystem` の
  symlink/deny 入力は resolved policy から取り、catalog 専用の policy
  解釈を持たない。manifest は project scope のみ。
- Review focus: entry path 検証が `resolve_workspace_path` +
  `create_path_denylist` の既存入力のみを使うこと、出力が manifest
  field + status のみで private data を含まないこと、fail-closed と
  diagnostics の二分が design どおりであること。
