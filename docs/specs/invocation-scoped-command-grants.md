---
id: "SPEC-034FlOz4hiN9m0nQW7lBN2"
type: "spec"
status: "approved"
title: "invocation-scoped command grant と grant-request フロー"
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
    - "../specs/policy-gated-command-tool.md"
    - "../specs/tool-operator-built-in-worker.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/tool-operator-built-in-worker.md"
    - "../specs/policy-gated-command-tool.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md"
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/invocation-scoped-command-grants.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../guides/command.md"
    - "../guides/workers.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# invocation-scoped command grant と grant-request フロー

## What Should Be Built

`command` tool（policy-gated-command-tool spec で確立）の grant 供給・要求・永続化を 3 つの component で拡張する。

1. **invocation-scoped grant（`--grant` flag）**: `cheaper run` と全 worker alias（`tool-operate` 等）が repeatable な `--grant <spec>` flag を受け付け、その run の有効期間に限り grant を注入する。invoker（parent agent / user）が approval authority となり、model 入力や project policy を経由しない grant 供給経路を提供する。
2. **grant-request → re-run flow**: tool-operator の output contract に machine-readable な capability-request surface を追加し、blocked / partial となった run が「どの executable の grant が不足したか」を機械可読で返す。parent agent が `result` を読み → user へ approval を求め → `--grant` 付きで re-run する loop を成立させる。
3. **`cheaper policy` 管理 command**: `cheaper policy grant {add,list,remove}` で、繰り返し必要になる grant を user-scope `<userConfigDir>/policy.yaml` へ validation 付きで永続化・列挙・削除する。

3 つの component はいずれも grant の runtime 挙動（deny list・argv pattern・executable pinning・env/cwd 規則・managed-process lifecycle）と validation を既存の `resolve_command_policy` 系統から継承し、独自の grant 検証経路を新設しない。

## Why Now

`command` tool は default deny で、grant は trusted layer（builtin + user-scope policy）でのみ authoring できる（ADR `command-grant-trust-boundary-and-managed-process-reuse`）。tool-operator worker は `command` を `cheaper.optionalTools` で opt-in 宣言し、grant が無い環境では degrade / blocked で応答する（ADR `opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker`）。

現状この境界には 3 つの実用上の gap がある:

- **grant が無い環境では run ごとに手で policy.yaml を編集するしかない**。parent agent は任意 command を自身で実行できる以上、1 run に限った grant 注入は権限拡大ではなく delegation 範囲の明示であるが、その機構が存在しない。
- **blocked 結果は free text（`unresolved` / `nextActions`）でしか返らず**、parent agent が「何の grant を要求すればよいか」を機械的に判断して re-run を組み立てられない。
- **恒久的に必要な grant を user policy へ安全に追加する CLI が無い**。手書き YAML は validation 無しで schema・deny list・argv pattern の誤りを実行時まで検出できない。

issue #105 が採用した 3 component でこれらを埋める。

## Users and Value

- 親 Coding Agent: grant が無いために degrade / blocked した tool-operator run を、`--grant` 付き re-run で機械的に回復できる。`grantRequests` から要求 spec を組み立て、user approval を経て再委譲する loop が成立する。frontier model が自ら command を実行するコストを回避できる。
- trusted user: run-lifetime の一時 grant（`--grant`、非永続）と永続 grant（`cheaper policy grant add`、validation 済みで保存）を明示的に制御できる。project repo や model が grant を authoring する経路は引き続き存在しない。

## Scope

### In Scope

- `cheaper run` および全 worker alias（`tool-operate` 等、`ParsedCli::Run` に fold される command）への repeatable `--grant <spec>` flag。複数指定で複数 grant を注入できること。`cheaper workflow run` も同一 flag を受ける（ADR で採用 — `prepare_workflow_context` が同一の `load_policy` を通るため parity が自動的に保たれる）。`workflow interface` およびその他の utility command へは適用しない。
- `run`・worker alias・`workflow run` 以外の command（`usage`・`count-tokens`・`models`・`auth`・`policy`・`workflow interface`）への `--grant` は明示 error で reject する。`Vec<String>` の default-empty 受け皿に依存した silent drop を許さない。
- `--grant` で注入された grant が、当該 run のみで user-policy grant と同一の runtime 挙動（deny list・argv pattern・executable pinning・env/cwd 規則・managed-process lifecycle）を持つこと。
- `--grant` grant が一切の policy file へ永続化されないこと、および model 入力・project policy から到達不能であること。project trust boundary（`ProjectPartial` schema が `command` section を reject する既存契約）は不変。
- invocation-sourced grant を policy-sourced grant と区別できる audit surface（run record / span / diagnostic のいずれかで観測可能であること — 要件として固定し、具体機構は ADR で確定）。
- tool-operator output contract への machine-readable capability-request field の追加（optional top-level field。`grantRequests[]` のような `{executable, argv hint?}` を運ぶ配列を想定するが shape は ADR で確定）。`unresolved` / `nextActions` の free text は human-readable のまま維持。
- tool-operator worker prompt の更新: gated tool が必要だったが blocked / partial となった run で request field を emit させる。
- parent-facing docs の更新: 「blocked + grantRequests を読む → user approval → `--grant` 付き re-run」の loop を `cheaper-usage` SKILL、`cheaper-tool-operate` SKILL、および必要に応じて workers / usage / command guide に文書化。
- output schema の strict-mode 適合維持（`additionalProperties: false` 下では新 field を schema に宣言必須）。
- `cheaper policy grant add` / `list` / `remove` の追加。対象は user-scope `<userConfigDir>/policy.yaml` のみ。user-scope 解決は `--cwd` に依存しない env/home 由来の `resolve_user_config_dir`（`crates/cheaper-core/src/config/dirs.rs:37-53` — `CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` → `<home>/.config/cheaper`）で足りる。project policy への grant authoring は schema 上そもそも不可能であり、この command も project policy を対象にしない。`policy` command が受け付ける flag は utility 規約の `--format` / `--quiet` / `--log` / `--log-format` のみとし、`--worker-file` / `--grant` / `--cwd` / `--input` / `--timeout` 等の run 系・他 command 系 flag は reject する。
- `policy grant add` は write 前に `resolve_command_policy` 系統の静的検査（schema + deny list + argv compile + env/cwd 規則）を全て通し、atomic に書き込む。executable の存在確認は add 時には行わない（pin 失敗は scope 生成時に `POLICY_VIOLATION` として報告される — ADR 決定）。`list` は effective user grants を表示、`remove` は name で削除する。
- `ParsedCli::Policy` は utility command 規約に従う — `ParsedCli::format()` / `quiet()` accessor（`args.rs` の match arm）へ `format` / `quiet` を populate し、`command_name` は `"policy"`、成功 envelope は `utility_success_envelope` を使う。
- error は標準 error envelope、成功は utility envelope。`cheaper policy` は utility command であり `usage.jsonl` には記録しない。success envelope の `result` に top-level `tokens` key を含めない（`usage_log` が utility envelope の `result.tokens` を count record の判定に使うため — `crates/cheaper-cli/src/usage_log.rs:100-107`）。
- HELP 更新、引数解析の拡張、関連 test の追加。

### Out of Scope

- model 出力経由の grant authoring（設計上、経路自体を作らない）。
- project policy への grant 永続化・grant authoring key の追加（既存 trust boundary を維持）。
- `--grant` flag 値の最終形式、collision 意味論、workflow への適用範囲、`grantRequests` の具体 shape、`policy grant` の UX 形状、YAML 書き込み方式、audit surface の具体機構 — いずれも ADR で確定する decision であり、本 spec は要件のみを固定する。
- `process.tasks` への同型 invocation-scope 機構。
- cheaper 内部での interactive confirm prompt（approval は parent agent 側の責務）。
- `command` tool 本体の contract 変更（deny list・argv match・managed-process lifecycle の意味論は不変）。

## Requirements

### `--grant` flag（invocation-scoped grant）

- `cheaper run`・全 worker alias・`cheaper workflow run` が `--grant <spec>` を repeatable に受け付ける。各指定は独立した grant spec として扱われる。
- flag 値は grant name を `name` key で内包する complete JSON object（`{"name":"<name>","executable":"...",...}`）とする（ADR 決定）。CLI は `name` を取り出して `command.grants.<name>` へ畳む — 残りの key は grant spec schema（`command.grants` map value）と同一 shape であり、`name` 自体は spec schema の `additionalProperties: false` で reject されるため validation 前に必ず strip する。これにより grant validation を `resolve_command_policy` 系統へそのまま乗せられ、validation を再実装する専用 mini-format（`key=value` 等）は採用しない。
- flag 値には parse 時のサイズ上限（64KiB）を設け、超過は `InvalidRequest` で reject する。`--grant` と `--worker-file` の併用は許可する（worker-file worker は `run` と同一の trust surface であり、grant が露出しない場合は warn-and-continue 規約で扱う）。
- `--grant` の flag 値（grant の `env` に secret を含みうる）は invoker の process cmdline（`ps`・`/proc`・shell history）に露出する。spawn される command は `env_clear` + grant `env` のみの最小 env で起動し ambient 環境変数は子プロセスへ届かないため、secret は flag 値ではなく user policy の grant `env` 値（POSIX `0600` の `policy.yaml`）か子プロセス自身が読む credential file へ置くことを docs deliverable（command guide・policy guide・配布 SKILL）で明記する。`policy grant add <name> '<json>'` も spec を cmdline へ一時的に載せる同種の caveat を持つ。
- 注入された grant はその run の `RuntimePolicy.command` の effective grant 集合に畳み込まれ、runtime 上は user-policy grant と区別なく振る舞う。declared-shape parity 契約（`create_worker_scope` の `PartialEq` check）を破らないことが必須。
- `--grant` の grant はいかなる policy file にも永続化しない。flag 無しの後続 run では効果を持たない。
- grant は invoker の CLI 引数からのみ供給される。model 入力・worker prompt・project policy のいずれからも `--grant` 相当の grant を authoring できない。invocation layer は grants-only に制約する — top-level key ⊆ `{version, command}` かつ `command` key ⊆ `{grants}` 以外の入力は `InvalidRequest` + `details.layer="invocation"` で reject し、第 2 の general trusted-policy channel 化を防ぐ（ADR 決定）。
- deny list に該当する executable を含む grant spec は `--grant` 経由でも reject される（invocation 経路が deny list を迂回しない）。error attribution は ADR の規約に従う — shared validation 由来の失敗（schema 違反・deny list 該当・argv compile 失敗・env/cwd 規則違反）は inner code（`InvalidRuntimePolicy`）を保持し `details.layer="invocation"` を付し、invocation 入力自体の構造的失敗（malformed JSON・`name` key 欠落/pattern 違反・同名重複・collision・grants 以外の key）は `InvalidRequest` + `details.layer="invocation"` とする。
- flag 由来 grant と builtin/user policy の同名 grant が衝突した場合、および同一 invocation 内で複数 `--grant` が同名 grant を指定した場合は、run 開始前に `InvalidRequest`（details に衝突した grant name）で失敗する（ADR 決定 — explicit error。silent な上書き・last-wins は行わない）。
- audit surface: invocation-sourced grant が適用された run を、policy-sourced grant のみの run と区別して観測できること（ADR 決定: `grants.invocation` tracing span field + run 開始時の stderr diagnostic line の 2 面）。

### grant-request → re-run flow

- tool-operator の output contract に optional の machine-readable capability-request field を追加する。`status` が `blocked` または `partial` で gated tool の欠如が原因となった request が列挙される（ADR 決定 — schema で status 連動は強制せず prompt contract で規定）。
- request field の shape は ADR で確定するが、少なくとも不足した `executable` を含み、parent agent が `--grant` の値を機械生成できる粒度であることを要件とする（argv hint を含めるかは ADR）。`unresolved` / `nextActions` は引き続き human-readable の free text とし、machine-readable 要求をこれらで代替しない。
- worker prompt は、gated tool（`command`）が必要だったが grant 欠如で利用できなかった場合に request field を emit するよう更新する。
- output schema は strict-mode 規約（`required` ⊆ `properties`、全階層 `additionalProperties: false`）を維持し、新 field を schema で宣言する。既存 field・evidence enum の意味論を変えない。
- parent-facing docs（`packages/apm/.apm/skills/cheaper-usage/SKILL.md`、`packages/apm/.apm/skills/cheaper-tool-operate/SKILL.md`、必要に応じて `docs/guides/workers.md` / `usage.md` / `command.md`）に、「`result.status == "blocked"` + request field を読む → user approval を取得 → `--grant` 付きで re-run」する loop を文書化する。approval は parent agent が user に求める責務であり、cheaper 側に interactive prompt は作らない。

### `cheaper policy` 管理 command

- `cheaper policy grant {add,list,remove}` を新設し、user-scope `<userConfigDir>/policy.yaml` の `command.grants` のみを対象とする。project policy は対象にしない。
- `add` は以下を write 前に全て通す: 入力の schema validation、deny list 検査（必須）、argv pattern の compile、env/cwd 規則 — いずれも `resolve_command_policy` 系統の静的検査を再利用し、command 独自の検査を追加しない。executable の存在確認は add 時には行わない — pin は tools 層の責務で ambient PATH・workspace 依存の実行時検査であり、存在しない executable は scope 生成時に `POLICY_VIOLATION` で報告される（ADR 決定。user policy 直書きと同一の意味論）。
- `add` / `remove` の書き込みは atomic（temp + rename 相当）で行い、既存の user policy の `command` 以外の section を破壊しない。POSIX では `0600` mode で書き込む（grant の `env` 値が secret を保持しうるため）。write path の caveat（並行 add・手編集との last-wins、serde_yml rewrite による comment/整形の喪失）は ADR で規定する。
- `list` は effective user grants（user policy が提供する grant 集合）を表示する。
- `remove` は grant name を指定して削除する。存在しない name の remove は `InvalidRequest` の error envelope を返す（utility command の入力検証として一貫させる）。
- error は標準 error envelope、成功は utility success envelope を使う。`policy` は utility command であり `usage.jsonl` に記録しない。
- `add` で deny list 該当 executable や invalid argv pattern を含む spec を渡した場合、policy file を変更せず error を返す。

### 横断要件

- 3 component 全てが grant validation を `resolve_command_policy` 系統の single authoritative path から得る。`--grant` flag 値、`policy grant add` 入力、policy file の load で同一の検査が走ること。重複する検証実装を作らない。
- `command` tool の既存 contract（default deny・trusted-layer-only authoring・fail-closed gate・opt-in degrade）は不変。本 spec は供給・要求・管理の経路を増やすが、実行時の認可意味論を弱めない。
- trusted-layer-only grant authoring 契約（`policy-gated-command-tool` / ADR `command-grant-trust-boundary-and-managed-process-reuse`、issue #101 系）の "trusted layer" は本 spec の採用により合成 invocation layer を含む — `--grant` は invoker の CLI 引数からのみ構成される第 3 の trusted 供給元として load pipeline 内に layer 化される（ADR `invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer`）。model 入力・project policy が grant を authoring できない境界は不変。

## Acceptance Criteria

- [x] AC1: grant を持たない環境で degrade / blocked した tool-operator run に対し、`--grant` 付きで re-run すると当該 command が利用可能になり run が成功（または blocked でなくなる）ことを検証する test が通る。
- [x] AC2: `--grant` は永続化されない — flag 無しで同じ run を再度実行すると再び degrade / blocked する。policy file が一切変更されていないことを検証する。
- [x] AC3: deny list 該当 executable を含む `--grant` spec は run 開始前に reject される（invocation 経路で deny list を迂回できない）。error は ADR の attribution 規約に従い shared validation の inner code（`InvalidRuntimePolicy`）を保持し `details.layer="invocation"` を持つ。
- [x] AC4: flag 由来 grant と user policy の同名 grant が衝突した場合の挙動が ADR で定義した意味論どおりであり、test で固定されている。
- [x] AC5: blocked となった tool-operator run の成功 run envelope `result` から machine-readable な grant-request field が parse でき、宣言した output schema に適合する（`additionalProperties` discipline 維持）。
- [x] AC6: `cheaper policy grant add` が validation を通した grant を user policy へ永続化し、後続の flag 無し run がその grant で command を実行できる。invalid spec（deny list 該当・argv compile 失敗・schema 違反）は policy file を変更せず error を返す。
- [x] AC7: `cheaper policy grant list` が effective user grants を表示し、`remove <name>` で削除すると後続 run でその grant が効かなくなる。存在しない name の remove は `InvalidRequest` の error envelope を返す。
- [x] AC8: `--grant` を伴う run が、audit surface（ADR で確定した機構）で invocation-sourced grant の適用を区別して観測できる。
- [x] AC9: `cheaper policy` 系 command が `usage.jsonl` に record を残さない（utility command 規約）。
- [x] AC10: worker alias（`tool-operate`）経由でも `--grant` が同等に効く（`ParsedCli::Run` fold による parity）。
- [x] AC11: parent-facing docs（cheaper-usage / cheaper-tool-operate SKILL、必要に応じて guides）に grant-request → approval → `--grant` re-run loop が文書化されている。
- [x] AC12: `mise run check` が pass する。
- [x] AC13: 同一 invocation で同名 grant を複数 `--grant` に指定した場合 `InvalidRequest` で reject される（intra-invocation duplicate — flag-vs-policy collision と同じ fail-loud 規約）。
- [x] AC14: `cheaper workflow run --grant` が `run` と同一の意味論で grant を注入する（`prepare_workflow_context` の load path 共有による parity）。
- [x] AC15: repeatable `--grant` で複数 grant が 1 つの run に同時注入される。
- [x] AC16: `cheaper policy` の success envelope `result` に top-level `tokens` key が無い（`usage.jsonl` の count-record 判定との衝突を防ぐ）。
- [x] AC17: 非 run 系 command（`usage`・`count-tokens`・`models`・`auth`・`policy`・`workflow interface`）への `--grant` が全 command arm で明示 error（`InvalidRequest`）で reject される — silent drop しない。

## Deferred Design Concerns

discovery の decision axis として残存した問いは、並行 ADR（`../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`、ADR-034FlTS3BaiV3WKz8bX1Xv — accepted）が以下のとおり確定した:

- `--grant` flag 値の形式 → grant name を `name` key で内包する complete JSON object。CLI が `name` を strip して `command.grants.<name>` へ畳む。専用 `key=value` mini-format は validation 再実装のため却下済み。
- invocation grant の格納先・merge 順序 → `load_policy` の user merge 直後に `invocation` layer を挿入（Partial validate + `MergeMode::Replace`）。parity 契約を自動的に満たす。入力は grants-only の key 制約付き（top-level ⊆ `{version, command}`、`command` ⊆ `{grants}`）。
- flag vs user policy の同名 collision 意味論 → explicit error（`InvalidRequest` + `details.grant` + `details.layer="invocation"`）。intra-invocation の同名重複も同じく `InvalidRequest`。
- `cheaper workflow run` への `--grant` 適用範囲 → 適用する。`prepare_workflow_context` が同一 `load_policy` を通るため parity は自動。`kind: tool` job は `command: None` で parity check を skip するが `inner.command`（invocation grant 込みの resolved policy）が再注入されるため grant は tool job にも届く（user grant と同じ挙動）。
- `grantRequests` field の shape → optional top-level `grantRequests[]`。item は `required: ["executable"]` + `argvHint?: string[]`（items に maxLength 上界）+ `name?: string`（grant `propertyNames` と同一 `pattern: ^[a-z][a-z0-9_-]{0,63}$` + maxLength）、`additionalProperties: false`、`maxItems` 上界。emit 条件は `blocked` / `partial`（gated tool 欠如由来のみ）。
- `cheaper policy grant` の UX 形状 → `add <name> '<spec-json>'` / `list` / `remove <name>`。executable 存在確認は add 時に行わない（pin 失敗は scope 生成時の `POLICY_VIOLATION`）。
- user policy YAML の書き込み方式 → `serde_yml::to_string` wrapper + atomic write（`windows_remove_first: false`、POSIX `0600`）。comment/整形の喪失と lost-update は accepted caveat。
- audit surface の具体機構 → `grants.invocation` tracing span field + stderr diagnostic line の 2 面（`usage.jsonl` record 拡張は defer）。
- `--grant` を渡したが worker が `command` を expose していない場合の warning 要否 → warn-and-continue。ただし warn case は worker が `command` を非宣言の場合（`--worker-file` worker 含む）に限定 — worker が宣言済みで `tools.allow` が `command` を除外する場合は `check_capabilities` が run 全体を `POLICY_VIOLATION` で fail させ、pin 失敗も hard `POLICY_VIOLATION` であり warning ではない。

## Sources and Relations

- source: GitHub issue #105（3 component と採用方針の正本）。
- derives-from: `../discovery/invocation-scoped-command-grants.md`（DISC-034FlIAczzK2ruZW3HTcJi）。コードベース探索による injection point・parity 契約・validation 再利用 map・decision axis（軸1–7）を内包する。
- depends-on: `../specs/policy-gated-command-tool.md`（`command` tool の grant / trust boundary / validation の正本。本 spec の全要件はその不変契約の内側に置かれる）、`../specs/tool-operator-built-in-worker.md`（grant-request surface を載せる output contract と degrade 意味論の正本）。
- related: `../adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`（機構決定の正本 — 本 spec の requirements を合成 invocation layer・JSON flag 値・explicit collision error で実現する確定済み decision）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（trusted-layer-only grant authoring の確定済み decision — `--grant` は model/project を通らない invoker 経路としてこの境界を侵さない）、`../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`（`cheaper.optionalTools` 縮退機構 — grant-request はその blocked/degrade 結果を machine-readable 化する）。
- references: `../guides/command.md`・`../guides/workers.md`・`../guides/usage.md`（更新対象の user-facing docs）。
