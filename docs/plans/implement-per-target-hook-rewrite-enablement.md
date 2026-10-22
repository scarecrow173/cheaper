---
id: "PLAN-034KsKo6nYOBOpIpkNoviY"
type: "plan"
status: "approved"
title: "Codex/Cursor/Copilot hook rewrite enablement と approvalBypass opt-in の実装計画 (issue #150)"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
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
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
  implemented-by:
    - "docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md"
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
    - "docs/tasks/register-cursor-pretooluse-hook.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
    - "docs/tasks/document-approval-bypass-policy-flag.md"
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
    - "docs/test-specs/hook-route-rewrite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  derived-by:
    - "docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md"
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
    - "docs/tasks/register-cursor-pretooluse-hook.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
    - "docs/tasks/document-approval-bypass-policy-flag.md"
  verifies: []
  verified-by:
    - "docs/test-specs/route-rewrite-approval-bypass-policy-flag.md"
    - "docs/test-specs/codex-hook-rewrite-emit.md"
    - "docs/test-specs/cursor-pretooluse-registration-and-normalize.md"
  references:
    - "docs/discovery/devin-pretooluse-rewrite-runtime-verification.md"
    - "docs/discovery/cross-agent-native-routing-adapter-contract-re-verification.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-core/src/policy/schema.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
---

# Codex/Cursor/Copilot hook rewrite enablement と approvalBypass opt-in の実装計画 (issue #150) Implementation Plan

## Goal

`Verdict::Rewrite{command, fallback}` を codex で opt-in 有効化する
— `route.rewrite.approvalBypass`(hook target 名 list、trusted 層のみ
authoring 可)を新設し、flag に `"codex"` が含まれる場合のみ
`hookSpecificOutput` + `permissionDecision:"allow"` +
`updatedInput:{command}`(実機確定 shape、codex-cli 0.162.1
probe)を emit する。cursor は `preToolUse` event 登録と
`tool_input` nested payload normalize の受け口を先行整備し、
cursor/copilot の rewrite emit 本体は emit contract 実機検証まで
発行しない(flag の有無にかかわらず fallback render)。

## Scope

単一の implementation stream。flag schema(load・trust 境界)→
flag 供給経路(`evaluate_with_routes` 戻り値拡張)→ codex render →
cursor 受け口 → integration → docs は同じ critical path(flag が無ければ
render 分岐を書けず、render が無ければ integration を固定できない)に
乗るため分割しない。cursor/copilot emit 本体・旧 event 撤去・windsurf・
execpolicy 通過・env var override は out of scope(design §Scope と同一
境界)。

| In | Out |
|---|---|
| `policy.yaml` `route` section + `RoutePolicy`/`RewritePolicy` normalized 表現 + project partial 除外 | `Verdict` enum・`RouteEvalInput`/`EvaluateOptions`・`evaluate_route_arm`・dedup 規約(不変) |
| `targets.rs` render 層(codex bypassing emit・`evaluate_with_routes` 戻り値・cursor_normalize・bypass debug field) | `route/supply.rs`・`hook/mod.rs`・apm-hook-codex/copilot hooks.json(変更不要) |
| cursor `preToolUse` 登録 + normalize(emit は発行しない) | cursor/copilot rewrite emit 本体(実機検証 task へ gate) |
| golden fixture 再生成 + `RuntimePolicy{…}` literal 追記 + contract/integration test | claude(`pretooluse`)・devin・windsurf の emit 挙動(不変) |
| `docs/guides/configuration.md` の `route` 記法 | consumer 向け hook 登録手順・cursor 旧 event 撤去判断 |

## Upstream Documents

- `docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md`
  (SPEC-034KrX7RnDs0gLVcMPf7vy — R1-R17・acceptance criteria)
- `docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md`
  (ADR-034KrYGRHZuvOIx1d0S0nE — Option A・D1-D6 の個別決定)
- `docs/discovery/codex-cursor-copilot-rewrite-enablement.md`
  (DISC-034KrLM3o2H1kMylQFlg2U — codex 実機 probe 証跡・検証済み/
  未検証分類)

## Design Inputs

- `docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md`
  (DESIGN-034KsaxkFcfUHzvbR5hEB1、approved — AUTHORITATIVE な HOW:
  `RoutePolicy`/`RewritePolicy` 形・`approval_bypass_enabled` 単一
  owner・`hook_specific_output_rewrite(command, approval_bypass)`
  signature・`evaluate_with_routes` → `(Verdict, Option<RouteSupply>)`
  戻り値・cursor field 駆動判別・bypass debug event・file manifest)

## File Map

| Path | Responsibility | Notes |
|------|----------------|-------|
| `crates/cheaper-core/src/policy/schema.rs` | `route` section schema(`route_properties()`)・`RoutePolicy`/`RewritePolicy` normalized 型・`approval_bypass_enabled`・`normalize_policy` passthrough・project partial への非追加 | T1 が変更。`command` section 除外と同型の fail-closed |
| `crates/cheaper-cli/resources/policy.yaml` | builtin 既定 `route: {rewrite: {approvalBypass: []}}` 追記 | T1。self-documenting 既定値 |
| `crates/cheaper-core/src/policy/restrict.rs`・`capability.rs` | `RuntimePolicy{…}` literal へ `route` field 追記(:112・:86) | T1。compile 必須の追随変更 |
| `crates/cheaper-runtime/src/dev/fixture.rs` | `RuntimePolicy{…}` literal 追記(:226・:758) | T1。同上 |
| `crates/cheaper-runtime/tests/run_dev_seam.rs`・`ollama_e2e.rs` | `RuntimePolicy{…}` literal 追記(:189・:219) | T1。同上 |
| `crates/cheaper-core/tests/golden/policy_{load,schema,restrict}.rs` + `fixtures/policy/{load,restrict,schema}.json` | `RuntimePolicy.route` の serialize 追加に伴う `expected.policy` 再生成 | T1。schema 変更と同 commit で regen |
| `crates/cheaper-cli/src/hook/targets.rs` | `hook_specific_output_rewrite` の `approval_bypass` 引数・`codex_render` flag 分岐・`evaluate_with_routes` 戻り値拡張・`handle_hook` flag 供給 + `bypass` debug field・`cursor_normalize` nested `tool_input` | T2(emit 系)・T3(normalize)が変更 |
| `packages/apm-hook-cursor/.apm/hooks/hooks.json` | `preToolUse` entry 追加(sh/powershell 両 entry、`{"permission":"allow"}` echo fallback) | T3。旧 event 併存維持(D4) |
| `crates/cheaper-cli/tests/integration/route_run.rs` | user flag + grant で codex emit、flag 無し fallback、project `route` key fail-closed の integration test | T4。`devin_hook_rewrite_emits_nested_updated_input` precedent |
| `docs/guides/configuration.md` | `route` section 記法・trust 境界・per-target 検証状況 | T5 |

## Implementation Sequence

- [ ] TASK-034KsKLJ7FGv1ytMp8eLbx: `route` section schema +
      `RoutePolicy`/`RewritePolicy` + `approval_bypass_enabled` +
      builtin policy.yaml + literal 5 site + golden fixture 再生成 +
      schema/load unit test(domain 外値・project `route` key 拒否・
      default `[]`・merge)
- [ ] TASK-034KsKPPDTLAGrCkK8k17Y: `evaluate_with_routes` 戻り値
      `(Verdict, Option<RouteSupply>)` 化 + `handle_hook` codex arm の
      flag 供給 + `bypass` debug field + `hook_specific_output_rewrite`
      `approval_bypass` 引数 + `codex_render` 分岐 + unit test
      (emit shape・負 shape・flag 非対称不変)
- [ ] TASK-034KsKTKF4k3oBwHXkLXR6: cursor hooks.json `preToolUse`
      登録 + `cursor_normalize` nested `tool_input` 対応 + unit test +
      `check-apm-payload.mjs`
- [ ] TASK-034KsKXLwLr4ZYjemKg6Hr: `route_run.rs` integration test
      (flag 有り emit・flag 無し fallback・project `route` key
      fail-closed)
- [ ] TASK-034KsL2829yYfRRGKl92t1: `configuration.md` 追記 +
      `mise run check` で閉じる

## Dependency Graph

| Step | Depends on | Blocks |
|------|------------|--------|
| T1 flag schema | — | T2, T4 |
| T2 codex emit | T1 | T4 |
| T3 cursor 登録 + normalize | — | T5 |
| T4 integration test | T1, T2 | T5 |
| T5 docs + 最終検証 | T3, T4 | — |

critical path は T1 → T2 → T4 → T5。T2 が T1 に依存するのは
`approval_bypass_enabled` と `RuntimePolicy.route`(flag の供給元)が
T1 の成果だから。T3 は flag に非依存(登録・normalize は emit contract
と別軸)のため T1/T2 と並列可。T4 は codex emit の end-to-end を
固定するため T1+T2 が前提。T5 は全変更を含む `mise run check` で
閉じる最終段。

## Dependencies and Risks

| # | Risk | Likelihood | Impact | Mitigation |
|---|------|-----------|--------|------------|
| 1 | 誤 emit(`updatedInput` 単独・`"ask"` 併記)で codex が無視し原 command が走る silent no-op | med | high | emit shape を probe 確定形に固定し、負 shape test(T2)で byte レベル近似で pin |
| 2 | flag 追加で golden fixture・literal site が compile/test 破壊 | high(必発) | med | T1 で schema 変更と literal 追記・fixture 再生成を同 commit に原子化 |
| 3 | cursor `preToolUse` 応答 schema が実機未検証のまま登録だけ出荷 | med | med | matcher 無し全 tool firing を受ける登録のため fail-open(schema-valid allow)を維持し、risk は doc で明記。emit 本体は引き続き検証 gated |
| 4 | cursor 二重 firing(preToolUse + beforeXxx 併存) | med | low | allow/deny は dedup が吸収、Rewrite は非 persist で再 eval(cost は extra eval 1 回) — design §Risks の dedup nuance |
| 5 | `approvalBypass: [cursor]` が no-op である誤解 | low | low | doc(T5)で「flag は許可であり有効化は emit contract 確定後」と明記 |
| 6 | data-loss / breaking-change | — | — | append-only schema 追加で既存 policy・hook 応答・record 非互換なし |
| 7 | external dependency — なし | — | — | 新規 crate 依存ゼロ。cursor/copilot CLI は実機検証 task の前提で本 plan 内では不要 |

Rollback strategy: 全変更は git commit 単位の通常 revert で戻せる。
不可逆 step(migration・外部 format 固定)は無い — flag 不在時の
実行時挙動は現行と完全同一(render 層のみの差分)。

## Task Breakdown

- T1 `docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md`
  — flag schema・normalized 型・builtin 既定・literal/fixture 追随
  (schema 変更の原子単位)。verified by
  `route-rewrite-approval-bypass-policy-flag`
- T2 `docs/tasks/add-codex-flag-gated-rewrite-emit.md`
  — flag 供給経路 + codex bypassing emit + observability。
  verified by `codex-hook-rewrite-emit`
- T3 `docs/tasks/register-cursor-pretooluse-hook.md`
  — cursor `preToolUse` 登録 + normalize 受け口(emit 無し)。
  verified by `cursor-pretooluse-registration-and-normalize`
- T4 `docs/tasks/add-codex-rewrite-integration-tests.md`
  — end-to-end contract(flag 有/無・project 拒否)。
  verified by `route-rewrite-approval-bypass-policy-flag` +
  `codex-hook-rewrite-emit`
- T5 `docs/tasks/document-approval-bypass-policy-flag.md`
  — user-facing 記法・trust 境界・検証状況

## Verification

- [ ] `mise exec -- cargo test -p cheaper-core --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` で schema/load/
      golden contract 全 pass(T1)
- [ ] `mise exec -- cargo test -p cheaper-cli --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration
      hook::` + `route_run` module filter で codex emit 3 ケースが
      pass(T2・T4)
- [ ] `node scripts/check-apm-payload.mjs` で cursor hooks.json
      変更が slim payload check に整合(T3)
- [ ] `mise exec -- cargo test --workspace --features
      cheaper-runtime/dev-seam,cheaper-cli/dev-seam` 全 green
      (fixture regen 込み、flag 不在時の実行時挙動不変)
- [ ] `mise run check` 全 green(biome/fmt/clippy/doc-id/lockfile/
      payload — T5)
- [ ] spec R1-R17・ADR D1-D6 の Acceptance Criteria が task/test
      spec の検証条件へ写像されていること

## Open Questions / Gaps

- cursor `preToolUse` の payload shape(`tool_name`/`tool_input` か
  cursor 固有か)・`tool_name` domain・応答 schema(`permission` 系
  flat が正しいか)は実機未検証 — T3 は field 駆動判別で両 shape
  hypothesis を吸収する normalize のみ実装し、確定値を仮定しない。
  emit contract 確定は cursor CLI 環境での別検証 task(design
  §Deferred Design Concerns)。
- copilot emit field(`modifiedArgs`/`updatedInput`)と
  `permissionDecision:"ask"` の approval 温存可否も同様に実機検証
  task へ defer — 本 plan の task はいずれも copilot の登録・
  normalize・render を変更しない。
- `hook response` debug event の `bypass` field は codex arm のみ —
  cursor/copilot の emit 有効化時に同型 field を追加するかはその
  issue の design で決める(本 plan では flag gated emit を持つのが
  codex のみのため対称化しない、YAGNI)。

## Self-Review

- [x] Every spec requirement maps to a task.(R1-R5/R15→T1、
      R6-R8/D2→T2、R9-R11→T3、R10/R12/R13 の emit gate→T2/T3 の
      fallback 不変 assert、R14/R16/R17→T1-T4 の不変条件 test、
      AC「user-facing doc」→T5)
- [x] No placeholders remain.(未検証事項は Open Questions へ明示)
- [x] File paths, relations, status values, and terminology are
      consistent.(design manifest と同一 path 集合)

## Implementation Handoff

- Recommended mode: delegated or subagent-capable implementation —
  T3(cursor 登録 + normalize)は flag 系(T1→T2→T4)と独立の
  work stream で並列実行可能。
- Independent work streams: T1→T2→T4(critical path)、T3(並列可)、
  T5(最終)。
- User approval required before dispatch: yes
- If approved: discover available implementation and delegation
  capabilities in the current environment, then execute task-by-task
  with review checkpoints.
- If declined or unavailable: execute inline using the same dependency
  graph and verification matrix.

## Review Handoff

- 焦点: flag が render 層のみに効く構造(T2 が `Verdict`/
  `evaluate_route_arm`/dedup に触れないこと)、project partial
  schema への `route` 非追加が `command` 除外と同型に fail-closed
  で動くこと(T1 の test で pin)、emit しない負 shape が flag
  状態にかかわらず成立すること(T2)。
- 明示仮定: cursor `preToolUse` 応答は flat `{"permission":…}` schema
  を再利用(docs 報告ベース、実機未検証 — spec の文言規則上 spec
  保証ではなく risk として扱う)。
- 未解決依存: cursor/copilot の実機検証は別環境 task — 本 plan の
  完了条件には含まない(emit gate を task で固定するのが本 plan の
  責務)。
- 上流: SPEC-034KrX7RnDs0gLVcMPf7vy / ADR-034KrYGRHZuvOIx1d0S0nE /
  DESIGN-034KsaxkFcfUHzvbR5hEB1。
