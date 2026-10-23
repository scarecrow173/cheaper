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
ADR(ADR-034KTMnPFDF25x710LFSKY・proposed — O1a/O2a/O3a/O4a/O5a/
O6a/O7a 全採用)で方針は確定済みであり、discovery
(DISC-034KTzzHPIbmtD8j3iHBtg・resolved)が全接点を実測固定している。
前提となる unified exec grant model(#165 —
`ExecGrant`・`is_route_eligible`・`project_grant`・
`check_exec_execution`・`pin_exec_grant` の単一 seam)は実装済み。

本 design の責務は、「routes.yaml schema 追加の parse/dedupe/
宣言層制約の規約・viability 分割の具体形・`argv_shell_re` 除去
の面積・`project_grant` の set 版一般化・`expected_input` anchor
と member job の bounds 規約・record `executed` の provenance・
全 error 契約・churn inventory」を実装契約として固定すること。
引用する file:line は全て branch
`feature/issue-166-route-grant-set-projection` の現行 code で
実測済み。

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
- **O7a** — `grants` の宣言は trusted route 層(builtin/user)
  限定。project 層 routes.yaml の route entry が `grants` key
  を含む場合は fail-loud route-load error で拒否する
  (member argv は typed command と独立のため、untrusted 層へ
  member 宣言権を渡すと O4c が塞いだ審査迂回が route 宣言
  経由で再発する)。anchor `route.grant` 単数は従来どおり
  project 層でも宣言可。

## Scope

### In Scope

- routes.yaml route entry への optional `grants: [name,…]` 追加 —
  `Route` struct field・`KNOWN_KEYS`・`validate_route` parse +
  dedupe・`Route::grant_set()` helper
  (`crates/cheaper-cli/src/route/routes.rs`)。
- `grants` 宣言の層制約 — trusted 層(builtin/user)限定。
  project 層 route entry の `grants` key presence を
  `validate_route` で fail-loud `schema_error` へ倒す(§(a) —
  O7a)。
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
  `KNOWN_KEYS` は全層共通のため `"grants"` は全層で
  recognized となり、層制約は unknown-key ではなく別の
  明示判定で行う(下記)。
- **`grants` 宣言の層制約(O7a)** — 宣言できるのは trusted
  層(`RouteLayer::Builtin`/`RouteLayer::User`)のみ。project
  層 file は `load` の層 table(:95-111 — `:107-110` で
  `opts.project_dir.join("routes.yaml")` 起点相対 root)として
  `validate_routes_doc(&doc, layer, …)`(:120)へ渡され、
  `layer` は `:270` で `validate_route` の `:309` param へ
  流れる — 判別に新規 param は不要。検査位置は `grants`
  の parse 位置(`grant` の `required_string` :399 直後)と
  同一 site で、member 形状検査より先に presence で拒否する
  (untrusted 層の宣言は内容を評価せず fail-closed):

```rust
// `grants` は trusted 層限定(O7a)— member argv は typed
// command と独立のため project 層への member 宣言権委譲は
// O4c の審査迂回を route 宣言経由で再発させる。
// `trusted:true` は project 自己申告の shadowing gate で
// あり判別に使えない — 判別子は宣言層 provenance。
if layer == RouteLayer::Project && entry.contains_key("grants") {
    return Err(schema_error(
        file,
        &format!("{label}.grants: not declarable in the project layer"),
    ));
}
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

  error は fail-loud InvalidRequest-class の契約で、concrete
  code は既存 routes.yaml schema 規約(`schema_error` —
  `routes.rs:227-233` が `InvalidRuntimePolicy` を返す)と
  同じ `InvalidRuntimePolicy` — details `{path: <file>,
  reason: "schema", violation: "routes[i].grants: …"}` で
  file と対象 route entry を名指し。project partial
  schema が `command`/`security`/`route` key を拒否するのと
  同じ file-layer authoring 制約(`policy/load.rs:381-388`
  先例 — こちらも `InvalidRuntimePolicy`)であり、`route
  resolve`/`route run` の `supply::load` 厳密経路は error
  をそのまま返し、hook の `load_or_none` は `None` fold で
  route arm 不活性へ落ちる(既存 fail-open 規約、
  `supply.rs:138-152`)。anchor `route.grant` 単数の
  project 宣言は従来どおり受理 — byte-match が typed
  command へ拘束するため審査迂回にならない(R7、§(f))。

- member 名の検証は non-empty string のみ — `grant` 単数が
  名前 pattern を持たず存在/eligibility を viability へ委譲
  するのと同じ規約を member にも適用する(第二の検証規則を
  作らない — R2)。未存在/project-scope/fixed 名の宣言自体は
  legal で、失敗は viability `NoGrant` へ fold される(§(b))。
  (project 層では `grants` key 自体が load error なのでこの
  規約が適用されるのは builtin/user 層の宣言のみ。)
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
- `\0` 含 arg の扱い変化: `tokenize_shell` は `\0` を拒否
  しない(`routing_policy.rs:1241` で token 内容へ落ちる —
  `\r` は whitespace separator :1197、`\n` は segment
  separator :1202、`\0` は `is_token_unsafe`/`is_segment_unsafe`
  対象外 :1041/:1046)。`route resolve --argv` は JSON 入力で
  tokenize も経由しないため、`\0` 含 arg は除去後に
  `Runnable` まで到達し得て、`route run` → spawn
  (`std::process::Command`)の `InvalidInput` で exec job が
  fail する — 旧 regex 適用時の `UnsafeCommand`(原 command
  直接実行 fallback)とは fail 位置が変わるが、bounded job
  error であり内容審査の再導入はしない(O5b 不採用 —
  Verification Notes に test case)。

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
- 実装注記: member 失敗を `NoGrant` へ fold する前に
  `tracing::debug!` へ失敗 member 名を載せると、viability 値
  上は anchor 失敗と区別不能でも debug log で帰属できる
  (record・`route resolve` 出力 shape は不変 — O3a)。
- 新規失敗 domain: member pin は viability と job scope 構築で
  二度解かれる。scope 構築時の `pin_exec_grants`(`registry.rs:
  1234-1249`)は投影 policy の **全 `!fixed` grant** を pin する
  ため、viability 通過後(PATH 変動 race・policy 更新等)に
  member の pin が失敗すると、その command job の
  `create_worker_scope` が `PolicyViolation` で fail し、
  workflow が当該 member を呼ばなくても routed run 全体が
  hard error になる(captured=None → finish の record 書込み
  経路へ到達しない)。 viability 事前検査が保証するのは
  「解決時点で pin 可能」までで、scope 構築との gap は
  `verify_pinned_executable` が spawn 前に検出する既存機構
  と同じ race domain — 新設する pin cache は持たない(§Risks)。

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

(run.rs 側に `BTreeSet` の import は不要 — `route.grant_set()`
の戻り型がそのまま渡るため型推論で足りる。`exec.rs` は
`BTreeSet` を既に import 済み(:23)。)

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
  `kind:"route"` record の `kind`/`ok`/`originalTokens`/
  `returnedTokens`/`postprocessFailures` のみを読み
  (`route-refetch` record は `kind`/`bytesRead`)— additive
  field は無視されるため**code 変更不要**。`kind:"route"` が
  通常 run 集計へ混入しない既存分離も不変。

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
- **member job の authoring 規約 — `job_id != "exec"`** —
  member grant を呼ぶ `command` job は `job_id == "exec"` で
  宣言してはならない。byte-match gate は
  `job_id == "exec" && job.name == "command"` の job に
  `{name: route.grant, args: exec_args}` を要求するため、
  member 名を指定した exec-id job は
  `exec-input-mismatch` で失敗する(`workflow_run.rs:467-484`)。
  builtin `routed-exec.workflow.json` の `exec` job は anchor
  専用で、member を呼ぶ job は別 `job_id` を持つ通常 job
  として宣言する(routed-exec convention は exec job の
  unconditional 起動のみを要求し、追加 job の job_id には
  制約がない — ただし `exec` との衝突だけは上記 gate が
  防ぐ)。
- **member job は自身の grant 宣言 bounds で動く** —
  `job_id != "exec"` の `command` job は expected_input gate を
  通らず(`route_exec` filter が None)、`route_context: None` で
  通常の `command::execute` 経路を通る:
  `resolve_exec_grant(投影 policy, input.name, fixed=false)` →
  `check_exec_execution` → member grant の `argv` pattern 照合・
  `pin`・宣言 `cwd`/`env`/`timeout`/`killGrace`/`maxOutputBytes`/
  `exitCodes`(`tools/command/mod.rs:218-244`)。
- **interactive member は筛かれない(known semantics)** —
  `is_interactive`/`StdinUnsupported` の screening は原
  command の `executable`/`exec_args` のみを対象とする
  (`resolve.rs:352-355,:164-173`)。member grant が stdin 前提の
  executable(REPL・editor 等)を pin しても member 検査では
  止まらず、member job は自身の grant `timeout` まで hang
  してから fail する。route 層で member の interactive 分類を
  追加しない(operator 責務)。
- **member timeout は workflow deadline に clamp される
  (known semantics)** — workflow 全体の deadline は anchor
  grant の `timeout` のみから導出される(`run.rs:356-366` —
  `deadline_ms(route.grant.timeout)` →
  `policy.execution.timeout_ms`)。member がより長い
  `timeout` を宣言していても member job は workflow
  deadline に黙って clamp される(max-of-set 化は本 issue
  では行わない — Deferred)。
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
- **`grants` 宣言権は trusted 層限定(§(a) O7a — 層境界)**:
  anchor `route.grant` は `expected_input` byte match で
  typed command へ拘束されるため project 層でも宣言して
  よい(R7 の argv-match が保護する)。しかし member の
  argv は typed command と独立であり、project 層が
  `grants: [narrow-argv-trusted-grant]` を宣言できると、
  typed command では到達不能な trusted grant 群を投影へ
  密輸できてしまう — O4c が member 値側(project-scope
  grant の member 化)で塞いだ審査迂回が、宣言側では
  対称に閉じていない非対称 hole だった。`trusted: true`
  は project 自己申告で shadowing のみを gate する
  merge 制御(`routes.rs:134-146`)なので判別には使えず、
  判別子は宣言層 provenance(`layer == RouteLayer::
  Project` — load 時に判明)とする。member 名の選択権を
  untrusted 層へ渡さないことで、confused-deputy 半径の
  宣言境界が「route 宣言 + trusted 層」まで確定する。

### (g) signature 契約(変更後)

| function/struct | 変更後 signature | 備考 |
|-----------------|------------------|------|
| `Route` | `pub grants: Vec<String>` を `grant` 直後へ追加 | `routes.rs:37-55`。member 間重複・`route.grant` 参照は parse 時除去済み |
| `Route::grant_set` | `(&self) -> BTreeSet<String>` | `routes.rs` 新設 — `{route.grant} ∪ grants` の有効 set 規則の単一 owner |
| `KNOWN_KEYS` | `"grants"` を `"grant"` 直後へ追加 | `routes.rs:314-322` — unknown-key fail-loud 規約内(全層共通) |
| `validate_route` | signature 不変(`layer: RouteLayer` は :309 で既存) | project 層 entry の `grants` key presence を member 形状検査より先に `schema_error` へ倒す(O7a) |
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
| `route/routes.rs` | :399 直後 | `grants` parse(配列 + non-empty string + self/member dedupe)+ project 層 presence 拒否(:309 の `layer` param — 新規 param 不要) |
| `route/routes.rs` | :37-55,:451-460 | `Route.grants` field + literal 構築 |
| `route/routes.rs` | —(新設) | `Route::grant_set()` helper + unit test(grants parse・dedupe・型不整合・unknown key・project 層 `grants` presence 拒否 + builtin/user 受理) |
| `route/resolve.rs` | :10 | `argv_shell_re` import 除去 |
| `route/resolve.rs` | :282-291 | stale comment 訂正 + `argv_shell_re` 条件除去(caller flag + CWD_CHANGING のみ) |
| `route/resolve.rs` | :351 直後 | member 検査ループ(存在 + `is_route_eligible` + `pin_exec_grant` → `NoGrant` fold) |
| `route/resolve.rs` | :369-380,:488-494 | `Route` literal へ `grants` 追加 + `a;rm` 期待値 `Runnable` 化 + member viability case 追加 |
| `route/run.rs` | :121-123 | `project_grant(&route.grant)` → `project_grants(&route.grant_set())`(import 追加不要 — 戻り型が推論で足りる) |
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
| project 層 route entry が `grants` key を含む | `InvalidRuntimePolicy` — `schema_error(file,"{label}.grants: not declarable in the project layer")` | `routes.rs::validate_route`(`:309` の `layer` param) | O7a — fail-loud route-load error(details `{path,reason:"schema",violation}` で file と entry を名指し)。member 形状検査より先に presence で拒否 — project partial schema の section 除外と同型の file-layer authoring 制約(`policy/load.rs:381-388` 先例)。anchor `route.grant` 単数は project 層でも受理 |
| `grants` typo・unknown key | 同上(unknown key) | `routes.rs:323-327` | fail-loud 規約不変 |
| member 未存在 / `scope:"project"` / `fixed` | `Viability::NoGrant`(enum — error ではない)→ `route run` は `denied("no-grant")` `PolicyViolation` | `resolve.rs` §(b-2) ループ → `run.rs:93-97` | anchor 失敗と同一 fold — 新規 viability 値なし |
| member pin 不能(unresolvable executable・deny 衝突) | `Viability::NoGrant` → 同上 | `pin_exec_grant` | `execDenyPatterns` は member にも uniform 適用 |
| member pin の scope 構築時再失敗(viability 通過後の PATH 変動・policy 更新等) | `PolicyViolation` — `create_worker_scope` が Err → 当該 command job 失敗 → captured=None で route-run hard error | `registry.rs:1234-1249`(`pin_exec_grants` が投影 policy の全 `!fixed` grant を pin) | workflow が当該 member を呼ばなくても発生 — 新規失敗 domain(§(b-2)) |
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
builtin/user/project ─→ routes::load(層別 file — :95-111)
  └─ validate_route(layer 判別済み — :309):
       KNOWN_KEYS(+grants) → grant 確定
       → grants parse(member dedupe・self-ref 除去)
       → Route{grant, grants:[member…], …}
       ※ 形状不正・project 層の `grants` presence(O7a)は
         route-load error(fail-loud — InvalidRuntimePolicy)

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
  閉じ、宣言権は trusted 層(builtin/user)の routes.yaml に
  限定される(O7a)。未宣言 grant 名・member の宣言 argv
  外 args・deny 衝突 executable・project 層 `grants` 宣言は
  投影 policy・member 宣言・deny 適用点・load 時層制約で
  拒否される。
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
- **`grants` authoring の層境界(security)** — anchor
  `route.grant` は `expected_input` byte match で typed
  command へ拘束されるため project 層でも宣言してよい
  (member job が呼ぶのは anchor の argv pattern が許す
  executable のみ)。一方 `grants` member の job argv は
  typed command と独立(R7 の argv-match 保護が member
  には届かない)ため、member の宣言は exec 面を typed
  command の外へ広げる — trusted 層(builtin/user)限定と
  し、project 層の `grants` presence は load 時に
  fail-loud 拒否する(§(a) O7a)。`trusted: true` は project
  自己申告の shadowing gate(`routes.rs:134-146`)で判別
  には使えず、判別子は宣言層 provenance。anchor は
  project-authorable のまま、member 選択権のみ trusted
  authoring へ閉じる非対称が正しい境界。
- **member timeout の silent clamp** — workflow deadline
  は anchor grant `timeout` のみ由来(`run.rs:356-366`)
  のため、member が長い `timeout` を宣言しても
  workflow deadline で切られる。max-of-set は set 全体の
  timeout 宣言を発火 gate と独立に読む設計判断を要し、
  本 issue では現行 anchor 由来 semantics を維持する
  (Deferred — member の timeout 期待が発火側と乖離
  する運用が出た場合に再評価)。
- **record の機微値露出** — `executed.argv` は args 値を記録
  する(例: `git commit -m "…"` の message)。usage.jsonl
  への**初めての平文 arg 内容**であり(`commandHash` は
  一方向 hash)、route log が stdout/stderr を平文保持する
  既存露出面と同等だが、usage.jsonl は削除運用が異なるため
  parent-agent-integration・usage.md の記述で露出を明記
  する(env-prefix 値は prefixed argv が prepare で拒否
  されるため経由せず、arg 値自体の secret のみが残る)。
- **schema 拡大の churn 面積** — `Route` literal 3 箇所・
  `project_grant` 改名・`argv_shell_re` 削除の波及を下表で
  列挙済み。`route_cmd`/`supply`/`workflow_run`/`usage_report`/
  `payload`/`hook` 本体は不変 — 変更は route schema・viability・
  投影・record の 4 面に閉じる。

### Churn 一覧(実測 — signature/field 追随)

- **route schema** — `route/routes.rs`:`Route` struct(:37-55)、
  `KNOWN_KEYS`(:314-322)、`validate_route` grants parse(:399
  直後)+ project 層 presence 拒否(:309 `layer` param)、
  literal 構築(:451-460)、`Route::grant_set()` 新設、
  `:548` 以降の unit test( grants parse/dedupe/error case +
  project 層 `grants` 拒否・builtin/user 受理)。
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
  非適用)。**層制約** — `grants` を宣言できるのは
  builtin/user 層のみで、project routes.yaml に書くと
  route-load error になることを明記(anchor `grant:` 単数は
  project でも可)。**workflow authoring 規約** — member を
  呼ぶ command job は `job_id != "exec"` で宣言する
  (`job_id=="exec" && tool=="command"` は expected_input
  byte-match gate が `route.grant`+原 argv を要求し
  `exec-input-mismatch` で失敗する)。record 節(:349-357 付近)
  に `executed:{grant,argv}` を additive field として記述し、
  `executed.argv` が **平文の arg 値** を記録する露出を明記
  (usage.jsonl 初の cleartext arg — `commandHash` は一方向
  hash・env-prefix 値は prefixed argv 拒否で漏洩しないが
  arg 値自体の secret は残る)。
- `docs/guides/configuration.md` — unified exec 参照節
  (:215-217)の「route の `grant:` 参照は trusted+`!fixed` のみ」
  を `grants:` member も同 predicate へ拡張する記述へ更新
  (`grants` 宣言自体は builtin/user 層限定 — project routes.yaml
  は持てない旨も併記)。
- `docs/guides/command.md` — route grant 参照記述(:38-42)を
  `grants` member も同じ eligibility 規則へ従う旨へ更新。
- `docs/guides/usage.md` — additive-only 規約節(:296-303)に
  `kind:"route"` record の `executed` を additive field の例
  として追記し、`executed.argv` が平文 arg 値を保持する点を
  1 文で明記(`route` record field の canonical 記述は
  parent-agent-integration 側 — DRY)。
- `crates/cheaper-cli/resources/routes.yaml` — `grant:` 直下に
  `# grants: [name, …]  # optional — route.grant 以外の追加委譲 grant(builtin/user 層限定)` の shape comment を追加。
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  — refine 対象 spec として route field 一覧(R1 — :215-229
  付近)に `grants?`(trusted 層限定)・viability 記述(R12 —
  :299-306 付近)に member 検査・record field(R34 —
  :447-457 付近)に `executed` を追記する(content update のみ
  — 本 design からの `refines` relation は張らず、spec chain
  側の管理に委ねる)。

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
- **workflow deadline の max-of-set 化** — deadline は anchor
  grant `timeout` のみ由来(`run.rs:356-366`)。member が
  長い `timeout` を宣言する運用が出た場合に
  `deadline = max(anchor, members)` への変更を検討する
  (§e — 現行は silent clamp で known semantics)。
- **`grants` を project 層へ開くか** — O7b(受理+文書化)は
  O4c の審査迂回防止と非対称で不採用。project workflow が
  member を必要とする実需要が確立した場合、anchor 相当の
  argv-match 拘束を member にも掛ける別機構
  (declared-member-argv の workflow 側照合等)とセットで
  再評価する。

## Verification Notes

test 計画(spec Acceptance Criteria への対応):

- [ ] **schema/declaration** — `grants: [a,b]` parse で
  `Route.grants` が member のみ保持・member 間重複と
  `route.grant` self-ref が除去される・`grants:[]` ≡ 未指定・
  非 array/非 string/空 string で route-load error・unknown key
  拒否維持(`routes.rs` unit test。R1 AC)。
- [ ] **宣言層制約** — project 層 routes.yaml の `grants` key
  presence が `InvalidRuntimePolicy` route-load error で拒否
  され(file 名・entry 名指しの details)、builtin/user 層の
  `grants` 宣言は受理される。project 層の `route.grant` 単数
  宣言は従来どおり有効(`routes.rs` unit + `route.rs`
  integration。R1 AC — O7a)。
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
- [ ] **spawn 不可 byte の fail 位置** — `tokenize_shell` は
  `\0` を拒否しない(`routing_policy.rs:1241` で token 内容へ
  落ちる)ため、`route resolve --argv '["…","a\0b"]'` の
  `\0` 含 arg は `runnable` まで到達し得て、spawn
  (`std::process::Command`)で `InvalidInput` → exec job fail
  として観測されることを固定(viability 層へ `[\0\r\n]`
  regex を残さない O5b 不採用の帰結 — bounded job error。
  R5 関連)。
- [ ] **record split** — `kind:"route"` record に
  `executed:{grant,argv}` が必須で載り、`commandHash` 不変・
  `schemaVersion:1`・`cheaper usage` の route bucket 集計が
  additive field を無視して従来どおり動く(`usage_log.rs`
  unit + `route_run.rs` record case。R6 AC)。
- [ ] **anchor 規約** — `expected_input.name == route.grant`
  で `exec-input-mismatch` gate 通過・exec job が `route.grant`
  の lifecycle bounds・member job が自身 grant の宣言
  lifecycle/cwd/env/exitCodes で動く(`workflow_run`/`run.rs`
  test。R7 AC)。member を呼ぶ job が `job_id != "exec"` で
  動き、`job_id == "exec"` で member 名を指定すると
  `exec-input-mismatch` になる(authoring 規約の回帰)。
- [ ] **投影境界** — `project_grants` が `{route.grant} ∪
  grants ∪ {fixed}` を retain し、set 内 1 件 ineligible で
  false・空集合で false(`exec.rs` unit + `run.rs` prepare
  test。R4 AC)。undeclared grant 名の member job 参照が
  `resolve_exec_grant` で失敗(`route_run.rs`/`proc_invocation`
  parity)。
- [ ] **fail-open/load 規約** — `load_or_none` が route/policy
  load 失敗で `None` を維持・grants schema error が fail-loud
  route-load error(`routes.rs`/`supply` 経路。R8 AC)。
