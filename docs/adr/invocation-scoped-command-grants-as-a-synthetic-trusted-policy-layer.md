---
id: "ADR-034FlTS3BaiV3WKz8bX1Xv"
type: "adr"
status: "accepted"
title: "invocation-scoped command grant を合成 trusted layer として policy load に注入する"
created: "2026-10-14"
updated: "2026-10-14"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/105"]
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
    - "../adr/command-grant-trust-boundary-and-managed-process-reuse.md"
    - "../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md"
    - "../specs/policy-gated-command-tool.md"
    - "../specs/invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from: ["../discovery/invocation-scoped-command-grants.md"]
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["scarecrow173"]
    consulted: []
    informed: []
---

# Invocation-scoped command grants as a synthetic trusted policy layer

## Context and Problem Statement

issue #105 は `command` tool の grant 供給・要求・永続化を 3 つの component で拡張する: (1) invoker が run-lifetime の grant を注入する repeatable な `--grant` CLI flag、(2) tool-operator output へ載せる machine-readable な grant-request hint（`blocked` → user 確認 → `--grant` re-run loop の成立）、(3) user policy へ validation 付きで永続化する `cheaper policy grant add/list/remove` 管理 command。spec `../specs/invocation-scoped-command-grants.md` が製品要件（trusted-layer-only の供給元、非永続性、validation 再利用、audit 可能性）を固定し、機構面の選択を本 ADR へ defer した。本 ADR が決めるのはこれら 3 component に共通する機構の問いである。

現行の policy load 順序（`crates/cheaper-core/src/policy/load.rs:52-132`）は builtin（Complete schema）→ user `<userConfigDir>/policy.yaml`（Partial + `MergeMode::Replace`、:58-67）→ typed re-resolve（workflow / process / command / execution、:68-106）→ project（`ProjectPartial` + `RestrictConcurrency`、:108-126 — `command` section は schema reject）→ Complete revalidate + `normalize_policy`（:128-129）。`LoadPolicyOptions`（:44-48）は `builtin_policy_path` / `user_config_dir` / `project_dir` のみを持ち、invocation 由来の入力を受ける経路が存在しない。

主要な制約は invocation parity 契約である。`create_invocation_tool_runtime` は `options.command` を `resolve_command_policy` して `inner.command` へ格納し（`crates/cheaper-tools/src/invocation.rs:389-447`）、`create_worker_scope` は `options.runtime.command` を再 resolve して `inner.command` と `PartialEq` 比較する declared-shape parity check を持つ（invocation.rs:227-238）。run 経路の 2 call site は同一の `deps.policy.command` を serialize し（`crates/cheaper-runtime/src/run/mod.rs:284`・`:436`）、workflow 経路では `ensure_invocation` が `shared.policy.command` を serialize する（`crates/cheaper-cli/src/workflow_run.rs:243`）。したがって grant は `RuntimePolicy.command` に畳み込まれなければならず、別 field・別 param で運ぶ設計は parity を即座に破る。

その他の接続面: `args.rs` の `STRING_OPTIONS` table（:228-）と `Values::set_string`（:299-318）は last-wins セマンティクスであり、repeatable `--grant` には `Vec<String>` + push が要る（`RunCliArgs` populate は :903-912、alias command は `ParsedCli::Run` に fold されるため alias 経由は無料）。`ParsedCli::Policy` variant・`PolicyCliArgs`・dispatch arm・`command_name`（`cli.rs:450`）・`utility_success_envelope`（`cli.rs:365`）は新設が要る。`cheaper policy` の書き込みに必要な 3 点 — policy.yaml 読み出し（`read_yaml_layer` は `pub(crate)`）、YAML write、atomic write（`crates/cheaper-runtime/src/fs_util.rs:28-59` の `pub(crate)`）— は現状 crate-internal。tool-operator output schema（`crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`）は全階層 `additionalProperties: false` で evidence kind は closed enum のため、grant-request は新規 optional top-level field が要る（strict schema 制約: `required` ⊆ `properties`、`format` 不可、union `type` は non-null 2 個まで — `crates/cheaper-core/src/worker/schema/strict.rs`）。audit 面は `tools.degraded` span field（`run/mod.rs:419`・`:482` で record）と stderr diagnostic が先例として存在し、`usage.jsonl` record は fixed key list で拡張に code 変更を要する。

## Decision Drivers

* invoker が approval authority: grant の供給元は `--grant` flag と user policy に限り、model 入力・project policy は grant を authoring できない（`ProjectPartial` schema が `command` section を reject する既存境界は不変）。
* validation の単一権威経路: `--grant` 値・`policy grant add` 入力・policy file load で同一の `resolve_command_policy` 系統（schema 検査・deny list・`ArgPattern::compile`・env/cwd 規則 — `crates/cheaper-core/src/policy/command.rs:283`〜）が走ること。第 2 の検証経路を作らない。
* declared-shape parity の維持: `create_worker_scope` の `PartialEq` check を破らず、scope 生成後の広い grant set の密輸を防ぐ。
* fail-loud: silent な上書き・silent な無効化を避け、invoker の意図と policy の状態が衝突したら error で表面化する。
* KISS / YAGNI / DRY: 最小の layer 追加と既存機構の再利用を選び、表現 drift（grant spec の別構文・別 merge 規則）を持ち込まない。
* 観測可能性: invocation-sourced grant を policy-sourced grant と区別して観測できること（spec の audit 要件）。
* 機密値の露出面: `--grant` の flag 値（grant の `env` に secret を含みうる）は invoker の process cmdline（`ps`・`/proc`・shell history）に露出する。spawn される command は `env_clear` + grant `env` のみの最小 env で起動し ambient 環境変数は子プロセスへ届かないため、secret を flag 値へ置かない導線（docs warning + user policy の grant `env` / 子側 credential file への誘導）と、永続化先の `policy.yaml` 自体の保護（POSIX `0600`）が要る。

## Considered Options

primary decision は「`--grant` 値を effective policy の `command.grants` へどう畳み込むか」である。

* Option A: `LoadPolicyOptions` に `invocation: Option<serde_json::Map<String, Value>>` を追加し、user merge 直後・typed re-resolve 直前（load.rs:67-70 の gap）に `PolicySchemaMode::Partial` validate + `MergeMode::Replace` で merge する合成 trusted layer を設ける。
* Option B: CLI edge（`crates/cheaper-cli/src/run.rs:147` の `load_policy` 呼び出し後）で `RuntimePolicy` を post-merge に mutate し、grant を `command.grants` へ直接挿入する。
* Option C: grant を `RuntimePolicy` とは別の field / param として options struct 群（`WorkerScopeOptions` / `ToolRuntimeOptions` / `ensure_invocation` 入力）を通して運ぶ。

副次的決定（flag 値形式・collision 意味論・workflow 適用範囲・`grantRequests` shape・`cheaper policy` UX・audit surface・`--grant` 非露出時の扱い）は Decision Outcome の「副次的決定」で個別に options を評価する。

## Decision Outcome

Chosen option: **Option A** — `LoadPolicyOptions.invocation` として合成 trusted layer を user merge 直後に Partial validate + `MergeMode::Replace` で merge する。

理由: Option A は injection point が load pipeline の内側にあるため、merge 後に走る typed re-resolve（`resolve_command_policy`、load.rs:87）が invocation grant に対してもそのまま適用され、deny list・argv compile・env/cwd 規則の full validation を何も再実装せず継承する。grant は `RuntimePolicy.command` の一部として normalize されるため、run 2 call site・`ensure_invocation`・`create_worker_scope` parity check の全てが無変更で成立し、表現 drift が生じない。merge 順序として invocation layer は user layer の後・project layer の前に位置し、project layer は `command` section を schema reject するため invocation grant を override 不能 — 「invoker が approval authority」の意味論が layer ordering にそのまま写る。Option B は同じ validation を CLI edge で再実装する必要があり（post-merge mutation は resolved `RuntimePolicy` を直接触るため layer 規則の保護を受けない）、trusted-layer validation の責務を core の load 層から cli 層へ漏らす worse boundary である。Option C は `RuntimePolicy.command` の外に grant を運ぶため declared-shape `PartialEq` parity が即座に破綻し、run/mod.rs 2 箇所・`ensure_invocation`・scope 側への並行 plumbing を要求する — 表現 drift の温床であり却下。

採用にあたり、`invocation` layer の入力形状を **grants-only** に制約する: `LoadPolicyOptions.invocation` は `Option<&Map<String, Value>>` 相当の型とするが、merge 前に top-level key ⊆ `{version, command}` かつ `command` key ⊆ `{grants}` を検査し、それ以外の key を持つ入力は `InvalidRequest` + `details.layer="invocation"` で reject する。`Partial` schema validate だけでは `execution`・`tools`・`model` 等他 section が素通りするため、この key 制約が無いと invocation layer は第 2 の general trusted-policy channel となり flag 表面の責務（grant 供給のみ）を超える。型でなく merge 前検査で保証するのは、CLI 側で既に `{"version":1,"command":{"grants":{...}}}` 形状へ組み立てる経路と、将来の内部 caller 双方を同一規則で縛るためである。

### 副次的決定

* **flag 値の形式**: `--grant '<json-object>'` — 1 回の指定で 1 grant を表す complete JSON object を受け、grant name は object 内の `name` key で供給する。例: `--grant '{"name":"build","executable":"make","argv":["build"]}'`。CLI は `name` を取り出し、残りの key をそのまま grant spec（policy.yaml の `command.grants.<name>` map value と同一 shape）として `{"version":1,"command":{"grants":{<name>:<spec>}}}` の invocation layer object を構成する。spec shape と 1:1 のため `resolve_command_policy` 系統へ直通する。代替の `{<name>:<spec>}` map 形式（flag 1 回 = 1 grant は同じ）も同等に畳み込めるが、name を JSON key として入れ子にする分 shell quoting が一段深くなり、name を spec の外へ出す `<name>=<spec>` 分割形式は parse を一段増やし spec 内 `name` 混入時の扱いを別途定義する必要があるため、self-contained な flat object を採る。専用 `key=value` mini-format（`executable=...,argv=...`）は grant spec の validation（deny / ArgPattern / cwd / env）を再実装することになるため却下。`name` は grant spec schema の `additionalProperties: false` で reject される key であるため、CLI は validation 前に必ず `name` を strip する — flag 値に `name` を残したまま schema へ流す実装は常に失敗する。flag 値には parse 時のサイズ上限を設け、64KiB を超える値は `InvalidRequest` で reject する（cmdline 長制限と無制限入力への sanity bound を兼ねる）。`--grant` と `--worker-file` の併用は許可する — worker-file worker も `run` と同一の trust surface であり、grant が露出しない worker への付与は後述の warn-and-continue 規約で扱う。
* **repeatable flag**: `--grant` は複数指定で複数 grant を注入する。`Values` は `Vec<String>` + push とし、args.rs の文書化された last-wins 規約からの deliberate deviation（Node `parseArgs` の `multiple: true` 相当）として HELP と code comment に明記する。alias command（`tool-operate` 等）は `ParsedCli::Run` fold により無変更で `--grant` を受ける。同一 invocation 内で同名 grant を複数 `--grant` に指定した場合（intra-invocation duplicate）は `InvalidRequest`（details に重複した grant name + `details.layer="invocation"`）で reject する — flag-vs-policy collision と同じ fail-loud 規約であり、宣言順 push の暗黙 last-wins を許さない。
* **collision 意味論**: flag 由来 grant と builtin + user layer 既存 grant の同名衝突は **explicit error** とする。`load_policy` の invocation merge 時に `effective.command.grants` の key と invocation layer の grant name を比較し、衝突があれば `ErrorCode::InvalidRequest`（error envelope、`details.grant` に衝突した grant name、`details.layer="invocation"`）で load を fail させる。layer ordering に従う silent replace-wins は merge 規則の自然な帰結で追加経路が不要だが、invoker が「既存 grant を上書きした」と誤認する余地を残し audit 面でも不可視なため却下 — 意図しない上書きは loud に失敗させ、本当に差し替える場合は別名を使うか user policy を `policy grant` で更新するよう導線を固定する。
* **error attribution**: invocation layer 由来の検証失敗は一貫した provenance 規約を持つ — (a) shared validation（`validate_policy_schema` / `resolve_command_policy`）が返す失敗（schema 違反・deny list 該当・argv compile 失敗・env/cwd 規則違反）は inner error code（`InvalidRuntimePolicy`）を保持し、`details.layer="invocation"` を付す — 同一 spec を user policy に置いた場合と error code を揃え、policy 内容由来の失敗であることを表す。(b) invocation 入力自体の構造的失敗（malformed JSON flag 値・`name` key 欠落 / pattern 違反・grants 以外の key・intra-invocation 同名重複・policy との collision）は `InvalidRequest` + `details.layer="invocation"` とし、grant name が関与する場合は `details.grant` に載せる。規約は `load_policy` の invocation 処理と CLI の flag parse で共有する単一の rule として文書化する。
* **workflow 適用範囲**: `cheaper workflow run` も `--grant` を受ける。`prepare_workflow_context` が同一の `load_policy` を通るため（`workflow_context.rs:277`）`LoadPolicyOptions.invocation` をそのまま渡すだけで安価に含められ、`ensure_invocation` は `shared.policy.command` を serialize するため parity も自動的に保たれる。`kind: tool` job の `command: None`（workflow_run.rs:468）は不変 — `command: None` は declared-shape parity check を skip するが、scope 側は `inner.command`（invocation grant を含む resolved policy）を再注入する（`invocation.rs:268-272`）ため、grant は tool job にも届き user grant 由来の挙動と一致する。`execute_interface`（workflow_run.rs:665・:701）は同じ `prepare_workflow_context` を共有するが interface 検査経路に grant を流す意味がないため `invocation: None` を渡す。
* **`grantRequests` contract**: tool-operator output schema に新規 optional top-level field `grantRequests` を追加する。item shape は `additionalProperties: false`・`required: ["executable"]` で、`executable`（不足した executable — 必須。bounded string）、`argvHint`（`string[]`、optional — model が意図した argv の形状 hint。items に grant spec の bounded string と同じ `maxLength` 上界を設ける）、`name`（`string`、optional — 提案する grant name。grant の `propertyNames` と同一の `pattern: ^[a-z][a-z0-9_-]{0,63}$` と `maxLength: 64` で制約し、request から組み立てた grant name がそのまま spec schema を通ることを保証する）を持ち、`maxItems` で上界を設ける。emit 条件は prompt contract で「gated tool の欠如が原因で `status` が `blocked` または `partial` となった場合のみ」とする（schema で status 連動を強制しない）。`grantRequests` は実行すべき grant ではなく invoker へ表面化する request であり、parent agent が user approval を経て `--grant` 値を組み立てる材料に留める。executable のみの最小 hint は re-run の grant spec を parent が推測し直す必要があるため採らず、`unresolved` / `nextActions` の free text 定型文は machine-readable 要件を満たさないため却下。evidence kind enum への追加は「request は observation ではない」ため誤り。
* **`cheaper policy` UX**: `cheaper policy grant add <name> '<spec-json>'` / `list` / `remove <name>` とする。`add` の spec-json は `command.grants` map value と同一 shape（name を含まない — `add` は positional arg で name を受ける）で、`{"version":1,"command":{"grants":{<name>:<spec>}}}` を `validate_policy_schema(Partial)` に流すことで `resolve_command_policy` 経由の full validation を再利用する。**executable の存在確認は add 時には行わない** — executable pinning は tools 層の責務であり ambient PATH・workspace に依存する実行時検査であって、add 時に存在しない executable は scope 生成時に `POLICY_VIOLATION` で loud に失敗する。これは user policy を手書きした場合と同一の意味論であり、policy authoring 経路へ実行環境依存の検査を持ち込まない。structured flags（`--executable` / `--argv` 等）は grant spec の全 field を flag 化する表面積が大きく spec schema との drift を生むため却下。`list` は grant name + executable を表示し、`remove` は name 指定で削除する。存在しない name の `remove` は `InvalidRequest` の error envelope を返す（utility command の入力検証として一貫させる）。書き込み対象は user-scope `<userConfigDir>/policy.yaml` のみで、user-scope 解決は `resolve_user_config_dir`（`crates/cheaper-core/src/config/dirs.rs:37-53` — `CHEAPER_CONFIG_DIR` → `XDG_CONFIG_HOME/cheaper` → `<home>/.config/cheaper` の env/home 由来）を使い `--cwd` に依存しない。`policy` command が受ける flag は utility 規約の `--format` / `--quiet` / `--log` / `--log-format` のみで、`--worker-file` / `--grant` / `--cwd` / `--input` / `--timeout` 等の run 系・他 command 系 flag は reject する。`ParsedCli::Policy` は `format()` / `quiet()` accessor の match arm を populate する utility command 規約に従い、`command_name` は `"policy"`、成功は `utility_success_envelope`（`envelope.rs:65`）を返す — `result` に top-level `tokens` key は載せない（`record_usage` が utility envelope の `result.tokens` を count record の判定に使うため、`usage_log.rs:100-107`）。既存 section を破壊しないよう read-modify-write とし、atomic write（temp + rename）で保存する。
* **user policy write の基盤**: 必要な 3 点を core の public API として提供する。(a) user `policy.yaml` の layer 読み出し（`read_yaml_layer` の薄い公開 wrapper）、(b) YAML write は `serde_yml::to_string` wrapper を `yaml.rs` に置く（serde_yml は既に core の workspace pinned 依存であり新規依存を持ち込まない。JSON-as-YAML-subset 書き込みは人間が読む user policy が JSON になるため採らない）、(c) atomic write は `crates/cheaper-runtime/src/fs_util.rs:28-59` の `atomic_write` を **core へ移動**し、runtime 側は移動後の core API を参照する（複製はしない）。policy.yaml の write option は `windows_remove_first: false` とする — `std::fs::rename` は `MoveFileExW`+`MOVEFILE_REPLACE_EXISTING` semantics で Windows でも atomic rename が成立し、事前 remove は置換前の非 atomic な窓を作るため credentials 継承 workaround を持ち込まない。POSIX では `unix_mode: Some(0o600)` で書き込む — grant の `env` 値が secret を保持しうるため policy.yaml を owner-only readable にする。write path の caveat は Consequences に記す（read-modify-write の lost-update・serde_yml rewrite による comment/整形の喪失）。cmdline 露出対策としての `--grant-file`（file からの spec 読み込み）は本 iteration では実装せず、将来の follow-up 候補として残す。
* **audit surface**: 2 面を採用する。(a) tracing span field `grants.invocation` — `tools.degraded` の先例（run/mod.rs:419・`:482` の metadata-only field record）に倣い、invocation layer が供給した grant name の宣言順 join を run stage span に記録する。(b) stderr diagnostic line — run 開始時に invocation 由来の grant name 一覧を stderr へ 1 行出力し、log 無効でも常時観測可能にする。`usage.jsonl` record への field 追加は fixed key list の拡張と集計意味論の設計を伴うため本 iteration では defer（YAGNI）。
* **非 run command での `--grant` reject**: `"grant"` を `STRING_OPTIONS`（`args.rs:228-`）へ追加し、`Values` は `grants: Vec<String>` + push で保持する。`Vec<String>` は default empty のため、各 command arm での flag 検査を書き忘れると silent drop になる — これを許さず、非 run 系の全 command arm（`usage` / `count-tokens` / `models` / `auth` / `policy`）は `!values.grants.is_empty()` を既存の per-flag reject 規約（`--worker-file is only valid for \`run\`` 等）に倣って `InvalidRequest` で明示 reject し、`workflow` arm の allowlist には `run` 側のみ `"grant"` を追加する（`workflow interface` は拒否側）。各 arm の reject は `tests/args.rs` で command ごとに assert し、guard の抜けを test で固定する。
* **`--grant` の非露出**: warn-and-continue の範囲を限定する。**warn case**: `--grant` を渡したが worker が `command` を宣言していない場合（`--worker-file` worker を含む — grant は注入されるが tool が declared set に無い）、および grant は注入されたが `command` が worker の declared tool に含まれない・opt-in 縮退した場合は、stderr warning 行を出して run を続行する（「使われなかった grant」を失敗に変換しない — invocation grant が不要だった run は正常系であり得る）。**warn ではない case**: worker が `command` を宣言しているのに policy の `tools.allow` が `command` を除外する場合、`check_capabilities` が run 全体を `POLICY_VIOLATION` で fail させる既存契約（`capability.rs:24-40`）がそのまま適用される。同様に flag 由来 grant が `pin_command_grants` で pin 失敗（deny 該当・PATH 未解決・workspace-resident）した場合も warning でなく hard `POLICY_VIOLATION`（`registry.rs:1201-1210`）。grant 注入自体は admission（`tools.allow`・scope gate）を迂回しない hard 契約のままとする。

### Consequences

* Good, because grant validation は `resolve_command_policy` 系統の single authoritative path に畳み込まれ、`--grant` flag・`policy grant add`・policy file load の 3 経路で同一の検査（deny list・argv compile・env/cwd 規則）が走る。deny list 該当 executable は `--grant` 経由でも load 時に reject される（spec AC3）。
* Good, because grant が `RuntimePolicy.command` に normalize されるため declared-shape parity・run 2 call site・`ensure_invocation`・alias fold が全て無変更で成立し、workflow への適用も load path 共有で安価に得られる。
* Good, because invocation layer は user merge 直後・project 前の ordering に固定され、project layer は `command` を schema reject するため invocation grant を override 不能 — trust boundary が layer 順序として構造的に表れる。
* Good, because 同名 collision を `INVALID_REQUEST` で fail させることで invoker の意図と policy 状態の衝突が常に表面化し、`grantRequests` → approval → `--grant` re-run loop は machine-readable な `executable` + `argvHint` で機械生成可能になる。
* Good, because `cheaper policy grant` が validation 済みの永続化経路を提供し、手書き YAML の schema / deny / argv 誤りを write 前に検出できる。
* Good, because `invocation` layer の key 制約（top-level ⊆ `{version, command}`、`command` ⊆ `{grants}`）により、flag / 将来の内部 caller が `execution`・`tools`・`model` 等他 section を注入して第 2 の general trusted-policy channel 化することを構造的に防ぐ。
* Bad, because `LoadPolicyOptions` の signature 変更で全 caller の更新が要る — `run.rs:143`・`workflow_context.rs:277`（`workflow run` と `execute_interface` が共有。`execute_interface` は `invocation: None`）・`count.rs:69`（`None`）に加え、core の golden `tests/golden/policy_load.rs:18` と `load.rs` unit test（:163/:193/:207/:222）も `invocation` field の追加に追随する。load 順の意味論に第 4 の layer（invocation）が増える。ordering 意味論は `load.rs` の doc comment と docs に明記する。
* Bad, because `--grant` の flag 値は invoker の process cmdline（`ps`・`/proc`・shell history）に露出する — grant の `env` 値に secret を含めると credential が漏洩しうる。spawn される command は `env_clear` + grant `env` のみの最小 env で起動し ambient 環境変数は子プロセスへ届かないため、secret は user policy の grant `env` 値か子プロセス自身が読む credential file へ置く運用を docs deliverable（command guide・policy guide・配布 SKILL）で推奨する。`policy grant add <name> '<json>'` も spec を一時的に cmdline へ載せる同種の caveat を持つ。`policy grant add` による policy.yaml の write は POSIX `0600` で行う。cmdline を経由しない `--grant-file` は将来の follow-up 候補。
* Bad, because `policy grant add`/`remove` の read-modify-write は lost-update を防げない — 並行する `add` や user の手編集が割り込むと last-wins になり、atomic write が保証するのは torn write（中途半端な内容の file）の防止のみである。docs に明記する。
* Neutral, because `serde_yml` による policy.yaml rewrite は既存 file の comment・key 順・整形を保存しない — user policy の可読性が落ちる accepted trade-off であり、手編集併用時の注意として docs に明記する。
* Neutral, because `policy grant add` は executable の存在確認を行わない — pin は tools 層の実行時検査であり、存在しない executable は scope 生成時に `POLICY_VIOLATION` で報告される（user policy 直書きと同一の意味論）。
* Bad, because repeatable `--grant` は args.rs の last-wins 規約からの deliberate deviation であり、HELP・docs・code comment の 3 箇所で明示しないと将来の reader が last-wins を期待して誤読する。
* Bad, because `atomic_write` の core 移動は `cheaper-runtime` の既存 caller に波及する（移動自体は機械的だが、公開面が増える）。
* Bad, because `grantRequests` は advisory であり、model が不正確な `argvHint` を emit し得る — approval 判断は parent agent / user の責務であり、cheaper は request の正しさを保証しないことを docs に明記する。
* Neutral, because `usage.jsonl` record shape は不変（audit は span field + stderr line の 2 面のみ）であり、将来集計要件が生じた場合に別途 record 拡張を検討する。

### Confirmation

* dev-seam fixture test: grant の無い環境で `command` が縮退した worker へ `--grant` 付き re-run を行い、degrade が解消して command が実行できることを検証する（`run_dev_seam.rs` — `command_user_policy` / `FIXTURE_COMMAND_CALL` パターン）。flag 無しの再実行で再び縮退し、policy file が一切変更されていないこと（非永続性、AC2）を同 test で確認する。
* `tests/args.rs` の parse table: repeatable `--grant`（`--grant <v>` / `--grant=<v>` 両形・複数指定の順序保持）、alias command 経由での受理。非 run 系 command（`usage` / `count-tokens` / `models` / `auth` / `policy` / `workflow interface`）への `--grant` が全 command arm で `InvalidRequest` reject されることを command ごとに assert する（`Vec<String>` default-empty による silent drop の guard）。
* `run.rs` / load 経路の test: 同名 `--grant` の重複指定が `InvalidRequest`（details に grant name + `details.layer="invocation"`）で fail すること、64KiB 超の flag 値が `InvalidRequest` で reject されること、`grants` 以外の `command` key や `command`/`version` 以外の top-level key を含む invocation layer が `InvalidRequest` + `details.layer="invocation"` で reject されること。
* `policy grant` の write-mode test: POSIX で `policy.yaml` が `0600` で書かれること、`windows_remove_first: false` 経路で atomic rename が行われること。
* `cheaper policy grant` の round-trip test: temp config dir で `add` → `list`（name + executable 表示）→ `remove` が動作し、invalid spec（deny list 該当・argv compile 失敗・schema 違反）は policy file を変更せず error envelope を返す。存在しない name の `remove` は `INVALID_REQUEST` の error envelope。
* collision test: user policy と同名の `--grant` で `INVALID_REQUEST`（`details.grant` + `details.layer="invocation"`）となり run が開始前に失敗する。
* core の golden `policy_load` test に invocation layer case を追加（invocation grant が user grant に追加・project 層より優先されること、collision で error になること）。
* `proc_invocation` parity test が変更無しで green であること（`RuntimePolicy.command` への畳み込みにより parity 契約を破らない）。
* `mise run check`（biome / fmt / clippy / version・doc-id check / `cargo test --workspace`（dev-seam feature 付き）/ node tests / lockfile・payload check）が pass する。

## Implementation Plan

* Affected paths:
  * `crates/cheaper-core/src/policy/load.rs`: `LoadPolicyOptions` に `invocation: Option<&Map<String, Value>>` を追加。merge 前に grants-only key 制約（top-level ⊆ `{version, command}`、`command` ⊆ `{grants}`）を検査し違反は `InvalidRequest` + `details.layer="invocation"` で reject。user merge 直後（:67-70 の gap）に `Value::Object(invocation)` を `PolicySchemaMode::Partial` で validate → `MergeMode::Replace` で merge。merge 前に `effective` の `command.grants` key との同名衝突を検査し `InvalidRequest`（`details.grant` + `details.layer="invocation"`）で fail。shared validation の失敗には `details.layer="invocation"` を付して inner code を保持する（error attribution 規約）。`tracing::debug!(policy.layer = "invocation", ...)` を追加し、doc comment に 4 層目の ordering 意味論と key 制約を明記。
  * `crates/cheaper-cli/src/args.rs`: `"grant"` を `STRING_OPTIONS`（:228-）へ追加し、`Values` に `grants: Vec<String>`（push、`set_string` に arm 追加）を持たせる。`RunCliArgs` / `WorkflowRunCliArgs` に `grants: Vec<String>` を追加し populate 経路（:903-912）を更新。flag 値の 64KiB 上限を parse 段で enforce。非 run 系の全 command arm（`usage` / `count-tokens` / `models` / `auth` / `policy`）に `!values.grants.is_empty()` の明示 reject を追加し、`workflow` arm の allowlist には `run` 側のみ `"grant"` を加える（last-wins からの deviation と per-command guard を comment で明記）。HELP 本文を更新。
  * `crates/cheaper-cli/src/run.rs`: :143 の `LoadPolicyOptions` 構築で `args.grants` を parse（各値 `serde_json::from_str` → `name` 取出し・strip → `{"version":1,"command":{"grants":{name:spec}}}` 合成 → intra-invocation 同名重複の `InvalidRequest` 検査）し `invocation` へ渡す。run 開始時に grant name の stderr diagnostic line と、非露出時（worker が `command` を declared tool に持たない場合）の stderr warning を emit — `tools.allow` 除外は `check_capabilities` の `POLICY_VIOLATION` に任せ warning に変換しない。
  * `crates/cheaper-cli/src/workflow_context.rs` + `crates/cheaper-cli/src/workflow_run.rs`: `prepare_workflow_context` の `load_policy` へ `invocation` param を追加し、`execute_run` は `WorkflowRunCliArgs.grants` から合成した layer を、`execute_interface`（:665・:701 — 同関数を共有）は `None` を渡す。`kind: tool` job の `command: None` は不変（parity check は skip されるが `inner.command` が再注入される — user grant と同じ挙動）。
  * `crates/cheaper-cli/src/count.rs`:69: `invocation: None` を渡す（count-tokens は grant を受けない）。他の `LoadPolicyOptions` caller — core の golden `tests/golden/policy_load.rs:18` と `load.rs` unit test（:163/:193/:207/:222）— も `invocation: None` で更新する。
  * `crates/cheaper-cli/src/cli.rs` + `args.rs`: `ParsedCli::Policy` variant・`PolicyCliArgs`（`grant {add <name> <spec-json>,list,remove <name>}` subcommand 構造）・dispatch arm・`command_name`（`"policy"`）・HELP・成功時 `utility_success_envelope`（`envelope.rs:65`）を接続。`ParsedCli::format()` / `quiet()` accessor（args.rs:181-203）の match arm に `Policy` を追加し utility command 規約に従う。success envelope の `result` に top-level `tokens` key を載せない（usage_log.rs:100-107 の count-record 判定との衝突回避）。
  * `crates/cheaper-core/src/policy/`（`load.rs` / `yaml.rs` / `command.rs` 周辺）: user `policy.yaml` 読み出しの公開 helper、`serde_yml::to_string` write wrapper、`grants` map の read-modify-write helper を追加。`crates/cheaper-runtime/src/fs_util.rs` の `atomic_write` / `AtomicWriteOptions` を core へ移動し runtime caller を更新。policy.yaml write は `windows_remove_first: false`（Windows の atomic rename を利用）+ POSIX `unix_mode: Some(0o600)`。
  * `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`: optional top-level `grantRequests`（item: `required:["executable"]`、`argvHint?: string[]`（items に `maxLength`）、`name?: string`（`pattern: ^[a-z][a-z0-9_-]{0,63}$` + `maxLength: 64` — grant `propertyNames` と同一）、`additionalProperties:false`、`maxItems` 上界）を追加。
  * `crates/cheaper-cli/resources/workers/tool-operator.md`: blocked / partial が gated tool 欠如由来の場合に `grantRequests` を emit する prompt contract を記述（request は grant の実行ではなく invoker への提示である旨を明記）。
  * `crates/cheaper-runtime/src/run/mod.rs`: `cheaper.run.tool_scope` span に `grants.invocation` field（宣言順 join）を record する経路を追加（`tools.degraded` と同型の metadata-only field）。
  * `crates/cheaper-cli/tests/args.rs`、`crates/cheaper-cli/tests/run_dev_seam.rs`、regression harness（`extra_args` + `config.files` で user policy seeding）、core `policy_load` golden test、`proc_invocation` parity test。
  * `docs/guides/command.md`・`workers.md`・`usage.md`・`policy` 系 guide、`packages/apm/.apm/skills/cheaper-usage/SKILL.md`・`cheaper-tool-operate/SKILL.md`: `--grant` contract（repeatable・JSON 形式・collision error・非永続）、`grantRequests` → approval → re-run loop、`cheaper policy grant` の UX、および flag 値が process cmdline（`ps`/`/proc`/shell history）に露出する warning（spawn は `env_clear` + grant `env` のみの最小 env で ambient 環境変数は届かないため、secret は flag 値ではなく user policy の grant `env` / 子側 credential file へ）を文書化。policy.yaml の手編集併用時の caveat（last-wins・comment 非保存）も記す。
* Patterns to follow: `PolicySchemaMode::Partial` + `MergeMode::Replace` の既存 layer 規則、`resolve_command_policy` の single validation 経路、`invalid_request` helper の args error 形状、`tools.degraded` の metadata-only span field 先例、`utility_success_envelope` の utility command 出力、dev-seam `command_user_policy` fixture パターン。
* Patterns to avoid: post-merge `RuntimePolicy` mutation、grant の別 field / 別 param 運搬（parity 破綻）、`key=value` mini-format（validation 再実装）、structured flag 化（spec schema の drift）、evidence enum への request 混入、collision / intra-invocation duplicate の silent replace・last-wins、invocation layer への grants 以外の key 混入（第 2 trusted channel 化）、非 run command arm での `--grant` 無検査（default-empty `Vec` の silent drop）、`tools.allow` 除外や pin 失敗の warning 化（hard `POLICY_VIOLATION` を弱めない）、`--grant` flag 値への secret 埋め込みを推奨する docs、`policy grant add` 時の executable 存在確認（実行環境依存検査の混入）、`usage.jsonl` record shape の本 iteration での変更、success envelope `result` への top-level `tokens` key、`atomic_write` の複製・`windows_remove_first: true` での policy write。
* Dependencies or configuration: 新規依存なし。`serde_yml`（workspace pinned `=0.0.13`）は既に core の依存であり write wrapper は scoped `#[allow(deprecated)]` 規約に従う。

## Verification

* [ ] grant 不在環境で縮退した worker へ `--grant '<json>'` 付き re-run を行い `command` が実行できる dev-seam fixture test が通り、flag 無し再実行で再び縮退・policy file 不変であることが確認される（AC1・AC2）。
* [ ] deny list 該当 executable を含む `--grant` が load 時に reject される test が通る（AC3 — invocation 経路で deny list を迂回できない）。error は inner code（`INVALID_RUNTIME_POLICY`）を保持し `details.layer="invocation"` を持つ。
* [ ] flag 由来 grant と user policy 同名 grant の衝突が `INVALID_REQUEST`（`details.grant` + `details.layer="invocation"`）で fail し、intra-invocation の同名 `--grant` 重複も `INVALID_REQUEST` で fail する test が通る（AC4・AC13）。
* [ ] `grantRequests` を含む blocked / partial 出力が output schema 適合（`additionalProperties: false` discipline 維持）で envelope `result` から parse できる test が通る（AC5）。
* [ ] `cheaper policy grant add` が validation 通過 grant を user policy へ atomic に永続化し後続 run で有効になり、invalid spec は file 不変で error を返す test が通る（AC6）。
* [ ] `cheaper policy grant list` が name + executable を表示し、`remove <name>` 後に後続 run で grant が効かなくなる test・存在しない name の remove が `INVALID_REQUEST` の error envelope になる test・success envelope `result` に top-level `tokens` key が無い test が通る（AC7・AC16）。
* [ ] `--grant` run が `grants.invocation` span field と stderr diagnostic line で invocation-sourced grant を区別して観測できる test が通る（AC8）。
* [ ] `cheaper policy` command が `usage.jsonl` に record を残さないことが確認される（AC9 — utility command 規約）。
* [ ] worker alias（`tool-operate`）経由で `--grant` が同等に効く test が通る（AC10）。
* [ ] `--grant` を渡したが worker が `command` を宣言していない run（`--worker-file` worker 含む）で stderr warning が出て run が続行する test、および worker が `command` を宣言するが `tools.allow` が除外する run が warning でなく `POLICY_VIOLATION` で fail する test が通る。
* [ ] `cheaper workflow run` への `--grant` が `prepare_workflow_context` 経路で同一挙動となる test が通る（AC14）。`kind: tool` job が `inner.command` 再注入経由で invocation grant を受けることを確認する。
* [ ] repeatable `--grant` の parse（両形・複数指定・順序保持）と複数 grant の同時注入の `tests/args.rs` / run 経路 case が通り（AC15）、非 run 系 command（`usage` / `count-tokens` / `models` / `auth` / `policy` / `workflow interface`）への `--grant` が全 arm で `INVALID_REQUEST` reject される test が通る（AC17）。
* [ ] invocation layer を含む `policy_load` golden test（user への追加・ordering・collision・grants 以外の key の reject）が通る。
* [ ] parent-facing docs（cheaper-usage / cheaper-tool-operate SKILL、guides）に grant-request → approval → `--grant` re-run loop が文書化されている（AC11）。
* [ ] `mise run check` が pass する（AC12）。

## Pros and Cons

primary decision（invocation grant の格納先）の options 比較:

| Option | Pros | Cons |
| --- | --- | --- |
| A: `LoadPolicyOptions.invocation` 合成 layer（採用） | `resolve_command_policy` の full validation を merge 後の typed re-resolve で継承。`RuntimePolicy.command` に畳み込まれるため parity・run 2 call site・`ensure_invocation` が無変更。project は `command` reject のため override 不能で ordering 意味論が trust boundary をそのまま表す | `LoadPolicyOptions` signature 変更と全 caller 更新。load 順に第 4 の layer が増え ordering 意味論の文書化が要る |
| B: CLI edge で post-merge `RuntimePolicy` mutation | load.rs 不変。変更面が cli crate に閉じる | resolved struct を直接触るため layer 規則の保護がなく、trusted-layer validation を CLI edge で再実装する worse boundary。grant spec を policy 表現と runtime 表現で二度扱う drift 源 |
| C: `RuntimePolicy` と別の field / param で運搬 | policy load 経路を一切触らない | `create_worker_scope` の declared-shape `PartialEq` parity が即破綻。run/mod.rs 2 箇所 + `ensure_invocation` + scope 側への並行 plumbing で表現 drift の温床 |

## More Information

* source: GitHub issue #105（`--grant` flag・grant-request hint・`cheaper policy grant` の 3 component 要求の正本）。
* derives-from: `../discovery/invocation-scoped-command-grants.md`（DISC-034FlIAczzK2ruZW3HTcJi。injection point・parity 契約・validation 再利用 map・decision axis 軸1–7 の調査の正本）。
* related: `../specs/invocation-scoped-command-grants.md`（製品要件の正本 — 機構選択を本 ADR へ defer）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（ADR-034EMgaatFhsXjR97kH14c。trusted-layer-only grant authoring と fail-closed registration の正本 — invocation layer はこの境界の trusted 側に位置する）、`../adr/opt-in-soft-gating-of-policy-gated-tools-for-the-tool-operator-worker.md`（ADR-034ESBcMTWxNCMDI9obdb8。`cheaper.optionalTools` 縮退機構 — `grantRequests` はその blocked / degrade 結果を machine-readable 化する）、`../specs/policy-gated-command-tool.md`（`command` tool の grant・validation・deny list 契約の正本）。
