---
id: "IMPL-034ZjfVbK9rQxNsPmHdTu3"
type: "impl"
status: "completed"
title: "Add workflow list CLI command"
created: "2026-11-17"
updated: "2026-11-17"
owners: []
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
    - "docs/tasks/add-workflow-list-cli-command.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Add workflow list CLI command

## Summary

`cheaper workflow list` を read-only utility command として実装した。
`{catalog:{path,present}, workflows, diagnostics, truncated}` を
`worker:null` の utility envelope で返し、manifest 破損は error
envelope + 非 0 exit。`interface|run` の grammar・envelope・exit は
不変。

## Context

`TASK-034ZiG2kRrXqHk9bVf8pQs`。approved plan の slice 2。
`read_catalog` pub API(slice 1)の上に CLI 境界を載せる。

## Implementation

### Changes

- `args.rs`: `WorkflowListCliArgs{cwd,format,quiet,limit}` と
  `ParsedCli::WorkflowList` を追加。`workflow` arm で `list`
  (positionals==2)を受理し、allowed option は
  `cwd/format/quiet/log/log-format/limit` のみ。`--limit` の値検証は
  history と同様 dispatch 側で行う。bare `workflow` の error message は
  `workflow requires list | interface|run <path>` へ更新。
- `cli.rs`: `history_limit` の数値検証を `pub(crate) fn
  parse_limit(value, max)` へ抽出し `workflow list` と共有。
  dispatch arm は `record_usage` を呼ばない utility 規約。
  `command_name` → `workflow-list`、HELP の Commands/Options に
  `workflow list` と `--limit <1..256>` を追加。
- `workflow_list.rs` 新規: `resolve_workspace_layout` → `load_policy`
  (builtin+user+project, invocation: None) → `read_catalog` →
  `utility_success_envelope` で result を投影。`--limit` で
  `workflows` を打ち切り `truncated:true` を立てる(`diagnostics` は
  全件・index は manifest 順)。
- `tests/workflow.rs`: 6 test 追加。no-catalog 空一覧・public field
  射影と副作用なし(workspace/config tree snapshot + usage.jsonl 非作成
  + private key 非漏出)・malformed manifest の error envelope・
  entry diagnostics(duplicate/missing/escape)・`--limit` truncated・
  `--cwd` 別 workspace。
- `tests/integration/args.rs`: `workflow list` grammar 受理・
  他 command option の reject・`interface|run` allowlist 不変を固定。
- 非 history command 向けの結合 error message を分割:
  `--kind/--status/--cursor are only valid for history` と
  `--limit is only valid for history or workflow list`
  (`workflow list` が `--limit` を受理するようになったため従来文言は
  不正確だった)。

### Decisions During Implementation

- `execute_list` は `worker:null` の utility envelope で返し、exit
  code は成功時 0 固定(error は汎用 catch の error envelope が担う)。
- `workflow_list.rs` は policy 以外の workflow_context 部品(tool
  catalog・worker index・inference context)を構築しない。filesystem
  policy のみ必要な最小構成とし、read-only 境界を構造で担保する。

## Related Experiments

### Adopted

なし。

### Rejected

なし。

## Validation

- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test workflow` —
  25 pass。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
  args::` — 51 pass。integration 全体 394 pass。
- `cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  pass。
- `cargo fmt --all -- --check` — pass。
- scratch workspace への実 binary 実行で ok/missing/invalid 表示・
  `--limit` truncated・malformed `INVALID_INPUT` exit 2・config dir
  非作成を確認。

## Risks

- `workflow list` は catalog manifest のみを読み、`.cheaper/workflows/`
  内の未登録 workflow file は列挙しない(spec の scope どおり)。
  discover(#203)が未登録資産を探す場合は別経路が必要。

## Follow-ups

- task(c)で `docs/guides/` へ catalog 契約と `workflow list` 使用法を
  記載する。
