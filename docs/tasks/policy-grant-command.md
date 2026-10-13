---
id: "TASK-034FpqibELWYvjxZennqEn"
type: "task"
status: "done"
title: "T3: policy-grant-command — cheaper policy grant {add,list,remove} と core write 基盤"
created: "2026-10-13"
updated: "2026-10-13"
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
    - "docs/plans/implement-invocation-scoped-command-grants.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-invocation-scoped-command-grants.md"
    - "docs/tasks/invocation-policy-layer.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/invocation-scoped-command-grants.md"
  references: []
  defers: []
  deferred-by: []
---

# T3: policy-grant-command — cheaper policy grant {add,list,remove} と core write 基盤

## Scope

user-scope `<userConfigDir>/policy.yaml` の `command.grants` を
validation 付きで管理する utility command `cheaper policy grant
{add,list,remove}` と、その前提となる core write 基盤（`read_policy_yaml`
公開・`stringify_yaml`・`atomic_write` の core 移動）を実装する
（design §(g)(h)）。`ParsedCli::Policy` variant の追加は
`format()`/`quiet()`/`command_name`/dispatch の exhaustive match を
全て compile-forced にするため、parse 表面（`PolicyCliArgs`/
`PolicyAction`/accessor/enumeration/`policy` arm の `--grant` reject）と
dispatch を本 task で一体実装する。`cheaper policy` は utility command
規約 — `record_usage` を呼ばず、success envelope `result` に top-level
`tokens` key を載せない（AC9・AC16）。

- In: core write 基盤（`read_policy_yaml`・`stringify_yaml`・
  `YamlError` pub 化・`pub mod yaml`・`fs_util` の runtime → core 移動
  + runtime caller 更新）、args.rs の `policy` parse 表面、cli.rs
  dispatch + `command_name` + HELP、`src/policy.rs` の
  add/list/remove、`tests/policy.rs`、`tests/args.rs` の policy arm
  parse table（`policy ... --grant` reject で AC17 完結）。
- Out: model 出力経由・project policy 経由の grant authoring（経路
  非存在を維持）、`--grant-file`、`command.grants` 以外の section
  管理・project policy 対象化、run 経路の行為側検証（T5 —
  `tests/policy.rs` は dev-seam 非 gate target で model session を
  持てないため `run_dev_seam.rs` 側が所有）、docs（T6）。

### 所有ファイル

- `crates/cheaper-core/src/policy/load.rs` — `read_policy_file` を
  `pub fn read_policy_yaml` として公開（`LayerKind`/`read_yaml_layer`
  は `pub(crate)` のまま）。
- `crates/cheaper-core/src/yaml.rs` — `pub fn stringify_yaml` 追加 +
  `YamlError` の `pub` 化（`#![allow(deprecated)]` module scope は
  既存のまま）。
- `crates/cheaper-core/src/lib.rs` — `pub(crate) mod yaml` →
  `pub mod yaml`（:24）、`pub mod fs_util` 追加。
- `crates/cheaper-core/src/fs_util.rs`（新規）— runtime から移動した
  `atomic_write` / `AtomicWriteOptions`（`pub` — 複製しない。parent
  dir を作らない caller 責務の doc 規約は runtime `fs_util.rs:23-25`
  のまま維持）。
- `crates/cheaper-runtime/src/{fs_util.rs,lib.rs}` — `fs_util.rs`
  削除 + `mod fs_util`（lib.rs:35）除去。
- `crates/cheaper-runtime/src/{auth/store.rs,catalog/persisted.rs}` —
  `crate::fs_util::` → `cheaper_core::fs_util::` 参照へ機械的書き換え
  （:263-271・:107-115）。
- `crates/cheaper-cli/src/args.rs` — `PolicyCliArgs`/`PolicyAction`
  （`GrantAdd{name,spec}`/`GrantList`/`GrantRemove{name}`）/
  `ParsedCli::Policy` + format()/quiet() accessor arm（:181-204）+
  `policy` arm parse（`add <name> <spec-json>` 4 positional /
  `list` 2 / `remove <name>` 3 受理・grant name
  `^[a-z][a-z0-9_-]{0,63}$` 検査・utility flag allowlist 方式で
  `--grant`/`--worker-file`/`--cwd`/`--input`/`--timeout` 等を
  reject）+ `command required:` enumeration（:484）へ `policy`。
- `crates/cheaper-cli/src/cli.rs` — `ParsedCli::Policy` dispatch arm
  （utility command 規約、`cheaper.cli.command` span instrument・
  `record_usage` 非呼出し）、`command_name`（:450-462）に `"policy"`、
  HELP `Commands:` に `policy grant` 3 形態。
- `crates/cheaper-cli/src/policy.rs`（新規）—
  `pub async fn execute(&PolicyCliArgs, &mut CliIo, &CliHooks, f64) ->
  Result<CommandOut, BoxError>`。`resolve_user_config_dir`
  （`UserConfigDirOptions { env, homedir: None }`）で user scope 解決。
  add: spec parse → `read_policy_yaml`（absent は `{"version":1}`
  base）→ 構造前提 pre-check（非 object `command`/`command.grants`・
  `version` 欠落/≠1 は `InvalidRuntimePolicy`、既存名は
  `InvalidRequest` + `details.grant`）→ merged full-object
  `validate_policy_schema(Partial)` → `stringify_yaml` →
  `atomic_write`（`windows_remove_first: false` + POSIX
  `unix_mode: Some(0o600)` — `<userConfigDir>` 不在時は先に
  `create_dir_all`、dir 作成・write の io error は `InternalError` +
  `details {file, ioError}` — `auth/store.rs::io_error` :275-283 と
  同一 shape）→ `{grant:{name,executable}}` envelope。list: name 順
  sort の `{grants:[{name,executable}]}`、file absent は空配列。
  remove: 同 pre-check + delete + 同 full-object validation + 同
  write path、absent name は `InvalidRequest`。
- `crates/cheaper-cli/src/lib.rs` — `pub mod policy;`。
- `crates/cheaper-cli/tests/args.rs` — `policy grant` positional parse
  table（3 形態受理 + 形状違反・name pattern 違反 reject）+ flag
  allowlist reject（`policy` arm の `--grant` reject — AC17 完結）。
- `crates/cheaper-cli/tests/policy.rs`（新規 — dev-seam 非 gate の
  utility-command test target、`required-features` 無し）— temp
  `CHEAPER_CONFIG_DIR` で add → list → remove round-trip、invalid
  spec（deny 該当・argv compile 失敗・schema 違反）の file 不変 +
  error envelope、`version` 欠落/≠1・非 object `command`/
  `command.grants` の `InvalidRuntimePolicy` + file 不変、存在しない
  name の remove が `InvalidRequest`、既存名 add が `InvalidRequest` +
  `details.grant`、`command` 以外の section 保存、`result.tokens`
  不在（AC16）、usage.jsonl 非記録（AC9）、POSIX `0600` write-mode。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-invocation-scoped-command-grants.md` — 実装対象
  plan（T3）
- `docs/designs/invocation-scoped-command-grants.md` — §(g)(h) の設計判断
- `docs/specs/invocation-scoped-command-grants.md` — AC6・AC7・AC9・
  AC16・AC17
- `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
  — `cheaper policy` UX・`read_policy_yaml`/`stringify_yaml`/
  `atomic_write` 基盤・write caveat（last-wins・comment 非保存）
- `docs/test-specs/invocation-scoped-command-grants.md` — 検証 spec
  （write path validation・utility command 規約の行為保証）

## Dependencies

- `docs/tasks/invocation-policy-layer.md`（T1 — `load.rs` 共有 file。
  `read_policy_yaml` 公開は layer 実装と同 file のため T1 着地後に
  着手する）

## Steps

1. core write 基盤を整備する — `read_policy_yaml` 公開・
   `yaml.rs` の `stringify_yaml` + `YamlError` pub 化・`pub mod yaml`
   昇格・`fs_util`（`atomic_write`/`AtomicWriteOptions`）の runtime →
   core 移動と runtime caller 2 site の参照更新 + `mod fs_util` 除去。
2. args.rs に `policy` parse 表面（`PolicyCliArgs`/`PolicyAction`/
   `ParsedCli::Policy`・accessor arm・positional 受理 + name 検査 +
   flag allowlist・enumeration）を実装し、cli.rs に dispatch arm +
   `command_name` + HELP を追加する。
3. `src/policy.rs` の `execute`（add/list/remove）を実装する —
   write 前に merged full-object `validate_policy_schema(Partial)`
   を通し、invalid spec は file を一切変更しない。
4. `tests/policy.rs` と `tests/args.rs` の parse table を整備する。
5. core・cli・runtime（`fs_util` 移動 caller）の test と workspace
   build が green であることを確認する。

## Done When

- [x] `policy grant add/list/remove` の round-trip が temp
  `CHEAPER_CONFIG_DIR` で動作し、add 済み grant が user
  `policy.yaml` に `0600`（POSIX）+ atomic write で永続化される
- [x] invalid spec・構造前提違反・存在しない name が file 不変 +
  正しい error 分類（shared validation 由来は `InvalidRuntimePolicy`、
  入力構造の失敗は `InvalidRequest`）で返る
- [x] success envelope `result` に `tokens` key が無く、
  usage.jsonl に record が残らない（AC9・AC16）
- [x] `policy` arm の flag allowlist が `--grant` 等を `InvalidRequest`
  で reject する（AC17 完結）
- [x] `fs_util` 移動後の runtime caller（credentials・persisted
  catalog）test が green で半移動状態を残さない

## Rollback

- task 単位の coherent commit を `git revert`。`fs_util` core 移動を
  revert する場合は core file 削除 + runtime 側 `mod fs_util` 復元 +
  caller 2 site の import 復元を一体で戻す（半移動状態を残さない）。
  `policy` command は flag/plumbing（T2）から独立して revert 可能。
- revert 後に
  `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  が pass すること。

## Verification

- [x] `mise exec -- cargo test -p cheaper-cli --test policy` exits 0
- [x] `mise exec -- cargo test -p cheaper-cli --test args` exits 0
- [x] `mise exec -- cargo test -p cheaper-core` exits 0
- [x] `mise exec -- cargo test -p cheaper-runtime --features cheaper-runtime/dev-seam` exits 0（`fs_util`/`atomic_write` の移動 caller は `cheaper-runtime` 側 `auth/store.rs`・`catalog/persisted.rs` — core/cli の test だけでは半移動を検出できないため必須）
- [x] `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam` exits 0
