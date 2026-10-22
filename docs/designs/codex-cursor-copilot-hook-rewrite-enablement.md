---
id: "DESIGN-034KsaxkFcfUHzvbR5hEB1"
type: "design"
status: "approved"
title: "Codex/Cursor/Copilot hook rewrite enablement"
created: "2026-10-23"
updated: "2026-10-22"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-core/src/policy/schema.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/hooks.json"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_load.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_schema.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/policy_restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-runtime/src/dev/fixture.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/run_dev_seam.rs"
      - type: "file"
        path: "crates/cheaper-runtime/tests/ollama_e2e.rs"
      - type: "file"
        path: "docs/guides/configuration.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  implemented-by: []
  depends-on:
    - "./pretooluse-route-resolver-and-delegated-workflow-execution.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-hook-request-evaluation-and-hook-config-registration-lifecycle.md"
    - "./hook-eval-dedup-verdict-cache-and-leader-election.md"
    - "./cross-agent-native-routing-adapter-architecture.md"
    - "./invocation-scoped-command-grants.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/devin-pretooluse-rewrite-runtime-verification.md"
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
    - "../../crates/cheaper-cli/src/hook/targets.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/hook/mod.rs"
    - "../../crates/cheaper-cli/src/route/supply.rs"
    - "../../crates/cheaper-core/src/policy/schema.rs"
    - "../../crates/cheaper-core/src/policy/load.rs"
    - "../../crates/cheaper-cli/resources/policy.yaml"
    - "../../packages/apm-hook-codex/.apm/hooks/hooks.json"
    - "../../packages/apm-hook-cursor/.apm/hooks/hooks.json"
    - "../../packages/apm-hook-copilot/.apm/hooks/hooks.json"
    - "https://developers.openai.com/codex/hooks"
    - "https://github.com/openai/codex/pull/20527"
    - "https://cursor.com/docs/hooks"
    - "https://docs.github.com/en/copilot/reference/hooks-configuration"
  defers: []
  deferred-by: []
---

# Codex/Cursor/Copilot hook rewrite enablement

## Context

`Verdict::Rewrite{command, fallback}`(SPEC-034J7SxWvGL3oSbeBpo8a3
R36、DESIGN-034JbqoCWm85pKC033fgfd)は claude(`pretooluse`)と
devin のみが `hookSpecificOutput.updatedInput` を emit し、
codex/cursor/copilot は `fallback` verdict を再帰 render する
構造で defer された(approval-bypass 検証が必要だったため —
ADR-034J8eZHNPqARi25HLFOL6 D3)。issue #150 はこの 3 target の
有効化を要求する。

本 session の codex-cli 0.162.1 実機 probe
(DISC-034KrLM3o2H1kMylQFlg2U)で **codex の `updatedInput` は
`permissionDecision:"allow"` 併記時にのみ適用される**ことが確定
(単独・`"ask"` 併記は codex が無視して原 command をそのまま実行
パスへ渡す — silent no-op)。この emit は親 agent の approval
prompt を素通しするため、trusted policy 層の per-target opt-in
flag `route.rewrite.approvalBypass` を新設し、flag 含有時のみ
bypassing emit を行う(ADR-034KrYGRHZuvOIx1d0S0nE D1/D2)。

cursor/copilot は emit contract(field 名・`permission`/`permissionDecision`
併記の approval 意味)が未検証のため、issue の文言規則「実機検証で
確認できない挙動は仕様上の保証として扱わない」に従い **emit は
発行しない** — flag の有無にかかわらず fallback render を維持する
(SPEC-034KrX7RnDs0gLVcMPf7vy R10/R12、ADR D3)。cursor は
`preToolUse` event 登録と payload normalize の受け口のみ先行して
整え、emit shape の確定は実機検証 task へ gate する。

## Scope

### In Scope

- `policy.yaml` `route` section 新設 —
  `route.rewrite.approvalBypass`(bypassing emit を許可する hook
  target 名の配列、domain `codex`/`cursor`/`copilot`)。complete /
  partial schema へ optional property として追加し、project
  partial schema には property 自体を置かない(`command` section
  除外と同型の fail-closed — untrusted checkout が approval bypass
  を自己宣言できない構造保証、R3)。builtin `resources/policy.yaml`
  に `route: {rewrite: {approvalBypass: []}}` を追記し既定値を
  self-documenting にする。
- `RuntimePolicy` への `route` normalized 表現
  (`RoutePolicy{rewrite: RewritePolicy{approval_bypass}}`)と
  `normalize_policy` の passthrough 解決。
- flag の render 層供給 — `evaluate_with_routes` が `(Verdict,
  Option<RouteSupply>)` を返し、`handle_hook` が codex arm で
  flag を読み `codex_render` へ渡す。`Verdict` enum・
  `RouteEvalInput`/`EvaluateOptions`・`evaluate_route_arm`・dedup
  規約は不変(R14)。
- codex の bypassing emit — flag に `"codex"` 含有時のみ
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"allow","updatedInput":{"command":…}}}`
  + exit 0(R6/D2)。
- cursor の `preToolUse` event 登録
  (`packages/apm-hook-cursor/.apm/hooks/hooks.json`)と
  `cursor_normalize` の `tool_input` nested payload 対応。旧
  `beforeReadFile`/`beforeShellExecution` 登録は併存維持
  (D4 — 撤去可否は実機検証依存)。
- user-facing doc(`docs/guides/configuration.md` の `policy.yaml`
  section に `route` 記法・trust 境界・per-target 検証状況を追記)。
- contract/schema/integration test(§Verification Notes)。

### Out of Scope

- cursor/copilot の Rewrite emit 本体 — emit contract が実機検証で
  確定するまで発行しない(R10/R12)。flag domain に両 target を
  含めるのは、検証完了後に emit arm を flag gated で解放する際の
  opt-in を先に定義するため。
- windsurf — rewrite 非対応のまま(flag 対象外、fallback render
  維持)。
- claude(`pretooluse`)・devin — 現行 emit 不変(approval 温存
  emit を既に持つ、flag 不要)。
- codex の file-read interception — upstream gap(openai/codex
  #18491/#20204)で PreToolUse を emit しない。rewrite は Shell
  request のみに効く現行構造(R16)。
- cursor `preToolUse` entry の `matcher` field・旧 event 撤去 —
  payload/tool 名 domain と併せて実機検証で確定。
- codex execpolicy の通過可否 — hook `allow` は approval 面のみを
  bypass し codex 自身の command policy は迂回しない(実機確認済み
  `blocked by policy`)。
- env var による flag override(policy.yaml のみ、YAGNI)。

## Components and Boundaries

- **`crates/cheaper-core/src/policy/schema.rs`** — `route` section
  の schema・normalized 型の単一 owner。`route_properties()` を
  追加し complete/partial schema の properties へ `"route":
  object_schema(&[], route_properties())` を載せる(required には
  入れない — 未指定は default `[]`)。project partial schema には
  `"route"` property を置かず、`additionalProperties: false` が
  `route` key を持つ project policy を `InvalidRuntimePolicy` で
  拒否する(個別 field 列挙ではなく section 丸ごと除外 —
  将来 `route` 配下に field が増えても自動的に project から閉じる
  `command` と同型の規約)。
- **`RoutePolicy` / `RewritePolicy`** — `RuntimePolicy.route` の
  normalized 表現。yaml 表現と 1:1 の構造を保つ(表現ドリフト
  防止):
  `RoutePolicy{rewrite: RewritePolicy{approval_bypass: Vec<String>}}`、
  serialize は `approvalBypass`(camelCase)。`approvalBypass` の
  schema type は `["array","null"]` とする(`execution.maxTurns`
  の `{"type": ["integer","null"]}` 先例 schema.rs:196 に倣う
  nullable 宣言) — 明示 `null` ≡ 未指定 ≡ `[]` を schema 層で
  受理し、R2 の default 等価を loader の暗黙解釈ではなく schema
  で保証する。
  `RewritePolicy::approval_bypass_enabled(&self, target: &str) -> bool`
  が flag 判定の単一 owner — render 層は policy path を直接
  たどらない。
- **`RuntimePolicy` 派生 diff** — `RuntimePolicy` は `Serialize`
  を持つため `route` field は serialize 出力へ載り、
  `expected.policy` を保持する golden fixture
  (`crates/cheaper-core/tests/golden/fixtures/policy/{load,restrict,schema}.json`
  と対応 harness `policy_{load,schema,restrict}.rs`)の再生成が
  必要。また `RuntimePolicy{…}` literal 構築 site は `route`
  field 追記が必要:
  `cheaper-runtime/src/dev/fixture.rs:226,758`、
  `cheaper-core/src/policy/restrict.rs:112`、
  `cheaper-core/src/policy/capability.rs:86`、
  `cheaper-runtime/tests/run_dev_seam.rs:189`、
  `cheaper-runtime/tests/ollama_e2e.rs:219`。
- **`crates/cheaper-cli/src/route/supply.rs`** — 変更不要。
  `RouteSupply.policy` が effective `RuntimePolicy` を保持済みで、
  flag は `s.policy.route.rewrite.approval_bypass_enabled(target)`
  で到達できる。`RouteEvalInput`/`eval_input()` も不変(flag は
  eval ではなく render に効く)。
- **`crates/cheaper-cli/src/hook/targets.rs`** — render 層の差分
  境界。`evaluate_with_routes` の戻り値拡張、`codex_render` の
  flag 引数と bypassing emit 分岐、`hook_specific_output_rewrite`
  の `permissionDecision:"allow"` 併記 option、`cursor_normalize`
  の nested `tool_input` 対応、`handle_hook` の flag 供給。
  加えて codex arm の `cheaper.hook` 完了 debug event
  (targets.rs:778-783 `hook response`)へ `bypass: bool` field
  を追加し、approval-bypass emit の発生を log で観測可能にする
  (bypassing emit は silent exit 0 で stderr 痕跡を持たないため
  debug event が唯一の観測面)。
- **`packages/apm-hook-cursor/.apm/hooks/hooks.json`** — `preToolUse`
  entry 追加(登録面の変更のみ、emit 契約は別途検証)。
- **`crates/cheaper-cli/src/hook/mod.rs`** — 変更不要。
  `KNOWN_EVENTS` は `pre-tool-use` を既に含む。
- **`packages/apm-hook-codex`・`apm-hook-copilot` の hooks.json** —
  変更不要(codex `PreToolUse` matcher は `Bash`/`Powershell` を
  既に含む、copilot `preToolUse` matcher `view|bash|powershell|shell`
  は shell を既に含む — R8/R12)。

## Data and Control Flow

### flag の load・供給経路

```text
stdin payload → cursor_normalize / pretooluse_normalize / copilot_handle
  → RoutingRequest
  → evaluate_with_routes(request, deps)
      ├─ route::supply::load_or_none(cwd, env, builtin_dir)   // 1 firing 1 回
      │    └─ load_policy → builtin(complete) → user(partial)
      │         → project(project-partial: route key 拒否)
      │         → complete re-validate → normalize_policy
      │              └─ RuntimePolicy.route.rewrite.approval_bypass
      ├─ opts.route = supply.eval_input()                     // eval 供給(不変)
      └─ eval_cache::evaluate_dedup → Verdict                 // Rewrite 生成条件不変
  → (Verdict, Option<RouteSupply>) を handle_hook へ返す
  → handle_hook: codex arm のみ
      bypass = supply.policy.route.rewrite.approval_bypass_enabled("codex")
      → codex_render(verdict, bypass)
```

- load 失敗(`load_or_none` → `None`)は flag 未供給 = bypass 無効
  (conservative default、R15)。flag 読み取り失敗のみを理由に新しい
  error 経路は増やさない。
- project 層 policy.yaml が `route` key を持つと project partial
  schema 違反で `load_policy` 全体が失敗 → `load_or_none` が `None`
  → route arm 不活性 + bypass 無効。untrusted checkout は flag を
  「消す」ことしかできず「有効化」は構造的に不能(R3 — `command`
  section 除外と同型)。
- `Verdict::Rewrite` は `RoutingRequest::Shell` の route arm のみが
  生成する(`routing_policy.rs:1451 evaluate_route_arm`)。
  `verdict_name` の `"rewrite"` 分類・dedup 非 persist 規約は不変。

### codex render(R6/R7/D2)

```rust
/// `hookSpecificOutput` rewrite emit — `approval_bypass` 時のみ
/// `permissionDecision:"allow"` を hookEventName と updatedInput の
/// 間に挿入する(field 順序は probe 観測順、byte 互換規約
/// `stringify_ordered` に従う)。
fn hook_specific_output_rewrite(command: &str, approval_bypass: bool) -> HookResponse
//   inner: hookEventName="PreToolUse"
//          if approval_bypass { permissionDecision="allow" }
//          updatedInput={command}

fn codex_render(verdict: Verdict, approval_bypass: bool) -> HookResponse {
    match verdict {
        Verdict::Deny { reason } => hook_specific_output_deny(&reason),
        Verdict::Allow => silent_allow(),
        Verdict::Rewrite { command, .. } if approval_bypass =>
            hook_specific_output_rewrite(&command, true),
        Verdict::Rewrite { fallback, .. } => codex_render(*fallback, approval_bypass),
    }
}
```

- `pretooluse_render`/`devin_render` は
  `hook_specific_output_rewrite(&command, false)` — emit byte は
  現行と完全一致(共有 emit 点を 1 箇所に保ち、claude/devin の
  wire 表現を変えない)。
- `updatedInput` 単独・`permissionDecision:"ask"` 併記の emit は
  **いかなる flag 状態でも発行しない** — codex が両者を無視して
  原 command を走らせる silent no-op になることが実機確認済み
  (R7/D2)。

### cursor normalize + 登録移行(R9-R11/D3/D4)

- `hooks.json` に `preToolUse` flat entry を追加し
  `cheaper hook pre-tool-use --target cursor` を呼ぶ(sh/powershell
  両 entry、失敗時は `{"permission":"allow"}` echo — 現行
  `beforeReadFile`/`beforeShellExecution` entry と同型)。
  `matcher` field は payload/tool 名 domain が未検証のため付けず
  全 tool firing を受ける(normalize 失敗 → schema-valid allow の
  fail-open で安全)。旧 2 event は併存維持 — 同一 firing の二重
  評価は allow/deny verdict で dedup が吸収する(rewrite verdict
  は非 persist で再 eval — §Risks の dedup nuance)。
- `cursor_normalize` は top-level `file_path`/`command`
  (beforeXxx payload)に加え、nested `tool_input.file_path` /
  `tool_input.command` も field 駆動で読む — `tool_name` の
  domain(`"Shell"`/`"Read"` 等)は未検証のため name matching
  ではなく field の有無で判別し、両方の payload shape hypothesis
  を吸収する。source_tool は `cursor:preToolUse`。なお field
  駆動判別は over-match し得る — `tool_name` domain が確定する
  まで、read 以外の tool が `file_path` input を持つ場合にも
  FileRead として評価され(巨大 target で deny になり得る)、
  matcher でも tool 種別を絞れない誤判定面が残る(§Risks)。
- deny/allow の render は現行 `cursor_respond` の flat
  `{"permission":…}` schema を再利用(preToolUse 応答 schema も
  `permission` 系 — docs 報告、実機要確認)。
- Rewrite arm は `cursor_render(*fallback)` のまま不変 —
  `permission`/`updated_input`(or `updatedInput`)併記の emit
  contract が確定するまで flag の有無にかかわらず fallback
  (R10/AC)。

### copilot(R12/R13/D3)

- 登録・normalize・render のいずれも変更しない —
  `modifiedArgs`/`updatedInput` の実効 field と `permissionDecision`
  併記値(`"ask"` が approval を温存するか)は実機検証で確定後に
  flag gated emit を追加する。非 0 exit = deny 規約は不変。
- `copilot_handle` の foreign-payload dual-schema allow 経路
  (cursor 系 `command`/`file_path` payload への `{permission,
  permissionDecision:"allow"}` emit)は現行のまま — cursor
  preToolUse payload が copilot hook へ cross-fire しても壊れない。

## Non-Functional Constraints

- **byte 互換 JSON**: emit は `stringify_ordered` /
  `cheaper_core::json::stringify` + `"\n"` の既存規約。field 挿入
  順は probe 観測順(`hookEventName` → `permissionDecision` →
  `updatedInput`)。
- **fail-open**: normalize 失敗・malformed stdin・panic は
  target 適合の allow(codex/copilot silent、cursor schema-valid
  JSON)。route supply load 失敗は `None` = bypass 無効 + route
  arm 不活性(R15)。
- **fail-loud policy**: `approvalBypass` の未知値・重複・非配列・
  非文字列 item、非 object の `rewrite`/`route` は
  `InvalidRuntimePolicy`(R1 — 「invalid policy is never silently
  ignored」)。`null`・未指定・空配列は default `[]` 等価(R2)。
- **fail-closed trust 境界**: project 層から `route` section を
  authoring 不能(`policy_project_partial_schema` に property を
  置かない)。builtin/user/invocation 層のみ trusted authoring。
  なお invocation 層は `{"version","command.grants"}` 限定の合成
  layer で、`route` を含むと既存の grants-only key 検査で拒否
  される(追加変更不要)。
- **merge 規則**: `route` は既存 layer merge(Replace)に乗る —
  `approvalBypass` 配列は後勝ち置換(object deep-merge、array は
  wholesale replace)。merge コードの変更は不要。
- **dedup/eval 不変**: `Verdict` enum・`evaluate_route_arm`・
  `eval_cache` の Rewrite 非 persist 規約を変更しない — flag は
  render 出力のみに効く(R14/R16/R17)。
- **hook trust**: codex exec 経路は `--dangerously-bypass-hook-trust`
  等の trust 確立が前提 — consumer 向け登録手順は対象外だが、
  未 review hook は実行されない旨を doc に触れる。

## Risks and Trade-offs

- **誤 emit → silent no-op**: `updatedInput` 単独/`"ask"` 併記を
  codex が無視すると「rewrite したつもりで原 command が走る」。
  emit shape を probe 確定形に固定し、contract test で field 集合を
  byte レベルに近い精度で固定する(負 shape test を含む)。
- **cursor/copilot の未検証 emit**: flag があっても emit しない
  構造のため、user が `approvalBypass: [cursor]` を書いても現状
  no-op — doc で「flag は許可であり有効化は emit contract 確定後」
  と明記する。誤解 cost は低い(fallback は従来挙動)。
- **cursor `preToolUse` 応答 schema は未検証のまま出荷**:
  `preToolUse` 登録自体は本 change に含まれ、応答の flat
  `{"permission":…}` shape は実機未検証(docs 報告のみ) —
  issue の文言規則上これは spec 保証ではない。Cursor が shape
  を拒否・解釈不能にした場合、deny/allow の通常応答も含めて
  tool call を block し得る(matcher 無しで全 tool firing を
  受ける登録のため影響面は rewrite に留まらない)。rewrite
  emit 自体は引き続き検証 gated — 本 risk は登録・応答面固有
  のもので emit gate とは別軸。
- **cursor 二重 firing(dedup nuance)**: `preToolUse` と
  beforeXxx の併存で同一 tool call が 2 回 hook 評価される。
  allow/deny verdict は dedup cache が absorb して verdict は
  同一だが、`Verdict::Rewrite` は `is_shareable`
  (eval_cache.rs:674)で persist/serve 対象外のため、rewrite が
  成立する command(beforeShellExecution + preToolUse の両方が
  発火する shell tool call)は 2 回目の firing でも再 eval
  される — verdict は同一で cost は extra eval 1 回。登録面の
  複雑さは旧 event 撤去判断(検証依存)で解消予定。
- **user 層 flag の trust anchor**: user 層 policy.yaml を書ける
  主体は既に `command.grants`(任意 executable の実行許可)を
  制御している — `approvalBypass` はこの境界を超える privilege
  を追加しない。`CHEAPER_CONFIG_DIR` redirect で user 層を差し
  替える経路も同一 trust class であり、flag 固有の脅威面は
  増えない。
- **execpolicy は迂回しない**: hook `allow` は approval prompt の
  みを素通しし codex execpolicy 評価は残る — 「policy opt-in で
  全検査を迂回する」誤解を doc で防ぐ(実機で `blocked by
  policy` を確認済み)。
- **schema diff の面積**: 3 schema mode + `RuntimePolicy` +
  `normalize_policy` の追加 — ただし既存の section 追加 pattern
  に倣う定型的変更で、project partial への非追加が保証する
  fail-closed 性を test で固定する。

## Deferred Design Concerns

- cursor emit contract(`updated_input` vs `updatedInput`・
  `permission:"allow"` 併記要否・approval 素通し可否)・
  `preToolUse` payload shape(`tool_name`/`tool_input` か
  cursor 固有か)・`matcher` field・旧 event 撤去可否 |
  cursor binary が環境に不在 | cursor CLI が利用可能な環境での
  実機検証 task | 未検証のまま emit すると silent no-op・
  approval 誤 bypass・schema 不適合の risk。
- copilot emit field(`modifiedArgs`/`updatedInput`)・
  `permissionDecision:"ask"` の approval 温存可否 | 同上 |
  copilot CLI 環境での検証 | 温存形が実在するなら flag 不要の
  無条件 emit 化を別 issue で検討可能(R5)。
- codex rewritten command の end-to-end 実行(execpolicy 通過) |
  本環境の execpolicy が全 `pwsh -Command` を拒否 | 別環境での
  補完 verification | 構造的適用は確認済みで blocker ではない。
- codex interactive TUI の `permission_mode`・approval prompt 挙動 |
  exec は `bypassPermissions` 報告 | TUI session での観測 |
  `allow` 必須の構造的事実は確定済み。

## Verification Notes

- [ ] schema test(`cheaper-core/src/policy/schema.rs` tests):
      `route` section が complete/partial で受理・project partial で
      `route` key 拒否。`approvalBypass` の未知 target 名・非配列・
      非文字列 item・重複・非 object `rewrite`/`route` が
      `InvalidRuntimePolicy`。`null`/未指定で default `[]`。
- [ ] load test(`cheaper-core/src/policy/load.rs` tests): user
      policy の `route` merge が effective policy に反映、project
      policy の `route` key が load error(untrusted project が
      bypass を有効化できない構造保証)。
- [ ] targets.rs unit test: `codex_render(rewrite, true)` が
      `hookSpecificOutput` 内 `permissionDecision:"allow"` +
      `updatedInput.command` + `hookEventName:"PreToolUse"` を
      emit し exit 0。`codex_render(rewrite, false)` は従来
      `codex_rewrite_renders_fallback_only` の assert を維持
      (signature 変更のみ)。
- [ ] targets.rs unit test: pretooluse/devin の emit が flag 非対称
      で不変(`hook_specific_output_rewrite(…, false)` で現行 byte
      と同一 — `permissionDecision` 不在)。
- [ ] targets.rs unit test: `cursor_normalize` が nested
      `tool_input.command` → Shell・`tool_input.file_path` →
      FileRead を返す(preToolUse payload hypothesis)。
- [ ] integration test(`route_run.rs`、
      `devin_hook_rewrite_emits_nested_updated_input` precedent):
      user policy `route.rewrite.approvalBypass: [codex]` + grant
      で `--target codex` firing が `permissionDecision:"allow"`
      + `updatedInput.command`(`route run --payload` 形)を emit
      し exit 0。flag 未設定では fallback render のまま
      `updatedInput` を emit しない。project `.cheaper/policy.yaml`
      の `route` key で hook が rewrite を発行しない(fail-closed)。
- [ ] `node scripts/check-apm-payload.mjs` — cursor hooks.json
      変更が slim payload check に整合。
- [ ] golden fixture 再生成 + `RuntimePolicy{…}` literal 追記 —
      `RuntimePolicy.route` が serialize 出力へ載るため
      `expected.policy` を持つ
      `tests/golden/fixtures/policy/{load,restrict,schema}.json`
      (harness `policy_{load,schema,restrict}.rs`)を再生成し、
      literal 構築 site(`fixture.rs:226,758`・`restrict.rs:112`・
      `capability.rs:86`・`run_dev_seam.rs:189`・
      `ollama_e2e.rs:219`)へ `route` field を追記する。「既存
      test が無変更で pass」は実行時挙動に限り、fixture・構築
      site は変更を伴う。
- [ ] `cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で fixture
      再生成込みの全 contract が pass(flag 不在時の実行時挙動は
      完全に不変 — R17)。
- [ ] `docs/guides/configuration.md` に `route` section 記法・
      trust 境界(builtin/user のみ authoring 可)・各 target の
      検証状況(codex 実機確定済み、cursor/copilot 検証 gated)を
      追記。
