---
id: "DISC-034KrLM3o2H1kMylQFlg2U"
type: "discovery"
status: "resolved"
title: "Codex/Cursor/Copilot rewrite 有効化 (issue #150)"
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
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "./pretooluse-auto-routing-to-tasks-and-workflows.md"
    - "./devin-pretooluse-rewrite-runtime-verification.md"
    - "./cross-agent-native-routing-adapter-contract-re-verification.md"
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
    - "../adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "../adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  verifies: []
  verified-by: []
  references:
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
metadata:
  discovery:
    question: "Codex / Cursor / Copilot の各 hook target で `Verdict::Rewrite` を有効化するには何が必要か — per-target emit shape・event 登録変更・approval-bypass opt-in の設計空間を確定する"
    scope: "in-repo code 調査 (targets.rs / APM hooks.json / route supply / policy schema) + Codex CLI 0.162.1 実機 probe (隔離 CODEX_HOME)。Cursor / Copilot CLI は環境不在のため docs + schema のみ"
    outcome: "Codex は実機検証で確定: `updatedInput` は `permissionDecision:\"allow\"` 併記時のみ適用 (単独・\"ask\" 併記は無視)、allow は approval を素通しするが execpolicy は迂回しない。Cursor / Copilot は emit shape が未検証のまま — opt-in flag は trusted policy 層 (project 除外) の per-target list として設計"
---

# Codex/Cursor/Copilot rewrite 有効化 (issue #150)

## Exploration Goal

issue #148 で `Verdict::Rewrite{command, fallback}` が導入された際、
claude (`pretooluse`) と devin のみ `updatedInput` を emit し、
codex / cursor / copilot は `fallback` verdict を render する構造で
defer された (SPEC-034J7SxWvGL3oSbeBpo8a3 R38、
ADR-034J8eZHNPqARi25HLFOL6 D3 — 「approval-bypass 検証が必要」)。
issue #150 はこの 3 target での rewrite 有効化を要求する。本
discovery が解く問い:

1. 各 target で rewrite を agent へ伝えるために必要な emit shape
   と登録 event は何か (現状との差分)。
2. emit が上流 approval を素通しする場合、どこに opt-in を置くか
   (config schema の配置・粒度・trust 境界)。
3. issue の文言規則「実機検証で確認できない挙動は仕様上の保証と
   して扱わない」に従い、何が検証済みで何が未検証かを分類する。

## Environment and Setup

- in-repo 調査: `crates/cheaper-cli/src/hook/targets.rs` (render
  paths)、`crates/cheaper-cli/src/hook/routing_policy.rs`
  (`Verdict::Rewrite` 生成経路 :1438-1486)、
  `crates/cheaper-cli/src/hook/mod.rs` (`KNOWN_EVENTS` :31-37)、
  `packages/apm-hook-{codex,cursor,copilot}/.apm/hooks/hooks.json`
  (登録 event)、`crates/cheaper-core/src/policy/schema.rs` +
  `crates/cheaper-core/src/policy/load.rs` (trust 境界)、
  `crates/cheaper-cli/src/route/supply.rs` + `route/routes.rs`
  (config load 経路)。
- Codex 実機 probe: codex-cli `0.162.1` (mise shim、ChatGPT login
  済み)、隔離 `CODEX_HOME` (`target/codex-hook-probe/home/` —
  auth.json copy + minimal config.toml `approval_policy=on-request` /
  `sandbox_mode=read-only` + 独自 hooks.json)、
  `codex exec --ephemeral --dangerously-bypass-hook-trust
  --skip-git-repo-check --json`。hook は PowerShell script が stdin
  payload を `stdin-<mode>.jsonl` へ記録しつつ応答 JSON を emit。
- Cursor CLI・GitHub Copilot CLI はこの環境に存在しない
  (`which cursor` 不在、`copilot` shim は「GitHub Copilot CLI was
  not found」と応答)。両 target は docs/schema 根拠のみで、
  emit shape は未検証 hypothesis として扱う。

## Key Findings (current code, verified in-repo)

### 登録 event と normalize/render の現状

| target | 登録 event (APM hooks.json) | normalize (targets.rs) | Rewrite render | 差分 |
| --- | --- | --- | --- | --- |
| codex | `PreToolUse` (nested group, matcher `Read|Grep|Bash|Powershell`) | `pretooluse_normalize` — `tool_name`/`tool_input` (`Bash`/`PowerShell` → Shell) | `codex_render` :572 → fallback | render のみ — 登録変更不要 |
| cursor | `beforeReadFile` + `beforeShellExecution` (flat entries) | `cursor_normalize` :630 — top-level `file_path`/`command`/`cwd` | `cursor_render` :675 → fallback | **event 移行必須** (`preToolUse`) + normalize の payload shape 差 |
| copilot | `preToolUse` (flat, matcher `view|bash|powershell|shell`) | `copilot_handle` :358 — `toolName`/`toolArgs` | `copilot_render` :435 → fallback | render のみ — 登録変更不要 (matcher は shell 系を既に含む) |

- rewrite が発行されるのは **`RoutingRequest::Shell` の route arm
  のみ** (`routing_policy.rs:1451 evaluate_route_arm`)。FileRead /
  Search request は Rewrite を生成しない — per-target の実装差分は
  shell tool の応答に限定される。
- `codex_render` / `cursor_render` / `copilot_render` はいずれも
  `Verdict::Rewrite{fallback, ..} => render(*fallback)` で
  rewrite が構造的に届かない (C6 — fallback が read-candidate Deny
  を保持するため deny の素通しは起きない)。
- devin は実機検証済み
  (DISC-gHDyRKX2Xbdm58SDpCFLLt): `hookSpecificOutput.updatedInput
  .command` を `permissionDecision` 無しで emit → native approval
  温存。claude も同じ nested shape を emit
  (`hook_specific_output_rewrite` :538)。

### Codex 実機検証 (session 内で完了 — codex-cli 0.162.1)

隔離 CODEX_HOME で hook 応答の 3 pattern を `codex exec` へ投入:

| hook 応答 | 観測結果 |
| --- | --- |
| `{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow","updatedInput":{"command":"echo CHEAPER_REWRITTEN_ALLOW_OK"}}}` | codex は **rewrite 後の command を実行パスへ渡した** (exec JSONL error: `pwsh -Command 'echo CHEAPER_REWRITTEN_ALLOW_OK'` — 原 command `echo CHEAPER_ORIGINAL_A` ではない) |
| `{"hookSpecificOutput":{...,"updatedInput":{"command":"echo CHEAPER_REWRITTEN_NOPERM"}}}` (permissionDecision 無し) | codex は **原 command `echo CHEAPER_ORIGINAL_B` をそのまま実行パスへ** — `updatedInput` は**無視** |
| `{"hookSpecificOutput":{...,"permissionDecision":"ask","updatedInput":{"command":"echo CHEAPER_REWRITTEN_ASK"}}}` | 同上 — **原 command `echo CHEAPER_ORIGINAL_D`**、`updatedInput` 無視 |

確定事項:

- **`updatedInput` は `permissionDecision:"allow"` 併記でのみ適用
  される** — 単独・`"ask"` 併記では無視。upstream PR #20527 の
  「allow 必須」報告を実機で裏付け。emit shape は claude/devin と
  同じ nested `hookSpecificOutput` だが `permissionDecision:"allow"`
  が必須 → render 分岐が必要。
- **payload shape は claude と同一** — 実測 stdin:
  `{"session_id":…,"turn_id":…,"cwd":…,"hook_event_name":
  "PreToolUse","model":"gpt-6.1-sol","permission_mode":
  "bypassPermissions","tool_name":"Bash","tool_input":{"command":
  "echo CHEAPER_ORIGINAL_B"},"tool_use_id":"exec-…"}`。
  `pretooluse_normalize` そのままで正規化可能 (matcher の
  `Read|Grep` は codex が file-read tool を PreToolUse で emit
  しない upstream gap #18491/#20204 で不発 — shell のみ意味を持つ)。
- **hook `allow` は approval を素通しするが codex execpolicy は
  迂回しない** — rewrite 後 command も `Rejected: blocked by
  policy` で execpolicy 評価を受けた (この環境では read-only
  sandbox + 既定 execpolicy が全 `pwsh -Command` を拒否し
  end-to-end 実行は観測不能)。`permission_mode` は exec で
  `bypassPermissions` と報告されるため、bypass 面は
  **interactive approval prompt** に限定される。
- **hook trust model**: exec では `--dangerously-bypass-hook-trust`
  が無いと enabled hooks は未 review で実行されない旨の警告が出る
  — consumer への登録手順は trust 確立を含める必要がある。
- codex exec の `-c` config override では hooks.json を差し替えられ
  ない (hooks は `CODEX_HOME/hooks.json` 固定)。検証には隔離
  CODEX_HOME が必須だった。

### Cursor / Copilot — docs 根拠のみ (実機未検証)

- **Cursor**: `updated_input` (snake) / `updatedInput` (camel) は
  `preToolUse` event でのみ供給 (docs)。現行登録は
  `beforeReadFile`/`beforeShellExecution` — **event 移行が必須**。
  `preToolUse` の payload shape (tool_input 型か cursor 固有
  `command`/`file_path` 型か) と、`permission:"allow"` 併記が
  Cursor approval を素通しするかは未検証。Cursor response の
  `permission` は `allow|deny|ask` (matrix doc confirmed) —
  `ask` + `updated_input` が approval 温存 rewrite を構成するか
  どうかは実機要確認。
- **Copilot**: `modifiedArgs`/`updatedInput`/`additionalContext` は
  v1.0.24+ で公式 schema に存在 (prior investigation) が、実応答
  shape・`permissionDecision:"ask"` 併記時の approval 温存可否は
  未検証。現行 `preToolUse` + matcher `view|bash|powershell|shell`
  の登録は shell rewrite にはそのまま使える (emit のみ追加)。

## Key Issues and Assumptions

- [confirmed] Codex: rewrite emit は `hookSpecificOutput.
  {hookEventName:"PreToolUse", permissionDecision:"allow",
  updatedInput:{command}}` — **approval bypass が構造的に必須**。
- [confirmed] Codex: `allow` は interactive approval を素通しするが
  execpolicy/sandbox は迂回しない (verification 結果)。
- [confirmed] rewrite emit の判定点は render 層 — verdict 自体は
  `fallback` を保持するため flag 無効時は従来挙動へ完全に戻せる。
- [hypothesis] Cursor: `preToolUse` への event 移行 +
  `updated_input`/`updatedInput` emit。`"allow"` 併記要否と
  approval 温存可否は実機未検証。
- [hypothesis] Copilot: `modifiedArgs`/`updatedInput` の実応答
  field と `"ask"` の approval 意味は実機未検証。
- [assumption] opt-in flag は **trusted layer (user/builtin) のみ
  authoring 可能** にすべき — project 層 checkout が approval
  bypass を自己宣言できると supply-chain hole になる (`command`
  grants が project partial schema から除外されるのと同じ論理、
  `policy/schema.rs:289 policy_project_partial_schema`)。

## Alternatives and Comparison

### A. opt-in flag の配置先

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A1: `policy.yaml` 新 `route` section (`route.rewrite.approvalBypass: [target…]`) | 既存 3 層 trust model (complete/partial/project-partial) に乗る。project partial schema から `route` section 自体を除外すれば構造的に fail-closed (`command` と同型)。`RouteSupply` が firing ごとに load 済み → render への供給経路が既存 | policy schema 3 種 + `RuntimePolicy` + `normalize_policy` の追加変更。`route` section 新設は policy 面の拡張 | **yes** |
| A2: `routes.yaml` top-level `rewrite` key | route 文脈に近い | project routes.yaml は untrusted load — flag を trusted 層限定にすると merge 規則が複雑化 (現行 merge は per-route)。file 単位の trust が無い | no |
| A3: per-route `rewrite.approvalBypass` key | route 粒度の制御 | flag の意味は route ではなく **target の応答 contract** — 粒度が誤り。untrusted project route が新規 command に付けられる | no |
| A4: env var (`CHEAPER_HOOK_REWRITE_…`) | schema 変更不要 | ambient・audit 性なし・config 面と不整合 (policy が canonical surface) | no (補助としては将来可) |

### B. flag の粒度

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| B1: per-target list (`approvalBypass: ["codex"]`) | target の応答 contract と粒度一致。verify 済み target だけを有効化する明示的手段 | なし | **yes** |
| B2: global boolean | 単純 | cursor/copilot の形状が未検証のまま一律 bypass になる — issue の「未検証は保証にしない」に反する | no |
| B3: per-target object map | 拡張性 | YAGNI — 現時点で per-target の追加 field 要件は無い | no |

### C. emit 有効化の条件構造

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| C1: flag が「approval-bypassing emit の許可」のみを意味し、approval 温存 emit (検証で `ask`/無決定が機能すると判明した target) は flag 不要 | issue の文言規則と整合 — 検証済みの non-bypassing emit は無条件有効化できる | cursor/copilot の温存可否が未検証のため初期は flag 経由のみ | **yes** |
| C2: 全 rewrite emit を flag 必須にする | 保守的 | devin/claude 相当の温存形が実在する target で不要な opt-in を課す | no |

## Tentative Conclusions and Hypotheses

- [confirmed] **codex emit shape**:
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"allow","updatedInput":{"command":<rewrite>}}}`
  + exit 0。deny は現行 `hookSpecificOutput.permissionDecision:
  "deny"` のまま。
- [confirmed] **codex 登録変更不要** — 現行 `PreToolUse` + matcher
  `Read|Grep|Bash|Powershell` で shell (Bash) は既に発火する。
  normalize (`pretooluse_normalize`) も実測 payload と一致。
- [confirmed] **flag 不在時の fallback 維持** — `Verdict::Rewrite`
  の `fallback` verdict を render する現行機構のまま。flag は
  render への供給として `RouteSupply` → `eval_input` 系に載せる。
- [hypothesis] cursor: `preToolUse` event へ登録移行
  (`hooks.json` の `beforeReadFile`/`beforeShellExecution` →
  `preToolUse`) + `updated_input` emit。`"allow"` 要否・approval
  意味は **実機検証必須** (未検証のまま emit は spec 保証にしない)。
- [hypothesis] copilot: `modifiedArgs` or `updatedInput` emit (field
  名自体が未検証)。`"ask"` 温存可否も **実機検証必須**。
- [confirmed] opt-in 設計: `policy.yaml` の `route` section
  `route.rewrite.approvalBypass` を target 名 list とし、
  **project partial schema から `route` を除外** (untrusted
  project が approval bypass を有効化できない構造保証)。default
  `[]` = 現行 fallback 挙動。
- [open] codex execpolicy との関係: 本環境では全 `pwsh -Command`
  が `blocked by policy` だったため rewritten command の
  end-to-end 実行は未観測 — execpolicy を通過する環境での補完
  verification が望ましい (構造的適用は確認済み)。

## Open Questions

- Cursor `preToolUse` payload shape (`tool_input` か cursor 固有か)
  と `updated_input` 併記 `permission` の approval 意味 — **実機
  検証が必要** (cursor binary 不在のため本 session では未解決)。
- Copilot `modifiedArgs`/`updatedInput` の正確な応答 field と
  `"ask"` 併記時の approval 温存可否 — **実機検証が必要**
  (copilot CLI 不在)。
- Codex interactive TUI (非 exec) での `permission_mode` と
  approval prompt 挙動 — exec では `bypassPermissions` 報告のため
  TUI 側の観測は残るが、rewrite 適用条件の本質 (`allow` 必須) は
  検証済みで blocker ではない (manageable downstream)。

## Promotion Candidates

- [x] spec-doc needed: codex/cursor/copilot 各 target の rewrite
      emit shape・event 登録変更・opt-in flag contract・検証済み/
      未検証の区別を要件化する
- [x] adr-doc needed: `route.rewrite.approvalBypass` の配置
      (policy `route` section + project partial 除外) と粒度
      (per-target list)、codex emit の `permissionDecision:"allow"`
      併記、cursor event 移行方針を記録する

## Sources and Relations

- issue: <https://github.com/scarecrow173/cheaper/issues/150>
- upstream discovery:
  `docs/discovery/pretooluse-auto-routing-to-tasks-and-workflows.md`
  (DISC-034J7LwMEdPR5HxFkgOivZ — issue #142 確定判断 14 項目・
  alternative E 却下の経緯)、
  `docs/discovery/devin-pretooluse-rewrite-runtime-verification.md`
  (DISC-gHDyRKX2Xbdm58SDpCFLLt — devin rewrite 実機検証)、
  `docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md`
  (per-target contract matrix — codex file-read emit gap #18491/
  #20204、cursor `permission: allow|deny|ask`、copilot
  `permissionDecision` deny 規約)。
- parent spec/ADR: `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  (SPEC-034J7SxWvGL3oSbeBpo8a3 R38 — 3 target を issue #150 へ
  defer)、`docs/adr/pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md`
  (ADR-034J8eZHNPqARi25HLFOL6 D3)。
- code evidence: `crates/cheaper-cli/src/hook/targets.rs`
  (`codex_render` :572、`cursor_render` :675、`copilot_render`
  :435、`hook_specific_output_rewrite` :538、
  `hook_specific_output_deny` :508、`copilot_handle` :358、
  `cursor_normalize` :630)、`crates/cheaper-cli/src/hook/
  routing_policy.rs` (`Verdict::Rewrite` :129、`evaluate_route_arm`
  :1451、`build_rewrite_command` :1493)、`crates/cheaper-cli/src/
  hook/mod.rs` (`KNOWN_EVENTS` :31)、`crates/cheaper-cli/src/route/
  supply.rs` (`RouteSupply` :28、`load_or_none` :127)、
  `crates/cheaper-cli/src/route/routes.rs` (route schema :307)、
  `crates/cheaper-core/src/policy/schema.rs` (3 mode schema :238/
  :266/:289、`normalize_policy` :450)、`crates/cheaper-core/src/
  policy/load.rs` (層 merge :103-)、`packages/apm-hook-codex/.apm/
  hooks/hooks.json`、`packages/apm-hook-cursor/.apm/hooks/hooks.json`、
  `packages/apm-hook-copilot/.apm/hooks/hooks.json`。
- upstream refs: codex hooks doc
  (<https://developers.openai.com/codex/hooks>)、upstream PR #20527
  (`updatedInput` + `permissionDecision:"allow"` required)、
  openai/codex#18491・#20204 (file-read emit gap)、cursor hooks doc
  (<https://cursor.com/docs/hooks>)、copilot hooks reference
  (<https://docs.github.com/en/copilot/reference/hooks-configuration>)。
- real-CLI probe 証跡: `target/codex-hook-probe/` (gitignore 対象 —
  `stdin-*.jsonl` に codex PreToolUse payload 実測、exec JSONL
  output に rewrite 適否の観測)。
