---
id: "TSPEC-034KVxaBMzQE5YtVOx4j3w"
type: "test-spec"
status: "approved"
title: "route grant-set projection の検証契約 (issue #166)"
created: "2026-10-23"
updated: "2026-10-23"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
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
    - "docs/plans/route-grant-set-projection.md"
    - "docs/discovery/route-grant-set-projection-issue-166.md"
    - "docs/test-specs/unified-exec-grant-model.md"
  refines: []
  refined-by: []
  derives-from:
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
    - "docs/adr/route-grant-set-projection.md"
  derived-by: []
  verifies:
    - "docs/specs/route-grant-set-projection.md"
    - "docs/designs/route-grant-set-projection.md"
  verified-by: []
  references:
    - "docs/designs/permission-model-responsibility-redesign.md"
    - "docs/specs/permission-model-responsibility-redesign.md"
    - "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "docs/specs/unified-exec-grant-model.md"
    - "docs/adr/unified-exec-grant-model.md"
    - "docs/impl/ir/unified-exec-grant-model-issue-165.md"
    - "docs/impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route/resolve.rs"
    - "crates/cheaper-cli/src/route/run.rs"
    - "crates/cheaper-cli/src/usage_log.rs"
    - "crates/cheaper-cli/src/hook/routing_policy.rs"
    - "crates/cheaper-core/src/policy/exec.rs"
    - "crates/cheaper-tools/src/tools/exec/mod.rs"
    - "crates/cheaper-tools/src/tools/command/mod.rs"
  defers: []
  deferred-by: []
---

# route grant-set projection の検証契約 (issue #166)

## Purpose

issue #166(#151 実装分解 4/7)で確定した「route の exec 委譲面を
単一 `grant` から宣言 set `{route.grant} ∪ grants` へ拡張する
grant-set projection + viability 条件緩和 + usage record
split」を、declaration schema・宣言層制約・viability 分割・
内容審査除去・投影境界・record attribution・anchor 規約の
7 面で固定する。SPEC-034KTMfXYD5ZlGZCRb8Paj R1-R8・Acceptance
Criteria と DESIGN-034KUgF5QB3WuzFG6PmVvr の Verification
Notes が authoritative。

保つべき意図は 4 つ。(1) **byte-matched 不変条件**:
anchor `route.grant` は発火 gate・`expected_input` anchor・
record attribution anchor の単一のまま完全検査(存在 +
eligibility + pin + argv 照合)を維持し、`grants` は発火条件・
exec anchor のいずれにも影響しない。(2) **fail-safe fold**:
member 失敗は anchor 失敗と同じ `Viability::NoGrant` 一種へ
畳み(新規 viability 値なし)、宣言委譲面が壊れた route は発火
させず原 command 直接実行へ倒す。(3) **routing ≠
authorization(D2)**: `argv_shell_re` 除去で route 層から内容
審査を排除し、分類に残るのは caller 提供の structural 情報
(`unsafe_command` flag・executable 名)のみ — member argv 照合は
job 実行時の `check_exec_execution` へ遅延する(direct
`workflow run --grant` と同じ検査位置)。(4) **confused-deputy
境界**: `grants` 宣言権は trusted 層(builtin/user)限定で
project 層の `grants` presence は fail-loud route-load error、
投影後 policy は `{route.grant} ∪ grants ∪ {fixed}` に閉じ、
未宣言名の job は `resolve_exec_grant` で拒否される。

退役条件: `grants` 宣言の層制約・viability 分割構造・
`project_grants` 投影規則・record `executed` 契約が変わるまで
存続する。substitute mode(#167)で `expected_input`/`executed`
anchor が切り替わっても、byte-matched mode の本規約
(`grants` は発火条件に影響しない・member は宣言 bounds で動く)
は変わらない。

## Feature

routes.yaml route entry の optional `grants: [name,…]`(trusted
層限定)を `Route.grants` へ parse/dedupe し、`Route::grant_set()`
= `{route.grant} ∪ grants` の有効 set を viability・投影へ供給
する。viability は anchor 完全検査(不変)→ member 軽量検査
(存在 + `is_route_eligible` + `pin_exec_grant` のみ)→
`StdinUnsupported` → `Runnable` の順で評価し、unsafe-command
分岐は `input.unsafe_command` + `CWD_CHANGING_COMMANDS` のみを
見る(`argv_shell_re` 除去・定義削除)。route run 時は
`project_grants(&route.grant_set())` が `{set} ∪ {fixed}` を
policy に残し、`kind:"route"` record は `command`/`commandHash`
(原 command identity)を維持しつつ additive `executed:{grant,
argv}`(captured `details.{grant,argv}` の実測 echo)を必須で載せ
る。member を呼ぶ `command` job は `job_id != "exec"` で宣言し、
自身の grant 宣言 lifecycle/cwd/env/exitCodes で動く。

## Rules

- Rule: **declaration schema と dedupe**(spec R1 — `routes.rs`
  unit test)
  - `grants: [a, b]` の parse で `Route.grants` が member のみを
    宣言順で保持する。member 間重複(`[a, a]`)と `route.grant`
    自身の参照(`[grant, a]`)は冪等に除去され、`grants: []` は
    未指定と同値。
  - `grant`(単数)は required のまま — `grants` のみで `grant`
    を省略した entry は既存 `required_string` で route-load
    error。
  - malformed shape — `grants` が非 array・非 string member・
    空 string member — は `schema_error` →
    `InvalidRuntimePolicy` + `details.{path, reason:"schema",
    violation:"routes[i].grants: …"}` で file と対象 entry を
    名指しする route-load error。`grants` の typo(`grantss`
    等)は unknown key 拒否のまま(fail-loud 規約不変)。
  - member 名の検証は non-empty string のみ — 未存在・
    project-scope・fixed 名の宣言自体は legal で、失敗は
    viability `NoGrant` へ fold される(`grant` 単数と同じ
    「存在/eligibility は viability へ委譲」規約 — 第二の
    検証規則を作らない)。宣言数上限は課さない
    (`passthroughEnv` と同規約)。

- Rule: **宣言層制約 — trusted 層限定**(spec R1・ADR O7a —
  `routes.rs` unit + `route.rs` integration)
  - project 層(`<project_dir>/routes.yaml`)の route entry が
    `grants` key を含む場合、`validate_route` が member 形状
    検査より**先に** presence で `schema_error("{label}.
    grants: not declarable in the project layer")` →
    `InvalidRuntimePolicy` を返す fail-loud route-load error
    (untrusted 層の宣言は内容を評価しない fail-closed)。
    `load` の厳密経路(`route resolve`/`route run`)は error を
    そのまま返し、hook の `load_or_none` は `None` fold で
    route arm 不活性へ落ちる(既存 fail-open 規約)。
  - builtin/user 層の `grants` 宣言は受理される。project 層の
    `route.grant` 単数宣言は従来どおり有効(anchor は
    `expected_input` byte match で typed command へ拘束される
    ため審査迂回にならない)。`trusted: true` 宣言は判別に
    使わない(project 自己申告の shadowing gate — 判別子は宣言
    層 provenance)。

- Rule: **`Route::grant_set()` — 有効 set の単一 owner**
  (spec R1/R4 — `routes.rs` unit test)
  - `grant_set()` が `{route.grant} ∪ grants` の `BTreeSet` を
    返す — `grants: []` なら `{route.grant}` のみ。
    `{route.grant} ∪ grants` 規則を projection 呼出しと doc で
    二度書かない単一の供給点であること。
  - viability の member 検査は `route.grants`(member のみ —
    anchor を含まない)を走査し、`project_grants` には
    `grant_set()` の全 set(anchor 含む)を渡す — 使い分けが
    混同されないこと。

- Rule: **viability 分割 — anchor 完全検査不変 + member 軽量
  検査**(spec R3・ADR O2a/O3a — `resolve.rs` unit +
  `route.rs` integration)
  - anchor `route.grant` の既存検査(lookup + `is_route_eligible`
    → `pin_exec_grant` → `ExecToolOptions` 構築 →
    `check_exec_execution` argv 照合)は一切変更しない —
    anchor argv pattern と payload argv の不一致は引き続き
    `no-grant` で route 不発 → 原 command が直接実行される
    byte-matched fallback(既存 resolve/run test がそのまま
    pass)。
  - member ループは anchor 検査の**直後**(stdin-unsupported
    分岐より前)に置き、member ごとに存在 +
    `is_route_eligible`(scope==Trusted && !fixed)+
    `pin_exec_grant` のみを検査する — `check_exec_execution`・
    `verify_pinned_executable`・cwd prepare は呼ばない(pin
    解決結果は検査のみで破棄)。
  - member が未存在・`scope:"project"`・`fixed:true`・pin 不能
    (unresolvable executable・`execDenyPatterns` 衝突)のいずれ
    かで `Viability::NoGrant` — anchor 失敗と同一 fold で新規
    viability 値・enum 優先順位変更・`route resolve` 出力
    shape 変更はない。
  - member 全通過 + anchor 通過で `Runnable` — viability が
    `Runnable` を返した route は「全 member が trusted+!fixed
    かつ pin 可能」が保証され、`route run` の投影と hook eval
    で同じ宣言面が観測される。

- Rule: **`argv_shell_re` 除去と unsafe-command 分類の維持**
  (spec R5・ADR O4a — `resolve.rs` unit + `route.rs`
  integration + grep)
  - unsafe-command 分岐は `input.unsafe_command`(caller 報告
    flag)`|| CWD_CHANGING_COMMANDS.contains(executable 名)` の
    2 条件のみ — tokenize 済み argv の metachar 内容は分類
    しない(stale comment「grant 側は metachar を拒否するため
    安全性分類へ倒す」は正確な根拠へ書き換え)。
  - `argv_shell_re` の定義(`policy/exec.rs`)+ import
    (`resolve.rs`)を削除し、`grep -r argv_shell_re crates/`
    が 0 件。
  - 維持される structural gate: unquoted metachar による複数
    segment/pipe/redirect は `tokenize_shell` の
    `unsafe_segment`/`unsafe_token` で検出され、hook arm
    skip・`route run` prepare の `rejected("unsafe-command")`・
    `route resolve` の `unsafe_command` flag の 3 入口は不変。
    `CWD_CHANGING_COMMANDS`・`StdinUnsupported` 分類も不変。

- Rule: **metachar/NUL byte の到達点**(spec R5・O5b 不採用
  の帰結 — `resolve.rs`/`route.rs` + `route_run.rs`)
  - quoted metachar を含む argv(`git log -G 'a|b'`・
    `cargo test "a;rm"`)は `tokenize_shell` が unsafe flag を
    立てないため `unsafe-command` でなく `runnable` になり
    得る(既存の `a;rm` 期待値は `Runnable` へ反転 — grant
    argv pattern 照合まで到達する)。
  - `\0` 含 arg は `tokenize_shell` が拒否しないため
    `route resolve --argv '["…","a\0b"]'` は `runnable` まで
    到達し得て、`route run` → spawn
    (`std::process::Command`)の `InvalidInput` で exec job が
    fail する — bounded job error として観測されることを pin
    (viability 層への `[\0\r\n]` regex 再導入はしない)。

- Rule: **`project_grants` 投影境界**(spec R4・ADR O1a —
  `exec.rs` unit + `run.rs` prepare test + `proc_invocation`
  parity)
  - `project_grants(&mut self, names: &BTreeSet<String>)` は
    `names` の全件が存在かつ `is_route_eligible` の場合のみ
    `{names} ∪ {fixed grants}` を retain して true を返す —
    1 件でも未存在/ineligible なら false で policy を変更
    しない。`names.is_empty()` も false(空集合で retain する
    と全 `!fixed` grant が落ちる fail-safe)。
  - `route run` prepare は `project_grants(&route.grant_set())`
    を呼び、false なら `denied("no-grant")` `PolicyViolation`
    (viability 通過後の二重防御)。
  - 投影後 `supply.policy.exec.grants` は `{route.grant} ∪
    route.grants ∪ {fixed grants}` に限定され、undeclared 名を
    `input.name` に指定した member job は
    `resolve_exec_grant` で `PolicyViolation`("unknown or
    unauthorized command grant" + `details.grant`)— proc
    invocation parity の `exec: None` 再注入 + parity check
    構造は不変。
  - `execDenyPatterns` は member の `pin_exec_grant`
    (viability)と scope 構築 pin の両方に uniform に効く
    (R8 — deny される member は ineligible → `NoGrant`)。

- Rule: **record split — additive `executed`**(spec R6 —
  `usage_log.rs` unit + `tests/integration/usage_log.rs` +
  `route_run.rs` record case)
  - `kind:"route"` record に `executed:{grant, argv}` が
    `commandHash` 直後に必須で載る — `executed.grant` は
    captured `details.grant`、`executed.argv` は `details.argv`
    (string array)の実測 echo。byte-matched mode では
    `executed.grant == route.grant`・`executed.argv == 原
    exec_args` と一致する。
  - `command`/`commandHash` 不変(`payload.command` の
    `DefaultHasher` hex)・`schemaVersion: 1` 不変・
    `executed` は additive field — 既存 reader
    (`usage_report.rs` の `route_bucket`・`route-refetch`
    join)は field を無視して `cheaper usage` 集計が従来どおり
    動き、新旧 record が同一 `usage.jsonl` に混在してよい。
  - `executed.argv` は env-prefix 値を含まない(exec_args は
    env-prefix strip 済み — route log と同じ露出面)。
    record 書込み位置・best-effort 規約(失敗は `"usage-record"`
    postprocess failure + stderr `usage-log:` 行・exit/stdout
    不変)は不変。
  - `details.{grant,argv}` の欠落・非 string は
    `InternalError`(`exitCode` 欠落と同一 invariant —
    `expected_input` gate が `command` surface の details を
    保証済みのため infra 違反として fail、record に silent
    absence を残さない)。

- Rule: **anchor 規約と member job bounds**(spec R7・ADR
  O6a — `workflow_run`/`run.rs` test + `route_run.rs`
  integration)
  - `expected_input = {"name": route.grant, "args": exec_args}`
    不変 — byte-match gate(`job_id=="exec" && tool=="command"`
    のみ適用)・`reason:"exec-input-mismatch"` `PolicyViolation`
    ・文言は不変。
  - exec job の bounds(`deadline_ms`・workflow inputs の
    `grant`・routed-exec context — `exec_cwd`/passthrough
    `extra_env`/`accept_any_exit_code`/`log_tee`)は
    `route.grant` の宣言に従う。
  - member を呼ぶ `command` job は `job_id != "exec"` で宣言
    する — `job_id == "exec"` で member 名を指定すると
    `exec-input-mismatch` で失敗する(workflow authoring
    規約の pin)。`job_id != "exec"` の member job は
    expected_input gate を通らず `route_context: None` の通常
    `command::execute` 経路で、自身の grant 宣言
    lifecycle/cwd/env/`exitCodes`/`maxOutputBytes` と宣言 argv
    pattern 照合(`check_exec_execution` → `authorize_args`)
    で動く。
  - member argv が member grant の宣言 argv pattern に一致
    しない job は実行時 `check_exec_execution` で拒否されるが
    route 発火自体は止まらない(検査位置は direct
    `workflow run --grant` と同一)。
  - known semantics の pin: member が stdin 前提 executable を
    pin しても member 検査では止まらず member job は自身の
    `timeout` まで hang して fail(interactive screening は原
    command の `executable`/`exec_args` のみが対象)。member が
    より長い `timeout` を宣言しても workflow deadline は
    anchor `route.grant.timeout` 由来のみで silent clamp
    される。
  - record attribution anchor は `route` + `route.grant`
    (`commandHash` は原 shell identity)で不変 —
    `executed` は byte-matched 実測 echo であり、member job の
    実行は `kind:"route"` record に直接は載らない。

- Rule: **fail-open/load 規約と既存分類の回帰無し**(spec R8 —
  既存 route/hook/supply test)
  - `load_or_none` が route/policy load 失敗で `None` を維持し
    hook arm が不活性へ落ちる(grants schema error も同じく
    fold)。`supply::load` 厳密経路は error をそのまま返す。
  - `unsafe-command`(caller flag / cwd-changing)・
    `invalid-route`・`missing-target`・`stdin-unsupported` の
    既存分類と hook arm の unsafe segment skip が回帰無し
    (既存 test 群 pass)。
  - `route_cmd` 出力 shape・`payload` schema・`workflow_run`
    gate・`usage_report` 集計・`routed-exec.workflow.json` は
    無改訂で維持される。

- Rule: **full pipeline**(`mise run check` 全緑 +
  `node scripts/check-doc-ids.mjs` + `audit_docs` 0 error —
  design Verification Notes 末尾)

## Examples

- Example: `grants: [fmt, report]` を宣言した route で発火した
  routed workflow 内の `job_id != "exec"` の `command` job が
  `input.name = "fmt"`・`"report"` で実行でき、`input.name =
  "undeclared"` の job は `resolve_exec_grant` で
  `PolicyViolation` + `details.grant`(spec AC1・R4)。
- Example: project 層 routes.yaml の route entry が
  `grants: [a]` を含む → `load` が `InvalidRuntimePolicy`
  (violation `…grants: not declarable in the project layer`、
  file・entry 名指し)で file 全体を拒否。builtin/user 層の同じ
  宣言は受理。project 層の `grant:` 単数は受理(spec AC2)。
- Example: `grants: [missing]`(policy に無い名)→ viability
  `no-grant`。`grants: [fixed-task]`・`grants: [project-grant]`
  ・deny 衝突 member も同じく `no-grant` — `route resolve`
  出力は従来どおり理由を区別しない(spec AC4・R3(c))。
- Example: anchor `route.grant` の argv pattern と payload argv
  が不一致 → `no-grant` で route 不発・原 command 直接実行 —
  member が全て eligible でも anchor 照合は免除されない
  (byte-matched 回帰・spec AC3)。
- Example: `git log -G 'a|b'`(quoted metachar)→ `tokenize_shell`
  は unsafe flag を立てず、grant argv pattern 照合まで到達して
  `runnable`(除去前は `unsafe-command` へ誤分類 — spec AC6)。
  `grep -r argv_shell_re crates/` が 0 件(spec AC7)。
- Example: `route resolve --argv '["git","a\0b"]'` →
  `runnable` 到達可 → `route run` で spawn `InvalidInput` の
  exec job fail(`[\0\r\n]` regex は viability に残さない —
  O5b 不採用の bounded job error pin)。
- Example: `grants: [a]` の `a` が `pin_exec_grant` に通るが
  member argv pattern 外 args の job → 実行時
  `check_exec_execution` で `PolicyViolation`、route 発火は
  止まらない(spec AC5・O2a 遅延検査)。
- Example: `job_id == "exec"` の `command` job に member 名を
  指定 → `exec-input-mismatch` `PolicyViolation`。`job_id !=
  "exec"` の member job は自身 grant の `exitCodes` で非 0
  exit を判定し、`route_context`(exec_cwd/extra_env/log_tee)
  が付かない(spec AC9・R7)。
- Example: `kind:"route"` record が `executed:{grant: "git-ran",
  argv: ["status","-s"]}` を `commandHash` 直後に持ち、
  `commandHash`/`schemaVersion:1` 不変・`cheaper usage` の
  route bucket 集計が `executed` を無視して動く
  (spec AC8)。
- Example: `captured.details` に `grant`/`argv` が無い fixture
  → finish() が `InternalError`(record 未記録 — `exitCode`
  欠落と同じ invariant 扱い)。
- Example: `project_grants` に空 `BTreeSet`・`{a, missing}`・
  `{a, project-grant}` を渡す → いずれも false で policy 不変。
  `{route.grant, a}` 全 eligible → grants map が
  `{route.grant, a, fixed-*}` のみを残す(spec AC・R4)。

## Guarantees

- 実装者が design を再解釈しなくても、schema parse/dedupe・
  層制約 error・`grant_set` union・viability member loop・
  `argv_shell_re` 除去・投影境界・record `executed`・anchor
  規約の全項目が unit + integration + parity test で機械的に
  検証できる。
- anchor 完全検査・`expected_input` byte match・exit mirror・
  routed-exec convention が byte-matched mode で一切退化しない
  — `grants` は発火条件でも発火後 anchor でもない。
- member 失敗は全て `NoGrant` 一種へ畳まれ、新規 viability
  値・出力 shape・`route resolve` surface が増えない。
- 宣言権が trusted 層に限定され、project 層の `grants`
  presence が fail-loud に拒否される非対称境界が pin される
  (anchor `grant:` 単数は project で宣言可)。
- record `executed` が additive で `schemaVersion:1`・
  `commandHash` を侵さず、details 欠落が `InternalError` で
  観測可能になり silent absence を残さない。
- 内容審査が route 層から完全に排除され、unsafe-command 分類が
  caller 提供の structural 情報のみに依ることが grep + test
  で保証される。

## Non-goals

- substitute mode(#167)の anchor 切替・`substitute.grant`
  宣言・非 byte-matched 起動形(spec/design Out of Scope —
  record split はその前提整備だが substitute 自体は対象外)。
- `route resolve` 出力への `grants` echo・失敗 member 名の
  viability 分解(R11 出力 shape 不変・O3a の NoGrant fold
  確定 — 診断需要は Deferred)。
- workflow deadline の max-of-set 化・member job への
  routed-exec context 適用・member interactive screening
  (known semantics として pin — 変更しない)。
- `grants` 宣言数上限・pin cache 新設(viability + scope 構築
  の 2 回 pin 解決は現行どおり — YAGNI)。
- viability 事前検査と scope 構築 pin の間の PATH 変動 race の
  完全解消(`verify_pinned_executable` と同じ既存 race domain
  として受理 — design Risk)。
- member job 自体の usage-bearing record 新設(`kind:"route"`
  record は routed exec の計測 — member job の観測は別 issue)。
- `usage_report` code 変更(additive field は既存 reader が無視
  — 変更不要の pin)。
- 検証コマンド・実行結果の記録 — task の `## Verification` と
  impl record の責務。

## Risk

- member loop を anchor 検査の前に置く・member に
  `check_exec_execution` を掛けると、byte-matched 発火 gate の
  意味論(anchor argv 不一致 → 不発)が壊れ member 宣言の
  存在だけで route が不発/誤発する — 順序と検査深度の pin が
  必要。
- `argv_shell_re` の片方だけ除去(import 残し・定義残し)だと
  stale code/comment が残り内容審査の再導入を招く —
  `grep argv_shell_re` 0 件と stale comment 訂正の両方を pin。
- project 層 `grants` を受理・または形状検査より後に presence
  を見ると、O4c が塞いだ審査迂回が route 宣言経由で再発する
  (member argv は typed command と独立)— presence 先出し +
  `InvalidRuntimePolicy` の pin が防衛線。
- `executed` を optional・best-effort にすると新旧 record の
  attribution が混在し substitute mode の前提整備が不完全に
  なる — 必須挿入 + 欠落 `InternalError` の pin が必要。
- `Route` literal・`project_grant` 呼出しの churn 追随漏れは
  compiler が捕捉するが、test 内 literal(fixture)の stale 値は
  semantics を誤って pin する — design Churn 一覧の site 網羅
  と integration case の追加で検出する。
- `details.{grant,argv}` 欠落を silent default(`""`/`[]`)に
  すると record attribution が嘘を記録し、後で「なぜ空か」を
  追跡不能にする — `InternalError` pin が誤 default を防ぐ。
