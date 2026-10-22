---
id: "IMPL-034KtIguXiJnzT9ykiH7uV"
type: "impl"
status: "completed"
title: "Implement per-target hook rewrite enablement and approval bypass issue 150"
created: "2026-10-23"
updated: "2026-10-23"
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
        path: "crates/cheaper-core/src/policy/load.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/restrict.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/capability.rs"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/load.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/restrict.json"
      - type: "file"
        path: "crates/cheaper-core/tests/golden/fixtures/policy/schema.json"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/targets.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "packages/apm-hook-cursor/.apm/hooks/hooks.json"
      - type: "file"
        path: "scripts/test/apm-install-targets.test.mjs"
      - type: "file"
        path: "scripts/test/lib/apm-fixture.mjs"
      - type: "file"
        path: "crates/cheaper-cli/resources/policy.yaml"
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
    - "docs/tasks/add-route-rewrite-approval-bypass-policy-schema.md"
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
    - "docs/tasks/register-cursor-pretooluse-hook.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
    - "docs/tasks/document-approval-bypass-policy-flag.md"
  implemented-by: []
  depends-on:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
    - "docs/test-specs/hook-route-rewrite.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
  derived-by: []
  verifies: []
  verified-by:
    - "docs/test-specs/route-rewrite-approval-bypass-policy-flag.md"
    - "docs/test-specs/codex-hook-rewrite-emit.md"
    - "docs/test-specs/cursor-pretooluse-registration-and-normalize.md"
  references:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# Implement per-target hook rewrite enablement and approval bypass issue 150

## Summary

issue #150 — claude(`pretooluse`)・devin のみが `hookSpecificOutput.updatedInput`
を emit し codex/cursor/copilot が fallback render に留まっていた
`Verdict::Rewrite{command, fallback}` の per-target 有効化を、
design(DESIGN-034KsaxkFcfUHzvbR5hEB1)/ ADR(ADR-034KrYGRHZuvOIx1d0S0nE)の
契約どおりに出荷した。

1. `route.rewrite.approvalBypass` policy flag を新設 — bypassing emit
   (親 agent の approval prompt を素通しする
   `permissionDecision:"allow"` 併記の rewrite 応答)を許可する hook
   target 名(domain `codex`/`cursor`/`copilot`)の **opt-in・trusted
   層(builtin/user)限定** list。project partial schema は `route`
   property 自体を持たず、`route` key を含む project policy は
   `InvalidRuntimePolicy` で fail-closed(`command` section 除外と同型
   — untrusted checkout が bypass を自己宣言できない構造保証)。
   `["array","null"]` + `uniqueItems` + enum で、未指定・`null`・
   空配列は default `[]` 等価(bypass 無効の conservative default)。
2. codex の flag-gated emit — flag に `"codex"` が含まれる場合のみ
   `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
   "permissionDecision":"allow","updatedInput":{"command":…}}}` +
   exit 0 を emit(probe 確定 shape・field 順は観測順)。flag 無し
   (未指定・codex 非含有・route supply load 失敗)では従来どおり
   fallback verdict を render し `updatedInput` を emit しない。
   `updatedInput` 単独・`"ask"` 併記は codex が無視する silent
   no-op(実機確認済み)のため **いかなる flag 状態でも発行しない**。
3. cursor — `preToolUse` event を hooks.json へ登録(sh/powershell 両
   entry・matcher 無し・失敗時 `{"permission":"allow"}` echo・旧
   beforeXxx event 併存維持)し、`cursor_normalize` を nested
   `tool_input.command`/`tool_input.file_path` の field 駆動読みへ
   拡張。**emit は一切追加しない**(emit contract 未検証 — flag の
   有無にかかわらず fallback render を維持)。
4. copilot — 登録・normalize・render いずれも変更なし(emit field・
   `"ask"` の approval 意味が未検証のため gate 維持)。
5. observability — codex arm の `hook response` debug event に
   `bypass: bool` field を追加(bypassing emit は silent exit 0 で
   stderr 痕跡を持たないため debug event が唯一の観測面)。

## Context

TASK-034KsKLJ7FGv1ytMp8eLbx(T1 flag schema)・
TASK-034KsKPPDTLAGrCkK8k17Y(T2 codex emit)・
TASK-034KsKTKF4k3oBwHXkLXR6(T3 cursor 登録+normalize)・
TASK-034KsKXLwLr4ZYjemKg6Hr(T4 integration test)・
TASK-034KsL2829yYfRRGKl92t1(T5 docs)の 5 task chain。
PLAN-034KsKo6nYOBOpIpkNoviY / DESIGN-034KsaxkFcfUHzvbR5hEB1 /
SPEC-034KrX7RnDs0gLVcMPf7vy / ADR-034KrYGRHZuvOIx1d0S0nE に従う。
codex emit shape の実機確定証跡(codex-cli 0.162.1・隔離 CODEX_HOME
probe: `updatedInput` は `permissionDecision:"allow"` 併記時のみ適用、
単独・`"ask"` 併記は silent no-op)は
DISC-034KrLM3o2H1kMylQFlg2U に記録済み。

## Implementation

### Changes

- `crates/cheaper-core/src/policy/schema.rs` (T1): `route_properties()`
  を追加し complete/partial schema の optional property として
  `"route"` を登録(project partial には非追加 — section 丸ごと除外)。
  normalized 表現 `RoutePolicy{rewrite: RewritePolicy{approval_bypass:
  Vec<String>}}`(serialize は `approvalBypass` camelCase、yaml 表現と
  1:1)と `RuntimePolicy.route` を追加。`RewritePolicy::
  approval_bypass_enabled(target)` が flag 判定の単一 owner。
  `normalize_policy` は passthrough 解決(domain 検証は schema が
  fail-loud に担い normalize で二重解釈しない)。schema test
  (受理・default 等価・domain 外/重複/非配列/非 object 拒否・
  project `route` key 拒否・serialize 射影)を追加。
- `crates/cheaper-core/src/policy/load.rs` (T1): user policy の
  `route` merge が effective policy へ反映されること、project policy
  の `route` key が load error になることを pin する load test を追加。
- `crates/cheaper-cli/resources/policy.yaml` (T1): builtin 既定に
  `route: {rewrite: {approvalBypass: []}}` を追記(self-documenting
  既定値)。
- `crates/cheaper-core/src/policy/restrict.rs`・`capability.rs`・
  `crates/cheaper-runtime/src/dev/fixture.rs`・
  `crates/cheaper-runtime/tests/run_dev_seam.rs`・`ollama_e2e.rs` (T1):
  `RuntimePolicy{…}` literal 5 site へ `route` field を追記。
- golden fixture `crates/cheaper-core/tests/golden/fixtures/policy/
  {load,restrict,schema}.json` (T1): `RuntimePolicy.route` の
  serialize 追加に伴う `expected.policy` 再生成(schema 変更と同
  commit に原子化)。
- `crates/cheaper-cli/src/hook/targets.rs` (T2+T3):
  - `hook_specific_output_rewrite(command, approval_bypass)` —
    `approval_bypass` 時のみ `permissionDecision:"allow"` を
    `hookEventName` と `updatedInput` の間に挿入(probe 観測順・
    `stringify_ordered` byte 互換規約)。共有 emit 点を 1 箇所に
    保ち、claude(`pretooluse`)/devin は常に `false` で emit byte
    不変。
  - `codex_render(verdict, approval_bypass)` — flag 有りの
    Rewrite は bypassing emit、無しは従来どおり fallback verdict を
    再帰 render。
  - `evaluate_with_routes` が `(Verdict, Option<RouteSupply>)` を
    返すよう拡張 — load 済み supply を呼び出し側へ返し codex arm
    が flag を読む。load 失敗(`None`)は flag 未供給 = bypass 無効。
    `Verdict`/`RouteEvalInput`/`evaluate_route_arm`/dedup 規約は
    不変(R14)。
  - `handle_hook` codex arm のみ `supply.policy.route.rewrite.
    approval_bypass_enabled("codex")` で flag を供給し、
    `hook response` debug event に `bypass` field を追加(codex
    arm のみ — 対称化しない、plan Open Questions 記載どおり
    YAGNI)。
  - `cursor_normalize` (T3) — top-level flat
    `command`/`file_path`(beforeXxx payload)の判定を先に試し、
    nested `tool_input.command`/`tool_input.file_path` を field
    駆動で読む(source_tool: `cursor:preToolUse`)。`tool_name`
    domain 未検証のため name matching ではなく field の有無で
    判別し両 shape hypothesis を吸収。
- `packages/apm-hook-cursor/.apm/hooks/hooks.json` (T3): `preToolUse`
  flat entry を sh/powershell 両形式で追加(`cheaper hook
  pre-tool-use --target cursor`、失敗時 `{"permission":"allow"}`
  echo — beforeXxx entry と同型)。`matcher` は tool 名 domain 未検証
  のため付けず全 tool firing を受ける(fail-open)。旧 2 event は
  併存維持(D4)。
- `crates/cheaper-cli/tests/integration/route_run.rs` (T4):
  `devin_hook_rewrite_emits_nested_updated_input` precedent に倣い
  3 test を追加 — user flag + grant で codex が
  `permissionDecision:"allow"`+`updatedInput.command` を emit・
  flag 無しは fallback render のみ・project `.cheaper/policy.yaml`
  の `route` key が fail-closed。
- `scripts/test/apm-install-targets.test.mjs`・
  `scripts/test/lib/apm-fixture.mjs` (T3 追随・commit `03e3576`):
  cursor 期待 hook event matrix に `preToolUse` を追加(apm install
  target 検証の期待値整合)。
- `docs/guides/configuration.md` (T5): `route.rewrite.approvalBypass`
  の記法・default・値 domain・fail-loud 検証・builtin/user 限定
  trust 境界・per-target 現状(codex emit 確定済み / cursor・
  copilot は flag 列挙しても現状 no-op / claude・devin 対象外)・
  execpolicy 非迂回・hook trust 前提を追記。

### Decisions During Implementation

- emit 点の共有を維持 — `hook_specific_output_rewrite` に
  `approval_bypass` 引数を追加し claude/devin は `false` で呼ぶ
  形に集約した(codex 専用の emit 複製を作らず、flag 非対称 target
  の wire 表現を byte 単位で不変に保つ — D3)。
- `cursor_normalize` の判定順序 — top-level flat field を先に評価
  し nested `tool_input` を後続で読む。beforeXxx payload と
  preToolUse payload が同居する併存期間(D4)の誤判別を防ぐ順序。
- `matcher` 無し登録 + fail-open — cursor の tool 名 domain・
  payload shape・応答 schema が未検証のため、確定値を仮定せず
  全 tool firing を受け normalize 失敗は schema-valid allow へ
  畳む。応答の flat `{"permission":…}` schema は既存 beforeXxx
  応答を再利用(docs 報告ベース、実機要確認 — risk として記録)。
- field 駆動判別の over-match を許容 — `tool_name` domain が
  確定するまで read 以外の tool が `file_path` input を持つ場合に
  FileRead として評価され得る既知残 risk(design §Risks)を、
  name matching を仮定する代替案より安全側として採用。
- `bypass` debug field は codex arm のみ — cursor/copilot の emit
  有効化時に同型 field を追加するかはその issue の design で
  決める(plan Open Questions、YAGNI)。
- apm install 期待値の追随修正(`03e3576`)は T3 の hooks.json 変更が
  install event matrix test を破ったため別 commit で補正 —
  cursor hooks.json の preToolUse 登録は宣言・fixture 期待値・
  slim payload check の 3 面で整合させる必要があった。

## Related Experiments

### Adopted

なし(known-solution task。emit shape は DISC-034KrLM3o2H1kMylQFlg2U
の実機 probe で確定済み、design が flag 供給経路・render 分岐・
file manifest まで確定していたため Experiment Log は未作成)。

### Rejected

なし。

## Validation

- T1: `cargo test -p cheaper-core` — schema/load/golden contract
  全 pass(`route` 受理・default 等価・fail-loud 拒否・project
  fail-closed・fixture 再生成整合)
- T2: `cargo test -p cheaper-cli --test integration hook::` 等 —
  targets.rs unit test で emit shape(`permissionDecision:"allow"`
  + `updatedInput.command` + `hookEventName:"PreToolUse"` + exit
  0)・負 shape(flag 無し fallback・`updatedInput` 単独/"ask" 併記
  を発行しない)・claude/devin emit byte 不変を pin
- T3: cursor_normalize unit test(nested `tool_input` → Shell/
  FileRead・flat 優先・malformed fail-open)+
  `node scripts/check-apm-payload.mjs` で cursor hooks.json が
  slim payload check に整合
- T4: `route_run.rs` integration 3 test — flag 有り codex emit・
  flag 無し fallback のみ・project `route` key fail-closed が
  実配線(stdin payload → hooks.json → policy load → render)
  経由で pass
- T5: `mise run check` 全 step exit 0(biome・`cargo fmt --check`・
  clippy・version/doc-id check・`cargo test --workspace`・
  `node --test`・lockfile/apm-payload check)
- codex 実機検証(DISC-034KrLM3o2H1kMylQFlg2U、実装前 probe):
  codex-cli 0.162.1 + 隔離 `CODEX_HOME` で `permissionDecision:
  "allow"` + `updatedInput` emit の適用を確認済み

証跡 commit: `a6e7efe`(T1)→ `0dcb7b1`(T2)→ `d2c4085`(T3)→
`03e3576`(T3 追随 fix)→ `d11bf9f`(T4)→ `b45e0d9`(T5)。
feature/route-rewrite-approval-bypass から main へ ff merge 済み。

検証限界(正直な記録):

- codex の emit **適用**は実機 probe で確定済みだが、interactive
  TUI session での approval prompt 素通し挙動は未観測(exec は
  `bypassPermissions` 報告 — design §Deferred Design Concerns)。
- rewritten command の codex execpolicy 通過は本環境未検証(本
  env の execpolicy が全 `pwsh -Command` を拒否するため)。
- cursor の `preToolUse` payload shape・`tool_name` domain・応答
  schema・rewrite emit contract、copilot の emit field・
  `permissionDecision:"ask"` の approval 温存可否は **実機未検証**
  — issue の文言規則「実機検証で確認できない挙動は仕様上の保証
  として扱わない」に従い emit-gate で担保(未検証 target は flag
  の有無にかかわらず emit しない = spec 保証外に留める構造)。

## Risks

- cursor `preToolUse` 応答は flat `{"permission":…}` schema を
  実機未検証のまま出荷 — Cursor が shape を拒否・解釈不能にした
  場合、matcher 無しで全 tool firing を受ける登録のため tool call
  を block し得る(影響面は rewrite に留まらない。deny/allow 通常
  応答を含む)。
- cursor 二重 firing — `preToolUse` と beforeXxx の併存で同一
  tool call が 2 回 hook 評価される。allow/deny は dedup が吸収
  するが `Verdict::Rewrite` は非 persist で再 eval(extra eval
  1 回の cost)。旧 event 撤去可否は実機検証依存。
- `approvalBypass: [cursor|copilot]` は現状 no-op — configuration
  guide で「flag は許可の宣言であり有効化は emit contract 確定後」
  と明記済み(fallback は従来挙動で誤解 cost は低い)。
- field 駆動判別の over-match — read 以外の tool が `file_path`
  input を持つ場合に FileRead として評価され、巨大 target で
  deny になり得る誤判定面が `tool_name` domain 確定まで残る。
- bypassing emit は親 agent の approval prompt を素通しするが
  codex execpolicy は迂回しない — consumer 側で codex hook の
  trust 確立(`--dangerously-bypass-hook-trust` 等の未 review
  hook 実行前提)が emit の前提条件。

## Follow-ups

followup-triage 分類: **`followup-terminal`** — 本 implementation
chain(issue #150)由来の未分類・未処理の follow-up はない。

- 残る検証債務(cursor payload/応答 schema・emit contract、
  copilot emit field・`"ask"` の approval 意味、codex TUI
  approval 挙動・execpolicy 通過)はすべて design §Deferred Design
  Concerns で「cursor/copilot CLI が利用可能な環境での実機検証
  task」へ **意図的に defer 済み・分類済み** のものであり、
  本 chain の未処理作業ではない。issue の文言規則「実機検証で
  確認できない挙動は spec 保証にしない」は emit-gate(flag が
  あっても未検証 target は emit しない構造)で満たされており、
  spec/ADR 契約上の未充足は残っていない。
- これらの検証は cursor binary・copilot CLI が本環境に存在しない
  ため **repo 内で実行可能な作業ではない** — 環境依存の前提
  条件が解消されるまで code/doc 側で進められる follow-up はなく、
  chain は terminal で閉じるのが適切。
- **推奨(orchestrator 判断)**: 検証債務の受け皿として
  「cursor/copilot rewrite emit 実機検証(+ codex TUI/execpolicy
  補完検証)」の tracking issue を切ることを推奨する — 検証
  環境確保時に emit contract(cursor: `updated_input` vs
  `updatedInput`・`permission` 併記要否・`tool_name` domain・
  `matcher`・旧 event 撤去可否 / copilot: `modifiedArgs` vs
  `updatedInput`・`"ask"` の approval 温存可否)を確定し、
  flag gated emit arm を解放する後続 chain の起点となる。
  issue 作成自体は orchestrator の判断に委ね、本 IR では
  推奨の記録に留める。
