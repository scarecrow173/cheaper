---
id: "DESIGN-034KUgF5QB3WuzFG6PmVvr"
type: "design"
status: "draft"
title: "route grant-set projection — grants 宣言・viability 条件緩和・record split"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/166"
  changes:
    added: []
    modified:
      - type: "file"
        path: "crates/cheaper-cli/src/route/routes.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/resolve.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/route/run.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-core/src/policy/exec.rs"
      - type: "file"
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/usage_log.rs"
      - type: "file"
        path: "crates/cheaper-tools/tests/proc_invocation.rs"
      - type: "file"
        path: "docs/guides/parent-agent-integration.md"
      - type: "file"
        path: "docs/guides/configuration.md"
      - type: "file"
        path: "docs/guides/command.md"
      - type: "file"
        path: "docs/guides/usage.md"
      - type: "file"
        path: "docs/specs/pretooluse-route-resolver-and-workflow-delegation.md"
      - type: "file"
        path: "docs/specs/route-grant-set-projection.md"
      - type: "file"
        path: "docs/adr/route-grant-set-projection.md"
      - type: "file"
        path: "docs/discovery/route-grant-set-projection-issue-166.md"
    deleted:
      - type: "item"
        path: "crates/cheaper-core/src/policy/exec.rs::argv_shell_re"
        note: "crates 内唯一の consumer(route/resolve.rs)が外れたため定義ごと削除(R5)"
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
    - "../designs/unified-exec-grant-model.md"
    - "../designs/opt-in-exec-deny-patterns.md"
    - "../designs/exec-contract-content-audit-removal.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../discovery/route-grant-set-projection-issue-166.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../adr/unified-exec-grant-model.md"
    - "../specs/unified-exec-grant-model.md"
    - "../impl/ir/unified-exec-grant-model-issue-165.md"
    - "../impl/ir/exec-contract-content-audit-removal-issue-163.md"
    - "../guides/parent-agent-integration.md"
    - "../guides/configuration.md"
    - "../guides/command.md"
    - "../guides/usage.md"
    - "../../crates/cheaper-cli/src/route/routes.rs"
    - "../../crates/cheaper-cli/src/route/resolve.rs"
    - "../../crates/cheaper-cli/src/route/run.rs"
    - "../../crates/cheaper-cli/src/route/supply.rs"
    - "../../crates/cheaper-cli/src/route/payload.rs"
    - "../../crates/cheaper-cli/src/route_cmd.rs"
    - "../../crates/cheaper-cli/src/hook/routing_policy.rs"
    - "../../crates/cheaper-cli/src/workflow_run.rs"
    - "../../crates/cheaper-cli/src/usage_log.rs"
    - "../../crates/cheaper-cli/src/usage_report.rs"
    - "../../crates/cheaper-core/src/policy/exec.rs"
    - "../../crates/cheaper-tools/src/tools/exec/mod.rs"
    - "../../crates/cheaper-tools/src/tools/command/mod.rs"
    - "../../crates/cheaper-cli/resources/routes.yaml"
    - "../../crates/cheaper-cli/resources/workflows/routed-exec.workflow.json"
  defers: []
  deferred-by: []
---

# route grant-set projection — grants 宣言・viability 条件緩和・record split

## Context

issue #166(#151 実装分解 — issue 4「route 側の整理」)は、byte-matched
route の exec 委譲面を単一 `grant` から宣言 set `{route.grant} ∪ grants`
へ拡張し、viability の条件を「anchor 完全検査 + member 軽量検査」へ
分割し、usage record に実測 `executed:{grant,argv}` を追加する。
spec(SPEC-034KTMfXYD5ZlGZCRb8Paj・proposed — R1-R8)と
ADR(ADR-034KTMnPFDF25x710LFSKY・proposed — O1a/O2a/O3a/O4a/O5a/O6a
全採用)で方針は確定済みであり、discovery
(DISC-034KTzzHPIbmtD8j3iHBtg・resolved)が全接点を実測固定している。
前提となる unified exec grant model(#165 —
`ExecGrant`・`is_route_eligible`・`project_grant`・
`check_exec_execution`・`pin_exec_grant` の単一 seam)は実装済み。

本 design の責務は、「routes.yaml schema 追加の parse/dedupe
規約・viability 分割の具体形・`argv_shell_re` 除去の面積・
`project_grant` の set 版一般化・`expected_input` anchor と member
job の bounds 規約・record `executed` の provenance・全 error
契約・churn inventory」を実装契約として固定すること。引用する
file:line は全て branch `feature/issue-166-route-grant-set-projection`
の現行 code で実測済み。

確定済み方針(ADR Decision Outcome):

- **O1a** — `grant` は required のまま発火 gate/anchor。optional
  `grants: [name,…]` を追加し、有効 set は
  `{route.grant} ∪ grants`(冪等 dedupe・順序意味なし・
  `grants: []` ≡ 未指定)。
- **O2a** — `route.grant` は現行完全検査(pin +
  `check_exec_execution` argv 照合)を維持。member は存在 +
  `is_route_eligible` + pin 解決のみ route 解決時に行い、argv
  照合は job 実行時へ遅延(direct `workflow run --grant` と同じ
  検査位置)。
- **O3a** — member eligibility は `scope == Trusted && !fixed`
  の `is_route_eligible` 単一 seam。member 失敗は
  `Viability::NoGrant` 一種へ fold(新規 viability 値なし)。
- **O4a** — viability の unsafe-command 分岐から
  `argv_shell_re` を除去し、定義(`policy/exec.rs:147-150`)を
  削除。`input.unsafe_command`・`CWD_CHANGING_COMMANDS`・
  `StdinUnsupported` は維持。
- **O5a** — `command`/`commandHash` を維持しつつ additive
  `executed:{grant,argv}` を追加(`schemaVersion: 1` 不変)。
- **O6a** — `expected_input.name = route.grant` anchor を維持。
  member を呼ぶ追加 job は自身の grant 宣言 bounds で動く。

## Scope

### In Scope

- routes.yaml route entry への optional `grants: [name,…]` 追加 —
  `Route` struct field・`KNOWN_KEYS`・`validate_route` parse +
  dedupe・`Route::grant_set()` helper
  (`crates/cheaper-cli/src/route/routes.rs`)。
- viability 分割 — unsafe-command 分岐の `argv_shell_re` 除去 +
  no-grant 段への member 検査ループ追加
  (`crates/cheaper-cli/src/route/resolve.rs`)。
- `argv_shell_re` 定義削除 + stale comment 訂正
  (`crates/cheaper-core/src/policy/exec.rs`・`resolve.rs:282-285`)。
- 投影一般化 — `project_grant` → `project_grants(set)` の
  `{route.grant} ∪ grants ∪ {fixed grants}` 拡張
  (`policy/exec.rs` + `route/run.rs:121`)。
- record split — `RouteRecordInput` への `executed` 入力追加 +
  `build_route_record` の `executed:{grant,argv}` 出力 +
  `route/run.rs` finish の captured `details.{grant,argv}` 抽出
  (`usage_log.rs` + `route/run.rs`)。
- anchor 規約の明文化 — `expected_input.name = route.grant`・
  member job の宣言 lifecycle/pin/cwd 適用・record attribution
  (code 変更なしの契約固定を含む)。
- 上記に伴う unit/integration test・routes.yaml shape 記述・
  docs 更新。

### Out of Scope

- substitute mode(#167)— `expected_input`/`executed` anchor の
  切替・`substitute.grant` 宣言・非 byte-matched 起動形は本 design
  の対象外(spec Deferred と同一)。
- `route resolve` 出力への `grants` echo — R11 出力 shape
  (`route_cmd.rs:105-128`)は変更しない。
- viability enum・優先順位の変更 — `NoGrant` への fold のみで
  新規値を追加しない。
- captured 前 exec job 失敗経路の record — record timing は
  現行どおり captured result 必須。
- member job への routed-exec context(`exec_cwd`/`extra_env`/
  `accept_any_exit_code`/`log_tee`)適用 — member job は通常
  command job として動く(§(e) で固定)。
- `grants` 宣言数上限 — `passthroughEnv` と同じく課さない
  (R1 確定値)。
- non-exec routing(target.kind 拡張)— 親 chain の deferred
  concern で本 design には関係しない。

## Components and Boundaries

### (a) route declaration — `crates/cheaper-cli/src/route/routes.rs`

`Route` struct(:37-55)に member set field を追加し、schema
検証・dedupe・有効 set 供給の全責務をこの file に閉じ込める。

```rust
pub struct Route {
    // …既存 field 不変…
    /// `command.grants` 参照名(route は grant を生成しない)。
    /// 発火 gate・expected_input anchor・record attribution の
    /// 単一 anchor — `grants` member は含めない。
    pub grant: String,
    /// `grants` — route.grant 以外の追加委譲 grant 名(宣言順)。
    /// parse 時に member 間重複と `route.grant` 自身の参照を
    /// 冪等に除去済み — `[]` ≡ 未指定。有効 set は
    /// `grant_set()` = {grant} ∪ grants。
    pub grants: Vec<String>,
    // …既存 field 不変…
}
```

- `KNOWN_KEYS`(:314-322)に `"grants"` を `"grant"` 直後へ追加 —
  fail-loud unknown-key 規約(:323-327)を維持し、`grants` の
  typo は unknown key として route-load error になる。
- parse 位置は `grant` の `required_string`(:399)直後 — grant
  を先に確定させてから member を処理し、self-reference 除去を
  parse 単位で完結させる:

```rust
// `grants: [name, …]` — route.grant 以外の追加委譲 grant 名。
// member 間重複・route.grant 自身の参照は冪等に除去(順序は
// 意味を持たない — 宣言順の先勝ちを保持)。
let mut grants = Vec::new();
match entry.get("grants") {
    None => {}
    Some(v) => {
        let items = v.as_array().ok_or_else(|| {
            schema_error(file, &format!("{label}.grants: not an array"))
        })?;
        for item in items {
            let name = item.as_str().filter(|s| !s.is_empty()).ok_or_else(|| {
                schema_error(
                    file,
                    &format!("{label}.grants: member must be a non-empty string"),
                )
            })?;
            if name != grant && !grants.iter().any(|g| g == name) {
                grants.push(name.to_string());
            }
        }
    }
}
```

- member 名の検証は non-empty string のみ — `grant` 単数が
  名前 pattern を持たず存在/eligibility を viability へ委譲
  するのと同じ規約を member にも適用する(第二の検証規則を
  作らない — R2)。未存在/project-scope/fixed 名の宣言自体は
  legal で、失敗は viability `NoGrant` へ fold される(§(b))。
- `Route` literal 構築(:451-460)へ `grants` を追加。
- 有効 set の供給は単一 helper へ集約(「{route.grant} ∪ grants」
  という同一規則を projection 呼出しと doc で二度書かない):

```rust
/// 有効投影 set — `{route.grant} ∪ grants`(anchor 必須を型で
/// 表さないため helper が規則の owner)。viability の member
/// 検査は `self.grants`(member のみ)を使い、`project_grants`
/// 投影には本 helper の全 set を渡す。
pub fn grant_set(&self) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    set.insert(self.grant.clone());
    set.extend(self.grants.iter().cloned());
    set
}
```

### (b) viability — `crates/cheaper-cli/src/route/resolve.rs`

`viability()`(:276-357)の unsafe-command 分岐と no-grant 段を
修正する。分岐順序(unsafe → invalid-route → missing-target →
no-grant → stdin-unsupported → runnable)・`Viability` enum
(:20-28)・`as_str`(:30-42)は不変。

**(b-1) unsafe-command 分岐(:287-291)の `argv_shell_re` 除去:**
caller 報告 flag と executable 名の 2 条件のみへ縮小:

```rust
// unsafe-command — caller 報告の unsafe/複数 segment 入力、
// または executable が cwd 変化 command。tokenize 済み argv の
// metachar 内容は分類しない — authorization は grant argv 契約が
// 担い routing 層は内容審査を持たない(D2)。
let name = out.executable.as_deref().unwrap_or_default();
if input.unsafe_command
    || CWD_CHANGING_COMMANDS.contains(&name.to_lowercase().as_str())
{
    return Viability::UnsafeCommand;
}
```

- import(:10)を `use cheaper_core::policy::exec::ResolvedExecPolicy;`
  へ縮小(`argv_shell_re` を落とす)。
- 従来の「grant 側は metachar を拒否するため安全性分類へ倒す」
  という :282-285 の comment は #163 で grant argv pattern が
  metachar を拒否しなくなった後の stale 記述であり、上記の
  正確な根拠へ書き換える。
- 効果: single-quoted metachar(`git log -G 'a|b'`)は
  `tokenize_shell` が `unsafe_token` を立てない
  (`routing_policy.rs:1159-1167` — quote 内は `;`/`|` を data
  として扱う)ため、これまで `UnsafeCommand` に倒れていた入力が
  grant argv pattern 照合まで到達し `Runnable` になり得る
  (spec AC — `a;rm` を含む unit test :488-494 の期待値は
  `Runnable` へ反転する)。
- 維持される安全性: unquoted metachar による複数 segment/pipe/
  redirect は `tokenize_shell` の `unsafe_segment`/`unsafe_token`
  で検出され、hook arm(`routing_policy.rs:1461-1463`)・
  `route run` prepare(`run.rs:66-73`)・`route resolve`
  (`route_cmd.rs:144-158` の `parse_command` → `unsafe_command`
  flag)の 3 入口全てで viability 到達前または flag 経由で止まる。
  本除去は「tokenize を通過した argv token の内容」を審査対象
  から外すだけで、入口の structural gate は不変。

**(b-2) no-grant 段(:308-351)の member 検査追加:** anchor
`route.grant` の既存完全検査(:315-351 — lookup+eligibility →
`pin_exec_grant` → `ExecToolOptions` 構築 →
`check_exec_execution` argv 照合)を**一切変更せず**、直後に
member ループを挿入する:

```rust
// no-grant (b) — 追加 set member: 存在 + is_route_eligible +
// pin 解決のみ(argv 照合は job 実行時の check_exec_execution へ
// 遅延 — direct `workflow run --grant` と同じ検査位置、O2a)。
// 失敗は anchor 失敗と同じ NoGrant へ fold する。
for member in &route.grants {
    let Some(member_grant) = input
        .exec_policy
        .policy
        .grants
        .get(member)
        .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
    else {
        return Viability::NoGrant;
    };
    if cheaper_tools::tools::exec::pin_exec_grant(
        member,
        member_grant,
        input.exec_policy.root,
        input.exec_policy.symlink_policy,
        input.exec_policy.denylist,
        input.exec_policy.exec_deny,
    )
    .is_err()
    {
        return Viability::NoGrant;
    }
}
```

- member には `check_exec_execution`・`verify_pinned_executable`・
  cwd prepare を呼ばない — pin 解決結果は検査のみで破棄する
  (job 実行時に scope pin 構築と `check_exec_execution` が
  再評価する)。member argv は宣言されていない(hook argv と
  member job argv は別物)ため、route 解決時点で照合する対象が
  そもそも存在しない。
- 順序は anchor 完全検査 → member ループ → stdin-unsupported
  (:352-355) — no-grant 段内部の検査のみで、enum 優先順位
  自体は不変。
- 投影位置の不変条件: viability が `Runnable` を返した route は
  「全 member が trusted+!fixed かつ pin 可能」であることが
  保証されるため、`route run` の投影(§(c))と hook eval で同じ
  宣言面が観測される。

### (c) projection — `crates/cheaper-core/src/policy/exec.rs`

`project_grant`(:118-130)を set 版へ一般化する。呼出し側は
`route/run.rs:121` のみのため、第 2 API を増やさず signature を
置き換える:

```rust
/// route run の grant set 投影(#166 — 単一版の一般化)。`names`
/// は `{route.grant} ∪ grants` の有効 set — 1 件でも不在・
/// route-ineligible(非 trusted/fixed)なら false で policy を
/// 変更しない。投影は `{names} ∪ {fixed grants}` を残す —
/// routed workflow から task を呼べる挙動と C9 confused-deputy
/// 防止(非 fixed の他 grant を隠す)を set でも維持する。
pub fn project_grants(&mut self, names: &BTreeSet<String>) -> bool {
    if names.is_empty()
        || !names.iter().all(|n| {
            matches!(self.grants.get(n), Some(g) if Self::is_route_eligible(g))
        })
    {
        return false;
    }
    self.grants.retain(|n, g| names.contains(n) || g.fixed);
    true
}
```

- `names` は caller が `route.grant_set()` で供給する有効 set —
  anchor(`route.grant`)を含まない集合を渡すと投影から anchor
  が落ちて exec job が fail するため、「有効 set の構築責務は
  `Route::grant_set()` に置く」ことを契約として固定する。
- `names.is_empty() → false` は到達不能入力への fail-safe
  (空集合で retain すると anchor ごと全 `!fixed` grant が
  落ちる)。
- eligibility を `project_grants` 内部でも再適用するのは
  viability との二重防御であり、TOCTOU ではなく「投影関数の
  自身の不変条件」として維持する(現行単一版と同じ構造)。
- `argv_shell_re`(:144-150)の定義 + comment を削除 —
  `route/resolve.rs` の import 除去後は crates 内 consumer が
  存在しない(spec AC — `grep argv_shell_re` が残らない)。
- `is_route_eligible`(:112-116)・`pin_exec_grant(s)`・
  `check_exec_execution`・`authorize_args` は不変 — member も
  anchor と同じ seam を通る。

`route/run.rs` の呼出し(:121-123)を set 版へ追随:

```rust
if !supply.policy.exec.project_grants(&route.grant_set()) {
    return Err(denied("no-grant"));
}
```

(`BTreeSet` の import が必要 — run.rs に `use std::collections::`
系 import を追加するか `route.grant_set()` の戻り型に合わせる。)

### (d) record split — `crates/cheaper-cli/src/usage_log.rs` + `route/run.rs`

`kind:"route"` record に実測 attribution を additive 追加する。
`schemaVersion` は `1` のまま(:372)、`command`/`commandHash`
は `payload.command` の `DefaultHasher` hex 規約で不変
(:364-365,:377-380)。

**入力契約 — `RouteRecordInput`(:332-357)に 2 field 追加**
(record 形状の owner は `build_route_record` に残し、caller は
typed 値だけを渡す — `{grant,argv}` object の組み立てを
run.rs へ漏らさない):

```rust
/// 実測 exec grant 名 — captured `details.grant`。
/// byte-matched mode では `route.grant` と一致する(O6a)。
pub executed_grant: String,
/// 実測 exec argv — captured `details.argv`。
/// byte-matched mode では原 `exec_args` と一致する。
pub executed_argv: Vec<String>,
```

**出力 — `build_route_record`(:362-404)** が `commandHash`
直後に挿入(record が書かれる全経路で必須 — R6):

```rust
base.insert(
    "executed".to_string(),
    json!({
        "grant": input.executed_grant.clone(),
        "argv": input.executed_argv.clone(),
    }),
);
```

**provenance — `route/run.rs` finish()(:440-)**:
`captured.details`(:465 — `command::execute` が
`ProcIdentity.details` へ `("grant", name)`・`("argv", args)`
を書く `tools/command/mod.rs:263-272`)から既存 `detail_i64`
accessor と同じ style で抽出し、`exitCode`(:467-474)と同じ
contract で欠落を `InternalError` にする:

```rust
// `details.{grant,argv}` は `job_id=="exec" && tool=="command"`
// の expected_input gate で `command` surface が保証済み —
// 欠落は infra invariant 違反として exit code と同じく
// InternalError に倒す(record に silent absence を残さない)。
let executed_grant = details
    .and_then(|d| d.get("grant"))
    .and_then(Value::as_str)
    .ok_or_else(|| {
        Box::new(CheaperError::new(
            ErrorCode::InternalError,
            "captured exec result has no executed grant",
        )) as BoxError
    })?;
let executed_argv = match details
    .and_then(|d| d.get("argv"))
    .and_then(Value::as_array)
{
    Some(items) if items.iter().all(Value::is_string) => items
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<String>>(),
    _ => {
        return Err(Box::new(CheaperError::new(
            ErrorCode::InternalError,
            "captured exec result has no executed argv",
        )) as BoxError)
    }
};
```

`RouteRecordInput` literal(:575-588)へ `executed_grant`・
`executed_argv` を追加するだけで、record 書込み位置
(`record_route` :574-594)・best-effort 規約(失敗時は
`"usage-record"` postprocess failure :595-597 + stderr `usage-log:`
行・exit/stdout 不変)は不変。

- `executed.argv` は原 shell argv であり env-prefix 値を含まない
  (`exec_args` は env-prefix strip 済み — route log と同じ
  露出面。payload.env は record field に含めない構造保証は不変)。
- record 消費側: `usage_report.rs` の `route_bucket`(:503-538)は
  `kind`/`ok`/`exitCode`/`originalTokens`/`returnedTokens`/
  `logRef`/`postprocessFailures`/`refetches` のみを読み、
  additive field は無視されるため**code 変更不要**。
  `kind:"route"` が通常 run 集計へ混入しない既存分離も不変。

### (e) anchor 規約 — `expected_input` と member job bounds

code 変更はないが、spec R7/O6a の契約を実装境界として固定する。

- **`expected_input` = `{"name": route.grant, "args": exec_args}`**
  — `RouteExecution::new`(:148-182)の `:176` が構築し、呼出し
  (:374-382)が `&prepared.route.grant`(:378) を渡す。
  `workflow_run.rs` の byte-match gate(:467-484)は
  `ctx.job_id == "exec" && ctx.job.name == "command"` の job のみ
  に適用され、mismatch は `PolicyViolation` +
  `details.reason = "exec-input-mismatch"`。この gate・文言・
  reason は**不変**。
- **exec job の bounds は `route.grant`** — grant timeout の
  `deadline_ms`(:356-364)・workflow inputs の `grant`(:385)・
  routed-exec context(`exec_cwd` = payload cwd・`extra_env` =
  passthrough capture・`accept_any_exit_code`・`log_tee` — `:521`
  の `route_exec.map(|r| r.context())` が exec job のみに付く)
  は anchor の宣言に従う。
- **member job は自身の grant 宣言 bounds で動く** —
  `job_id != "exec"` の `command` job は expected_input gate を
  通らず(`route_exec` filter が None)、`route_context: None` で
  通常の `command::execute` 経路を通る:
  `resolve_exec_grant(投影 policy, input.name, fixed=false)` →
  `check_exec_execution` → member grant の `argv` pattern 照合・
  `pin`・宣言 `cwd`/`env`/`timeout`/`killGrace`/`maxOutputBytes`/
  `exitCodes`(`tools/command/mod.rs:218-244`)。
- **member job に routed-exec convention は付かない** —
  `exec_cwd` override・passthrough `extra_env`・
  `accept_any_exit_code`・route log `log_tee` は exec job 専用。
  member job の非 0 exit は member grant の `exitCodes` 宣言で
  判定され、output は route log ではなく workflow の通常
  result/artifact 経路へ流れる。これは「member は route firing
  の一部ではなく、宣言済み capability を投影 policy の下で呼ぶ
  通常 job」という責務境界であり、本 design で固定する
  (route context を member job へ拡張する選択肢は採らない —
  env-prefix/passthrough の意味論が「元 shell command」へ
  結び付くため、member job へ横流しすると attribution が
  曖昧になる)。
- **record attribution anchor は `route` + `route.grant`** —
  record の `route` field・`commandHash`(原 shell identity)は
  不変で、`executed` は byte-matched 実測 echo。member job の
  実行は `kind:"route"` record に直接は載らない(route record
  は routed exec の計測 — member job 自体の usage-bearing
  record は新設しない)。

### (f) confused-deputy 境界

`project_grants` 後の `supply.policy.exec.grants` は
`{route.grant} ∪ route.grants ∪ {fixed grants}` に限定され、
invocation/scope 両経路の policy 解決はこの投影 policy のみを
見る(proc_invocation parity は `create_worker_scope` で再評価
される — `workflow_run.rs:503` の `exec: None` → inner exec
再注入 + parity check 構造は不変)。

- **undeclared 名の拒否**: 投影外の `!fixed` grant を
  `input.name` に指定した member job は
  `resolve_exec_grant` で `PolicyViolation`
  "unknown or unauthorized command grant" + `details.grant` で
  失敗する — 「trusted grant 全てが route 経由で呼べる」状態に
  ならないことが本設計の核心であり、有効 set が投影境界の
  唯一の宣言面である。
- **member argv の束縛**: member の `argv` pattern は member
  grant 自身の宣言値であり、job 実行時の
  `check_exec_execution` → `authorize_args` が強制する。したがって
  「route が呼び出せる exec 面」は「宣言 member 群の argv
  pattern の union + anchor の argv pattern + fixed task 群」に
  閉じる — member 名を宣言すれば任意 argv が通るわけではない。
- **deny の uniform 適用**: `execDenyPatterns` は member の
  `pin_exec_grant`(viability §(b-2) と scope pin 構築の両方)に
  同じく効き、deny される member は ineligible → `NoGrant`
  (R8)。`verify_pinned_executable` も member job の spawn 前に
  一律適用される。
- **宣言なしの capability 拡大は発生しない**: `grants` は
  routing の発火条件を変えない(match は `route.command` の
  byte-prefix のみ)。member 追加は「発火した routed workflow
  の内部から届く名の集合」を広げるだけであり、route 自体の
  firing surface は狭いまま。

### (g) signature 契約(変更後)

| function/struct | 変更後 signature | 備考 |
|-----------------|------------------|------|
| `Route` | `pub grants: Vec<String>` を `grant` 直後へ追加 | `routes.rs:37-55`。member 間重複・`route.grant` 参照は parse 時除去済み |
| `Route::grant_set` | `(&self) -> BTreeSet<String>` | `routes.rs` 新設 — `{route.grant} ∪ grants` の有効 set 規則の単一 owner |
| `KNOWN_KEYS` | `"grants"` を `"grant"` 直後へ追加 | `routes.rs:314-322` — unknown-key fail-loud 規約内 |
| `ResolvedExecPolicy::project_grants` | `(&mut self, names: &BTreeSet<String>) -> bool` | `exec.rs:118-130` — `project_grant(&mut self, name)` の置き換え(呼出し側は `route/run.rs:121` のみ)。全名 eligible かつ非空でのみ `{names} ∪ {fixed}` を retain |
| `RouteRecordInput` | `executed_grant: String`・`executed_argv: Vec<String>` 追加 | `usage_log.rs:332-357` — `executed` object 組み立ては `build_route_record` が担当 |
| `build_route_record` | signature 不変 | `:362-404` — `commandHash` 直後に `executed:{grant,argv}` を必須挿入 |

**除去される function**: `argv_shell_re`
(`policy/exec.rs:147-150`)— consumer ゼロ化後に定義ごと削除。

**変更しない signature**: `resolve::resolve`/`ResolveInput`/
`ExecPolicyView`/`Viability`/`RouteResolution`・
`route::supply::{load,load_or_none,exec_view}`・
`check_exec_execution`/`pin_exec_grant(s)`/`authorize_args`/
`is_route_eligible`・`record_route`/`append_usage_log`・
`build_rewrite_command`/`tokenize_shell`・`RouteExecution::new`/
`expected_input`/`workflow_run` gate — 全て不変。

### (h) call site 配線(全一覧)

| site | lines(現行) | 変更内容 |
|------|------------|----------|
| `route/routes.rs` | :314-322 | `KNOWN_KEYS` に `"grants"` 追加 |
| `route/routes.rs` | :399 直後 | `grants` parse(配列 + non-empty string + self/member dedupe) |
| `route/routes.rs` | :37-55,:451-460 | `Route.grants` field + literal 構築 |
| `route/routes.rs` | —(新設) | `Route::grant_set()` helper + unit test(grants parse・dedupe・型不整合・unknown key) |
| `route/resolve.rs` | :10 | `argv_shell_re` import 除去 |
| `route/resolve.rs` | :282-291 | stale comment 訂正 + `argv_shell_re` 条件除去(caller flag + CWD_CHANGING のみ) |
| `route/resolve.rs` | :351 直後 | member 検査ループ(存在 + `is_route_eligible` + `pin_exec_grant` → `NoGrant` fold) |
| `route/resolve.rs` | :369-380,:488-494 | `Route` literal へ `grants` 追加 + `a;rm` 期待値 `Runnable` 化 + member viability case 追加 |
| `route/run.rs` | :121-123 | `project_grant(&route.grant)` → `project_grants(&route.grant_set())` + `BTreeSet` import |
| `route/run.rs` | :465 以降 | `details.{grant,argv}` 抽出(欠落・非 string は `InternalError`)|
| `route/run.rs` | :575-588 | `RouteRecordInput` literal へ `executed_grant`/`executed_argv` |
| `route/run.rs` | :701-719 | `prepare_projects_grants_to_single_route_grant` test を set 投影名・内容へ更新 |
| `policy/exec.rs` | :118-130 | `project_grant` → `project_grants(BTreeSet)` |
| `policy/exec.rs` | :144-150 | `argv_shell_re` 定義削除 |
| `policy/exec.rs` | :1039-1062 | `route_eligibility_and_project_grant` test を set 呼出し + multi-member case へ更新 |
| `usage_log.rs` | :332-357,:362-404 | `RouteRecordInput` 2 field + `executed` 挿入 + test(`executed` shape・`commandHash` 不変・`schemaVersion:1`) |
| `hook/routing_policy.rs` | :2080-2091 | `Route` literal へ `grants: Vec::new()`(本体は resolve 呼出し構造不変) |
| `route_cmd.rs` | :105-128 | **不変** — R11 出力 shape(grants echo なし)|
| `route/supply.rs` | :29-53,:70-152 | **不変** — `RouteSupply`/`exec_view`/`load`/`load_or_none` 構造変更なし |
| `workflow_run.rs` | :467-484,:521 | **不変** — gate・`route_context` 配線・`exec: None` parity 構造 |
| `usage_report.rs` | :503-538 | **不変** — additive field 無視・route bucket 分離維持 |
| `route/payload.rs` | 全体 | **不変** — payload schema に `grants` は載らない |
| `hook.rs`/`hook_cmd.rs` | route 入力供給経路 | **不変** — `load_or_none` fail-open 規約・`RouteEvalInput` 構築 |
| `resources/routes.yaml` | :1-8 | `grants` field の shape comment 追加(宣言面の記述のみ — builtin route は member 非宣言) |
| `tests/integration/route.rs` | :8-19,:50-118 | `grants` 宣言 fixture + member 未存在/fixed → `no-grant`・quoted metachar → `runnable` case |
| `tests/integration/route_run.rs` | 各 fixture | member job が declared set 内を呼べる・undeclared 名で失敗する routed workflow case |
| `tests/integration/usage_log.rs` | :299-338 | `route_record_shape` に `executed:{grant,argv}` assertion |
| `tests/proc_invocation.rs` | :706-720 | `project_grant` 相当 comment を set 投影へ更新 + multi-grant 投影 case(投影外 `input.name` 拒否は既存構造の回帰) |

### (i) error 契約

| 条件 | code / 分類 | 境界 | 備考 |
|------|------------|------|------|
| `grants` が非 array / 非 string member / 空 string | `InvalidRuntimePolicy` — `schema_error(file,"{label}.grants: …")` | `routes.rs::validate_route` | route-load error — `load`/`load_or_none` 経路全てで fail-loud(hook は `load_or_none` で route arm 不活性へ fold — 既存規約) |
| `grants` typo・unknown key | 同上(unknown key) | `routes.rs:323-327` | fail-loud 規約不変 |
| member 未存在 / `scope:"project"` / `fixed` | `Viability::NoGrant`(enum — error ではない)→ `route run` は `denied("no-grant")` `PolicyViolation` | `resolve.rs` §(b-2) ループ → `run.rs:93-97` | anchor 失敗と同一 fold — 新規 viability 値なし |
| member pin 不能(unresolvable executable・deny 衝突) | `Viability::NoGrant` → 同上 | `pin_exec_grant` | `execDenyPatterns` は member にも uniform 適用 |
| member argv 不一致 | `PolicyViolation` — `check_exec_execution`/`authorize_args` の grant_violation | member job 実行時(`command::execute`) | route 発火は止めない — 宣言 bound 外の job argv が失敗する位置は direct `workflow run --grant` と同一 |
| `route.grant` 未存在/ineligible/pin 不能/argv mismatch | `Viability::NoGrant` | 既存 :315-351(不変) | byte-matched 不発 → 原 command 直接実行 |
| `project_grants` false(ineligible/不在名混入) | `PolicyViolation` — `denied("no-grant")` | `run.rs:121-123` | viability 通過後の二重防御 |
| `expected_input` mismatch | `PolicyViolation` — `reason:"exec-input-mismatch"` | `workflow_run.rs:471-484`(不変) | `job_id=="exec" && tool=="command"` のみ |
| `details.{grant,argv}` 欠落・非 string | `InternalError` | `run.rs` finish() | `exitCode` 欠落(:467-474)と同一 invariant 扱い |
| usage record 書込み失敗 | —(best-effort)stderr `usage-log:` + `postprocessFailures` に `usage-record` | `usage_log.rs:428-446` + `run.rs:595-597` | exit code・stdout 不変(現行規約) |
| `load_or_none` の route/policy load 失敗 | `None` — route arm 不活性 | `route/supply.rs:138-152`(不変) | hook fail-open 維持 |
| `route run` payload の unsafe segment/metachar | `PolicyViolation` — `rejected("unsafe-command")` | `run.rs:66-73`(不変) | tokenize-level structural gate は残る |

## Data and Control Flow

```
routes.yaml layers          declaration                    viability / projection / record
───────────────────────────────────────────────────────────────────────────────────────────
builtin/user/project ─→ routes::load
  └─ validate_route: KNOWN_KEYS(+grants) → grant 確定
       → grants parse(member dedupe・self-ref 除去)
       → Route{grant, grants:[member…], …}
       ※ 形状不正は route-load error(fail-loud)

hook eval / route resolve / route run prepare
  └─ supply::load → ExecPolicyView → resolve::resolve
       ├─ unsafe-command: input.unsafe_command || CWD_CHANGING
       │    (argv_shell_re 除去 — tokenize 済み argv は審査しない)
       ├─ invalid-route / missing-target
       ├─ no-grant(a): route.grant — is_route_eligible
       │    → pin_exec_grant → check_exec_execution(exec_args)
       │    … 完全検査(anchor・byte-matched 発火 gate)
       ├─ no-grant(b): route.grants member — 存在 + eligible
       │    → pin_exec_grant のみ(argv 照合は job 実行時へ遅延)
       │    … 失敗は NoGrant へ fold
       └─ stdin-unsupported → runnable

route run prepare(Runnable のみ)
  └─ project_grants(route.grant_set())
       → supply.policy.exec.grants = {route.grant} ∪ grants ∪ {fixed}
       → WorkflowContext(projected policy) → execute_routed

routed workflow 内
  ├─ exec job(job_id=="exec" && tool=="command")
  │    expected_input {name:route.grant, args:exec_args} byte match
  │    → route_context(exec_cwd/extra_env/accept_any_exit/log_tee)
  │    → command::execute → details.{grant,argv} captured
  └─ member job(input.name = member grant)
       expected_input gate 非適用・route_context なし
       → resolve_exec_grant(投影 policy — undeclared 名は拒否)
       → check_exec_execution(member argv pattern 照合)
       → member grant 宣言 lifecycle/cwd/env/exitCodes

finish(): captured.details → executed{grant,argv}
  └─ RouteRecordInput{…,executed_grant,executed_argv}
       → build_route_record → commandHash(不変) + executed(必須)
       → record_route → <userConfigDir>/usage.jsonl
       (best-effort — 失敗は usage-record postprocess failure)
```

## Non-Functional Constraints

- **routing ≠ authorization(D2)** — viability は
  `is_route_eligible` 判定と `pin_exec_grant`/
  `check_exec_execution` への委譲のみを行い、route 層に
  内容審査(metachar 走査・引数意味の判定)を持たない。
  `argv_shell_re` 除去はこの不変条件の回復であり、分類に残る
  のは caller 提供の structural 情報(`unsafe_command` flag・
  executable 名)のみ。
- **fail-safe fold** — member 失敗・anchor 失敗のいずれも
  `Viability::NoGrant` で route 不発 = 原 command が直接実行
  される。宣言委譲面が壊れた route を発火させて中途半端な
  投影 policy で job 失敗を起こさない。
- **byte-matched 不変条件** — anchor の完全検査(:315-351)・
  `expected_input` byte match・exit mirror・routed-exec
  convention(`outputs.execResult` 全 binding・unconditional
  `exec` job)は一切変更しない。`grants` は発火条件・発火後
  exec anchor のいずれにも影響しない。
- **declared capability 境界** — confused-deputy 半径は
  `project_grants` 後の policy = 宣言 argv union + fixed に
  閉じる。未宣言 grant 名・member の宣言 argv 外 args・
  deny 衝突 executable は投影 policy・member 宣言・deny
  適用点の 3 層で拒否される。
- **fail-open 保存** — hook 経路の `load_or_none`・hook arm の
  unsafe segment skip(`routing_policy.rs:1461-1463`)・route
  resolve の `unsafe_command` flag 規約は不変。grants parse
  error は load error として fail-open に fold される(hook
  の従来評価へ落ちる)。
- **cost 上界** — member 数 × PATH/pin 解決コストが viability
  へ乗るが、宣言数は operator 責務(`passthroughEnv` と同じ
  no-cap 規約 — R1)。pin 解決は viability 時・scope 構築時の
  2 回行われる(現行 anchor と同じ二重評価で追加機構なし)。
- **record additive 規約** — `executed` は additive field で
  `schemaVersion: 1` 不変。既存 reader(`route_bucket`/
  `route-refetch` join)は field を無視し、新旧 record が同一
  `usage.jsonl` に混在してよい(usage.md の additive-only
  契約に一致)。`executed.argv` は原 argv の実測 echo で
  route log が既に raw output を保持する露出面と同等 — ただし
  env-prefix 値は strip 済みで record/payload 双方に残らない。

## Risks and Trade-offs

- **NoGrant fold の診断性** — anchor 失敗と member 失敗が
  `route resolve` の `no-grant` で区別不能になる。spec が採る
  単一 fold は fail-safe を優先する確定値であり、失敗 member
  名を出す `grants` echo は診断需要が確立した時点で再検討
  (spec Deferred と同一)。
- **member argv の遅延評価** — member job argv は route 解決
  時に未確定のため検査不可能であり、job 実行時失敗は routed
  workflow 内の job failure として観測される。direct
  `workflow run --grant` と同じ検査位置(O2a)で、route 層へ
  事前検査を二重化しない。
- **投影後 policy と pin の 2 回評価** — member pin は
  viability(§b-2)と scope 構築(`pin_exec_grants`)で二度解く。
  PATH 変動 race は `verify_pinned_executable` が spawn 前に
  検出する既存機構でカバーされ、pin cache を新設しない
  (KISS — anchor が同じ構造で運用済み)。
- **member job の context 差分** — routed-exec context
  (cwd/env/exitCodes/log tee)が member job に付かないため、
  member の cwd は自身の `grant.cwd` 宣言・exit code は
  `exitCodes` 宣言で判定される。route 側 `passthroughEnv` の
  env 値も exec job 専用で member job へは流れない —
  「routed exec の context は原 shell command の再現」という
  境界を越えて member job へ横流ししない決定(§e)。
- **record の機微値露出** — `executed.argv` は args 値を記録
  する(例: `git commit -m "…"` の message)。route log が
  stdout/stderr を平文保持する既存露出面と同等だが、
  usage.jsonl は削除運用が異なるため parent-agent-integration
  の記述で露出を明記する。
- **schema 拡大の churn 面積** — `Route` literal 3 箇所・
  `project_grant` 改名・`argv_shell_re` 削除の波及を下表で
  列挙済み。`route_cmd`/`supply`/`workflow_run`/`usage_report`/
  `payload`/`hook` 本体は不変 — 変更は route schema・viability・
  投影・record の 4 面に閉じる。

### Churn 一覧(実測 — signature/field 追随)

- **route schema** — `route/routes.rs`:`Route` struct(:37-55)、
  `KNOWN_KEYS`(:314-322)、`validate_route` grants parse(:399
  直後)、literal 構築(:451-460)、`Route::grant_set()` 新設、
  `:548` 以降の unit test( grants parse/dedupe/error case)。
- **viability/resolve** — `route/resolve.rs`:import(:10)、
  comment+条件(:282-291)、member ループ(:351 直後)、
  `Route` literal(:369-380)、`:488-494` の metachar 期待値・
  member viability 新規 case。`hook/routing_policy.rs`:`Route`
  literal(:2080-2091)のみ(評価本体 :1451-1486 は不変)。
- **core policy** — `policy/exec.rs`:`project_grant` →
  `project_grants`(:118-130)、`argv_shell_re` 削除(:144-150)、
  test(:1039-1062)の set 呼出し + member case。
- **run/projection** — `route/run.rs`:`project_grants` 呼出し
  (:121)+ `BTreeSet` import、`details.{grant,argv}` 抽出
  (:465 以降)、`RouteRecordInput` literal(:575-588)、
  `prepare_projects_grants…` test(:701-719)・`finish` test の
  `completed_exec_result`(:899-912 — `details` に grant/argv
  既存)・`executed` assertion 追加。
- **record** — `usage_log.rs`:`RouteRecordInput`(:332-357)、
  `build_route_record`(:362-404)。`tests/integration/
  usage_log.rs`:`route_record_shape`(:299-338)に `executed`
  shape + `commandHash`/`schemaVersion` 不変の assertion。
- **投影 parity** — `tests/proc_invocation.rs`(:706-720)の
  `project_grant` 相当 comment・projected `grants` JSON shape を
  set 投影へ追随。
- **fixture/docs** — `resources/routes.yaml`(:1-8 — shape
  comment)、`tests/integration/route.rs`(:8-19 fixture +
  :50-118 output/viability case)、`tests/integration/route_run.rs`
  (member job 起動・undeclared 拒否・record `executed` case)、
  docs 4 file + refine 対象 spec(§Docs 更新契約)。
- **code 不変のまま回帰対象** — `route/supply.rs`・
  `route_cmd.rs`・`route/payload.rs`・`route/logs.rs`・
  `workflow_run.rs`(:467-484,:521)・`usage_report.rs`
  (:503-538)・`hook.rs`/`hook_cmd.rs`・
  `resources/workflows/routed-exec.workflow.json` — 無改訂で
  既存挙動が維持されることを確認する churn 境界。

## Docs 更新契約

- `docs/guides/parent-agent-integration.md` — routes.yaml 宣言例
  (:248-259 付近)と field 説明に `grants` を追加(意味:
  `{route.grant} ∪ grants` 有効 set・dedupe・`trusted && !fixed`
  eligibility・member argv は自身の grant pattern で bound・
  member job は宣言 lifecycle/cwd/env/exitCodes・route context
  非適用)。record 節(:349-357 付近)に `executed:{grant,argv}`
  を additive field として記述し、`executed.argv` が args 値を
  含む露出を明記。
- `docs/guides/configuration.md` — unified exec 参照節
  (:215-217)の「route の `grant:` 参照は trusted+`!fixed` のみ」
  を `grants:` member も同 predicate へ拡張する記述へ更新。
- `docs/guides/command.md` — route grant 参照記述(:38-42)を
  `grants` member も同じ eligibility 規則へ従う旨へ更新。
- `docs/guides/usage.md` — additive-only 規約節(:296-303)に
  `kind:"route"` record の `executed` を additive field の例
  として 1 行追記(`route` record field の canonical 記述は
  parent-agent-integration 側 — DRY)。
- `crates/cheaper-cli/resources/routes.yaml` — `grant:` 直下に
  `# grants: [name, …]  # optional — route.grant 以外の追加委譲 grant` の shape comment を追加。
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  — refine 対象 spec として route field 一覧(R1 — :215-229
  付近)に `grants?`・viability 記述(R12 — :299-306 付近)に
  member 検査・record field(R34 — :447-457 付近)に `executed`
  を追記する(content update のみ — 本 design からの `refines`
  relation は張らず、spec chain 側の管理に委ねる)。

## Deferred Design Concerns

- **substitute mode(#167)の anchor 切替** — `expected_input`・
  `executed` の意味は byte-matched mode で固定。`substitute.grant`
  導入時に R6/R7 anchor 規約を再評価する(spec Deferred と同一
  — 先回りした substitute 前提 field は入れない)。
- **`route resolve` 出力の `grants` echo** — R11 出力 shape
  (:105-128)は不変。member 失敗と anchor 失敗を分ける診断
  field は viability 区別の需要が確立した時点で検討。
- **member 失敗の viability 分解** — 「anchor 失敗 vs member
  失敗」を区別する新規 viability 値は採らない(ADR O3a)。
  分解するには enum 拡張と downstream 影響(hook・route_cmd・
  全 test)を伴い、発火可否の意味論が変わるため spec で却下。
- **captured 前 exec 失敗の record 欠落** — record は captured
  result 必須の現行 timing を維持。exec 不発経路の観測盲區は
  route 観測性の別 issue。
- **`grants` 宣言数上限** — 現行 no-cap(R1 確定)。pin 解決
  コストが問題化した場合のみ cap 導入を検討(各 member は
  viability + scope 構築で 2 回 PATH 解決される)。
- **member job への routed-exec context 適用** — `exec_cwd`・
  passthrough env・`accept_any_exit_code`・`log_tee` を member
  job へ広げる需要が出た場合は原 command 再現の意味論との
  関係を含めて別途設計(§e で「適用しない」を固定)。

## Verification Notes

test 計画(spec Acceptance Criteria への対応):

- [ ] **schema/declaration** — `grants: [a,b]` parse で
  `Route.grants` が member のみ保持・member 間重複と
  `route.grant` self-ref が除去される・`grants:[]` ≡ 未指定・
  非 array/非 string/空 string で route-load error・unknown key
  拒否維持(`routes.rs` unit test。R1 AC)。
- [ ] **member 起動** — `grants:[a,b]` 宣言 route の routed
  workflow で `input.name` が `a`・`b` の command job が実行
  でき、未宣言名 job が `PolicyViolation` で拒否される
  (`route_run.rs` integration。R4 AC)。
- [ ] **byte-matched 回帰** — anchor argv pattern と payload
  argv 不一致で `no-grant` → route 不発(既存 resolve/run
  test がそのまま pass。R3(a) AC)。
- [ ] **member ineligible fold** — `grants` member が未存在・
  project-scope・fixed・pin 不能の各場合に viability が
  `no-grant`(新規値なし・`resolve.rs`/`route.rs` test。
  R2/R3(b,c) AC)。
- [ ] **member argv 遅延検査** — member の宣言 argv pattern
  外 args の job が実行時 `check_exec_execution` で拒否
  され、route 発火自体は止まらない(`route_run.rs`
  integration。R3(b) AC)。
- [ ] **`argv_shell_re` 除去** — quoted metachar を含む argv
  (例: `git log -G 'a|b'`・`cargo test "a;rm"`)が
  `unsafe-command` でなく `runnable` になり、
  `grep -r argv_shell_re crates/` が 0 件(`resolve.rs`
  unit + `route.rs` integration + grep。R5 AC)。
- [ ] **unsafe-command 分類維持** — caller flag・
  `CWD_CHANGING_COMMANDS`・`stdin-unsupported` 分類・hook arm
  の unsafe segment skip・`route run` の `rejected("unsafe-command")`
  が全て回帰無し(既存 hook/route/run test。R5/R8 AC)。
- [ ] **record split** — `kind:"route"` record に
  `executed:{grant,argv}` が必須で載り、`commandHash` 不変・
  `schemaVersion:1`・`cheaper usage` の route bucket 集計が
  additive field を無視して従来どおり動く(`usage_log.rs`
  unit + `route_run.rs` record case。R6 AC)。
- [ ] **anchor 規約** — `expected_input.name == route.grant`
  で `exec-input-mismatch` gate 通過・exec job が `route.grant`
  の lifecycle bounds・member job が自身 grant の宣言
  lifecycle/cwd/env/exitCodes で動く(`workflow_run`/`run.rs`
  test。R7 AC)。
- [ ] **投影境界** — `project_grants` が `{route.grant} ∪
  grants ∪ {fixed}` を retain し、set 内 1 件 ineligible で
  false・空集合で false(`exec.rs` unit + `run.rs` prepare
  test。R4 AC)。undeclared grant 名の member job 参照が
  `resolve_exec_grant` で失敗(`route_run.rs`/`proc_invocation`
  parity)。
- [ ] **fail-open/load 規約** — `load_or_none` が route/policy
  load 失敗で `None` を維持・grants schema error が fail-loud
  route-load error(`routes.rs`/`supply` 経路。R8 AC)。
