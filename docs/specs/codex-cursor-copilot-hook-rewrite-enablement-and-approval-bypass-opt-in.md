---
id: "SPEC-034KrX7RnDs0gLVcMPf7vy"
type: "spec"
status: "approved"
title: "Codex/Cursor/Copilot hook rewrite enablement and approval-bypass opt-in"
created: "2026-10-22"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/150"
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
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./bounded-native-hook-evaluation-and-managed-devin-registration.md"
    - "./invocation-scoped-command-grants.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "../adr/cli-owned-devin-hook-registration-lifecycle-with-apm-integration.md"
    - "../adr/copilot-adapter-separate-script-never-nonzero-exit.md"
  refines:
    - "./pretooluse-route-resolver-and-workflow-delegation.md"
  refined-by: []
  derives-from:
    - "../discovery/codex-cursor-copilot-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../discovery/devin-pretooluse-rewrite-runtime-verification.md"
    - "../discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
    - "../discovery/cross-agent-hook-distribution-and-contract-matrix.md"
    - "../discovery/pretooluse-auto-routing-to-tasks-and-workflows.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-cli/src/hook/mod.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
    - "packages/apm-hook-codex/.apm/hooks/hooks.json"
    - "packages/apm-hook-cursor/.apm/hooks/hooks.json"
    - "packages/apm-hook-copilot/.apm/hooks/hooks.json"
    - "https://developers.openai.com/codex/hooks"
    - "https://github.com/openai/codex/pull/20527"
    - "https://cursor.com/docs/hooks"
    - "https://docs.github.com/en/copilot/reference/hooks-configuration"
  defers: []
  deferred-by: []
---

# Codex/Cursor/Copilot hook rewrite enablement and approval-bypass opt-in

## What Should Be Built

`Verdict::Rewrite{command, fallback}`(SPEC-034J7SxWvGL3oSbeBpo8a3 R36)を
codex / cursor / copilot の 3 hook target で有効化する。現行は
`targets.rs` の各 render が `fallback` verdict を再帰 render しており、
rewrite が構造的に target へ届かない(codex :572・cursor :675・
copilot :435)。

本 spec が formalize する contract:

- **codex rewrite emit** — `hookSpecificOutput` 内へ
  `permissionDecision:"allow"` + `updatedInput:{command}` を併記する
  emit。実機検証済み(codex-cli 0.162.1、
  DISC-034KrLM3o2H1kMylQFlg2U): codex は `updatedInput` を
  `permissionDecision:"allow"` 併記時にのみ適用し、単独・`"ask"`
  併記では無視する。この emit は agent の approval prompt を
  素通しするため **opt-in 必須**。
- **cursor rewrite emit** — `preToolUse` event への登録移行 +
  `updated_input`/`updatedInput` emit。payload shape・`permission`
  併記要否・approval 意味は **実機検証必須**(現環境に cursor
  binary 不在)。
- **copilot rewrite emit** — 現行 `preToolUse` 登録のまま、
  `modifiedArgs`/`updatedInput` の実効 field を実機検証で確定して
  から emit。`permissionDecision:"ask"` が approval を温存するか
  未検証。
- **opt-in flag** — `policy.yaml` の `route` section に
  `route.rewrite.approvalBypass`(target 名の配列)を新設し、
  bypassing emit を許可する target を列挙する。**project 層
  policy からは authoring 不能**(untrusted checkout が approval
  bypass を自己宣言できないよう、project partial schema から
  `route` section を除外 — `command` section 除外と同型の
  構造保証)。default は `[]`(現行 fallback 挙動)。
- **issue の文言規則**: 実機検証で確認できない挙動は仕様上の保証
  として扱わない — cursor / copilot の emit contract は実機検証
  task の完了を有効化条件とする。

## Why Now

- parent spec(SPEC-034J7SxWvGL3oSbeBpo8a3)が codex/cursor/copilot
  の rewrite を「approval-bypass 検証が必要」として issue #150 へ
  明示 defer した(R38・Deferred Design Concerns)。claude/devin
  では rewrite が実機確認済みで、3 target 未対応が routing
  coverage の残ギャップになっている。
- 本 session で codex の実機検証が完了し、「`allow` 必須」が
  upstream 報告どおり確定した。残る不確実性は cursor/copilot の
  emit contract と approval 意味であり、実機検証前提の staged
  有効化境界を contract として先に固定する必要がある。
- approval を素通しする emit を opt-in 無しで出荷すると、user が
  意図しない承認省略が発生する — flag の配置・粒度・trust 境界を
  先に spec 化しないと実装時に trust 境界を侵す risk がある。

## Users and Value

- **Codex / Cursor / Copilot 上の親 coding agent**: routed
  command が `cheaper route run` へ rewrite され、巨大 tool output
  が frontier context へ流入する前に delegation 経路へ倒れる。
  現行 fallback のままでは route match しても原 command がそのまま
  実行される。
- **cheaper user**: `route.rewrite.approvalBypass` で bypassing
  emit を target 単位で明示 opt-in できる — 検証済みで approval
  を温存する emit のみ無条件、approval を素通しする emit は user
  の明示同意が必要という security posture を維持する。
- **cheaper maintainer**: emit 差分が render 層に閉じ(verdict・
  eval・dedup は不変)、追加は per-target の小さい局所変更に
  なる。

## Scope

### In Scope

- `policy.yaml` への `route` section 追加と
  `route.rewrite.approvalBypass`(target 名 list)の schema・
  3 層 load・normalized policy への反映。値の domain は
  rewrite-capable target 名(`codex`/`cursor`/`copilot` — 将来
  bypassing emit を持つ target が増えた場合に拡張)。未知名・非
  配列・非文字列 item は config error(fail-loud、「invalid
  policy is never silently ignored」規約)。
- project 層 policy(`<workspace>/.cheaper/policy.yaml`)からの
  `route` section 除外 — project partial schema が `route` を
  受理しないことで、untrusted checkout が approval bypass を
  有効化できない構造保証。
- codex target の Rewrite render 分岐 — flag に `codex` が含まれる
  場合のみ `hookSpecificOutput` + `permissionDecision:"allow"` +
  `updatedInput:{command}` を emit(exit 0)。flag 不在時は
  従来どおり fallback verdict を render。
- cursor target の `preToolUse` event 登録(APM
  `packages/apm-hook-cursor/.apm/hooks/hooks.json`)と payload
  normalize の追加 — ただし emit contract は実機検証完了を
  有効化条件とし、検証済み field 名(`updated_input` か
  `updatedInput` か)と `permission` 併記要否を contract test で
  固定する。
- copilot target の Rewrite emit — `modifiedArgs`/`updatedInput`
  の実効 field と `permissionDecision` 併記値を実機検証で確定し、
  flag gating(copilot が list されているか)と併せて固定。
- flag 判定の供給経路 — firing ごとの `RouteSupply` load 済み
  effective policy から render 層へ渡す(flag は render 出力の
  みに影響し、verdict 生成・dedup・fail-open 規約を変えない)。
- per-target contract test(render shape)と schema/load test
  (project 層拒否・未知 target 名拒否・default 空)。

### Out of Scope

- windsurf — rewrite 非対応のまま(fallback render を維持、
  flag 対象外)。
- claude(`pretooluse`)・devin — 現行 emit 不変(approval 温存
  emit を既に持ち、flag 不要)。
- codex の file-read interception — upstream gap(openai/codex
  #18491/#20204)により PreToolUse を emit しない。rewrite は
  Shell request のみに効くため本件の非対象。
- cursor `beforeReadFile`/`beforeShellExecution` 登録の扱い
  (preToolUse への統合/併存/置換の選択)の最終確定 — event 移行
  は ADR で記録し、旧 event の撤去可否は実機検証の結果に依存する。
- copilot `additionalContext` の利用 — rewrite とは別機構。
- codex execpolicy の通過可否 — hook `allow` は approval 面のみを
  bypass し codex 自身の command policy は迂回しない(実機確認済み
  — `blocked by policy`)。rewritten command が codex policy を
  通るかは codex 側の設定問題。
- env var による flag override — config 面の整合性を優先し
  policy.yaml のみ(YAGNI)。

## Requirements

### opt-in flag(`route.rewrite.approvalBypass`)

- R1: `policy.yaml` に `route` section を追加し、
  `route.rewrite.approvalBypass` を hook target 名の string 配列と
  する。値 domain は rewrite emit を持ちうる target 名
  (`codex`/`cursor`/`copilot`)。未知値・重複・非配列・非文字列
  item・非 object の `rewrite`/`route` は config error
  (`InvalidRuntimePolicy`)として fail-loud に表面化する —
  「invalid policy is never silently ignored」規約
  (`crates/cheaper-core/src/policy/load.rs` :101)に従う。
- R2: `approvalBypass` の default は `[]`(未指定・`null`・空
  配列は等価)で、bypassing emit が無効=現行 fallback 挙動。
- R3: **project 層 policy から `route` section を authoring
  できない** — `policy_project_partial_schema`
  (`policy/schema.rs:289`)から `route` を除外し、project
  policy.yaml が `route` key を持つ場合は schema 違反で load
  error とする(`command` section 除外と同型 — untrusted
  checkout が approval bypass を自己宣言する経路を構造的に閉じる)。
  builtin / user 層のみ authoring 可能。
- R4: flag は effective policy の merge 結果から読む — user 層で
  列挙された target のみ bypassing emit が有効。merge 規則は既存
  policy layer(Replace)に従う。
- R5: flag の意味は「当該 target へ affirmative permission
  decision を併記した rewrite emit の送信を user が許可した」—
  approval prompt を素通ししない emit(実機検証で approval 温存が
  確認された shape)は flag 対象外として後続で無条件有効化できる
  が、本 issue で新規に無条件化する emit は無い(cursor/copilot
  の温存可否は未検証)。

### codex rewrite emit

- R6: codex の Rewrite render は、flag に `"codex"` が含まれる
  場合に限り
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"allow","updatedInput":{"command":<rewrite
  command>}}}` + exit 0 を emit する。field 順序・nested shape は
  既存 `hook_specific_output_*` emit 規約(JSON.stringify byte
  互換)に従う。
- R7: flag に codex が無い場合、codex は従来どおり `fallback`
  verdict を render する(deny → `hookSpecificOutput.
  permissionDecision:"deny"`、allow → silent exit 0)。
  `updatedInput` を `permissionDecision` 無し・`"ask"` 併記で
  emit してはならない — codex は両者を無視し原 command を実行
  することが実機確認済み(誤 emit は「rewrite したつもりで原
  command が走る」silent no-op になる)。
- R8: codex の登録 event・matcher・normalize は変更しない —
  現行 `PreToolUse` + matcher `Read|Grep|Bash|Powershell`
  (`packages/apm-hook-codex/.apm/hooks/hooks.json`)で shell
  tool(`Bash`)は既に発火し、実測 payload(`tool_name:"Bash"`・
  `tool_input.command`)は `pretooluse_normalize` と一致済み。

### cursor rewrite emit(実機検証 gated)

- R9: cursor の rewrite 有効化には `preToolUse` event への登録が
  必須 — 現行の `beforeReadFile`/`beforeShellExecution` 登録では
  `updated_input` を返せない。`packages/apm-hook-cursor/.apm/hooks/
  hooks.json` に `preToolUse` entry を追加する(旧 event の併存/
  撤去は ADR 確定事項)。
- R10: cursor の `preToolUse` payload shape・`updated_input`/
  `updatedInput` の実効 field・`permission:"allow"` 併記が
  approval を素通しするかは**実機検証で確定するまで emit しない**
  (issue の文言規則)。検証完了までは flag に cursor があっても
  fallback render を維持するか、検証済み shape への emit を flag
  gated とする — 未検証 shape の emit はいかなる flag 状態でも
  行わない。
- R11: cursor の schema-valid JSON 必須規約(`cursor_respond`
  :660、`fallback_allow` :790)は emit shape 確定後も維持する —
  normalize 失敗・malformed stdin でも schema 適合の allow を返す。

### copilot rewrite emit(実機検証 gated)

- R12: copilot の rewrite emit field(`modifiedArgs` か
  `updatedInput` か)と `permissionDecision` 併記値(`"ask"` が
  approval を温存するか)は**実機検証で確定するまで emit しない**。
  現行 `preToolUse` + matcher `view|bash|powershell|shell` の登録は
  変更不要(shell tool は matcher 済み)。
- R13: copilot はいかなる非 0 exit も deny と解釈する既存規約を
  維持する — rewrite emit を含む全応答は exit 0。

### 共通不変条件

- R14: flag 判定は render 層のみに効く — `Verdict` enum・
  `evaluate_route_arm` の Rewrite 生成条件・`eval_cache` の dedup
  規約(Rewrite は persist/serve しない — routing_policy.rs:128)
  は変更しない。
- R15: route supply / policy の load 失敗は従来どおり fail-open —
  `load_or_none`(`route/supply.rs:127`)が `None` を返す場合は
  flag 未供給=bypass 無効(conservative default)として route arm
  自体も不活性のまま。flag 読み取り失敗のみを理由に新しい error
  経路を増やさない。
- R16: non-Shell request(FileRead/Search)は Rewrite を生成しない
  現行構造を維持 — flag は shell rewrite の render のみに効く。
- R17: hook fail-open 規約を維持 — normalize 失敗・catch-all・
  panic は target 適合の allow へ倒す(codex/copilot silent、
  cursor schema-valid allow)。flag 不在時の実行時挙動は一切
  変わらない — ただし `RuntimePolicy.route` の serialize 追加に
  伴い `expected.policy` を持つ golden fixture
  (`policy/{load,restrict,schema}.json`)の再生成と
  `RuntimePolicy{…}` literal 構築 site への `route` field 追記は
  伴う(「無変更で pass」は実行時挙動に限る)。

## Acceptance Criteria

- [ ] `route.rewrite.approvalBypass` が builtin/user 層 policy で
  load され、値 domain 外(未知 target 名・非配列・非文字列
  item・非 object)が `InvalidRuntimePolicy` で reject されることが
  test で固定される(R1/R2)。
- [ ] project 層 policy.yaml が `route` key を持つと load error と
  なることが test で固定される(R3 — untrusted project が
  approval bypass を有効化できない構造保証)。
- [ ] flag に `"codex"` がある場合、`--target codex` の routed
  shell firing が `hookSpecificOutput` 内に
  `permissionDecision:"allow"` + `updatedInput.command`
  (`route run --payload` 形)を emit し exit 0 となることが test
  で固定される(R6 — `route_run.rs` `devin_hook_rewrite` test の
  precedent に倣う)。
- [ ] flag 未設定・空・codex 非含有の各状態で codex render が
  fallback verdict のみを返し `updatedInput` を emit しないことが
  test で固定される(R7)。
- [ ] `updatedInput` 単独・`permissionDecision:"ask"` 併記の
  codex emit が発行されないことが contract test で固定される
  (R7 — 実機で無視が確認済みの shape を emit しない)。
- [ ] cursor/copilot の Rewrite render は、emit contract が実機
  検証で確定するまで flag の有無にかかわらず fallback verdict を
  render することが test で固定される(R10/R12 — 未検証 shape
  の emit は発行しない)。
- [ ] cursor `preToolUse` 登録と payload normalize・emit field・
  `permission` 併記要否は、実機検証結果を反映した contract test
  で固定される(R9-R11 — 検証 task 完了を有効化条件)。
- [ ] copilot emit field(`modifiedArgs`/`updatedInput`)と
  `permissionDecision` 値は、実機検証結果を反映した contract test
  で固定される(R12/R13)。
- [ ] route supply load 失敗時に bypass が無効化されることが test
  で固定される(R15 — fail-open/conservative default)。
- [ ] flag 不在時の全既存挙動(各 target の normalize・deny・
  allow・fallback render・dedup)が維持される(R14/R16/R17 —
  golden fixture の `expected.policy` 再生成と `RuntimePolicy{…}`
  literal への `route` field 追記を伴う)。
- [ ] user-facing doc(policy.yaml `route` section の記法・
  trust 境界・各 target の検証状況と有効化条件)が更新される。

## Deferred Design Concerns

- cursor/copilot emit contract の実機検証 | 両 CLI が本環境に不在 |
  CLI が利用可能な環境での verification task | 未検証のまま emit
  すると silent no-op・approval 誤 bypass・schema 不適合の risk。
- cursor `preToolUse` payload shape・`beforeReadFile`/
  `beforeShellExecution` 旧 event の撤去可否 | 同上 | 同上 | 併存
  が残ると同一 firing の二重評価経路が残る(dedup が吸収するが
  登録面は複雑化)。
- copilot `permissionDecision:"ask"` + rewrite の approval 温存可否
  | 同上 | 検証次第で flag 不要の無条件 emit 化を別 issue で検討 |
  温存形が実在するのに flag 必須だと coverage を損なう。
- codex execpolicy 環境差 — rewritten command の end-to-end 実行
  | 本環境の execpolicy が全 `pwsh -Command` を拒否 | 別環境での
  補完 verification | rewrite 適用自体は実機確認済みのため
  blocker ではない。
- interactive TUI(非 exec)での codex approval 挙動 | exec mode は
  `permission_mode:"bypassPermissions"` を報告 | TUI session での
  観測 | `allow` 必須の構造的事実は確定済み。

## Sources and Relations

- upstream discovery:
  `docs/discovery/codex-cursor-copilot-rewrite-enablement.md`
  (DISC-034KrLM3o2H1kMylQFlg2U — in-repo 調査 + codex 実機 probe
  の検証済み/未検証分類、opt-in 設計空間の alternatives)。
- parent spec/ADR(defer 元):
  `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (SPEC-034J7SxWvGL3oSbeBpo8a3 R36-R40 — Rewrite verdict・
  claude/devin emit・3 target defer)、
  `docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  (ADR-034J8eZHNPqARi25HLFOL6 D3 — `updatedInput` のみ規約と
  alternative E 却下の経緯)。本 spec は R38 の defer を解消する
  下流 spec として `refines` で接続する。
- 関連検証: `docs/discovery/devin-pretooluse-rewrite-runtime-verification.md`
  (DISC-gHDyRKX2Xbdm58SDpCFLLt — devin の実機 rewrite・approval
  温存先例)、`docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md`
  (per-target contract matrix — cursor `permission: allow|deny|ask`・
  copilot `permissionDecision` deny 規約・codex emit gap)、
  `docs/discovery/cross-agent-hook-distribution-and-contract-matrix.md`
  (APM deploy 形態と per-target functional 状況)。
- trust 境界先例: `docs/specs/invocation-scoped-command-grants.md`・
  `docs/adr/invocation-scoped-command-grants-as-a-synthetic-trusted-policy-layer.md`
  (trusted-layer-only 認可の pattern)、`docs/adr/copilot-adapter-separate-script-never-nonzero-exit.md`
  (copilot 非 0 = deny 規約)。
- code evidence: `crates/cheaper-cli/src/hook/targets.rs`
  (`codex_render` :572、`cursor_render` :675、`copilot_render`
  :435、`hook_specific_output_rewrite` :538、
  `hook_specific_output_deny` :508、`cursor_respond` :660、
  `fallback_allow` :790)、`crates/cheaper-cli/src/hook/
  routing_policy.rs` (`Verdict::Rewrite` :129、`evaluate_route_arm`
  :1451、`build_rewrite_command` :1493)、`crates/cheaper-cli/src/
  hook/mod.rs` (`KNOWN_EVENTS` :31)、`crates/cheaper-cli/src/route/
  supply.rs` (`RouteSupply` :28、`load_or_none` :127)、
  `crates/cheaper-cli/src/route/routes.rs` (route schema・3 層 merge
  :91)、`crates/cheaper-core/src/policy/schema.rs` (schema 3 mode
  :238/:266/:289、`normalize_policy` :450)、`crates/cheaper-core/
  src/policy/load.rs` (層 merge :103)、`packages/apm-hook-codex/
  .apm/hooks/hooks.json`、`packages/apm-hook-cursor/.apm/hooks/
  hooks.json`、`packages/apm-hook-copilot/.apm/hooks/hooks.json`。
- upstream refs: codex hooks doc・upstream PR #20527
  (`updatedInput` + `permissionDecision:"allow"` required — 実機で
  裏付け済み)、openai/codex#18491・#20204(file-read emit gap)、
  cursor hooks doc(<https://cursor.com/docs/hooks>)、copilot hooks
  reference(<https://docs.github.com/en/copilot/reference/hooks-configuration>)。
