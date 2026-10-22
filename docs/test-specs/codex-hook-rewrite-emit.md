---
id: "TSPEC-034KsKGUDFHh6p9yWNcOHQ"
type: "test-spec"
status: "draft"
title: "Codex hook rewrite emit under approvalBypass opt-in (issue #150)"
created: "2026-10-22"
updated: "2026-10-23"
owners: []
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
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "docs/plans/implement-per-target-hook-rewrite-enablement.md"
    - "docs/test-specs/hook-route-rewrite.md"
    - "docs/discovery/codex-cursor-copilot-rewrite-enablement.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
    - "docs/adr/per-target-hook-rewrite-enablement-with-policy-gated-approval-bypass.md"
  derived-by: []
  verifies:
    - "docs/specs/codex-cursor-copilot-hook-rewrite-enablement-and-approval-bypass-opt-in.md"
    - "docs/designs/codex-cursor-copilot-hook-rewrite-enablement.md"
  verified-by:
    - "docs/tasks/add-codex-flag-gated-rewrite-emit.md"
    - "docs/tasks/add-codex-rewrite-integration-tests.md"
  references:
    - "docs/discovery/devin-pretooluse-rewrite-runtime-verification.md"
    - "crates/cheaper-cli/src/hook/targets.rs"
  defers: []
  deferred-by: []
---

# Codex hook rewrite emit under approvalBypass opt-in (issue #150)

## Purpose

codex-cli 0.162.1 の実機 probe(DISC-034KrLM3o2H1kMylQFlg2U)で
確定した「`updatedInput` は `permissionDecision:"allow"` 併記時に
のみ適用される」という構造的事実にもとづく codex の bypassing
rewrite emit が、SPEC-034KrX7RnDs0gLVcMPf7vy R6-R8 と
ADR-034KrYGRHZuvOIx1d0S0nE D2/D5 の contract を満たし続けることを
pin する。

保つべき意図は 3 つ。(1) **opt-in なしに bypass しない**: approval
prompt を素通しする emit は user が `route.rewrite.approvalBypass`
に `"codex"` を列挙した場合にのみ発行される。(2) **無視される
shape を emit しない**: `updatedInput` 単独・
`permissionDecision:"ask"` 併記は codex が実機で無視するため、
いかなる flag 状態でも発行しない — 誤 emit は「rewrite した
つもりで原 command が走る」silent no-op になる。(3) **render 層に
差分を閉じる**: flag は render 出力のみに効き、`Verdict` enum・
`evaluate_route_arm` の Rewrite 生成条件・dedup 非 persist 規約は
不変で、flag 不在時は全 target で現行挙動に完全に戻る。

codex 側が `updatedInput` の適用条件を変えた場合(例: `ask` 併記や
単独でも適用するようになったことが実機確認された場合)、あるいは
approval 温存を維持する別 shape が確定して flag 不要の無条件 emit に
移行する場合に退役する。

## Feature

`--target codex` の `pre-tool-use` firing が route match して
`Verdict::Rewrite` を生成したとき、effective policy の
`route.rewrite.approvalBypass` に `"codex"` が含まれる場合に限り
`{"hookSpecificOutput":{"hookEventName":"PreToolUse",
"permissionDecision":"allow","updatedInput":{"command":<rewrite>}}}`
+ exit 0 を emit する render 分岐。flag 未供給・codex 非含有では
従来どおり `fallback` verdict を render する。

## Rules

- Rule: bypassing emit は `hookSpecificOutput` 内に
  `hookEventName:"PreToolUse"` → `permissionDecision:"allow"` →
  `updatedInput:{command}` の順で emit する(field 挿入順は probe
  観測順、`stringify_ordered` の byte 互換規約に従う)+ exit 0。
  rewrite command は `route run --payload <b64url>` 形。
- Rule: flag に `"codex"` が無い場合(未指定・空・他 target のみ)、
  codex は `Verdict::Rewrite{fallback}` を再帰 render する —
  deny → `hookSpecificOutput.permissionDecision:"deny"`、allow →
  silent exit 0。`updatedInput` を `permissionDecision` 無し・
  `"ask"` 併記で emit してはならない(いかなる flag 状態でも)。
- Rule: flag 判定は codex arm のみで行い、
  `supply.policy.route.rewrite.approval_bypass_enabled("codex")`
  経由で render へ渡す — eval(`RouteEvalInput`/`EvaluateOptions`/
  `evaluate_route_arm`)は flag を見ず、Rewrite 生成条件・
  `verdict_name` の `"rewrite"` 分類・dedup 規約は不変。
- Rule: claude(`pretooluse`)・devin の Rewrite emit は
  `hook_specific_output_rewrite(&command, false)` — 共有 emit 点の
  `approval_bypass` 引数で flag 非対称を保ち、emit byte は現行と
  完全一致(`permissionDecision` を emit しない、D3)。
- Rule: codex 完了 debug event(`cheaper.hook` の `hook response`)
  に `bypass: bool` field を追加し、approval-bypass emit の発生を
  log で観測可能にする — bypassing emit は silent exit 0 で
  stderr 痕跡を持たないため debug event が唯一の観測面。
- Rule: non-Shell request(FileRead/Search)は Rewrite を生成しない
  現行構造を維持 — flag は shell rewrite の render のみに効く。
- Rule: hook fail-open 規約を維持 — normalize 失敗・catch-all・
  panic は silent allow へ倒す。route supply load 失敗は flag
  未供給 = bypass 無効で route arm 不活性(fail-open、R15)。

## Examples

- user policy `approvalBypass: [codex]` + grant 済み `routed ...`
  の codex firing → stdout に
  `{"hookSpecificOutput":{"hookEventName":"PreToolUse",
  "permissionDecision":"allow","updatedInput":{"command":
  "\"<exe>\" route run --payload <b64url>"}}}` + exit 0。
  top-level `decision`/`updatedInput` は無い。
- 同一 firing で flag 未設定・`[]`・`[cursor]` のみ →
  `hookSpecificOutput` に `updatedInput` を含まず、fallback
  verdict(allow → silent / deny → deny shape)のみ返る。
- deny grant の command + flag 有り → fallback Deny を保持し
  rewrite しない(deny は bypassing emit に変換されない)。
- route supply load 失敗(policy 破損・project `route` key 等)→
  flag 未供給で fallback render + route arm 不活性。
- `hook_specific_output_rewrite(cmd, false)`(pretooluse/devin)→
  `hookEventName` + `updatedInput` のみで `permissionDecision` を
  含まない(現行 byte と同一)。

## Guarantees

- flag に `"codex"` が含まれる firing でのみ codex は
  `permissionDecision:"allow"` + `updatedInput.command` を受け取る。
- `updatedInput` 単独・`permissionDecision:"ask"` 併記の codex
  emit は flag 状態にかかわらず発行されない(負 shape の固定)。
- flag 不在時の codex の全既存挙動(normalize・deny・allow・
  fallback render)が維持される。
- bypassing emit の発生が `cheaper.hook` debug event の `bypass`
  field で追跡できる(silent exit 0 でも観測可能)。

## Non-goals

- flag の schema・load・trust 境界(project 層拒否・domain 検証)は
  `route-rewrite-approval-bypass-policy-flag` test spec が担う。
- cursor/copilot の Rewrite emit — emit contract 未検証のため本
  spec では「発行しない」ことのみを他 spec(register-cursor
  preToolUse)と既存 fallback render 保証で扱う。
- codex の file-read interception(upstream gap — rewrite は Shell
  request のみに効く)。
- codex execpolicy の通過可否(hook `allow` は approval 面のみを
  bypass し、rewritten command が codex policy を通るかは codex 側
  の設定問題)。
- `route run` payload の内部 contract(decode・revalidate)は既存
  `route-run-delegated-exec` test spec が担う。

## Risk

- この保証を落とすと、user が opt-in していないのに approval
  prompt を素通しする emit が発行される(意図しない承認省略)か、
  codex が無視する shape を emit して「rewrite したつもりで原
  command がそのまま実行される」silent no-op になる。
- claude/devin の emit byte が変わると、approval を温存する設計上の
  根拠(`permissionDecision` を emit しない決定、parent ADR D3)が
  崩れる。
- `bypass` debug field が無いと、silent exit 0 の bypassing emit を
  audit・診断する観測面がなくなる。
