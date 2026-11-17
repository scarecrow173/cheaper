---
id: "DESIGN-034ZiAzpyOt6fMKWKB2XEE"
type: "design"
status: "approved"
title: "Workflow catalog manifest and read-only list boundary"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/201"
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
    - "../specs/workflow-catalog-issue-201.md"
    - "../adr/workflow-catalog-is-a-discovery-index-not-an-execution-authority.md"
    - "../discovery/workflow-catalog-issue-201.md"
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# Workflow catalog manifest and read-only list boundary

## Context

Epic #200 の3層導線(L0 判定 → L1 `cheaper-workflow-discover` → L2
`cheaper-workflow-create`)は、既存 Workflow の候補を共有の軽量 index から
引く。現行 CLI は `workflow interface|run <path>` の明示 path 専用で、
候補列挙手段がない。本設計は ADR-034ZixZvVK6PLmS1LMRJqu に従い、
catalog を発見用 index に限定する: catalog は Workflow 定義 file の
正本ではなく、権限・実行の根拠でもない。

## Scope

### In Scope

- `<workspace>/.cheaper/workflows/catalog.json` の schemaVersion 1
  manifest schema 契約と bounded read/validate。
- `cheaper workflow list` — read-only utility command(envelope・exit・
  option 受理・diagnostics・`--limit`)。
- manifest/entry の diagnostics 分類(malformed・unsupported version・
  bound 超過・malformed-entry・duplicate-id・missing-path・
  path-out-of-workspace・denied-path・not-regular-file)。
- create skill(#204)向け catalog 更新 file 契約と guide 反映。
- catalog unit test(cheaper-workflow filesystem_contracts)と CLI
  結合 test(`tests/workflow.rs`)。

### Out of Scope

- named lookup(`workflow run <name>`)、自動 routing、registry service、
  daemon/watcher、未登録 file の glob、compiler 実行、private
  DAG/prompt/binding の一覧出力。
- grant/policy 変更、usage/history 記録、workflow 実行。
- catalog 書込み CLI・自動修復・stale entry 自動削除(#204 以降)。
- user-global scope。

## Components and Boundaries

- `cheaper-workflow::catalog`(新規 `crates/cheaper-workflow/src/catalog.rs`)
  — manifest schema・bounded read・entry 検証・diagnostics の唯一の
  owner。Workflow file contract(loader)と同じ crate に置き、将来の
  書込み側(#204)も同じ schema/validation を再利用できるようにする。
  compiler/engine/resource inference には一切依存しない。
- `crates/cheaper-cli/src/args.rs` — `WorkflowListCliArgs { cwd, format,
  quiet, limit }` と `ParsedCli::WorkflowList` を追加。`workflow` arm に
  `list` action(positionals==2)を受理させ、許可 option は
  `cwd`/`format`/`quiet`/`log`/`log-format`/`limit` のみ。
- `crates/cheaper-cli/src/workflow_list.rs`(新規) — `execute_list`。
  `resolve_workspace_layout` → `load_policy`(filesystem policy のみ
  使用)→ `catalog::read_catalog` → utility envelope 投影。
- `crates/cheaper-cli/src/cli.rs` — dispatch arm・
  `command_name = "workflow-list"`・HELP の `workflow list` 行。
  `resolve_log_root` は `workflow` を既に `--cwd` 対象として扱うため
  変更不要。
- `cheaper-core` — 変更なし。`resolve_workspace_path`・
  `create_path_denylist`・`SymlinkPolicy` を既存のまま利用する。

## Data model

```text
catalog.json (schemaVersion 1)
  { "schemaVersion": 1,
    "workflows": [
      { "id": <string>,           // 1..=64 chars, ASCII [A-Za-z0-9._-], 先頭は英数字
        "path": <string>,         // workspace root 相対, 1..=512 chars
        "summary": <string>,      // 1 文の用途, 0..=500 chars
        "tags": [<string>]?       // optional, <=16 件, 各 <=64 chars
      } ] }
```

- bound: file <= 64 KiB、entries <= 256。超過は manifest 破損として
  fail closed。
- `path` は workspace root 相対の明示 path。`resolve_workspace_path`
  で workspace 内に解決できること、deny path に該当しないこと、
  symlink policy 適用後の実 path が regular file であることを検証する
  (compile・DAG/prompt 読込はしない)。
- `id` は scope 内一意。duplicate は両 entry を `status:"duplicate"` +
  diagnostics で報告し、黙った last-wins をしない。

### list 出力(`result`)

```text
{ "catalog": { "path": ".cheaper/workflows/catalog.json", "present": <bool> },
  "workflows": [ { "id","path","summary","tags":[...],"status" } ],
  "diagnostics": [ { "kind","index"?,"id"?,"path"?,"message" } ],
  "truncated": <bool> }
```

- `status`: `ok` | `missing` | `invalid` | `duplicate`。
  - `missing` — resolve できたが file が存在しない(stale entry)。
  - `invalid` — path escape・deny path・not regular file・空/不正
    shape で entry として成立しないが manifest 内に存在するもの。
  - `duplicate` — 同一 `id` が複数 entry に出現。
- `diagnostics[].kind`: `malformed-entry` | `duplicate-id` |
  `missing-path` | `path-out-of-workspace` | `denied-path` |
  `not-regular-file`。
- manifest 欠如 → `catalog.present:false`、`workflows:[]`、
  `diagnostics:[]`、exit 0。
- manifest 破損(JSON parse 失敗・`schemaVersion` 非対応・bound 超過・
  `workflows` 非 array)→ error envelope + 非 0 exit。
- `--limit`(default 全件 = manifest bound 内)`workflows` を打ち切り、
  `truncated:true` を立てる。

## Data and Control Flow

```text
cheaper workflow list [--cwd] [--limit] [--format] [--quiet]
  -> parse_cli_args (workflow arm, action=list)
  -> dispatch ParsedCli::WorkflowList
  -> execute_list
       resolve_workspace_layout(cwd) -> {root, dirs, builtin_dir}
       load_policy(builtin+user+project) -> policy.filesystem
       catalog::read_catalog(root, policy.filesystem)
         read <root>/.cheaper/workflows/catalog.json (bounded)
           absent -> present:false empty read
           parse -> schemaVersion/bounds check (fail -> error)
           per entry: shape -> id uniqueness -> path resolve/deny/stat
       project -> utility_success_envelope(result)
  -> write_output (json default / text)
  -> exit 0 (error envelope のとき error.exitCode)
```

## Non-Functional Constraints

- read-only: file 書込み・usage/history 記録・hook・network・model を
  一切行わない。
- bounded: manifest 64 KiB / 256 entries 以下のみ処理対象。list の
  入出力は常に小さく、親 Agent の context 消費を抑える。
- consistent boundary: entry path 検証は loader と同じ
  `resolve_workspace_path` + `policy.filesystem` 入力を使い、独自の
  escape/symlink 判定を持たない。
- fail closed: manifest 破損は空カタログと混同できない error とする。

## Risks and Trade-offs

- entry が壊れていても一覧は返す(ok:true + diagnostics)設計は、
  manifest 破損より多くの情報を consumer へ渡す反面、「一部 entry が
  壊れた catalog でも正常系」と誤読される余地がある → `status` と
  `diagnostics` を分離し、consumer が機械判定できる形にする。
- catalog.json を hand-edit する運用(#204 までは唯一の更新手段)は
  schema 違反を書き込める → file 契約 doc に検証規則と `workflow
  list` での確認手順を明記する。

## Deferred Design Concerns

- catalog 書込み CLI(`workflow register` 等)| #204 の L2 create 設計
  で再評価。file 契約のみで開始し、必要なら同 schema/validation を
  再利用する CLI 補助を追加する。
- user-global scope | loader が workspace 境界を維持する限り実行
  経路と整合しないため分離。

## Verification Notes

- [ ] `workflow list` が no-catalog workspace で
  `{present:false, workflows:[], diagnostics:[]}` を返す。
- [ ] malformed manifest が INVALID_INPUT 系 error envelope + 非 0。
- [ ] duplicate id・missing path・path escape が status/diagnostics
  に出て、他 entry を黙って選ばない。
- [ ] `--limit` の打ち切りで `truncated:true`。
- [ ] 一覧が compiler/worker/tool/model を起動しない(副作用なし)。
