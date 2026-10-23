---
id: "TSPEC-034FoDgwSnThCuTO5aNUYj"
type: "test-spec"
status: "approved"
title: "invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant の行為保証"
created: "2026-10-13"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/105"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "../test-specs/policy-gated-command-tool.md"
    - "../test-specs/tool-operator-built-in-worker.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "../plans/implement-invocation-scoped-command-grants.md"
    - "../specs/documentation-honesty-sweep-and-breaking-change-migration-guide.md"
    - "../adr/documentation-honesty-sweep-and-migration-guide.md"
    - "../guides/migration.md"
    - "../designs/documentation-honesty-sweep-and-migration-guide.md"
  refines: []
  refined-by: []
  derives-from:
    - "../designs/invocation-scoped-command-grants.md"
  derived-by: []
  verifies:
    - "../specs/invocation-scoped-command-grants.md"
  verified-by: []
  references: []
  defers: []
  deferred-by: []
---

# invocation-scoped command grant（--grant）・grantRequests・cheaper policy grant の行為保証

## Purpose

issue #105 で追加される 3 component —— invoker の CLI 引数から run-lifetime の grant を供給する repeatable `--grant` flag（合成 `invocation` policy layer）、tool-operator output の machine-readable `grantRequests[]`、user-scope `policy.yaml` を管理する `cheaper policy grant {add,list,remove}` —— の行為保証を固定する。保護する intent は「grant の供給・要求・永続化経路を増やしつつ、trusted-layer-only authoring 境界（model 入力・worker prompt・project policy は grant を authoring 不能）と `resolve_command_policy` 系統の single authoritative validation、`command` tool の実行時認可意味論（deny list・argv pattern・executable pin・env/cwd 規則・managed-process lifecycle）は一切緩めない」という一点にある。`--grant` の grant は永続化されず、invocation-sourced grant は policy-sourced grant と audit surface で区別できなければならない。`--grant` flag・`grantRequests` field・`cheaper policy` command が廃止されるか、合成 invocation layer 機構（ADR-034FlTS3BaiV3WKz8bX1Xv）が改訂されたときにのみ退役可能。

> （issue #169 注記 — deny 検査は opt-in `security.execDenyPatterns`（既定 `[]`）宣言時のみ効く機構へ条件化（#164）。argv pattern・executable pin・env/cwd 規則・lifecycle の意味論不変は維持）

## Feature

Feature: `cheaper run`・全 worker alias・`cheaper workflow run` が `--grant <json>` を repeatable に受け付け、flag 由来の grant は `load_policy` の user merge 直後に合成 `invocation` layer として `MergeMode::Replace` で畳み込まれ、当該 run で user-policy grant と同一の runtime 挙動を持つ一方、policy file に一切永続化されない。構造違反（malformed JSON・`name` 欠落/pattern 違反・同名重複・grants 以外の key・policy との collision）は `InvalidRequest` + `details.layer="invocation"`、shared validation 由来の失敗（deny 該当・argv compile・schema 違反）は inner code `InvalidRuntimePolicy` + `details.layer="invocation"` で run 開始前に fail する。tool-operator は gated tool 欠如由来の `blocked`/`partial` で `grantRequests[]` を emit でき、`cheaper policy grant` は validation 済み grant を user policy へ atomic に永続化・列挙・削除する utility command である。

> （issue #169 注記 — deny 該当による shared validation 失敗は `security.execDenyPatterns` 宣言時のみ（#164 — 既定 `[]` では deny 検査は非活性））

## Rules

- Rule: `--grant` は `run`・全 worker alias（`ParsedCli::Run` fold）・`workflow run` で repeatable に受理され、`--grant <v>` / `--grant=<v>` の両受理形と複数指定の宣言順が `grants: Vec<String>` に保持される。flag 値は 64KiB 上限で、超過は `InvalidRequest`（exit 2）（AC10・AC15 の parse 面）。対象: `crates/cheaper-cli/tests/args.rs` — `run_grant_flag_repeatable`（両形・複数指定・宣言順）、`run_alias_accepts_grant`（`tool-operate --grant` → `ParsedCli::Run` に fold）、`workflow_run_accepts_grant`、`grant_value_over_64kib_is_rejected`。
- Rule: 非 run 系 command への `--grant` は全 arm で `InvalidRequest`（exit 2）の明示 reject — `usage` / `count-tokens` / `models` / `auth` / `policy` / `workflow interface`。`Vec<String>` の default-empty 受け皿に依存した silent drop は許されず、command ごとの assert で guard の抜けを固定する（AC17）。対象: `tests/args.rs` — `grant_rejected_on_non_run_commands`（6 command の table 駆動 assert）。`policy` arm の `--grant` reject は `policy` command の flag allowlist（`--format` / `--quiet` / `--log` / `--log-format` のみ受理、`--worker-file` / `--cwd` / `--input` / `--timeout` 等も reject）と一体で `policy_grant_flag_allowlist` が担う。なお `cheaper hook` は `parse_cli_args` に到達しない独立 parse path（`cli.rs` の early branch → `hook::parse_hook_argv` が `--target` のみ認識し残りを fail-open で silent 無視）を持つため `--grant` も silent ignore される — `hook` は run 系 command でなく本 rule の reject 契約対象外の spec 適合 known gap。
- Rule: `policy grant` の parse 表面は `add <name> <spec-json>`（4 positional）/ `list`（2）/ `remove <name>`（3）のみ受理し、それ以外の形状は `InvalidRequest`。grant name は `^[a-z][a-z0-9_-]{0,63}$`（grant `propertyNames` と同一）を parse 段で検査する。対象: `tests/args.rs` — `policy_grant_parse_table`（3 形態の受理 + 形状違反・name pattern 違反の reject）。
- Rule: `build_invocation_layer` は各 flag 値を complete JSON object として parse し、`name` key（存在・string・name pattern 必須）を strip して `command.grants.<name>` へ畳み、`{"version":1,"command":{"grants":{...}}}` + 宣言順 names を返す。malformed JSON・`name` 欠落/非 string/pattern 違反・intra-invocation 同名重複は `InvalidRequest` + `details.layer="invocation"`（name 関与時は `details.grant`）。`name` を残したまま schema へ流すと grant spec の `additionalProperties: false` で必ず失敗するため strip は不変条件。`grants` 空なら `(None, vec![])`（AC13・AC15 の組立面）。対象: `crates/cheaper-cli/src/grant.rs` `#[cfg(test)]` unit test — `builds_layer_and_strips_name`、`rejects_malformed_json`、`rejects_missing_or_invalid_name`、`rejects_duplicate_names`（AC13）、`empty_returns_none`。
- Rule: `load_policy` の `invocation` layer は user merge 直後に挿入され、(1) grants-only key 制約（top-level ⊆ `{version, command}` かつ `command` ⊆ `{grants}` — 違反は `InvalidRequest` + `details.layer="invocation"`）、(2) `PolicySchemaMode::Partial` validate（内部で `resolve_command_policy` が走り deny list・`ArgPattern::compile`・env/cwd 規則を継承 — 失敗は inner code `InvalidRuntimePolicy` 保持 + `details.layer` 畳み込み）、(3) builtin+user 済み `command.grants` との key collision 検査（`InvalidRequest` + `details.grant` + `details.layer`）、(4) `MergeMode::Replace` merge の順序で処理される。validate 通過後に collision 検査を行い、malformed spec が collision error を装う順序を避ける（AC3・AC4 の layer 意味論 — plan T1 境界）。対象: `crates/cheaper-core/src/policy/load.rs` unit test — `invocation_layer_merges_after_user`（user grant + invocation grant が共存し typed re-resolve 後の `RuntimePolicy.command` に畳まれる）、`invocation_layer_rejects_non_grant_keys`（`{"version":1,"tools":{...}}` や `command: {timeout: ...}` の第 2 trusted-channel 化を `InvalidRequest`+`layer` で拒否）、`invocation_layer_denied_executable_keeps_inner_code`（deny 該当 executable → `InvalidRuntimePolicy` + `details.layer="invocation"`）、`invocation_layer_collides_with_user_grant`（AC4 — `InvalidRequest` + `details.grant` + `details.layer`）、`invocation_grant_survives_project_layer`（project partial は `command` section を schema reject するため invocation grant を override 不能 — trusted boundary の不変条件）。
  > （issue #169 注記 — deny 継承は `security.execDenyPatterns` 宣言時のみ（#164）。`RuntimePolicy.command` は #165 で `RuntimePolicy.exec` へ rename — file-level `command.grants` shape は有効のまま）
- Rule: `policy_load` golden は harness が `opts.invocation` fixture key を `LoadPolicyOptions.invocation` へ渡す経路を持ち、fixture `policy/load.json` に invocation case を追加する — user への追加（accept）・merge ordering・collision reject・grants 以外 key の reject・deny 該当 executable（`details.layer` を含む error shape を golden で固定）。既存 case の `expected` は不変（plan T1）。対象: `crates/cheaper-core/tests/golden/policy_load.rs` + `tests/golden/fixtures/policy/load.json`。
- Rule: grant 無しで `command` が `cheaper.optionalTools` opt-in 縮退した worker への `--grant` 付き re-run は、flag JSON 経由の grant で `command` を実登録し scripted `command` call が実 subprocess を spawn する — grant の runtime 挙動（argv pattern・executable pin・grant env 伝播）は user-policy grant と同一（AC1）。対象: `crates/cheaper-cli/tests/run_dev_seam.rs` — `run_fake_stream_grant_flag_un_degrades_command_and_spawns`。fixture: `opt_in_fixture` / `OPT_IN_COMMAND_WORKER_MD` + `FIXTURE_COMMAND_CALL`、grant spec は `command_user_policy(marker)` と同型の `command_grant_flag(marker)` helper が組み立てる flag JSON（executable = test binary 自身、`env` に `CHEAPER_PROC_CHILD_MODE=probe` + marker path）。assert: exit 0、`meta.toolCalls == 1`、marker file に `"ran"`（実 spawn + minimal env 経由の grant env 伝播の証明）。
- Rule: `--grant` は一切永続化しない — granted run の後に flag 無しで同一 run を再実行すると再び縮退し（`command` call は未登録で `meta.toolCalls` に計上されない）、`<CHEAPER_CONFIG_DIR>/policy.yaml` は run のどの時点でも書き込まれない（AC2）。対象: `run_dev_seam.rs` — `run_fake_stream_grant_flag_is_not_persisted`。assert: 2 run 目で `command` 縮退へ復帰（degraded call は計上されない・result が `blocked` 相当）、`f.cfg.path().join("policy.yaml").exists() == false`。
- Rule: flag 由来 grant と builtin/user policy の同名 grant の衝突、および同一 invocation 内の同名 `--grant` 重複は、run 開始前に `InvalidRequest`（exit 2、`INVALID_REQUEST`）+ `details.grant=<衝突名>` + `details.layer="invocation"` で fail する — silent 上書き・last-wins は存在しない（AC4・AC13 の run 経路）。対象: `run_dev_seam.rs` — `run_fake_stream_grant_collides_with_user_policy`（`<cfg>/policy.yaml` に grant `self` を seed + `--grant` に同名 spec → error envelope assert）、`run_fake_stream_grant_duplicate_name_rejected`（同名 2 指定 → `InvalidRequest`）。
- Rule: deny list 該当 executable（`is_denied_command_executable` / `is_shell_process_executable` — interpreter・exec-wrapper・shell/script shim）を含む `--grant` spec は run 開始前に reject され、invocation 経路は deny list を迂回しない。error は shared validation 由来として inner code `InvalidRuntimePolicy`（exit 3、`INVALID_RUNTIME_POLICY`）を保持し `details.layer="invocation"` を持つ（AC3 の run 経路）。対象: `run_dev_seam.rs` — `run_fake_stream_grant_deny_listed_executable_rejected`。schema 違反・argv compile 失敗・malformed JSON（`InvalidRequest`+`layer`）も同 test または load.rs unit test で pinned。
  > （issue #169 注記 — `is_denied_command_executable`/`is_shell_process_executable` は builtin deny set 時代の検査名。現行の deny は opt-in `security.execDenyPatterns` 宣言時のみ効き（#163/#164）、既定では `python` 等は deny 該当でない）
- Rule: `cheaper tool-operate` 経由でも `--grant` が同等に効く — builtin `tool-operator`（opt-in 宣言済み）が flag grant で `command` を実登録し scripted call が spawn する（AC10 の run 経路 — `ParsedCli::Run` fold による parity）。対象: `run_dev_seam.rs` — `run_fake_stream_tool_operate_alias_accepts_grant`（`FIXTURE_COMMAND_CALL` + flag JSON → marker spawn・`env["worker"] == "tool-operator"`）。
- Rule: repeatable `--grant` による複数 grant が 1 つの run に同時注入される — 2 つの grant が effective `command.grants` に共存し、stderr diagnostic の `grant:` 行に宣言順で両名が列挙される（AC15 の run 経路）。対象: `run_dev_seam.rs` — `run_fake_stream_multiple_grants_injected`（2 指定で両 grant が登録され、少なくとも一方の scripted call が spawn + stderr `grant: <n1>, <n2>`）。
- Rule: invocation-sourced grant が適用された run は audit surface で policy-sourced run と区別できる — run 開始後に stderr diagnostic 行 `grant: <names>`（宣言順、`--quiet` で抑制、`model:` line 規約に一致）が emit され、`cheaper.run.tool_scope` span に `grants.invocation` field が record される。`kind: tool` job は span を通らないため workflow 経路では stderr 行が唯一の audit surface（AC8）。対象: `run_dev_seam.rs` — `run_fake_stream_grant_emits_stderr_diagnostic`（stderr に `grant: self` を含む・`--quiet` で空）。`workflow_dev_seam.rs` — workflow run 経路でも同じ行を assert。span field は `observability_dev_seam.rs` の file-sink 経路（`CHEAPER_LOG=debug` subprocess → `.cheaper/logs` JSON line parse で `cheaper.run.tool_scope` span record の fields を検証 — `grants.invocation` は `tools.degraded` と同じ sync 区間で record されるため deterministic に assert 可能）で確定 assert する — 元の「stderr 行のみ必須」hedge は review で解消（plan risk 4 resolved）。in-process Fixture 経路は global subscriber を install できないため subprocess 経路で担保する。
- Rule: `command` を宣言しない worker（`--worker-file` 含む）への `--grant` は warn-and-continue — stderr に `warning: --grant supplied but worker does not declare the command tool` 相当の行を出して run は続行する。worker が `command` を宣言済みで `tools.allow` が除外する場合は `check_capabilities` の run-level `POLICY_VIOLATION`（`details.missing` 付き、exit 4）、pin 失敗は hard `POLICY_VIOLATION` のまま — warning への変換はしない（ADR の warn 範囲限定）。対象: `run_dev_seam.rs` — `run_fake_stream_grant_on_worker_without_command_warns`（`command` 非宣言 worker + `--grant` → warning 行 + exit 0）+ 既存 `run_fake_stream_opt_in_capability_denied_is_policy_violation` が無変更 green（admission 拒否不変の固定）。
- Rule: `cheaper workflow run --grant` は `run` と同一の意味論で grant を注入する — `execute_run_inner` → `build_invocation_layer` → `prepare_workflow_context` → `load_policy` の同一 merge 経路で parity が自動的に保たれる。`kind: worker` job（opt-in worker）で縮退が解消し、`kind: tool` job は `command: None` で parity check を skip するが `inner.command`（invocation grant 込み resolved policy）再注入経由で grant を受ける（AC14）。対象: `crates/cheaper-cli/tests/workflow_dev_seam.rs` — `workflow_run_grant_reaches_worker_job`（opt-in worker job + `--grant` → degraded でなく command call 到達・`grant:` stderr 行 assert）および `workflow_run_grant_reaches_tool_job`（`kind: tool` job の `command` call が grant で spawn — marker file 到達）。
  > （issue #169 注記 — `command: None`/`inner.command` は #165 で `exec: None`/`inner.exec` へ rename — `workflow_run.rs:500-503`・`invocation.rs:277-286`。file-level `command.grants` shape は有効のまま）
- Rule: tool-operator の成功 run envelope `result` から machine-readable な `grantRequests[]` が parse でき、item は `required: ["executable"]` + optional `argvHint` / `name` の宣言 output schema 適合（`additionalProperties: false` discipline — schema 未宣言なら strict-mode の schema validation が `INVALID_OUTPUT` に落とすため、success envelope への到達自体が field 宣言の証明となる）（AC5）。対象: `run_dev_seam.rs` — `run_fake_stream_tool_operator_blocked_emits_grant_requests`: `tool_operator_fixture` 拡張（`status: "blocked"` + `grantRequests: [{executable, argvHint, name}]` を含む最終 JSON）→ `env["ok"] == true`・`env["result"]["grantRequests"][0]["executable"]` の shape assert。`builtin_workers.rs::all_builtin_workers_resolve_with_schemas` が extended `tool-operator.output.schema.json` の strict-mode compile を間接担保（schema が strict-mode 規約を破れば builtin resolve で fail — AC5 の schema 面の追加カバレッジ）。
- Rule: `cheaper policy grant add <name> '<spec-json>'` は write 前に merged full-object `validate_policy_schema(Partial)`（spec parse → `read_policy_yaml` → 非 object `command`/`command.grants`・既存名の構造前提 pre-check → grants map 畳み込み → deny list・argv compile・env/cwd 規則・`version` const・`maxProperties` を含む全体検査）を通し、通過した場合のみ `stringify_yaml` → `atomic_write`（`windows_remove_first: false`・POSIX `0600`）で `<userConfigDir>/policy.yaml` へ永続化する。invalid spec（deny 該当・argv compile 失敗・schema 違反・malformed JSON・`name` key 混入・既存 grant 名）は file を一切変更せず error envelope を返す — shared validation 由来は `InvalidRuntimePolicy`（exit 3）、入力構造の失敗は `InvalidRequest`（exit 2）。add 済み grant は後続の flag 無し run でそのまま効く（AC6）。対象: `crates/cheaper-cli/tests/policy.rs`（新規、temp `CHEAPER_CONFIG_DIR` の `Fixture` 経由 — dev-seam 不要の非 gated target）— `policy_grant_add_persists_grant`（add → file に grant 存在 + success envelope `{grant:{name,executable}}` 相当）、`policy_grant_add_rejects_invalid_spec_without_writing`（deny 該当・bad argv・schema 違反 — file 不変）、`policy_grant_add_rejects_existing_name`（`InvalidRequest` + `details.grant`）、`policy_grant_add_preserves_other_sections`（seed policy の `tools`/`execution` section が rewrite 後も残存）、`policy_grant_add_fails_on_corrupt_existing_file`（`version` 欠落/≠1・非 object `command`/`command.grants` → `InvalidRuntimePolicy` + file 不変）、`policy_grant_add_writes_owner_only_file_mode`（`#[cfg(unix)]` で `0o600` assert）。e2e 側は `run_dev_seam.rs` — `run_fake_stream_policy_grant_add_then_run_executes`（`f.run(&["policy","grant","add","self","<json>"])` → flag 無し opt-in run → marker spawn）が担う — 同一 `Fixture` で `f.run` を 2 回呼ぶため `tests/common/mod.rs` の `Fixture::run` multi-call 修正（call 冒頭の out/err clear + stdin は caller が `f.stdin.data` で再装填 — `PipeStdin::read_all` が `mem::take` で消費するため）が前提。
  > （issue #169 注記 — deny list 検査は `security.execDenyPatterns` 宣言時のみ（#164）。schema・argv compile・env/cwd 規則は無条件維持）
- Rule: `cheaper policy grant list` は user policy の effective grants を name 順の `{grants:[{name,executable}]}` で返し（file absent は空配列）、`remove <name>` は同 pre-check + 同じ full-object validation + 同 write path で削除する。存在しない name・file absent の remove は `InvalidRequest` の error envelope。remove 後は後続 run でその grant が効かなくなる（AC7）。対象: `tests/policy.rs` — `policy_grant_list_returns_user_grants`（name 順・absent file は `[]`）、`policy_grant_remove_deletes_grant`（remove → list が空・後続 opt-in run で縮退に戻る確認は run_dev_seam 側 `run_fake_stream_policy_grant_remove_then_run_degrades` で固定 — 同じく `Fixture::run` multi-call 前提）、`policy_grant_remove_missing_name_is_invalid_request`、`policy_grant_remove_on_absent_file_is_invalid_request`。
- Rule: `cheaper policy` は utility command 規約 — `record_usage` を呼ばず `usage.jsonl` に record を残さず、success は `utility_success_envelope` で `result` に top-level `tokens` key を含めない（`usage_log::build_usage_record` の count-record 判定との衝突回避）（AC9・AC16）。対象: `tests/policy.rs` — `policy_grant_commands_do_not_record_usage`（add/list/remove 後に `f.usage_log()` が None または policy 由来 record を含まない）、`policy_grant_result_has_no_tokens_key`（`env["result"]["tokens"]` 不在 — add・list・remove の全形態）。
- Rule: `command` tool の既存 contract は不変 — grant 未指定（`invocation: None`）の既存経路に差分はなく、fail-closed gate（空 grants → `POLICY_VIOLATION` exit 4 "nonempty authorized grants"）・declared-shape parity・deny list の 3 適用点は既存 test が無変更 green で担保する。model 入力・worker prompt・project policy から grant を authoring する経路は設計上存在しない（経路非存在は compile 面と schema reject で保証 — 追加 test 対象外）。対象: 既存 `run_fake_stream_command_worker_denied_without_grants`・`run_fake_stream_command_grant_executes_and_marks_not_comparable`・`crates/cheaper-tools/tests/proc_invocation.rs` parity test 群・golden 既存 case `"project command section rejected"`（いずれも無変更 green のまま維持）。
  > （issue #169 注記 — builtin deny 除去（#163）で対応 test 群は #163/#164 で更新済み — deny の 3 適用点は opt-in deny（`execDenyPatterns` 宣言時のみ）の適用点として存続。「既存 test が無変更 green」の記述は builtin deny 前提の pin で、現行 test 群は opt-in 前提へ再 pin されている）
- Rule: issue #105 の e2e 結合面は regression corpus case でも固定する — `tests/regression/cases/` に `extraArgs`（case schema は camelCase — `harness.rs` の `rename_all = "camelCase"` で serde field は `extraArgs`。snake_case `extra_args` は silent ignore され false-green hazard）で `--grant` を付けた case を追加する（`config.files` の `policy.yaml` seed で user grant との collision・共存、fixture worker の `optional_tools` 宣言で縮退 → grant 注入の replay）。expected は `CHEAPER_REGRESSION_CAPTURE=1 cargo test -p cheaper-cli --features dev-seam --test regression` で生成し diff review する。static corpus は positive spawn を表現不能（grant executable は絶対・非 workspace path が必須で、bare name は deny set 該当または toolchain 非 deterministic — `current_exe` self-reshim は case JSON に書けない）ため、case は policy-layer 結果のみを固定する: grant 受理・collision・deny reject + error envelope の `details.layer`、`config.files` seeded user policy、`compare_stderr` opt-in による stderr `grant:` 行。positive spawn assert は dev-seam test のみとする。対象: `tests/regression/cases/run-grant-*.json` + `expected/`（plan T5）。
  > （issue #169 注記 — 「deny set 該当」は opt-in `security.execDenyPatterns` 宣言時のみ成立（#164 — 既定 `[]` では deny 該当なし））
- Traceability: AC11 → T6 doc audit（行為保証対象外）、AC12 → `mise run check`（verification gate であり test ではない）。

## Examples

- Example: builtin default policy（`command.grants: {}`）で `command` が縮退した opt-in worker へ `cheaper run opt-worker --grant '{"name":"self","executable":"<exe>","argv":[...]}'` を渡すと、`command` が実登録され scripted call が実 subprocess を spawn して workspace 内 marker を書く（AC1）。
- Example: 同じ run を flag 無しで再実行すると `command` は再び縮退し、`policy.yaml` は一度も存在しない — `--grant` の効果は当該 run のみ（AC2）。
- Example: `--grant '{"name":"x","executable":"python",...}'`（interpreter deny 該当）は run 開始前に `INVALID_RUNTIME_POLICY`（exit 3）+ `details.layer="invocation"` で fail — invocation 経路は deny list を迂回しない（AC3）。
  > （issue #169 注記 — `python` は既定では deny 該当でない — interpreter deny は builtin deny set として #163 で除去。`execDenyPatterns` 宣言時のみ同形の `INVALID_RUNTIME_POLICY` + `details.layer` 拒否が成立する（#164））
- Example: user `policy.yaml` に grant `self` がある環境で `--grant '{"name":"self",...}'` を渡すと `INVALID_REQUEST`（exit 2）+ `details.grant="self"` + `details.layer="invocation"` — silent 上書きはしない（AC4）。
- Example: `cheaper run w --grant '{"name":"a",...}' --grant '{"name":"a",...}'` は同名重複で `INVALID_REQUEST` — 宣言順 push の暗黙 last-wins を許さない（AC13）。
- Example: `cheaper usage --grant '{...}'`・`cheaper policy grant list --grant '{...}'` はいずれも `InvalidRequest`（exit 2）で reject — silent drop しない（AC17）。
- Example: blocked した tool-operator run の `result` は `{"status":"blocked", ..., "grantRequests":[{"executable":"rg","argvHint":["--files"],"name":"rg"}]}` を含み、宣言 schema 適合で parent agent が parse できる（AC5）。
- Example: `cheaper policy grant add self '{"executable":"<exe>","argv":[...]}'` が user `policy.yaml` を作成し（POSIX `0600`）、`policy grant list` が `{grants:[{name:"self",executable:"<exe>"}]}` を返し、flag 無し run が grant で command を実行し、`policy grant remove self` で削除される。`policy grant remove nosuch` は `INVALID_REQUEST` の error envelope（AC6・AC7）。
- Example: `policy grant add`/`list`/`remove` のいずれも `usage.jsonl` に行を追加せず、success envelope の `result` に `tokens` key を含まない（AC9・AC16）。
- Example: `cheaper workflow run wf.json --grant '{...}'` で `kind: worker` job（opt-in worker）の縮退が解消し、`kind: tool` job の `command` call も grant を受けて spawn する — stderr の `grant: <names>` 行が両経路の audit surface（AC8・AC14）。
- Example: `tools: [read]` のみの `--worker-file` worker に `--grant` を渡すと stderr に `warning: --grant supplied but worker does not declare the command tool` 相当の行を出して run は続行する — 一方 `tools.allow` からの `command` 除外は従来どおり `POLICY_VIOLATION` で run が fail する。
- Example: `{"version":1,"tools":{"allow":["read"]}}` のような grants 以外の key を持つ invocation layer 入力、および 64KiB 超の flag 値・malformed JSON・`name` 欠落は `InvalidRequest` + `details.layer="invocation"` で reject — invocation layer は第 2 の general trusted-policy channel 化しない。

## Guarantees

- `--grant` で注入された grant は、当該 run の `RuntimePolicy.command` の effective grant 集合で user-policy grant と区別なく振る舞う — deny list・argv pattern・executable pin・env/cwd 規則・managed-process lifecycle・declared-shape parity（`create_worker_scope` の `PartialEq` check）は完全に同一。
  > （issue #169 注記 — `RuntimePolicy.command` は `RuntimePolicy.exec` へ rename（#165・`schema.rs:134`）。deny list の同一性は `execDenyPatterns` 宣言時のみ（#164 — 既定 `[]` では非活性））
- `--grant` の grant は policy file・`usage.jsonl`・`.cheaper/` のいずれにも書き込まれず、flag 無しの後続 run に効果を持たない（非永続性）。
- grant authoring の経路は invoker の CLI 引数（`--grant`）と validation 済み user policy 管理 command（`policy grant`）と trusted policy file のみ — model 入力・worker prompt・project policy からの経路は存在せず、grants-only key 制約が invocation layer の general channel 化を防ぐ。
- collision（flag vs policy・intra-invocation 重複）・構造違反・非 run command への適用は全て run 開始前の fail-loud — silent 上書き・last-wins・silent drop は存在しない。
- error attribution は単一規約 — shared validation 由来の失敗は inner code `InvalidRuntimePolicy` を保持して `details.layer="invocation"` を付し、invocation 入力自体の構造失敗は `InvalidRequest` + `details.layer="invocation"`（grant 名関与時は `details.grant`）。
- `--grant`・`policy grant add`・policy file load の 3 経路で grant validation は `resolve_command_policy` 系統の single authoritative path のみを通る — 重複する検証実装は存在しない。
- invocation-sourced grant の適用は stderr `grant: <names>` 行（`--quiet` で抑制）と `grants.invocation` span field（log 有効時）の 2 面で policy-sourced grant と区別して観測できる。`usage.jsonl` record shape は不変。
- `policy grant` の write は atomic（temp + rename、`windows_remove_first: false`）・POSIX `0600`・既存 file の `command` 以外の section を保存・write 前に merged full-object `validate_policy_schema(Partial)` を通す — invalid spec は file を一切変更しない。
- tool-operator の blocked / partial run は machine-readable `grantRequests[]`（`{executable, argvHint?, name?}`、全階層 `additionalProperties: false`）を output schema 適合で返せ、re-run loop の組立に必要な粒度を持つ — field は advisory であり、re-run 時の deny list・argv compile 再検査により fail-closed は保たれる。
  > （issue #169 注記 — re-run 時の deny 再検査は `security.execDenyPatterns` 宣言時のみ（#164）。argv compile 再検査は無条件で fail-closed を保つ）
- `cheaper policy` は utility command 規約を守る — usage.jsonl 非記録・utility success envelope・`result.tokens` 不在。

## Non-goals

- parent-facing docs の記述内容（AC11 — `cheaper-usage` / `cheaper-tool-operate` SKILL と guides の grant-request → approval → `--grant` re-run loop は plan T6 の doc audit が担う。行為保証の対象外）。
- `mise run check` pipeline 通過そのもの（AC12 — task の runnable Verification が担う CI contract であり、本 test-spec の行為保証ではない）。
- `cheaper hook ... --grant` の reject（`hook` は `parse_cli_args` 未到達の独立 parse path で `--grant` を silent ignore する spec 適合の known gap — AC17 の reject 契約対象外）。
- `grantRequests` emit 条件の prompt 遵守（schema 適合と envelope parse は固定するが、model が「gated tool 欠如由来の blocked/partial のみ」で emit するかは `tool-operator.md` prompt contract の責務 — fake stream は emit 済み結果の受理のみを検証する）。
- `--grant` flag 値の secrets-in-argv 挙動（cmdline 露出は構造的 risk で docs warning のみ — `env_clear` + grant `env` minimal env は `command` tool の既存 contract が担い本 spec の検証対象外）。
- `policy grant` の lost-update race・serde_yml rewrite による comment/整形喪失（accepted caveat — 並行 add・手編集の割り込みは lock 非導入の範囲外）。
- `usage.jsonl` record への invocation-grant field（ADR defer — record shape 不変は regression corpus parity で担保）。
- `process.tasks` への同型 invocation-scope 機構・`--grant-file`・project policy 対象化・model 出力経由の grant authoring（spec Out of Scope — 経路を作らない側の保証）。
- merged map でのみ発火する検査への `details.layer` 付与（`grants` 合計 `maxProperties: 128` 超過等は layer 帰属なしの `InvalidRuntimePolicy` — accepted attribution gap、design §(a) / load.rs doc comment に明記）。
- `ollama_e2e.rs` 実 provider e2e 検証（dev-seam fixture + regression corpus が primary 検証）。
- `command` tool 本体の contract 検証そのもの（deny list・argv match・managed-process lifecycle の意味論は `policy-gated-command-tool` test-spec の責務 — 本 spec は不変性の維持（既存 test の無変更 green）のみを主張する）。
  > （issue #169 注記 — deny list 意味論の検証責務の言及は opt-in `security.execDenyPatterns` 前提（#164）として読む — 既定 `[]` では deny 検査は非活性）

## Risk

- grants-only key 制約や `name` strip が落ちると、invocation layer が `command.grants` 以外の trusted section を authoring する第 2 の policy channel 化し、model/project を通らない trusted-layer-only 境界（ADR `command-grant-trust-boundary-and-managed-process-reuse`）が構造的に崩れる。
- `--grant` が policy file へ永続化されると「run-lifetime のみ」の契約が破れ、flag を知らない後続 run に権限が残留して audit 不能な常駐 grant となる。
- collision を silent 上書き・last-wins にすると、user policy が管理する grant が flag で静かに置き換わり、trusted layer の explicit error 規約が失われる。
- `policy grant add` の write path 欠陥（validation 漏れ・非 atomic write・`command` 以外の section 破壊）で、deny 迂回 spec や破損した policy が trusted user layer に残留し、次回以降の全 run の fail-closed gate を歪める。
- `result.tokens` の混入や usage.jsonl 誤記録で `policy` command が utility 規約を破ると、count-record 誤判定・計測 drift を誘発する。
- `grantRequests` が schema 非適合だと strict `additionalProperties` 下で `INVALID_OUTPUT` 化し、blocked → approval → re-run の機械的 loop そのものが成立しない。
- 非 run command の `--grant` reject guard が抜けると flag が silent drop され、invoker の grant 意図がエラーなく無視される。
