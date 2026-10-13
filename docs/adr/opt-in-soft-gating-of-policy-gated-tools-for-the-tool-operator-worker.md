---
id: "ADR-034ESBcMTWxNCMDI9obdb8"
type: "adr"
status: "accepted"
title: "policy-gated tool の露出を worker opt-in の optional 宣言で縮退可能にする"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: ["https://github.com/scarecrow173/cheaper/issues/102"]
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
    - "../specs/policy-gated-command-tool.md"
    - "../specs/tool-operator-built-in-worker.md"
    - "../test-specs/tool-operator-built-in-worker.md"
  refines: []
  refined-by: []
  derives-from: ["../discovery/tool-operator-worker-built-in.md"]
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

# Opt-in soft gating of policy-gated tools for the tool-operator worker

## Context and Problem Statement

issue #102 は built-in worker `tool-operator`（CLI alias `tool-operate`）を追加し、`read` / `grep` / `find` / `ls` / `count_tokens` / `process` / `command` を宣言する。spec `../specs/tool-operator-built-in-worker.md` がこの tools 集合と「policy が `process` / `command` を許可しない環境でも worker は usable でなければならない（利用可能な tools で縮退実行するか、理由付き `blocked` を返す）」という挙動要件を固定し、実現機構の選択（discovery の Option A–D）を本 ADR へ defer した。

現行の scope 構築は fail-closed である。`create_tool_runtime`（`crates/cheaper-tools/src/registry.rs`）は `names` に `Process` を含み `process.tasks` が空なら `POLICY_VIOLATION` "process exposure requires nonempty authorized tasks"（registry.rs:1066-1071）、`Command` を含み `command.grants` が空なら同じく `POLICY_VIOLATION` "command exposure requires nonempty authorized grants"（registry.rs:1075-1085、加えて writeScope × command の不併存 reject）で scope 生成を拒否する。builtin policy（`crates/cheaper-cli/resources/policy.yaml`）は `process.tasks: {}`・`command.grants: {}` で出荷されるため、**7 tool を宣言した `tool-operator` は default install では必ず scope 構築で死に、model が一度も走らない**。

この gate は ADR-034EMgaatFhsXjR97kH14c（`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`）が固定した信頼境界の一部であり、弱めること自体は目的ではない。問題は「gate の不成立を run 全体の起動失敗として表現するか、worker への degraded tool 集合として表現するか」という scope 構築 contract の一点にある。

もう一層の admission として `check_capabilities`（`crates/cheaper-core/src/policy/capability.rs`）がある。worker 宣言 `tools` ⊆ `policy.tools.allow` と tier ≤ `maxTier` を検査し、`run_worker_inner`（`crates/cheaper-runtime/src/run/boundary.rs:100-101`）で scope 構築**より先**に実行される。`tools.allow` による tool 拒否と「tool は allow 済みだが content-gate（nonempty tasks / grants）が未成立」は意味が異なり、本決定はこの 2 層を混同しない。

## Decision Drivers

* fail-closed の維持: 未認可の `process` / `command` **call** は引き続き不可能であること。緩和するのは scope 構築時の「登録失敗を即 run 死亡とする」契約のみで、call 経路の grant / argv / executable 検査は一切緩めない。
* issue 要件の忠実性: 「無許可時は利用可能な tools の範囲で実行するか blocked を返す」。degraded run は model session まで到達しなければならず、run-level `POLICY_VIOLATION`（model 未到達・`status` を output schema で表現不能）はこの要件を満たさない。
* 後方互換: 既存 worker の意味論を変えない（opt-in のみ）。非 opt-in の `tools: [command]` worker は引き続き `POLICY_VIOLATION` で fail する（既存 test `run_fake_stream_command_worker_denied_without_grants` がそのまま green であり続けること）。
* KISS / YAGNI: 最小の opt-in 機構を選び、layer provenance tracking や merge 層の変更を要求しない。
* 表現 drift 防止: 「scope 構築時 content-gate を持つ tool」の集合定義を一箇所（core の tool vocabulary）に置き、validate 層と registry 層で二重定義しない。
* 観測可能性: degraded tool は model の tool 一覧と system prompt から見える形で外す。model が存在しない tool を呼んで turn を浪費する silent degradation にしない。

## Considered Options

* Option A: `process` / `command` を宣言したままにし、gate 不成立は現行通り scope 構築時 `POLICY_VIOLATION` とする。issue の "blocked" を run 起動失敗として解釈する。
* Option B: `process` / `command` を宣言せず、常時利用可能な read 系 tool（`read` / `grep` / `find` / `ls` / `count_tokens`）のみで縮退運用する。
* Option C: optional / gated tool 宣言の新機構を導入し、gate 不成立の tool は scope 構築で「未登録」へ縮退させる。variant:
  * C1: frontmatter の per-tool 注釈（新規 strict key。例: `cheaper.optionalTools: ["process", "command"]`）で、縮退してよい tool を worker ごとに列挙する。
  * C2: runtime が固定集合 `{process, command}` を全 worker で soft-gated 扱いにする。
  * C3: declaration は静的なまま、worker 単位の boolean opt-in flag で `{process, command}` の gate 不成立を「未登録」へ縮退させる。
* Option D: `process` / `command` を宣言し、builtin policy に非空の `tasks` / `grants` を同梱する。

## Decision Outcome

Chosen option: **Option C — variant C1（worker frontmatter の `cheaper.optionalTools: string[]` で per-tool opt-in）**。gate 不成立の gated tool は scope 構築時に error ではなく「未登録」へ縮退し、残りの tool で run が続行する。

保証範囲の明確化: issue の「許可されない環境でも usable」がカバーするのは empty `process.tasks` / `command.grants` の content-gate 不成立のみである。`tools.allow` による宣言 tool の除外は admin の admission 意図として従来通り run-level `POLICY_VIOLATION` であり、degraded path が `check_capabilities` を迂回することはない。

理由: C1 は issue の「利用可能な範囲で実行 or `blocked`」を model 到達後の挙動としてそのまま実現し、かつ緩和を「その worker が明示的に opt-in した tool」だけに閉じ込める。Option A は実装不要だが default install で worker が常に死ぬ（bad first-run UX）うえ、`blocked` が output schema に現れず spec の status 意味論と不整合になる。Option B は issue の明示的 tools 集合と command 使用要件を満たせない。Option D は常時認可面を作る security regression であり論外。C2 は opt-in を持たず既存 worker の意味論を静かに変える — `tools: [command]` のみの worker が grants 不在で「tool 0 個の run」へ縮退し、fail すべき構成誤りを握り潰すため blast radius が広すぎる。C3（boolean flag）は C1 と同じ core 変更量で粒度だけが粗い — 「`command` は縮退可・`process` は必須」のような将来の worker を表現できず、frontmatter 上でどの tool が absent になり得るか読者が固定集合の知識を要するため、self-documenting な list 形の C1 を採る。

### 機構の確定仕様

* frontmatter: `cheaper` 節に新規 strict key `optionalTools`（`string[]`）を追加する。validate 規則: 各要素は canonical tool 名、`tools` に宣言済み、かつ scope-gated tool（現行では `{process, command}` — 権威定義は `cheaper-core` の tool vocabulary が持つ `is_scope_gated_tool` predicate）であること。重複・未宣言・非 gated 名・vocabulary 外の名は `InvalidWorkerDefinition` で reject する（strict keys の既存方式に従う）。加えて `optionalTools` は `tools` の真部分集合でなければならず、`optionalTools ⊇ tools`（全 tool が縮退し得る = tool 0 個の run を許す nonsense 宣言）は reject する。
* scope 構築（`create_tool_runtime`）: `names` 内の gated tool について content-gate（`process` = nonempty tasks、`command` = nonempty grants）が不成立かつ worker の `optionalTools` に含まれる場合、その tool を登録集合から除外して続行する。opt-in の無い tool / worker は現行通り `POLICY_VIOLATION`。ここで「gate 不成立」は empty `process.tasks` / `command.grants` の content-gate 不成立のみを指す — policy の pin 失敗、`resolve_command_policy` / `resolve_process_policy` の error、`command` × writeScope の不併存 reject は opt-in tool に対しても縮退対象**外**の hard `POLICY_VIOLATION` のままとする（`command` が登録される限り writeScope 不併存は従来通り fail。`command` が空 grants で縮退した scope ではこの check は不発）。
* capability check との関係: `check_capabilities` の `tools.allow` ⊆ 検査は**不変・hard のまま**で、gating より先に評価される。`optionalTools` は `tools.allow` 拒否を免除しない — policy が `command` を含め `read` でも `tools.allow` から外した worker 宣言は従来通り `POLICY_VIOLATION` で fail する。理由: `tools.allow` は tool class 単位の明示的 admission であり、それを静かに縮退すると admin の明示的境界を run 成功で覆すことになる。一方「allow 済みだが tasks / grants が空」は tool class の拒否ではなく認可済み content の不在であり、ここだけが soft 化の対象。admin が `command` を禁止しつつ tool-operator の縮退実行を許したい場合は `tools.allow` ではなく grant を定義しない（builtin 既定）ことで実現する、という棲み分けを docs に明記する。
* 縮退の surface: `ToolRuntime` が縮退した tool 名を報告し、run 層（`crates/cheaper-runtime/src/run/mod.rs`）は `build_system_prompt` の "Available tools" を**実登録集合**から組み立て、加えて「宣言されたが現在の policy 下では利用不可」の tool 名列挙と「それが必要なら `status: blocked` で終了せよ」という注記を prompt contract へ載せる。model 側から見て degraded tool は存在しない — 名前での call は既存の unknown-tool / invalid-call 経路（`invalid_tool_retries` budget 内）で処理され、権限変化は無い。
* security 解析: 本機構は登録面を縮めるだけで広げない。縮退した `command` / `process` は registry に登録されないため実行経路自体が存在せず、per-call の grant 解決・argv pattern・executable 検証（native magic / shell / shim 拒否、`minimal_env`）は従来通りで、縮退後の tool に対しては到達不能。非 opt-in worker の fail-closed gate、project layer からの grant authoring 不能（ADR-034EMgaatFhsXjR97kH14c）、`tools.allow` の hard admission はいずれも不変。privilege の変化は無い。`optionalTools` は built-in に限らず全 worker origin（project / user / `--worker-file` 定義）で利用可能だが、strictly capability-shrinking であり grant の authoring・未登録 executor への到達はいずれの origin でも不可能。なお `command` × writeScope の reject は環境依存で挙動が分岐する — grants 非空で `command` が登録される環境では従来通り hard-fail し、grants 空で `command` が縮退する環境では check 自体が不発して silent に absent となる。

### 副次的決定

* `tool-operator` は `tools: [read, grep, find, ls, count_tokens, process, command]` + `cheaper.optionalTools: [process, command]` で宣言する。`process` も `command` も初期 tools に含める（issue の明示的 tools 集合。gate 条件は独立なので、tasks のみ defined な環境では `process` のみ残る）。
* per-worker execution 制限は設けない — global policy 制限（`execution.maxTurns=8` / `maxToolCalls=64` / `timeout=120s` / `output.maxTokens=2000`）に従う（issue・spec の Out of Scope に合致）。不足が実測された場合は strict `cheaper` keys の拡張として別 spec で扱う。
* `tool-operator` の model は `tier: low`、`workload.outputPerInput: 0.3` とする。output envelope は findings + evidence + unresolved + nextActions を持ち、bulk-reader の圧縮 findings（0.05）より発話量が多く、writer 系（1.0）より小さい中間的 workload。0.3 は code-writer の既存値と揃い、auto-tier の effectiveCost でも保守的に振る舞う。
* worker 別 APM skill `cheaper-tool-operate` を `packages/apm` に追加する（`cheaper-bulk-read` / `cheaper-code-write` / `cheaper-doc-write` との parity、AC12 の公開面文書の一部）。
* `ollama_e2e.rs` の実 provider 検証は本 iteration の対象外とする。primary 検証は dev-seam fake-provider fixture + regression corpus で行い、production session 経路自体は既存 worker の e2e で既にカバー済み。provider 依存の問題が出た場合に別途 revisit する。
* `savings.directlyComparable` は `is_directly_comparable_worker`（validate.rs:273-285）が**宣言** tools を見るため、`command` / `process` が縮退した run でも `false` のまま — 縮退の有無で comparable 判定を変えない保守的扱いを確認する。`meta.changes` は write / edit を宣言しないため常に空のまま（`command` 子 process 副作用を audited change と主張しない honest reporting、spec の契約）。
* envelope `meta` の shape は変えない。縮退した tool 名は `cheaper.run.tool_scope` stage span の field と system prompt 注記に記録し、result 側の `status: blocked` 意味論は spec（output schema）が担う。

### Consequences

* Good, because issue #102 の「無許可時は利用可能な tools で実行 or `blocked`」を model 到達後の挙動として満たせ、default install（builtin `tasks` / `grants` が空）でも `tool-operator` が read 系 tool で即座に使える first-run UX になる。
* Good, because 緩和が opt-in 宣言した worker の、opt-in 列挙した tool の、content-gate 不成立時の「未登録化」に閉じる。call 経路の認可機構は一切変わらず、非 opt-in worker の挙動と既存 test は完全に不変。
* Good, because `tools.allow`（admission）と content-gate（認可済み content の有無）の 2 層意味論が分離したまま保たれ、capability check は worker flag に依存しない単一の hard 層であり続ける。
* Good, because `optionalTools` の list 形は「この worker で absent になり得る tool」を frontmatter から自明に読める自己文書化であり、gated tool が将来増えても worker 側の宣言だけで対応できる。
* Bad, because `cheaper` strict keys に新規 key を足す core 変更（validate.rs + worker schema 面）が必要で、worker 定義フォーマットの表面が広がる。project / user 定義の既存 worker は影響を受けないが、新 key の誤用（非 gated tool や未宣言 tool の列挙）は validate error として通知する必要がある。
* Bad, because 「tool が allow 済みだが未登録」という中間状態が system prompt と span にしか残らず、envelope `meta` からは読めない。debuggability のために span field と prompt 注記の内容を cheaper-runtime test で固定する。
* Bad, because model が degraded tool 名を hallucinate して呼ぶ turn を完全には防げない（prompt 注記で抑止し、呼ばれても unknown-tool 経路で安全に失敗するが、invalid-tool retry budget を消費し得る）。
* Neutral, because builtin policy の `process.tasks` / `command.grants` は空のまま据え置く — `tool-operator` が optional 機構の最初の consumer となり、「builtin が認可面を持ち込む」Option D 型の設計は引き続き採用しない。

### Confirmation

* grants / tasks が空の builtin-default 環境で、`optionalTools` 宣言付き worker（`tool-operator` または `--worker-file` の opt-in fixture worker）が scope 構築を通過し、`process` / `command` を除く tool 集合で run が完走する dev-seam fixture test が pass する。CLI-level fixture は degraded run の envelope（`ok: true`、`meta.savings.directlyComparable == false`）のみを assert し、system prompt に "Available tools" 縮退集合と利用不可注記が含まれることの assert は cheaper-runtime test 層（`FakeSessionFactory` handle 経由で prompt を捕捉）で行う。
* 同 fixture で fake stream が `status: blocked` の最終 JSON を返す場合、output schema validation を通り envelope の `result.status == "blocked"` となることを assert する。
* 非 opt-in の `tools: [command]` worker が引き続き `POLICY_VIOLATION`（exit 4、"nonempty authorized grants"）で fail する既存 test（`run_fake_stream_command_worker_denied_without_grants`）が green のままであること。
* `tools.allow` から宣言 tool（例: `read`）が外れた policy では opt-in worker も capability check で `POLICY_VIOLATION`（`missing` details 付き）になる test。
* grant が定義された環境では opt-in worker の `command` が通常通り登録・実行できること（`command_user_policy` + `FIXTURE_COMMAND_CALL` パターン）。
* `optionalTools` の validate 規則（`tools` 未宣言・非 gated・重複・vocabulary 外・`optionalTools ⊇ tools` の reject、未指定時の既存挙動不変）の unit test。
* regression corpus に縮退 run の case が追加されること。
* `mise run check`（biome / fmt / clippy / version・doc-id check / `cargo test --workspace`（dev-seam feature 付き）/ node tests / lockfile・payload check）が pass する。

## Implementation Plan

* Affected paths:
  * `crates/cheaper-core/src/tools/vocabulary.rs`: scope-construction content-gate を持つ tool の一権威 predicate `is_scope_gated_tool`（現行 `{Process, Command}`）を追加し、validate 層と registry 層の双方から参照する（表現 drift 防止）。
  * `crates/cheaper-core/src/worker/validate.rs`: `CHEAPER_KEYS` に `"optionalTools"` を追加し `WorkerCheaper.optional_tools` として保持する。`normalize_tool_names` で名解決し、`tools` の真部分集合（`optionalTools ⊇ tools` は reject）・⊆ gated 集合・重複無しを検査する。empty-array / alias 正規化時の error text は optionalTools 固有の reason で wrap する（validation detail として design doc で確定）。
  * `crates/cheaper-tools/src/registry.rs`: `ToolRuntimeOptions` に縮退許可集合（例: `degradable: Vec<ToolName>`）を追加。`create_tool_runtime` の `Process` / `Command` gate 分岐で「gate 不成立 && `degradable` 含有 → `names` から除外して登録続行」を実装し、縮退した tool 名を `ToolRuntime` の accessor（例: `degraded_names()`）で報告する。writeScope × `command` reject は縮退対象外として hard のまま残す。managed-subprocess tracking（`is_process` flag）は実登録集合に対してのみ効くため変更不要。
  * `crates/cheaper-tools/src/invocation.rs`: `WorkerScopeOptions` → `ToolRuntimeOptions` 再構成経路に新 field を pass-through する。policy re-resolve parity check（process / command）は変更しない。
  * `crates/cheaper-runtime/src/run/mod.rs`（`create_tool_scope` / `create_session`）: worker 定義の `optional_tools` を scope option へ流し、session の `tool_names` を実登録集合（`scope.definitions()` の name）から組み立て、縮退名を `build_system_prompt` へ渡す。`cheaper.run.tool_scope` span に縮退名 field を載せる。
  * `crates/cheaper-rig-adapter/src/agent.rs`（`build_system_prompt`）: 第 4 引数 `unavailable_tools: &[String]` を追加し、非空時に "declared but unavailable under current policy" 行と `status: blocked` 指示を contract 部に emit する。`WorkerSessionSpec` 経由の既存行・文言は変えない（TS parity 維持）。
  * `crates/cheaper-cli/resources/workers/tool-operator.md` + `resources/workers/schemas/tool-operator.{input,output}.schema.json`: 新規 built-in worker（`tier: low`、`outputPerInput: 0.3`、`optionalTools: [process, command]`、worker prompt に縮退時挙動と `blocked` 返却指針を記述）。
  * `crates/cheaper-cli/src/args.rs`（`alias_worker` に `"tool-operate" => "tool-operator"`）、`crates/cheaper-cli/src/cli.rs`（HELP）、`crates/cheaper-cli/tests/args.rs`。
  * `crates/cheaper-cli/tests/run_dev_seam.rs`（縮退 run・grant 環境 run・capability 拒否の fixture test）、`crates/cheaper-cli/tests/regression/cases/` + `expected/`（縮退 run case）。
  * `crates/cheaper-cli/src/workflow_run.rs`: `WorkerScopeOptions` 構築経路へ同 field（`degradable` / `optional`）を compile-level で追加する。`kind: tool` job は worker frontmatter を持たず opt-in できないため空集合を渡すに留め、縮退が効くのは workflow `kind: worker` job が opt-in worker を実行する経路のみ。
  * `packages/apm/`（`.apm/` primitives）: `cheaper-tool-operate` skill を worker 別 skill parity で追加。
  * `README.md` built-in workers 表、`docs/guides/workers.md`、`docs/guides/usage.md`、`docs/guides/command.md`: tools 集合・alias・縮退挙動・`tools.allow` と content-gate の棲み分けを文書化。
* Patterns to follow: `require_strict_keys` による fail-fast field 検証、`normalize_tool_names` の canonical 名解決、registry.rs:1066-1085 の gate 形状、`build_system_prompt` の TS-parity comment 規約、`command_user_policy` / `FIXTURE_COMMAND_CALL` の dev-seam fixture パターン、`is_directly_comparable_worker` が宣言 tools を見る保守的判定。
* Patterns to avoid: `check_capabilities` への optional 免除（`tools.allow` 拒否は hard のまま）、固定集合の全 worker 適用（C2 — 非 opt-in worker の意味論を変える）、builtin policy への非空 `tasks` / `grants` 同梱（Option D）、gated 集合の registry 側二重定義（vocabulary.rs に集約）、`optionalTools` への非 gated / 未宣言名の silent 受理、縮退 tool の envelope `meta` への新 field 追加（shape を変えない）。
* Dependencies or configuration: 新規依存なし。`optionalTools` は frontmatter の新規 strict key であり、他の `cheaper` keys・policy schema・usage.jsonl record shape には変更を加えない。

## Verification

* [ ] builtin-default policy（`process.tasks` / `command.grants` が空）で opt-in worker が `POLICY_VIOLATION` 無しに scope 構築を通過し、`process` / `command` を含まない実登録集合で run 完走する dev-seam fixture test が通る。
* [ ] 縮退 run の system prompt が「実登録 tool 一覧」と「宣言済みだが利用不可」の注記を含むことが cheaper-runtime test 層（`FakeSessionFactory` handle）で assert され、fake stream の `status: blocked` 出力が output schema 適合で envelope `result.status == "blocked"` になる CLI-level test が通る。
* [ ] 非 opt-in `tools: [command]` worker の `POLICY_VIOLATION`（exit 4）を assert する既存 test が変更無しで green である。
* [ ] `tools.allow` から宣言 tool が外れた policy で opt-in worker が capability check で `POLICY_VIOLATION`（`missing` details）になる test が通る（optionalTools は allow 拒否を免除しない）。
* [ ] grants 定義環境で opt-in worker の `command` が登録され、`command_user_policy` パターンで実 spawn する test が通る。
* [ ] `optionalTools` の validate 規則（`tools` 未宣言名・非 gated 名・重複・vocabulary 外・`optionalTools ⊇ tools` の reject）と未指定 worker の既存挙動不変を assert する unit test が通る。
* [ ] 縮退 run の regression corpus case（`tests/regression/cases/` + `expected/`）が追加され harness で再現される。
* [ ] `command` × writeScope 宣言 worker は `command` が登録される限り従来通り hard reject される test が通る。
* [ ] workflow `kind: worker` job が opt-in worker を実行する経路で縮退が同一挙動となる test が通る（`kind: tool` job は worker frontmatter を持たず opt-in 対象外）。
* [ ] README・`docs/guides/workers.md`・`docs/guides/command.md` と `cheaper-tool-operate` skill が更新されている。
* [ ] `mise run check` が pass する。

## Pros and Cons

| Option | Pros | Cons |
| --- | --- | --- |
| A: 宣言維持 + fail-closed（POLICY_VIOLATION を blocked と解釈） | 実装変更不要。gate が既存の安全契約をそのまま適用し、信頼境界の緩和が発生しない | builtin 環境（`tasks` / `grants` 共に `{}`）では全 run が model 未到達で失敗し、default install で worker が事実上 dead。「blocked」が output schema の `status` として表現不能で spec の status 意味論・縮退実行要件を満たさない。run-level error を blocked の意味に読み替える解釈は利用側に prompt 無しの判断を強いる |
| B: `process` / `command` を宣言しない | gate に触れず常に起動可能。core / runtime 変更不要 | issue の明示的 tools 集合と command 使用要件を満たせず、grant 環境でも `command` を永久に使えない。worker の存在意義（操作 actor）が大きく痩せ、grant を整備した user にも機能を提供できない |
| C1: `cheaper.optionalTools` で per-tool opt-in（採用） | issue 要件を model 到達後の挙動として忠実に満たす。opt-in かつ列挙単位で緩和が閉じる。どの tool が absent になり得るか frontmatter から自明。将来の gated tool 追加も worker 宣言で対応可能 | `cheaper` strict keys の拡張 = core 変更。validate 規則・prompt 機構・報告面の実装が必要。「allow 済みだが未登録」の中間状態は prompt / span にしか残らない |
| C2: 固定集合 `{process, command}` を全 worker で soft-gated | worker 定義の変更・新 key が不要 | 非 opt-in な全 worker の意味論を静かに変える — `tools: [command]` だけの worker が grants 不在で tool 0 個の run へ縮退し、fail すべき構成誤りを握り潰す。既存の fail-closed test 意味論と衝突し blast radius が最大 |
| C3: boolean opt-in flag で固定集合を縮退 | C1 と同じ効果を 1 flag で得られ、validate が simplest | per-tool 粒度を表現できず（「`command` 縮退可・`process` 必須」が書けない）、どの tool が縮退対象か frontmatter から読めず固定集合の知識を要求する。C1 と core 変更量は同等で表現力だけが劣る |
| D: builtin policy に非空 `tasks` / `grants` を同梱 | worker が全 install で full tool 集合を得る | 全環境に常時認可の実行面を持ち込む security regression。grant は trusted user の判断で宣言する信頼境界の前提（ADR-034EMgaatFhsXjR97kH14c）を根本から破るため採用不能（完全性のため列記） |

## More Information

* source: GitHub issue #102（`tool-operator` worker の責務・tools 集合・「無許可時は縮退 or blocked」要件・acceptance の正本）。
* derives-from: `../discovery/tool-operator-worker-built-in.md`（DISC-034ESpIy7zAOucpwBxmjfd。fail-closed gate の touch point 調査と Option A–D 比較表の正本）。
* related: `../specs/tool-operator-built-in-worker.md`（製品要件の正本 — status 意味論・AC4 の「既定挙動は ADR 決定に従う」で本決定を参照）、`../specs/policy-gated-command-tool.md`（`command` の grant / 信頼境界契約）、`../adr/command-grant-trust-boundary-and-managed-process-reuse.md`（`command` の trusted-layer grant 境界と fail-closed registration の正本 — 本 ADR の緩和が触れない層を規定する）。
