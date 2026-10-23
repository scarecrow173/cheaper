---
id: "DISC-034KTzzHPIbmtD8j3iHBtg"
type: "discovery"
status: "resolved"
title: "route grant-set 投影 + viability 整理 + record split（#166）"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
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
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
  verifies: []
  verified-by: []
  references:
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../discovery/permission-model-responsibility-redesign.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../guides/usage.md"
  defers: []
  deferred-by: []
---

# route grant-set 投影 + viability 整理 + record split（issue #166）

## Exploration Goal（seed — briefing 前の最小記録）

issue #166（#151 実装分解 4/7）。route/hook 側で内容審査に依存
する分岐を整理し、非 exec routing deferred sketch
（ADR-034KtTzOJGqmzmBW3SwGp5）の採用要素 — grant-set 投影・
条件付き viability 弱化・record split — を実装する。
routing ≠ authorization（D2）は維持。

### Scope（issue 本文）

- **viability の内容審査呼出し整理** — viability は authorization
  （grant/allowlist 検査 = `check_command_execution` 委譲）を
  呼ぶが内容審査は呼ばない構造へ。
- **`project_grant` → route 宣言 `grants:[name,…]` set 投影へ
  一般化**（統一 grant モデル上 — #165 済）。grant 参照は
  trusted-scoped のみ解決（M3）。confused-deputy 半径は
  declared grant argv union に明示的に閉じる。
- **viability の条件付き弱化** — byte-matched mode では原
  argv = 実行 argv（D11）のため `check_command_execution`
  argv precheck（`resolve.rs:304-335`）は**維持**（無条件に
  弱めると argv-mismatch が「route 不発→原 command 直接実行」
  から「route 発火→exec job 失敗」へ後退）。弱めるのは set
  内の非 exec grant のみ — 存在・pin 解決可能性のみ
  viability で検査し argv 照合は job 実行時へ委譲（direct
  `workflow run --grant` と同一）。
- **`kind:"route"` usage record を `{command, executed}` に
  拡張** — 実行されなかった command を attribution にしない。
  route 帰属（route × firing gate の認可 grant）は
  `route.grant` に anchor。`executed` は実 exec argv（byte-
  matched では原 argv）を記録。
- **grant anchor 規約**: `expected_input.name` は exec job が
  実際に走らせる grant（byte-matched/grant-set mode では
  `route.grant`）。deadline/timeout 等の lifecycle bound は
  `expected_input` が指す実行 grant の宣言値（`command::execute`
  が named grant の `timeout`/`killGrace`/`exitCodes` を読む
  既存規約）。set 内の非 exec grant は呼ぶ追加 job で自身の
  宣言 bound を使う。

### Acceptance Criteria（issue 本文）

- viability が authorization を呼ぶが内容審査を呼ばない構造
- `grants:[name,…]` set 投影が trusted-scoped のみ解決
- byte-matched の argv precheck 維持・argv-mismatch で route
  不発→原 command 直接実行 fallback 保持
- set 内非 exec grant が存在+pin 解決可能性のみ viability 検査
- `kind:"route"` record が `{command, executed}` で attribution
  が `route.grant` に anchor

### seed の前提

- deps: #163・#165 両方 merged（PR #172・#174）。
- 非 exec routing 全体は #167（substitute-exec mode）の scope —
  本 issue は grant-set 投影・viability 整理・record split に
  限定。
- `argv_shell_re` viability 参照（`resolve.rs`）は #163 で
  「viability 側の審査依存整理は本 issue の scope」と記録済み —
  briefing で現行呼出しを実測。
- 引用 file:line は issue 本文のもの（main `c45872c` 時点 —
  #165 後の drift を briefing で実測修正する）。

## Key Issues and Assumptions

briefing 実測（本ブランチ HEAD 時点）で確定した論点:

1. **`argv_shell_re` は viability の unsafe-command 分岐に残存**
   — `crates/cheaper-core/src/policy/exec.rs:144-150` で定義され、
   crates 内 consumer は `crates/cheaper-cli/src/route/resolve.rs:10`
   の import と `:287-291` の unsafe-command 判定のみ。#163 で
   exec/policy 層の内容審査は除去済み（authorization 側の
   `authorize_args` は metachar を拒否しない）ため、viability が
   authorization より厳しい **divergence** が残っている。
   quoted metachar を含む正当 argv（`git log -G 'a|b'` —
   tokenize 済み token 内の `|` は data）で route が
   `unsafe-command` と誤分類され不発する偽陰性がある。
   分岐脇の comment（`resolve.rs:282-285`「grant 側は
   metachar を拒否するため安全性分類へ倒す」）は #163 後に
   stale 化している。

2. **`route.grant` の argv precheck は維持（byte-matched）** —
   `resolve.rs:308-351` の no-grant 分岐（grant lookup +
   `is_route_eligible` filter `:315-323` → `pin_exec_grant`
   `:324-333` → `ExecToolOptions` 構築 `:334-348` →
   `check_exec_execution(&options, &route.grant, &out.exec_args)`
   `:349-351` → 失敗で `Viability::NoGrant`）は、command tool
   spawn 前検査と同一の shared predicate seam による
   authorization 委譲であり内容審査ではない。argv mismatch →
   `no-grant` → route 不発 → 原 command 直接実行という既存
   fallback を保持するため無条件に弱めない。

3. **`project_grant` は `{route.grant} ∪ {fixed}` のみ投影** —
   `policy/exec.rs:123-130`（`is_route_eligible` 判定 + `retain`
   で named + fixed のみ残す）。呼出しは `route/run.rs:121`
   （viability 通過後・`WorkflowContext::from_policy` 前 —
   `:367`）。routed workflow の tool scope / command tool
   name 解決は投影後 policy のみを見るため、set 投影はここを
   `{route.grant} ∪ grants ∪ {fixed}` へ広げるだけで
   confused-deputy 半径が宣言 argv union に閉じる。

4. **route schema に `grants` field は無い** —
   `Route` struct（`route/routes.rs:37-55`）は `name`/
   `command`/`target_path`/`grant`/`token_threshold`/
   `passthrough_env`/`trusted`/`layer` のみ。`KNOWN_KEYS`
   （`:314-322`）にも `grants` は無く、未知 key は fail-loud
   reject。`grants` は新規 optional field として追加する
   （`grant` 単数は required のまま — 発火 gate・exec anchor・
   record attribution anchor）。

5. **route usage record は `commandHash` のみを保持** —
   `usage_log.rs:362-404` の `build_route_record` が
   `{schemaVersion:1, ts, root, kind:"route", route, commandHash,
   originalTokens, returnedTokens, reductionRatio, worker,
   durationMs{exec,analyze,total}, logRef, refetches,
   postprocessFailures, exitCode, ok, suppressed?}` を構築し、
   `record_route`（`:428-446`）経由で `route/run.rs:574-594`
   から書かれる。`command` 入力は `payload.command` 生文字列で
   hash（DefaultHasher hex16）のみ記録 — env-prefix 値を
   含み得るため raw 記録しない（`tests/integration/usage_log.rs`
   が `!contains("SECRET_TOKEN")` で防衛）。`executed` field は
   存在しない。consumer は `usage_report.rs::route_bucket`
   （`:503-538`）で `commandHash`/`command` を読まない —
   `executed` は additive で済む。

6. **`executed` の実測ソースは captured exec details** —
   `command::execute` の `CanonicalToolResult.details` が
   `{grant, argv, state, exitCode, signal, durationMs, stdout,
   stderr, …}` を持ち（`tools/command/mod.rs` details 構築・
   `ProcIdentity` が `grant`/`argv` を供給）、`route_exec.
   captured()` から取り出せる。byte-matched では
   `executed.grant = route.grant`・`executed.argv = 原 exec_args`。
   exec argv は env-prefix を持てない（prepare が argv の
   env-prefix を拒否 — `run.rs` argv-mismatch/precheck 経路）
   ので raw argv 記録で env 値漏洩は起きない。

7. **`expected_input` anchor は `route.grant` で不変** —
   `route/run.rs:176`（`RouteExecution::new` 内
   `json!({"name": route.grant, "args": exec_args})`）→
   `workflow_run.rs:467-484` が `job_id=="exec" && tool=="command"`
   の job に限り byte match gate（不一致で `exec-input-mismatch`
   POLICY_VIOLATION）。lifecycle bounds は `command::execute`
   が named grant の `timeout`/`killGrace`/`maxOutputBytes`/
   `exitCodes` を読む（`tools/command/mod.rs:231-244`）。
   route-level deadline は `route.grant` の timeout 由来
   （`run.rs:356-364`）。set 内非 exec grant を呼ぶ追加 job は
   `input.name` に自身の grant 名を指定し、自身の宣言
   bound・`pin.cwd`・`passthrough_env` が適用される。

8. **fail-open / deny 一様性の保持確認** — hook route 解決は
   `load_or_none`（`route/supply.rs:138-152`）で失敗時は
   route arm 不活性のまま通常 path。`execDenyPatterns` は
   `pin_exec_grant`（declared/resolved 適用点）と registry
   scope pin（`cheaper-tools/src/registry.rs:1234`）で set
   member にも一様に効く。`Viability` enum
   （`Runnable`/`NoGrant`/`MissingTarget`/`InvalidRoute`/
   `UnsafeCommand`/`StdinUnsupported`）と `route resolve`
   出力 shape は変更しない。

## Side-by-side Inventory（実測 file:line — 本ブランチ HEAD 時点）

### viability / resolve

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `argv_shell_re` 定義 | `crates/cheaper-core/src/policy/exec.rs:144-150` | NUL・CR・LF・shell metachar・`$(` に match |
| `argv_shell_re` import/使用 | `crates/cheaper-cli/src/route/resolve.rs:10`, `:287-291` | unsafe-command 分岐の一項 |
| stale comment | `resolve.rs:282-285` | 「grant 側は metachar を拒否するため」— #163 後に不成立 |
| no-grant 分岐（grant 解決+precheck） | `resolve.rs:308-351` | `is_route_eligible` filter + `pin_exec_grant` + `check_exec_execution` |
| `Viability` enum | `resolve.rs` | `Runnable`/`NoGrant`/`MissingTarget`/`InvalidRoute`/`UnsafeCommand`/`StdinUnsupported` |
| `ExecPolicyView` | `resolve.rs:67-80` | hook eval へ resolved policy を運ぶ view |

### policy / 投影

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `is_route_eligible` | `crates/cheaper-core/src/policy/exec.rs` | `scope == Trusted && !fixed`（単一 seam） |
| `project_grant` | `crates/cheaper-core/src/policy/exec.rs:123-130` | `{route.grant} ∪ {fixed}` を残す |
| 投影呼出し | `crates/cheaper-cli/src/route/run.rs:121` | viability 通過後・denied("no-grant") で fail |
| `pin_exec_grant` / `check_exec_execution` | `crates/cheaper-tools/src/tools/exec/mod.rs:163-211`, `:275-312` | shared predicate seam（deny 適用点込み） |

### route schema / run

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `Route` struct | `crates/cheaper-cli/src/route/routes.rs:37-55` | `grants` field 無し |
| `KNOWN_KEYS` / `validate_route` | `routes.rs:314-322`, `:307-460` | 未知 key fail-loud・`grants` 未登録 |
| `expected_input` 構築 | `route/run.rs:176` | `{"name": route.grant, "args": exec_args}` |
| `expected_input` gate | `crates/cheaper-cli/src/workflow_run.rs:467-484` | `job_id=="exec" && tool=="command"` に限定 |
| deadline 構築 | `route/run.rs:356-364` | `route.grant` の timeout 由来 |
| `load_or_none` | `route/supply.rs:138-152` | fail-open 維持 |
| routed-exec convention | `resources/workflows/routed-exec.workflow.json` | unconditional `exec` job + `outputs.execResult` 全 binding |

### usage record

| 項目 | 位置 | 現行 |
| --- | --- | --- |
| `build_route_record` | `crates/cheaper-cli/src/usage_log.rs:362-404` | `commandHash` 保持・`executed` 無し |
| `record_route` 呼出し | `route/run.rs:574-594` | captured exec result 必須経路 |
| record consumer | `crates/cheaper-cli/src/usage_report.rs:503-538`（`route_bucket`） | `commandHash`/`command` 非参照 → additive 可能 |
| lifecycle bounds 読出し | `crates/cheaper-tools/src/tools/command/mod.rs:231-244` | named grant の `timeout`/`killGrace`/`maxOutputBytes`/`exitCodes` |
| details shape | `tools/command/mod.rs`（`CanonicalToolResult.details`） | `{grant, argv, state, exitCode, …}` |

## Line Reference Drift 訂正表（issue 本文 → 実測）

| issue 引用 | 実測 | 差分 |
| --- | --- | --- |
| `check_command_execution` argv precheck `resolve.rs:304-335` | `resolve.rs:308-351`（`check_exec_execution` 呼出し `:349-351`） | #165 で関数名 `check_exec_execution` へ rename + 行 drift |
| `argv_shell_re` viability 参照（`resolve.rs`） | `resolve.rs:10`（import）+ `:287-291`（使用）+ 定義 `policy/exec.rs:144-150` | #163 で定義が `process.rs` → `policy/exec.rs` へ移動済み |
| `project_grant`（`command.rs:74` 相当） | `policy/exec.rs:123-130` | #165 で `ResolvedExecPolicy::project_grant` へ移動 |
| `command::execute` lifecycle 読出し | `tools/command/mod.rs:231-244` | `grant.timeout`/`kill_grace`/`exit_codes` を named grant から読む規約は維持 |

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| O1a `grant`（required）+ `grants?:[]` 追加 field | `grant` の発火 gate/anchor 意味不変・既存 routes.yaml 全件有効・意味差が field 名に載る | field 2 個 + dedupe 規約明記が要る | **採用** |
| O1b `grants:[]` 単一 field へ置換 | set を 1 か所で宣言 | 全既存 routes.yaml 破壊的変更・anchor 表現の新規規約要 | 不採用 |
| O1c `grant` を string-or-array union | field 数不変 | union 型の schema/serde/文書コスト大・anchor 意味規約は依然必要 | 不採用 |
| O2a `route.grant` 完全検査・member は存在+pin のみ | byte-matched 保持・member argv は job 実行時の正当位置で検査・member 欠陥は原 command 直接実行へ倒れる | member argv の実行保証は job 実行まで分からない | **採用** |
| O2b 全 member 完全検査 | 見た目厳格 | member 呼出 job の argv は route 解決時点で未確定 — 照合対象が無く検査として成立しない | 不採用 |
| O2c member 無検査 | 実装最小 | 「route 発火→exec 失敗→原 command 不実行」回帰を許す | 不採用 |
| O3a member 失敗は `NoGrant` fold | wire/enum/hook 挙動不変・fail-safe | 「route.grant 失敗 vs member 失敗」の区別が値だけでは付かない | **採用** |
| O3b 新規 viability 値 | 失敗理由を構造的に表現 | wire 契約変更・hook 挙動は同一（不発）で利益薄い | 不採用 |
| O3c `InvalidRoute` fold | 「宣言欠陥」印象 | grant 解決失敗は policy 状態であり route malformed と混同 → 誤診誘導 | 不採用 |
| O4a `is_route_eligible` 一律 | 同一 seam・第二規則なし・trust boundary 一致 | fixed を書くと fail（fail-loud として意図的） | **採用** |
| O4b fixed を vacuous member 許可 | 書いても失敗しない | 無意味な宣言を容認・fail-loud schema 規約に反する | 不採用 |
| O4c project-scope 許可 | project route が自身の grant を委譲可 | project layer が trusted 審査を迂回 → D2 trust boundary 侵害 | 不採用 |
| O5a `argv_shell_re` 分岐+定義除去 | 内容審査の route 層残存が消え D2 回復・偽陰性解消 | unsafe-command 分類が狭くなる（metachar argv で `runnable` — 実 exec は authorization が別途効く） | **採用** |
| O5b `[\0\r\n]` 縮小版 | 定義を小さく残せる | tokenize 段階で既に拒否 → 到達不能コード残留 | 不採用 |
| O5c 現行維持 | 変更なし | D2 違反と偽陰性が残る（本 issue の動機） | 不採用 |
| O6a `command`/`commandHash` 据え置き + `executed:{grant,argv}` | 「要求」と「実測」の attribution 分離・additive・substitute mode 対応可 | record 行増・旧 reader は `executed` 非観測 | **採用** |
| O6b `executed` を argv 文字列 join | 最小表現 | round-trip 不能・exec 契約 `{name,args}` と表現 drift | 不採用 |
| O6c `executed` を `{grant, argvHash}` | `commandHash` と対称・小さい | exec argv は env-prefix を持てず hash の保護意味薄・`route-logs/` に raw 既存で観測性だけ失う | 不採用 |
| O6d `command` を raw 文字列化 | 原 command が読める | env-prefix 値の secret 混入経路を復活・既存テスト防衛対象 | 不採用 |

## Tentative Conclusions and Hypotheses

- **決定**: O1a + O2a + O3a + O4a + O5a + O6a（採用 root は
  `docs/adr/route-grant-set-projection.md`）。
- route schema は `grant`（required・発火 gate/anchor）+
  `grants?:[name,…]`（追加委譲面）の 2 field。有効 set =
  `{route.grant} ∪ grants`。dedupe 冪等・順序意味なし・
  `grants:[]` ≡ 未指定・上限なし（`passthroughEnv` と同規約）。
- viability は `route.grant` のみ byte-matched 完全検査
  （`resolve.rs:308-351` 現行経路維持）。追加 member は
  存在 + `is_route_eligible` + `pin_exec_grant` のみ。
  member 失敗は `Viability::NoGrant` へ fold。
- `project_grant` は `{route.grant} ∪ grants ∪ {fixed}` を
  残す set 版へ一般化（呼出し位置 `route/run.rs:121` 不変）。
  全名に `is_route_eligible` 適用・1 件でも ineligible で
  `denied("no-grant")`。
- `argv_shell_re` は unsafe-command 分岐から除去 + 定義削除
  （crates 内 consumer は viability のみ — `grep` 実測）。
  `input.unsafe_command`・`CWD_CHANGING_COMMANDS`・
  `StdinUnsupported` は維持。stale comment（`resolve.rs:282-285`）
  を同時に修正。
- route record は `command`/`commandHash` 据え置き +
  `executed:{grant, argv}` additive 追加（captured
  `details.{grant,argv}` 由来・schemaVersion 1 維持・
  `route` field の attribution anchor 不変）。
- `expected_input.name = route.grant`・`job_id=="exec" &&
  tool=="command"` gate・deadline/lifecycle bound の named
  grant 読出しは不変。set member を呼ぶ追加 job は自身の
  grant の宣言 bound が適用される。

## Open Questions

- `route resolve` 出力へ `grants` echo を載せるか —
  本 briefing では「載せない（R11 shape 維持）」とするが、
  「どの member が `no-grant` 原因か」の診断需要が出た時点で
  再検討（spec Deferred Design Concerns に記録）。
- captured 前の exec job 失敗経路で route record が書かれない
  現行仕様を維持するか — 本 issue では維持（観測性は別
  issue）。`executed` は「record が書かれる全経路で付与」
  規約とした。
- `grants` 宣言数上限 — `passthroughEnv` 等と同じく上限なしと
  したが、member 分の pin 解決コスト（PATH 走査）が route
  解決時に乗るため、実装レビューで上限規約を入れる判断余地
  あり（spec R1 に「上限を課さない」明記済み・要 review）。
- `executed` の raw argv 記録 — env-prefix は持てないが args
  自体に機微値（`--password` 等）を埋める parent への note を
  guide/spec に残すかどうか（`route-logs/` に raw 出力が既存
  のため新規露出ではない判断・要 review）。

## Promotion Candidates

- [x] spec-doc needed: yes → `docs/specs/route-grant-set-projection.md`
      （SPEC-034KTMfXYD5ZlGZCRb8Paj — set 投影・viability 弱化・
      record split・anchor 規約の intent）
- [x] adr-doc needed: yes → `docs/adr/route-grant-set-projection.md`
      （ADR-034KTMnPFDF25x710LFSKY — O1-O6 の決定記録）

## Sources and Relations

- issue #166・親 issue #151 chain・非 exec routing deferred ADR
- #163 IR（argv_shell_re viability 参照の記録）・#165 IR（統一
  grant モデル・`is_route_eligible`・`project_grant` 投影）
- refine 対象 spec:
  `../specs/pretooluse-route-resolver-and-workflow-delegation.md`
  （grant semantics・record 契約・routed-exec convention）
- record additive 規約: `../guides/usage.md`（schemaVersion 1・
  additive-only の既存契約）
