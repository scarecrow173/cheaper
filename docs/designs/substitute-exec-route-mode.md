---
id: "DESIGN-034L0RxGVVTOqtaUUThowk"
type: "design"
status: "draft"
title: "substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・実行 anchor 切替"
created: "2026-10-23"
updated: "2026-10-23"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/167"
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
        path: "crates/cheaper-cli/src/hook/routing_policy.rs"
      - type: "file"
        path: "crates/cheaper-cli/resources/routes.yaml"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route.rs"
      - type: "file"
        path: "crates/cheaper-cli/tests/integration/route_run.rs"
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
        path: "docs/specs/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/adr/substitute-exec-route-mode.md"
      - type: "file"
        path: "docs/discovery/substitute-exec-route-mode-issue-167.md"
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements: []
  implemented-by: []
  depends-on:
    - "./route-grant-set-projection.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../designs/route-grant-set-projection.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../designs/pretooluse-route-resolver-and-delegated-workflow-execution.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "../specs/substitute-exec-route-mode.md"
    - "../adr/substitute-exec-route-mode.md"
    - "../discovery/substitute-exec-route-mode-issue-167.md"
    - "../specs/route-grant-set-projection.md"
    - "../adr/route-grant-set-projection.md"
    - "../designs/route-grant-set-projection.md"
    - "../specs/pretooluse-route-resolver-and-workflow-delegation.md"
    - "../designs/permission-model-responsibility-redesign.md"
    - "../specs/permission-model-responsibility-redesign.md"
    - "../adr/defer-non-exec-routing-to-permission-model-redesign.md"
    - "../adr/enforce-declared-contracts-over-content-audit.md"
    - "../specs/unified-exec-grant-model.md"
    - "../test-specs/route-resolution.md"
    - "../test-specs/route-run-delegated-exec.md"
    - "../test-specs/route-usage-records.md"
    - "../test-specs/route-grant-set-projection.md"
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

# substitute-exec route mode — substitute:{grant,argv} 宣言・viability retarget・実行 anchor 切替

## Context

issue #167(#151 実装分解 — issue 5)は、route が typed command を
実行せず宣言済みの substitute ペア `{substitute.grant,
substitute.argv}` を実行する **substitute-exec mode** を導入する
(非 exec routing deferred sketch・ADR-034KtTzOJGqmzmBW3SwGp5 の
採用要素 Option D)。typed command は `match.command` prefix の
発火契機に留まり、routed workflow の `exec` job は route 宣言の
substitute ペアを実行する。

spec(SPEC-034KXdoK2bMG3gQtIj8l6h・proposed — R1-R12 + AC)と
ADR(ADR-034KXk8h1AtwCkf2cQrb7m・proposed — O1a/O2a/O3a/O4a/
O5a/O6a/O7a/O8a 全採用)で方針は確定済みであり、discovery
(DISC-034KWGjJMiSKU5wBxcznho・resolved)が全接点を実測固定して
いる。前提となる #166(DESIGN-034KUgF5QB3WuzFG6PmVvr —
grant-set 投影・viability anchor/member 分割・`{command,
executed}` record split)は実装済みで、本 design はその構造を
そのまま拡張する: `substitute.grant` は `route.grant`・
`grants[]` member と同じ `is_route_eligible` 単一 seam
(`policy/exec.rs:111-116` — comment が substitute 参照を既に
明示)を通り、`project_grants` の set 投影・`executed` record
の実測 capture は変更不要の基盤である。

本 design の責務は、「`substitute:{grant,argv}` の schema/parse/
宣言層制約・viability retarget と `route.grant` member 級降格の
具体形・実行 anchor(`expected_input`・deadline・inputs)の切替
配線・record auto-capture の no-change 検証・stdin 分類の両側
適用・全 error 契約・churn inventory」を実装契約として固定する
こと。引用する file:line は全て branch
`feature/issue-167-substitute-exec-route-mode` の現行 code で
実測済み。

確定済み方針(ADR Decision Outcome):

- **O1a** — `substitute: {grant, argv}` nested object(`match`/
  `target`/`output` と同型の compound-value-as-object 規約)。
  `grant`・`argv` の required ペア、nested 未知 key 拒否、
  `argv` は executable を含まない args 列で `[]` 合法。
  `grant`(単数)は substitute の有無に関わらず required のまま。
- **O2a** — anchor 完全検査を `{substitute.grant,
  substitute.argv}` へ retarget し、`route.grant` は `grants[]`
  member と同じ member 級検査(存在 + `is_route_eligible` + pin)
  へ降格。原 argv の `route.grant` 照合は行わない(実行されない
  command の形骸化検査を除去)。
- **O3a** — `substitute` は trusted 層(builtin/user)限定。
  project 層 routes.yaml の `substitute` key presence を
  `validate_route` で `schema_error`/`InvalidRuntimePolicy` へ
  倒す(`grants` O7a と同じ判別子・同じ presence-first 規約)。
- **O4a** — `expected_input = {name: substitute.grant, args:
  substitute.argv}`。routed workflow inputs の `grant`/`execArgs`
  は実行ペアを運び、`argv`/`args`/`executable` は原 command
  identity として原値を維持(builtin routed-exec workflow が両
  mode で無変更動作)。
- **O5a** — record shape 不変: `route`(宣言 identity・
  `route.grant` anchor)+ `commandHash`(原 command)+
  `executed:{grant,argv}`(実測 substitute ペア)。schemaVersion
  1・新規 field なし。
- **O6a** — stdin-unsupported 分類は typed 側を維持しつつ
  substitute ペア(pin 解決 executable + 宣言 argv)にも適用。
- **O7a** — 投影 set = `{route.grant} ∪ grants ∪
  {substitute.grant} ∪ {fixed}`。
- **O8a** — Option E(exec-less route)は open 事項として残置。
  routed-exec convention(unconditional `exec` job +
  `outputs.execResult` whole binding)は不変。

## Scope

### In Scope

- routes.yaml route entry への optional `substitute: {grant,
  argv}` 追加 — `RouteSubstitute` struct・`Route.substitute`
  field・`KNOWN_KEYS`・`validate_route` の nested object parse
  (`crates/cheaper-cli/src/route/routes.rs`)。
- `substitute` 宣言の層制約 — trusted 層(builtin/user)限定。
  project 層 route entry の `substitute` key presence を
  `validate_route` で fail-loud `schema_error` へ倒す(§(a) —
  O3a・`grants` の :426-431 と同型)。
- viability retarget — 完全検査の対象を `Route::exec_pair` 経由の
  実行ペアへ向け直し、substitute mode の `route.grant` を member
  loop 位置へ降格、stdin 分類へ substitute ペア評価を追加
  (`crates/cheaper-cli/src/route/resolve.rs`)。
- 投影 set の拡張 — `Route::grant_set()` に `substitute.grant`
  を加える(`project_grants` 呼出し側・`policy/exec.rs` は
  code 不変)。
- 実行 anchor 配線 — `Route::exec_pair` helper 新設 +
  `route/run.rs` の deadline lookup(:356-363)・
  `RouteExecution::new` 呼出し(:374-382)・inputs `grant`/
  `execArgs`(:385/:392)を実行ペアへ切替。
- anchor 規約の明文化 — `expected_input`・D11 byte-match・
  lifecycle bound・exit mirror・record attribution の substitute
  mode での意味(code 変更なしの契約固定を含む)。
- 上記に伴う unit/integration test・routes.yaml shape 記述・
  docs 更新。

### Out of Scope

- **Option E(exec-less route)** — exec job を持たない routed
  workflow・`outputs.exitCode` 等の workflow 宣言値の exit
  mirror・「何も実行しない」route の一切の形。author 宣言 exit
  を信用する成功偽装 hole が未解決のため open 事項として残置
  (R11/O8a — `validate_routed_convention` は変更しない)。
- **workflow 宣言の substitute** — substitute ペアの出所は
  routes.yaml の route 宣言のみ。exec-input byte 一致が workflow
  側の組み立てを構造的に拒否する。
- **typed argv の substitute への転送・補間** — `substitute.argv`
  は宣言時点の固定値。prefix 置換 + 末尾 typed args 維持の需要が
  確立した時点で別 issue。
- **viability enum・`route resolve` 出力 shape の変更** —
  substitute 失敗は `NoGrant` へ fold、`RouteResolution` に
  substitute echo は載せない(R11 出力 shape :105-128 不変)。
- **record schemaVersion bump・新規 record field** —
  `{command, executed}` split(#166)のまま。`executed` が
  substitute ペアを記録するのは `command::execute` の実測 echo
  であり新 field は要らない(O5a)。
- **firing-gate anchor grant の record 明示 field** — O5b
  不採用(宣言 join 欠落は監査需要確立時に additive field で
  別途検討 — spec Deferred と同一)。
- **`grants[]` member の interactive screening** — 現行
  非検査のまま(known semantics — §(e))。
- **member argv の route 解決時照合** — member job argv は
  宣言されていないため job 実行時遅延のまま(#166 O2a 不変)。
- **`project_grants`・`check_exec_execution`・`pin_exec_grant`・
  `is_route_eligible` の signature/predicate 変更** — substitute
  も同じ seam を通る(core/tools 側 code 不変)。

## Components and Boundaries

### (a) route declaration — `crates/cheaper-cli/src/route/routes.rs`

`Route` struct(:37-62)に optional `substitute` field を追加し、
nested object の schema 検証・宣言層制約・有効 set/実行ペア供給の
全責務をこの file に閉じ込める。

```rust
/// `substitute` — substitute-exec mode の宣言ペア(R1/O1a)。
/// `grant` は既存 `command.grants` 参照名(route は grant を
/// 生成しない — pin が executable を供給)、`argv` は executable
/// を含まない実行 args 列(command tool input `args` と同形、
/// `[]` = executable 単独起動の明示宣言)。presence が mode
/// switch になる — `Some` で substitute mode。
/// trusted 層(builtin/user)のみ宣言可(project 層は load 時拒否)。
#[derive(Debug, Clone, PartialEq)]
pub struct RouteSubstitute {
    /// 実行 grant 名 — viability 完全検査・expected_input name・
    /// lifecycle bound・投影 member の execution anchor。
    /// `route.grant` と同名可(同一 grant の argv 差替)。
    pub grant: String,
    /// 実行 argv — grant の argv pattern が bound する宣言固定値。
    /// typed args の転送・補間はしない(宣言凍結 — R12)。
    pub argv: Vec<String>,
}
```

`Route` へ `grants`(:52)直後に field を追加:

```rust
/// `substitute` — substitute-exec mode 宣言(Option D)。
/// `Some` のとき実行対象は `{substitute.grant, substitute.argv}`
/// で、typed command は `match.command` prefix の発火契機に留まる。
/// firing gate・record attribution anchor は `route.grant` のまま。
pub substitute: Option<RouteSubstitute>,
```

- `KNOWN_KEYS`(:334-343)の `"grants"` 直後へ `"substitute"` を
  追加 — fail-loud unknown-key 規約(:344-348)を維持し、
  `substitute` の typo は unknown key として route-load error
  になる。`KNOWN_KEYS` は全層共通のため `"substitute"` は全層で
  recognized となり、層制約は unknown-key ではなく別の明示判定
  で行う(下記 — `grants` と同じ構造)。
- **`substitute` 宣言の層制約(O3a)** — 宣言できるのは trusted
  層(`RouteLayer::Builtin`/`RouteLayer::User`)のみ。`layer` は
  `validate_route` の :329 param で既に判別可能(新規 param
  不要)。検査位置は `grants` の層制約 + parse(:426-454)の直後、
  member 形状検査より先に presence で拒否する(`grants` O7a と
  同じ「untrusted 層の宣言は内容を評価せず fail-closed」規約):

```rust
// `substitute` は trusted 層限定(O3a)— substitute mode は typed
// command を一切実行せず `{substitute.grant, substitute.argv}` を
// 実行するため、`expected_input` byte-match が typed command へ
// 課していた拘束が乗り移らない。project 層が宣言できると
// untrusted authoring が「任意 typed prefix に対して任意の
// trusted grant を宣言 argv で起動する」実実行ペアの選択権を得る
// — `grants` member 密輸として O7a が塞いだ confused-deputy hole
// の、さらに強い形での再発を load 時に拒否する。
// `trusted:true` は project 自己申告の shadowing gate であり
// 判別に使えない — 判別子は宣言層 provenance。
if layer == RouteLayer::Project && entry.contains_key("substitute") {
    return Err(schema_error(
        file,
        &format!("{label}.substitute: not declarable in the project layer"),
    ));
}
// `substitute: {grant, argv}` — 宣言ペア。nested 未知 key は
// `match`/`target`/`output` と同じ fail-loud 規約で拒否する。
let substitute = match entry.get("substitute") {
    None => None,
    Some(v) => {
        let Some(obj) = v.as_object() else {
            return Err(schema_error(
                file,
                &format!("{label}.substitute: not an object"),
            ));
        };
        for key in obj.keys() {
            if key != "grant" && key != "argv" {
                return Err(schema_error(
                    file,
                    &format!("{label}.substitute: unknown key: {key}"),
                ));
            }
        }
        let grant = obj
            .get("grant")
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                schema_error(
                    file,
                    &format!("{label}.substitute.grant: missing or not a string"),
                )
            })?;
        let argv = obj
            .get("argv")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                schema_error(
                    file,
                    &format!("{label}.substitute.argv: missing or not an array"),
                )
            })?
            .iter()
            .map(|item| {
                item.as_str()
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .ok_or_else(|| {
                        schema_error(
                            file,
                            &format!(
                                "{label}.substitute.argv: element must be a non-empty string"
                            ),
                        )
                    })
            })
            .collect::<Result<Vec<String>, CheaperError>>()?;
        Some(RouteSubstitute { grant, argv })
    }
};
```

  - `substitute.grant` の検証は non-empty string のみ — `grant`
    単数が名前 pattern を持たず存在/eligibility を viability へ
    委譲するのと同じ規約(第二の検証規則を作らない)。未存在・
    project-scope・fixed 名の宣言自体は legal で、失敗は
    viability `NoGrant` へ fold される(§(b))。
  - `substitute.argv` は空 array 受理(executable 単独起動の
    明示宣言 — R1)。要素は non-empty string。
  - `substitute.grant == route.grant`・`substitute.grant ∈
    grants[]` はともに合法(冗長だが `grant_set()` が dedupe)。
    拒否規約は持たない。
  - `Route` literal 構築(:506-516)へ `substitute` を追加。

- **有効 set・実行ペアの供給は `impl Route`(:64-75)へ集約** —
  `{route.grant} ∪ grants ∪ {substitute.grant}` と「substitute
  presence で実行ペアが切り替わる」という 2 規則を二度書かない
  (表現 drift 防止 — viability と run 配線の双方が同じ helper
  を呼ぶ):

```rust
/// 有効投影 set — `{route.grant} ∪ grants ∪ {substitute.grant}`。
/// substitute mode でも `route.grant` は投影 member として残り
/// (firing gate・attribution anchor・member job からの呼出し可 —
/// O7a)。「投影名は全て viability 検証済み」の不変を維持する。
pub fn grant_set(&self) -> std::collections::BTreeSet<String> {
    let mut set = std::collections::BTreeSet::new();
    set.insert(self.grant.clone());
    set.extend(self.grants.iter().cloned());
    if let Some(substitute) = &self.substitute {
        set.insert(substitute.grant.clone());
    }
    set
}

/// 実行ペア `{grant名, argv}` — substitute mode は宣言
/// `{substitute.grant, substitute.argv}`、byte-matched は
/// `{route.grant, typed_args}`(caller は `resolution.exec_args`
/// を渡す)。viability 完全検査の対象・`expected_input`・inputs
/// `grant`/`execArgs`・deadline 導出が全てこの anchor に従う
/// (firing/attribution anchor の `route.grant` とは別系統 —
/// O2a/O4a/R6)。
pub fn exec_pair<'a>(&'a self, typed_args: &'a [String]) -> (&'a str, &'a [String]) {
    match &self.substitute {
        Some(substitute) => (substitute.grant.as_str(), substitute.argv.as_slice()),
        None => (self.grant.as_str(), typed_args),
    }
}
```

### (b) viability — `crates/cheaper-cli/src/route/resolve.rs`

`viability()`(:276-383)の no-grant 段と stdin-unsupported 段を
修正する。分岐順序(unsafe-command → invalid-route →
missing-target → no-grant → member → stdin-unsupported →
runnable)・`Viability` enum(:20-28)・`as_str`(:30-42)は不変。

**(b-1) anchor 完全検査の retarget(:305-348):** 検査対象を
`route.exec_pair(&out.exec_args)` 経由の実行ペアへ一本化する —
byte-matched では `{route.grant, out.exec_args}` で現行と同値、
substitute mode では `{substitute.grant, substitute.argv}`:

```rust
// no-grant — `command::execute` の事前 check 全通過が必須
// (routing≠authorization、D2)。実行ペアは `exec_pair` が選ぶ:
// byte-matched は {route.grant, 原 exec_args}、substitute mode
// は {substitute.grant, substitute.argv}(O2a/R3)。substitute
// argv は route 解決時点で確定済みの宣言値であり member の
// 「job 実行時へ遅延」とは質的が異なる — 「宣言通り実行する
// command」への完全検査が意味を持つ。
let (exec_name, exec_args) = route.exec_pair(&out.exec_args);
let Some(grant) = input
    .exec_policy
    .policy
    .grants
    .get(exec_name)
    .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
else {
    return Viability::NoGrant;
};
let Ok(pinned) = cheaper_tools::tools::exec::pin_exec_grant(
    exec_name,
    grant,
    input.exec_policy.root,
    input.exec_policy.symlink_policy,
    input.exec_policy.denylist,
    input.exec_policy.exec_deny,
) else {
    return Viability::NoGrant;
};
let options = ExecToolOptions {
    root: input.exec_policy.root.to_string(),
    symlink_policy: input.exec_policy.symlink_policy,
    exec: input.exec_policy.policy.clone(),
    pins: std::collections::BTreeMap::from([(exec_name.to_string(), pinned.clone())]),
    on_timeout: None,
    denylist: input.exec_policy.denylist.cloned(),
    exec_deny: input.exec_policy.exec_deny.clone(),
    write_scope: None,
    route_context: Some(RouteCommandContext {
        exec_cwd: Some(input.cwd.to_string()),
        accept_any_exit_code: true,
        ..RouteCommandContext::default()
    }),
};
if check_exec_execution(&options, exec_name, exec_args).is_err() {
    return Viability::NoGrant;
}
```

- `ExecToolOptions` の構築内容は現行と同一(routed scope 前提の
  `exec_cwd` + `accept_any_exit_code`)— 変わるのは lookup 名・
  pin map key・`check_exec_execution` の `name`/`args` だけ。
  `check_exec_execution`(:275-312)内部の `resolve_exec_grant` →
  `authorize_args` argv pattern 照合 → cwd prepare →
  `verify_pinned_executable` の順序・predicate は不変。
- `pinned` は後段の substitute 側 interactive 分類で再利用する
  ため `pins` map へは clone を入れる(§(b-3))。

**(b-2) `route.grant` の member 級降格(:349-377):** substitute
宣言のある route では `route.grant` を既存 member ループと同じ
位置・同じ検査(存在 + `is_route_eligible` + `pin_exec_grant`、
argv 照合なし)へ降格する:

```rust
// no-grant (b) — member 級検査: 存在 + is_route_eligible +
// pin 解決のみ。substitute mode では `route.grant` も member 級へ
// 降格する(R4 — firing gate/attribution anchor/投影 member の
// 役割は残るが実行対象ではない。原 argv への `route.grant` argv
// 照合は行わない — 実行されない command の形骸化検査であり、
// 「認可したくない command こそ置き換えたい」用途を壊す)。
// `grants[]` member の argv 照合は従来どおり job 実行時の
// check_exec_execution へ遅延。失敗は全て NoGrant へ fold。
let demoted = route.substitute.is_some().then_some(&route.grant);
for member in demoted.into_iter().chain(&route.grants) {
    let Some(member_grant) = input
        .exec_policy
        .policy
        .grants
        .get(member)
        .filter(|g| ResolvedExecPolicy::is_route_eligible(g))
    else {
        tracing::debug!(route = %route.name, member, "route grant member ineligible");
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
        tracing::debug!(route = %route.name, member, "route grant member unpinnable");
        return Viability::NoGrant;
    }
}
```

- `then_some` + `Option::into_iter` で substitute mode のみ先頭に
  `route.grant` を連結し、member ループ本体は**一切変更しない**
  — 「anchor 完全検査 ↔ member 級検査」の 2 段構造は維持し、
  検査位置(member ループ)をそのまま共有する。
- `route.grant` の member 級失敗も tracing::debug! へ member 名を
  載せる経路を共有する(viability 値上は区別不能 — O2a fold
  規約どおり)。

**(b-3) stdin-unsupported 段(:378-381)への substitute 評価追加:**
typed 側を維持しつつ substitute ペアへ同じ分類を適用する
(O6a/R10):

```rust
// stdin-unsupported — interactive command curated list(typed 側
// invariant 維持: interactive な typed command は substitute
// 宣言の有無に関わらず routing で置き換えない)。
if is_interactive(name, &out.exec_args) {
    return Viability::StdinUnsupported;
}
// substitute 側 — stdin 非接続で実際に spawn されるのは
// substitute ペアであり、interactive な substitute は発火後に
// grant timeout まで hang してから失敗する。pin 解決済み
// executable 名 + 宣言 argv で同じ分類を適用する(O6a/R10)。
if let Some(substitute) = &route.substitute {
    if is_interactive(&command_name(&pinned), &substitute.argv) {
        return Viability::StdinUnsupported;
    }
}
```

- `pinned` は (b-1) で pin 解決済みの substitute executable
  canonical path — `command_name`(:1324、dir prefix 剥離 +
  `.exe`/`.bat`/`.cmd` strip)で basename 化してから
  `INTERACTIVE_COMMANDS`/flag 判定へ掛ける。declared 名が shim/
  symlink でも resolved 実体で分類できる(pin は既に解決済みで
  追加コストなし)。`command_name` は :17 の既存 import で足りる。
- `grants[]` member は screening 対象外のまま(known semantics —
  member job は自身 grant bound で動く別経路、§(e))。

**(b-4) `RouteResolution` shape 不変:** `grant`(:91)は
`route.grant` echo のまま(:258)、`exec_args`/`args`/`argv`/
`executable` は原 command identity。substitute 情報は resolve
出力に載せない — `route run` の caller は `PreparedRoute.route`
(clone 済み `Route`、:98-106)から `exec_pair` で実行ペアを再導出
するため `RouteResolution` への substitute field 追加は不要
(R11 出力 shape 不変、O2a)。

### (c) projection — `crates/cheaper-core/src/policy/exec.rs` + `route/run.rs:121`

`project_grants`(:118-134)は `&BTreeSet<String>` 版で既に set
汎用 — **code 変更なし**。有効 set の構築責務は
`Route::grant_set()` にあり(§(a))、substitute mode の有効 set =
`{route.grant} ∪ grants ∪ {substitute.grant}` をそのまま渡す。

- `route/run.rs` の呼出し(:121-123)も `project_grants(&route.
  grant_set())` のまま**文面不変** — 変わるのは `grant_set()`
  の内容のみ。`substitute.grant` が投影に含まれなければ exec job
  の `resolve_exec_grant` が policy 解決失敗で落ちるため、set
  への追加は必須条件(R5/O7a)。
- `is_route_eligible`(:111-116)は `route.grant`・`grants[]`・
  `substitute.grant` の 3 参照を同じ `scope=="trusted" &&
  !fixed` で判定する単一 seam — predicate・comment ともに変更不要。
- 「set 内 1 件でも ineligible なら false」・`{names} ∪ {fixed}`
  retain の規約・viability との二重防御構造は #166 のまま。

### (d) 実行 anchor 配線 — `crates/cheaper-cli/src/route/run.rs`

`execute()`(:334-424)の 4 箇所を `exec_pair` 由来の実行ペアへ
切替える。`RouteExecution` 本体・`finish()`・`prepare()` の構造は
不変。

```rust
// 実行ペア — substitute mode は {substitute.grant, substitute.argv}、
// byte-matched は {route.grant, resolution.exec_args}(O4a)。
// deadline 導出・expected_input・inputs `grant`/`execArgs` は全て
// この anchor に従う。
let (exec_grant, exec_argv) = prepared.route.exec_pair(&prepared.resolution.exec_args);
let grant_timeout = prepared
    .supply
    .policy
    .exec
    .grants
    .get(exec_grant)              // :361 — route.grant → exec_grant
    .map(|g| g.timeout.clone())
    .ok_or_else(|| denied("no-grant"))?;
let deadline = deadline_ms(&grant_timeout)?;
// …(policy clone・ctx・count_cap は不変)…
let route_exec = Arc::new(RouteExecution::new(
    RouteLogStore::new(&layout.dirs.user_config_dir),
    prepared.extra_env.clone(),
    prepared.supply.root.clone(),
    exec_grant,                    // :378 — route.grant → exec_grant
    exec_argv,                     // :379 — resolution.exec_args → exec_argv
    count_cap,
    policy.workflow.artifacts.max_read_bytes,
));
let run_id = route_exec.run_id().to_string();
let inputs = Map::from_iter([
    ("grant".to_string(), json!(exec_grant)),          // :385
    ("executable".to_string(), json!(prepared.resolution.executable)),
    ("argv".to_string(), json!(prepared.resolution.argv)),
    ("args".to_string(), json!(prepared.resolution.args)),
    ("execArgs".to_string(), json!(exec_argv)),        // :392
    ("cwd".to_string(), json!(prepared.supply.root)),
    ("tokenThreshold".to_string(), json!(prepared.route.token_threshold.unwrap_or(8192.0))),
    ("runId".to_string(), json!(run_id)),
]);
```

- **`grant_timeout` lookup(:356-363)** — `prepared.supply.policy.
  exec.grants` は `prepare()` の `project_grants`(:121)で投影済み
  の policy で、`grant_set()` が `substitute.grant` を含むため
  投影内に残っている。lookup 名を `exec_grant` へ替えるだけで
  deadline が「実行 grant の宣言 `timeout` + 120s」に追従する
  (R8 — `deadline_ms` 自体は :325-332 で不変)。
- **`expected_input`(:176)** — `RouteExecution::new`(:148-182)
  の `json!({"name": grant, "args": exec_args})` 構築式は**不変**。
  caller が実行ペアを渡すため substitute mode では自動的に
  `{name: substitute.grant, args: substitute.argv}` になる。
  signature(`grant: &str, exec_args: &[String]`)も不変。
- **inputs** — `grant`/`execArgs` は実行ペア、
  `argv`/`args`/`executable` は原 command identity(worker の
  analyze prompt・診断用)として原値を維持。builtin
  `routed-exec.workflow.json`(:17 — `{name: $input.grant,
  args: $input.execArgs}`)が binding 無変更で両 mode に動く
  ための契約(O4a)。
- **D11 byte-match gate(`workflow_run.rs:467-484)** — `ctx.job_id
  == "exec" && ctx.job.name == "command"` の job のみに
  `expected_input` との `Value` 一致を要求する機構・文言・
  `reason:"exec-input-mismatch"`/`PolicyViolation` 分類は**一切
  不変**。照合対象値が substitute ペアへ替わるだけで、gate の
  コード・適用条件には触れない。`route_context` attach(:521)も
  同 gate 条件で不変。

### (e) anchor 規約 — lifecycle・exit・record の 2 系統 anchor

code 変更が伴わない契約固定。親 design の「実行 grant anchor」
と「route 帰属 anchor」の 2 系統分離を substitute mode の
実装境界として固定する。

- **実行 anchor = `substitute.grant`** — `expected_input.name`・
  deadline 導出・exec job lifecycle(`timeout`/`killGrace`/
  `exitCodes`/`cwd`/`env`/`maxOutputBytes`)は全て実行 grant の
  宣言値に従う。exec job の lifecycle は `command::execute` が
  named grant(= `substitute.grant`)の宣言値を読む既存規約
  (`tools/command/mod.rs:218-244` — `check_exec_execution` →
  `ProcLimits{timeout, kill_grace, max_output_bytes,
  accepted_exit_codes}` → `ProcPrepared{executable, argv, cwd,
  env}`)。routed-exec context(`exec_cwd` = payload cwd・
  passthrough `extra_env`・`accept_any_exit_code`・route log
  `log_tee` — `run.rs:220-229` の `RouteExecution::context()`)
  は exec job に mode 共通で適用される。
- **`grants[]` member job は従来どおり自身 grant bound** —
  `job_id != "exec"` の command job は expected_input gate を
  通らず、`route_context: None` で member grant の宣言
  `argv` pattern・lifecycle・`cwd`/`env`/`exitCodes` で動く
  (#166 §(e) 規約不変 — substitute mode でも同じ)。member job
  に routed-exec context は付かない。
- **workflow deadline は実行 grant `timeout` のみ由来** —
  member がより長い `timeout` を宣言しても silent clamp の
  known semantics(#166 と同じ — max-of-set 化はしない)。
  substitute mode では「実行 grant」= `substitute.grant`。
- **exit mirror は実行した command の exit** — `finish()` は
  captured `details.exitCode`(:467-474)を process exit code
  へ mirror し、`accept_any_exit_code` による非 0 透過も不変
  (spec R15「実行した command の exit」の再確認 — substitute
  mode では substitute command の exit をそのまま mirror する)。
  mechanism は mode 非依存で不変。
- **record attribution anchor = `route` + `route.grant`** —
  `kind:"route"` record の `route` field は宣言 `name`
  (`run.rs:606` — `prepared.route.name`)で変わらず、その
  route の firing gate 認可 grant は `route.grant`(宣言側
  anchor。`substitute.grant` は実行範囲の宣言であり attribution
  anchor を構成しない — O5a/O5c 不採用の根拠)。`commandHash`
  は原 `payload.command` の `DefaultHasher` hex 規約で不変
  (`usage_log.rs:370-387` — env-prefix 値を含み得るため hash
  のまま)。
- **`executed:{grant,argv}` は実測 echo で auto-capture** —
  `command::execute` が `ProcIdentity.details` へ
  `("grant", name)`・`("argv", args)` を書く既存経路
  (`tools/command/mod.rs:263-272`)が、exec job input =
  `{name: substitute.grant, args: substitute.argv}` である
  ことをそのまま details へ記録する。`finish()` の
  `details.{grant,argv}` 抽出(:480-504)と
  `RouteRecordInput.executed_grant`/`executed_argv`
  (`usage_log.rs:338-344`)・`build_route_record` の `executed`
  挿入(:392-398)は**全て不変** — record pipeline が
  substitute ペアを運ぶために schema・field・code の変更は
  一切要らない(O5a。`schemaVersion: 1`・additive 規約・
  record timing(captured result 必須)は #166 のまま)。
- **`exitCode`・`ok` は mirror した substitute exit 由来** —
  `finish()` の `ok: exit_code == 0 && failures.is_empty()`
  (:619)は変わらない。
- **member job の実行は `kind:"route"` record に直接載らない**
  — #166 規約不変。

### (f) confused-deputy / 宣言層境界

`project_grants` 後の `supply.policy.exec.grants` は
`{route.grant} ∪ route.grants ∪ {substitute.grant} ∪
{fixed grants}` に限定され、invocation/scope 両経路の policy
解決はこの投影 policy のみを見る(proc_invocation parity 構造
不変 — `workflow_run.rs:503` の `exec: None` 再注入 + parity
check)。

- **byte-match 保護は typed command へ乗り移らない** —
  byte-matched mode の `route.grant` は `expected_input` byte
  一致で「実行される exec = typed command(user が打鍵した
  argv)」へ拘束されるため、project 層でも `grant:` anchor 宣言
  が許されてきた(#166 O7a/R7 の argv-match 保護)。substitute
  mode はこの等式を壊す: typed command は作者制御の
  `match.command` prefix に退化し、実行される `{grant, argv}`
  は route 宣言値になる。project 層が `substitute` を宣言
  できると、untrusted authoring が「任意 typed prefix に対して
  任意の trusted grant を(pattern 適合の)任意の宣言 argv で
  起動する route」を書ける — `grants` member が「投影へ入る
  だけ」だったのに対し、substitute は「実際に実行される
  `{grant,argv}` の選択権」であり、O4c/O7a が塞いだ審査迂回を
  **さらに強い形**で再発させる。grant argv pattern bound は
  「どの grant を・どの宣言 argv で起動するか」の選択権を
  制約しない。よって宣言は trusted 層限定(§(a) O3a)。
- **undeclared 名の拒否** — 投影外の `!fixed` grant を
  `input.name` に指定した job は `resolve_exec_grant` で
  `PolicyViolation` — 「trusted grant 全てが route 経由で
  呼べる」状態にならない。`substitute.grant` は投影内にある
  ため exec job が解決できるが、投影への追加は宣言のみが経路
  (typed argv・payload・workflow 入力は `grant_set()` に届か
  ない — 宣言凍結 R12)。
- **`substitute.argv` の束縛** — substitute argv は
  `substitute.grant` の argv pattern が bound し、route 解決
  時点の `check_exec_execution` が完全照合する(§(b-1))。
  「宣言できる置換は grant 認可内に閉じる」— routing ≠
  authorization(D2)維持。
- **deny の uniform 適用** — `execDenyPatterns` は
  `substitute.grant`・member 級 `route.grant` の `pin_exec_grant`
  (viability §(b) と scope pin 構築の両方)に同じく効き、
  deny される grant は ineligible → `NoGrant`。
- **`trusted: true` は判別に使えない** — project 自己申告の
  shadowing gate(`routes.rs:152-166`)であり層 provenance では
  ない(O3a・#166 O7a と同じ結論)。判別子は宣言層 provenance。

### (g) signature 契約(変更後)

| function/struct | 変更後 signature | 備考 |
|-----------------|------------------|------|
| `RouteSubstitute` | `pub struct { pub grant: String, pub argv: Vec<String> }` | `routes.rs` 新設 — `Debug/Clone/PartialEq`(`Route` derive 整合)。`substitute` presence が mode switch |
| `Route` | `pub substitute: Option<RouteSubstitute>` を `grants` 直後へ追加 | `routes.rs:52` 直後。`Route` literal 3 箇所(`routes.rs:506`・`resolve.rs:396`・`routing_policy.rs:2081`)が追随 |
| `Route::grant_set` | `(&self) -> BTreeSet<String>` — set に `substitute.grant` を追加 | `routes.rs:69-74` — `{route.grant} ∪ grants ∪ {substitute.grant}` 規則の単一 owner |
| `Route::exec_pair` | `<'a>(&'a self, typed_args: &'a [String]) -> (&'a str, &'a [String])` | `routes.rs` 新設 — 実行ペア選択規則の単一 owner(viability・run 配線が共有) |
| `KNOWN_KEYS` | `"substitute"` を `"grants"` 直後へ追加 | `routes.rs:334-343` — unknown-key fail-loud 規約内(全層共通) |
| `validate_route` | signature 不変(`layer: RouteLayer` は :329 で既存) | project 層 entry の `substitute` key presence を形状検査より先に `schema_error` へ倒す + `{grant,argv}` required ペア parse(O1a/O3a) |

**変更しない signature**: `resolve::resolve`/`ResolveInput`/
`ExecPolicyView`/`Viability`/`RouteResolution`・
`RouteExecution::new`(param 名 `grant`/`exec_args` は実行ペア
一般名として維持 — substitute 専用名へ変更しない: builtin
workflow の `grant`/`execArgs` input 語彙と mode 非依存で一致)・
`project_grants`/`is_route_eligible`/`check_exec_execution`/
`pin_exec_grant(s)`/`authorize_args`・`RouteRecordInput`/
`build_route_record`/`record_route`/`append_usage_log`・
`build_rewrite_command`/`tokenize_shell`/`is_interactive`・
`validate_routed_convention`・`execute_routed`・
`workflow_run` gate — 全て不変。

### (h) call site 配線(全一覧)

| site | lines(現行) | 変更内容 |
|------|------------|----------|
| `route/routes.rs` | :334-343 | `KNOWN_KEYS` に `"substitute"` 追加 |
| `route/routes.rs` | :37-62 | `RouteSubstitute` struct 新設 + `Route.substitute` field(`grants` :52 直後) |
| `route/routes.rs` | :454 直後 | `substitute` の project 層 presence 拒否(`grants` :426-431 と同型)+ `{grant,argv}` parse(nested object・required ペア・nested 未知 key・non-empty 要素) |
| `route/routes.rs` | :506-516 | `Route` literal へ `substitute` 追加 |
| `route/routes.rs` | :64-75 | `grant_set()` に `substitute.grant` 追加 + `exec_pair()` 新設 |
| `route/routes.rs` | :574 以降 | unit test — substitute parse(`argv:[]` 受理・非 object/key 欠落/nested 未知 key/非 string 要素拒否)・project 層 presence 拒否 + builtin/user 受理・`grant_set` union・`exec_pair` 選択 |
| `route/resolve.rs` | :305-348 | anchor 完全検査の対象を `route.exec_pair(&out.exec_args)` へ retarget(lookup 名・pins key・`check_exec_execution` 引数を実行ペアへ) |
| `route/resolve.rs` | :349-377 | member ループの対象へ substitute mode のみ `route.grant` を先頭連結(ループ本体不変) |
| `route/resolve.rs` | :378-381 | typed 側 `is_interactive` 維持 + substitute ペア(`command_name(&pinned)` + `substitute.argv`)への同分類追加 |
| `route/resolve.rs` | :395-407,:411-422 | `route()` helper literal へ `substitute: None` + substitute viability 新規 case(retarget・route.grant 降格・stdin 両側) |
| `route/run.rs` | :356-363 | grant_timeout lookup を `exec_grant` へ(`exec_pair` 選出) |
| `route/run.rs` | :374-382 | `RouteExecution::new` 呼出しへ `exec_grant`/`exec_argv` |
| `route/run.rs` | :385,:392 | inputs `grant`/`execArgs` を実行ペアへ |
| `route/run.rs` | :733-770 他 | `prepare` 投影 test に substitute.grant 含有 case・`exec_pair` 配線 test(deadline 選出) |
| `hook/routing_policy.rs` | :2080-2091 | `route()` helper literal へ `substitute: None`(評価本体 :1451-1486 は resolve 呼出し構造不変) |
| `resources/routes.yaml` | :6-8 | `substitute` field の shape comment 追加(builtin route は substitute 非宣言 — 記述のみ) |
| `tests/integration/route.rs` | fixture + case | substitute 宣言 fixture・viability retarget case・`route resolve` 出力 shape 不変の確認 |
| `tests/integration/route_run.rs` | fixture + case | substitute e2e(exec job が substitute ペアを実行・exit mirror・record `executed`・member 併用・convention 拒否回帰) |
| `route_cmd.rs` | :105-128 | **不変** — R11 出力 shape(substitute echo なし・`grant` = `route.grant` anchor) |
| `route/supply.rs` | :29-53,:70-152 | **不変** — `RouteSupply`/`exec_view`/`load`/`load_or_none` 構造変更なし |
| `route/payload.rs` | 全体 | **不変** — payload schema に substitute は載らない |
| `workflow_run.rs` | :467-484,:521,:884,:902-945 | **不変** — D11 gate・`route_context` 配線・`validate_routed_convention`(Option E 境界) |
| `usage_log.rs` | :332-364,:369-422 | **不変** — `RouteRecordInput`/`build_route_record`/`record_route`(`executed` は実測 echo で auto-capture) |
| `usage_report.rs` | :503-538 | **不変** — additive field 無視・route bucket 分離維持 |
| `policy/exec.rs` | :111-134 | **不変** — `is_route_eligible`・`project_grants` が set 汎用で substitute を既にカバー(comment で substitute 参照を明示済み) |
| `tools/exec/mod.rs` | :163-211,:275-312 | **不変** — `pin_exec_grant`/`check_exec_execution` shared predicate seam |
| `tools/command/mod.rs` | :218-272 | **不変** — `details.{grant,argv}` 実測記録 |
| `hook.rs`/`hook_cmd.rs` | route 入力供給経路 | **不変** — `load_or_none` fail-open 規約・`RouteEvalInput` 構築 |
| `tests/integration/usage_log.rs` | :298-353 | **不変** — `route_record_shape` は `executed` shape を既に pin(record assertion は route_run.rs 側で substitute 値を検証) |
| `tests/proc_invocation.rs` | :702-749 | **不変** — set 投影 parity は `project_grants` set 汎用でカバー済み(必要なら投影 JSON に substitute 名を含める case の追加余地 — 必須ではない) |

### (i) error 契約

| 条件 | code / 分類 | 境界 | 備考 |
|------|------------|------|------|
| `substitute` が非 object | `InvalidRuntimePolicy` — `schema_error(file,"{label}.substitute: not an object")` | `routes.rs::validate_route` | route-load error — `load`/`load_or_none` 経路全てで fail-loud(hook は `load_or_none` で route arm 不活性へ fold — 既存規約) |
| `substitute` に nested 未知 key / `grant`・`argv` 欠落 / `grant` 非 string・空 / `argv` 非 array / `argv` 要素非 string・空 | 同上(`{label}.substitute(.grant/.argv): …`) | `routes.rs::validate_route` | `match`/`target`/`output` と同じ fail-loud 規約。`argv: []` は受理 |
| project 層 route entry が `substitute` key を含む | `InvalidRuntimePolicy` — `schema_error(file,"{label}.substitute: not declarable in the project layer")` | `routes.rs::validate_route`(:329 の `layer` param) | O3a — fail-loud route-load error(details `{path,reason:"schema",violation}` で file と entry を名指し)。形状検査より先に presence で拒否 — `grants` O7a と同型。`grant:` 単数・byte-matched route は project 層で従来どおり受理 |
| `substitute` typo・unknown key | 同上(unknown key) | `routes.rs:344-348` | fail-loud 規約不変 |
| `substitute.grant` 未存在 / `scope:"project"` / `fixed` / pin 不能 / `substitute.argv` が argv pattern 不一致 / cwd prepare・pin verify 失敗 | `Viability::NoGrant`(enum — error ではない)→ `route run` は `denied("no-grant")` `PolicyViolation` | `resolve.rs` §(b-1) retarget 分岐 → `run.rs:93-97` | anchor 失敗と同一 fold — 新規 viability 値なし。`route run` 不発 → 原 command 直接実行の fail-safe |
| substitute mode の `route.grant` 未存在 / ineligible / pin 不能 | `Viability::NoGrant` → 同上 | `resolve.rs` §(b-2) member ループ | `grants[]` member 失敗と同じ位置・同じ fold |
| `grants[]` member 未存在 / ineligible / pin 不能 | `Viability::NoGrant` → 同上 | `resolve.rs` §(b-2)(不変) | #166 規約 |
| substitute executable が interactive(`INTERACTIVE_COMMANDS`)または `substitute.argv` が interactive flag 含有 | `Viability::StdinUnsupported` → `route run` は `denied("stdin-unsupported")` `PolicyViolation` | `resolve.rs` §(b-3) | pin 解決済み executable 名で分類 — hang→timeout 失敗を resolve 時点へ前倒し |
| typed 側 interactive(原 executable/argv) | `Viability::StdinUnsupported` → 同上 | `resolve.rs` §(b-3)(不変) | 「interactive typed command は routing で置き換えない」invariant を mode 共通で維持 |
| `project_grants` false(投影 set 内 ineligible/不在名混入 — viability 通過後の変動等) | `PolicyViolation` — `denied("no-grant")` | `run.rs:121-123` | viability との二重防御(不変) |
| `expected_input` mismatch(`job_id=="exec" && tool=="command"` の name/args が実行ペアと不一致) | `PolicyViolation` — `reason:"exec-input-mismatch"` | `workflow_run.rs:471-484`(不変) | substitute mode では substitute ペアを要求 — `route.grant`・原 exec_args・投影外名の指定は失敗 |
| `details.{grant,argv}` 欠落・非 string | `InternalError` | `run.rs` finish() :480-504(不変) | `exitCode` 欠落(:467-474)と同一 invariant 扱い |
| usage record 書込み失敗 | —(best-effort)stderr `usage-log:` + `postprocessFailures` に `usage-record` | `usage_log.rs:446-459` + `run.rs:627-629` | exit code・stdout 不変(現行規約) |
| `load_or_none` の route/policy load 失敗(`substitute` schema error 含む) | `None` — route arm 不活性 | `route/supply.rs:138-152`(不変) | hook fail-open 維持 — `grants` と同じく load error が fail-open に fold |
| `route run` payload の unsafe segment/metachar | `PolicyViolation` — `rejected("unsafe-command")` | `run.rs:66-73`(不変) | tokenize-level structural gate は mode 非依存で残る |
| routed convention 違反(exec job 欠落・`outputs.execResult` whole binding 欠落) | `InvalidRuntimePolicy` — `reason:"missing-exec-job"`/`"exec-job-shape"`/`"execResult-output"` | `workflow_run.rs:884,:902-945`(不変) | Option E 境界 — substitute 宣言の有無に関わらず拒否 |

## Data and Control Flow

```
routes.yaml layers          declaration                    viability / projection / record
───────────────────────────────────────────────────────────────────────────────────────────
builtin/user/project ─→ routes::load(層別 file — :115-131)
  └─ validate_route(layer 判別済み — :329):
       KNOWN_KEYS(+substitute) → grant 確定
       → grants 層制約+parse(:426-454)
       → substitute 層制約+parse(:454 直後 — project presence 拒否後に
         {grant,argv} required ペア検証)
       → Route{grant, grants, substitute, …}
       ※ 形状不正・project 層の `substitute` presence(O3a)は
         route-load error(fail-loud — InvalidRuntimePolicy)

hook eval / route resolve / route run prepare
  └─ supply::load → ExecPolicyView → resolve::resolve
       ├─ unsafe-command / invalid-route / missing-target(不変)
       ├─ no-grant (実行ペア完全検査 — exec_pair が選ぶ):
       │    byte-matched: {route.grant, out.exec_args}
       │    substitute:   {substitute.grant, substitute.argv}   ← retarget
       │    存在 + is_route_eligible → pin_exec_grant
       │    → ExecToolOptions → check_exec_execution(実行ペア argv 照合)
       ├─ no-grant member 級(存在 + eligibility + pin のみ):
       │    substitute: [route.grant] ++ grants[]   ← anchor 降格
       │    byte-matched: grants[]                  (不変)
       └─ stdin-unsupported:
            is_interactive(原 executable, 原 exec_args)          (不変)
            + substitute 宣言時:
              is_interactive(command_name(pin解決 executable), substitute.argv)

route run prepare(Runnable のみ)
  └─ project_grants(route.grant_set())
       → supply.policy.exec.grants = {route.grant} ∪ grants
         ∪ {substitute.grant} ∪ {fixed}
       → WorkflowContext(projected policy) → execute_routed

route run execute — 実行ペア anchor(exec_pair):
  ├─ deadline_ms(投影 policy の exec_grant.timeout + 120s)
  ├─ RouteExecution::new(…, exec_grant, exec_argv)
  │    → expected_input = {name: exec_grant, args: exec_argv}
  └─ inputs: grant=exec_grant, execArgs=exec_argv,
       argv/args/executable=原 command identity

routed workflow 内
  ├─ exec job(job_id=="exec" && tool=="command")
  │    expected_input {name:exec_grant, args:exec_argv} byte match(D11 不変)
  │    → route_context(exec_cwd/extra_env/accept_any_exit/log_tee)
  │    → command::execute({name:substitute.grant, args:substitute.argv})
  │    → details.{grant,argv,exitCode} captured(実測 substitute ペア)
  └─ member job(input.name = grants[] member 名 — route.grant も可)
       expected_input gate 非適用・route_context なし
       → resolve_exec_grant(投影 policy — undeclared 名は拒否)
       → check_exec_execution(member argv pattern 照合)
       → member grant 宣言 lifecycle/cwd/env/exitCodes

finish(): captured.details → exitCode mirror + executed{grant,argv}
  └─ RouteRecordInput{route:宣言名, command:原 shell, executed_*:実測}
       → build_route_record → route/commandHash/executed(schemaVersion 1)
       → record_route → <userConfigDir>/usage.jsonl
       (best-effort — 失敗は usage-record postprocess failure)
```

## Non-Functional Constraints

- **routing ≠ authorization(D2)** — viability は
  `is_route_eligible` 判定と `pin_exec_grant`/
  `check_exec_execution` への委譲のみを行い、route 層に内容
  審査を持たない。`substitute.argv` は `substitute.grant` の
  argv pattern が bound するため、route 作者は grant 認可外の
  実行を宣言できない。retarget は委譲先の引数を変えるだけで
  seam は同一。
- **fail-safe fold** — substitute 側・member 級 `route.grant`・
  `grants[]` member のいずれの欠陥も `Viability::NoGrant` で
  route 不発 = 原 command が直接実行される。実行ペアが壊れた
  route を発火させて中途半端な投影 policy で job 失敗を起こさ
  ない(新規 viability 値なし — enum・`route resolve` 出力
  shape 不変)。
- **precheck strict 維持** — substitute argv は route 解決時点で
  確定済みの宣言値であり、anchor 完全検査と同じ完全検査(存在 +
  eligibility + pin + argv 照合 + cwd/pin verify 事前経路)が
  意味を持つ。#166 member の「job argv 未確定ゆえ job 実行時
  遅延」とは質的に異なり、「宣言通り実行する command」を検査
  する precheck の strict さは byte-matched と同レベル。
- **宣言凍結** — substitute ペアは route 宣言値で凍結され、
  typed argv・payload・workflow 入力が変更できない
  (`expected_input` byte-match が宣言値以外を拒否)。typed args
  の転送・補間経路は設けない。
- **宣言層境界** — substitute 宣言は trusted 層(builtin/user)
  限定。byte-match が typed command へ課していた拘束が
  substitute に乗り移らないため、project 層への「実実行
  `{grant,argv}` 選択権」委譲は O7a が塞いだ confused-deputy
  hole の再発として load 時に拒否する(§(f))。anchor
  `route.grant` 単数・byte-matched route は project 層で従来
  どおり宣言可。
- **semantic substitution は trusted 層 opt-in で受容** — 親
  agent は substitute の exit/output を原 command のものとして
  解釈する。routing 層が新たに開く意味差であり、trusted
  authoring 限定の主要根拠(R12)。
- **record additive 規約** — `executed` は #166 で分離済みの
  実測 field であり、substitute mode でも shape・
  `schemaVersion: 1`・timing・reader 互換は不変。新規 field・
  schemaVersion bump は行わない。
- **fail-open 保存** — hook 経路の `load_or_none`・unsafe
  segment skip・`unsafe_command` flag 規約は不変。substitute
  schema error は `grants` と同じく load error として fail-open
  に fold される。
- **cost 上界** — substitute mode の viability は byte-matched
  と同じ pin 解決回数(実行ペア 1 + member 数)。pin 解決は
  viability 時・scope 構築時の 2 回行われる現行構造を変えない。

## Risks and Trade-offs

- **authorization-of-intent の消滅** — substitute mode で原 argv
  は `route.grant` の argv pattern に照合されなくなり、「typed
  command が認可済みのときのみ発火」の gate は `match.command`
  prefix まで後退する(作者制御)。「認可したくない command こそ
  置き換えたい」用途(変更系 → dry-run)を可能にする意図的な
  trade-off であり、`substitute` を trusted 層限定にした理由
  そのもの(ADR Consequences)。
- **semantic substitution の意味差** — 親 agent は substitute
  の exit/output を原 command のものとして解釈する。user が
  打鍵した command と実際に走った command が異なることを
  trusted authoring opt-in で受容する(spec R15 再記述済みの
  帰結 — routing 層が新たに開く意味差)。
- **NoGrant fold の診断性** — substitute 失敗・member 級
  `route.grant` 失敗・`grants[]` member 失敗が `no-grant` で
  区別不能になる(#166 と同じ構造)。`route resolve` 出力の
  `grant` field は anchor(`route.grant`)を指し substitute 実行
  対象が見えない — 発火診断需要が確立した時点で resolve echo
  を再検討(Deferred — spec Deferred と同一)。
- **interactive 不発面の増加** — interactive executable を pin
  する `substitute.grant`・interactive flag を含む
  `substitute.argv`・interactive な typed command はいずれも
  `stdin-unsupported` で不発に倒れる(意図的な保守方向 — 「発火
  → hang → timeout 失敗」より resolve 時点分類が良い)。
- **record の anchor 時点性** — `kind:"route"` record だけでは
  firing-gate 認可 grant が `route` 名経由の宣言 join に依存
  する。routes.yaml 編集で `route.grant` が改名されると時点性の
  anchor が遡れない(現行 record 規約と同じ制約 — 監査需要が
  確立したら additive field で拡張、O5b 不採用)。
- **`route.grant` は投影に残る** — 実行しない grant を投影する
  ため半径は僅かに広いが、`route.grant` を member job が呼べ
  ない非対称・set 検査から anchor が外れる規約複雑化・record
  attribution が投影外の名を指す矛盾を避けるため O7b は不採用
  (substitute argv は宣言固定で半径は argv pattern bound に
  閉じている)。
- **schema 拡大の churn 面積** — `Route` literal 3 箇所・
  viability 分岐再編・`exec_pair` 配線・fixture・shape 記述・
  test を一括で触る変更面が広い(破壊的 schema 変更ではない
  が)。`route_cmd`/`supply`/`payload`/`workflow_run`/
  `usage_log`/`usage_report`/`policy/exec.rs`/`tools` 本体は
  不変 — 変更は route schema・viability・run 配線の 3 面に
  閉じる。

### Churn 一覧(実測 — signature/field 追随)

- **route schema** — `route/routes.rs`:`RouteSubstitute` 新設・
  `Route.substitute`(:52 直後)、`KNOWN_KEYS`(:334-343)、
  `validate_route` 層拒否+parse(:454 直後 — `:329` の `layer`
  param で新規 param 不要)、literal 構築(:506-516)、
  `grant_set()`/`exec_pair()`(:64-75)、unit test(:574 以降 —
  substitute parse/error case + project 層拒否 + builtin/user
  受理 + `grant_set` union + `exec_pair` 選択)。
- **viability/resolve** — `route/resolve.rs`:実行ペア完全検査
  retarget(:305-348)、member ループへ `route.grant` 連結
  (:349-377)、stdin 段へ substitute 分類追加(:378-381)、
  `route()`/`route_with_grants()` literal(:395-422)+ substitute
  viability 新規 case。`hook/routing_policy.rs`:`route()`
  literal(:2080-2091)のみ(評価本体 :1451-1486 は不変)。
- **run/anchor 配線** — `route/run.rs`:`exec_pair` binding 新設 +
  grant_timeout lookup(:361)・`RouteExecution::new` 呼出し
  (:378-379)・inputs `grant`/`execArgs`(:385/:392)の実行ペア化、
  `prepare_projects_grant_set` test(:733-770)の substitute
  投影 case・deadline 選出 test(`exec_pair` 由来 timeout)。
- **fixture/docs** — `resources/routes.yaml`(:6-8 — shape
  comment)、`tests/integration/route.rs`(substitute fixture +
  viability case)、`tests/integration/route_run.rs`(substitute
  e2e — exec substitute pair・exit mirror・record `executed`・
  member 併用・convention 拒否)、docs 4 file + refine 対象
  spec(§Docs 更新契約)。
- **code 不変のまま回帰対象** — `route/supply.rs`・
  `route_cmd.rs`・`route/payload.rs`・`route/logs.rs`・
  `workflow_run.rs`(:467-484,:521,:884,:902-945)・
  `usage_log.rs`(:332-459)・`usage_report.rs`(:503-538)・
  `policy/exec.rs`(:111-134)・`tools/exec/mod.rs`・
  `tools/command/mod.rs`・`hook.rs`/`hook_cmd.rs`・
  `resources/workflows/routed-exec.workflow.json`・
  `tests/integration/usage_log.rs`・`tests/proc_invocation.rs`
  — 無改訂で既存挙動が維持されることを確認する churn 境界。

## Docs 更新契約

- `docs/guides/parent-agent-integration.md` — routes.yaml 宣言例
  と field 説明に `substitute` を追加(意味: `{substitute.grant,
  substitute.argv}` が typed command の代わりに実行される
  substitute-exec mode・`argv` は executable を含まない宣言固定
  args・`argv:[]` 合法・trusted 層(builtin/user)限定で
  project routes.yaml に書くと route-load error・`grant:` 単数
  は substitute 宣言の有無に関わらず required — firing gate・
  attribution anchor)。viability の retarget(原 argv 照合を行わ
  ず substitute ペアを完全検査する)・typed/substitute 両側の
  interactive 分類・`expected_input` が substitute ペアを要求
  する点・record `executed` が substitute ペアを記録する点を
  明記。workflow authoring 規約 — exec job input は従来どおり
  `{name: $input.grant, args: $input.execArgs}`(inputs が実行
  ペアを運ぶため builtin binding は両 mode で無変更)。
- `docs/guides/configuration.md` — unified exec 参照節の
  「route の `grant:`/`grants:` 参照は trusted+`!fixed` のみ」
  に `substitute.grant` も同 predicate へ従う記述を追加し、
  `substitute` 宣言自体は builtin/user 層限定である旨を併記。
- `docs/guides/command.md` — route grant 参照記述を
  `substitute.grant` も同じ eligibility 規則へ従う旨へ更新。
- `docs/guides/usage.md` — `kind:"route"` record の `executed`
  記述に「substitute mode では `executed` が宣言 substitute
  ペアを記録する(route 帰属は宣言 identity・`commandHash` は
  原 command のまま)」を 1 文追記(record shape・additive-only
  規約は不変 — `route` record field の canonical 記述は
  parent-agent-integration 側、DRY)。
- `crates/cheaper-cli/resources/routes.yaml` — `grants:` 直下に
  `# substitute: {grant: name, argv: [arg, …]}  # optional — substitute-exec mode(builtin/user 層限定)` の shape comment を追加。
- `docs/specs/pretooluse-route-resolver-and-workflow-delegation.md`
  — refine 対象 spec として route field 一覧に `substitute?`
  (trusted 層限定)・viability 記述に実行ペア retarget・
  `expected_input`/record 記述に substitute mode の anchor 規約を
  追記する(content update のみ — 本 design からの `refines`
  relation は張らず、spec chain 側の管理に委ねる)。
- `docs/test-specs/*` — 検証契約は `route-resolution.md`
  (declaration/viability)・`route-run-delegated-exec.md`(run
  revalidation/delegated exec)・`route-usage-records.md`(record)・
  `route-grant-set-projection.md`(grant-set 投影・member 規約)の
  既存 test-spec が owner — substitute mode の契約追加は test-spec
  更新(または新規 test-spec)として後続 flow で扱い、本 design
  では Verification Notes で AC 対応を固定する。

## Deferred Design Concerns

- **Option E(exec-less route)の採否** — 親 chain の open 事項。
  author 宣言 exit(`outputs.exitCode` mirror・固定 mapping)を
  信用する成功偽装 hole の未解決・lint-only/analysis route の
  product 需要未確認。採否は需要確認と exit-code contract の
  信頼モデル設計が揃った時点の別 issue で判断。route `grant`
  field が firing gate として留保される設計だけは本 design で
  確定済み(required 維持 — spec R11/O8a)。
- **`route resolve` 出力への substitute echo** — R11 出力
  shape(:105-128)を維持したため、substitute mode の発火診断は
  `route`/`grant` field(anchor echo)だけでは「何が実行される
  か」が読めない。発火診断需要が確立した時点で `substitute`/
  `execPair` echo の additive 追加を検討(spec Deferred と同一)。
- **usage record への firing-gate anchor grant 明示 field** —
  record は実測 + route identity の規約を維持(O5a)。routes.yaml
  編集で `route.grant` が改名された場合に時点性の anchor が
  `route` 名経由の宣言 join でしか遡れない。監査需要が確立
  したら additive field として別途追加(O5b — spec Deferred
  と同一)。
- **substitute argv への typed args 転送・補間** — 宣言凍結を
  崩すため不採用。「prefix は substitute 化しつつ末尾 args は
  typed のまま」の需要が確立した時点で、byte-match 対象を宣言
  形に再設計する別 issue で検討。
- **`grants[]` member の interactive screening** — substitute
  ペアは main exec のため screening 対象としたが、member job
  は非検査のまま(known semantics — 自身 grant bound で動く別
  経路)。member の interactive 失敗が運用問題化した場合に
  member 側 screening を別途検討。
- **member timeout の silent clamp** — workflow deadline は実行
  grant(substitute mode では `substitute.grant`)の `timeout`
  のみ由来。member が長い `timeout` を宣言する運用が出た場合に
  `deadline = max(exec, members)` への変更を検討(#166 Deferred
  と同一 — substitute で構造は変わらない)。
- **`substitute` を project 層へ開くか** — O3b/O3c は byte-match
  保護が乗り移らない confused-deputy hole として不採用。
  project route が substitute を必要とする実需要が確立した
  場合、typed argv への何らかの宣言拘束(authorization-of-
  intent 相当)をセットに再評価する。

## Verification Notes

test 計画(spec Acceptance Criteria への対応 — AC1-AC12):

- [ ] **schema/declaration(AC1)** — `substitute: {grant, argv}`
      の parse で `Route.substitute` が `Some(RouteSubstitute)`
      になり・`argv: []` が受理され・非 object・`grant`/`argv`
      欠落・nested 未知 key・`grant` 非 string・`argv` 非 array・
      `argv` 要素非 string/空の各場合に `InvalidRuntimePolicy`
      route-load error(`details.{path,reason:"schema",violation}`
      で file+entry 名指し)・`substitute` typo は unknown key
      拒否(`routes.rs` unit test)。
- [ ] **宣言層制約(AC2)** — project 層 routes.yaml の
      `substitute` key presence が形状不正な値でも先に
      `InvalidRuntimePolicy` で拒否され、builtin/user 層の
      `substitute` 宣言は受理される。project 層の `grant:` 単数
      宣言・byte-matched route は従来どおり有効(`routes.rs`
      unit + `route.rs` integration — `grants` の
      `project_layer_grants_are_rejected` :873-922 と同型)。
- [ ] **substitute viability 失敗 fold(AC3)** —
      `substitute.argv` が `substitute.grant` argv pattern に
      不一致・`substitute.grant` 未存在・`scope:"project"`・
      `fixed`・pin 不能の各場合に viability が `no-grant`
      (新規値なし — `resolve.rs` unit + `route.rs` integration)。
- [ ] **retarget 回帰(AC4)** — substitute 宣言のある route で
      原 argv が `route.grant` の argv pattern に一致しなくても
      substitute ペアが健全なら `runnable`(原 argv 照合を行わ
      ない retarget の固定)。`route.grant` 未存在・ineligible・
      pin 不能では `no-grant`(member 級降格 — `resolve.rs` unit)。
- [ ] **exec-input 再 anchor(AC5)** — substitute route の routed
      workflow(builtin `routed-exec.workflow.json` 無変更)で
      exec job が `{name: substitute.grant, args: substitute.argv}`
      を実行し(child marker + `executed` details で証跡)・
      `expected_input` gate を通る。`job_id=="exec"` で
      `route.grant`・原 exec_args・member 名を指定する job は
      `exec-input-mismatch`(`route_run.rs` integration —
      `exec_job_named_member_is_input_mismatch` :594 と同型)。
- [ ] **exit mirror(AC6)** — substitute command の exit code
      (0・非 0)が `route run` process exit code として mirror
      される(`route_run.rs` — `io`/`ok` mode fixture + exit 7
      case で既存 mirror 機構が substitute exit を運ぶこと)。
- [ ] **lifecycle anchor(AC7)** — workflow deadline・exec job
      lifecycle が `substitute.grant` の宣言 `timeout`/
      `killGrace`/`exitCodes`/`cwd`/`env` に従う(`route.grant`
      の宣言値ではない — `envdump` fixture で substitute grant
      env が子へ届く・`route.grant` env は届かないことを検証。
      `run.rs` unit の deadline 選出 + `route_run.rs` integration)。
- [ ] **record shape(AC8)** — `kind:"route"` record が
      `route`(宣言名)・`commandHash`(原 command)・
      `executed:{grant: substitute.grant, argv: substitute.argv}`
      を持ち、`schemaVersion: 1`・新規 field 無しで `cheaper
      usage` 集計が従来どおり動く(`route_run.rs` record case —
      `finish`/`build_route_record`/`usage_report` は不変のため
      e2e record assertion で pin)。
- [ ] **stdin 両側分類(AC9)** — interactive な原 executable の
      typed command は substitute 宣言があっても
      `stdin-unsupported` で不発し、interactive executable を
      pin する `substitute.grant`・interactive flag を含む
      `substitute.argv` の route も `stdin-unsupported` で不発
      (`resolve.rs` unit — pin 解決 executable 名での分類を
      含む)。
- [ ] **member 併用(AC10)** — substitute route が `grants[]`
      member と併用でき、`input.name` が member 名の
      `job_id != "exec"` job は従来どおり動作し、投影外の grant
      名は `resolve_exec_grant` で拒否される(`route_run.rs`
      integration — `member_fx` :466-503 と同型の substitute
      fixture)。
- [ ] **Option E 構造的境界(AC11)** — exec job を持たない・
      `outputs.execResult` を持たない workflow は substitute
      宣言の有無に関わらず routed convention 違反で拒否される
      (`validate_routed_convention` :902-945 不変の確認 —
      `route_run.rs` で substitute 宣言 route が convention 違反
      workflow を指す fixture が `InvalidRuntimePolicy` になる)。
- [ ] **byte-matched 回帰(AC12)** — `substitute` を持たない
      既存 route(byte-matched・`grants` 宣言)の viability・
      exec-input・exit mirror・record 挙動に回帰がない(既存
      `resolve.rs`/`routes.rs`/`run.rs` unit・
      `route.rs`/`route_run.rs`/`usage_log.rs`/`hook.rs`/
      `proc_invocation.rs` test 群が無改訂で pass)。
- [ ] **hook 経路** — substitute 宣言 route が hook eval 経路
      (`evaluate_route_arm` :1451-1486)でも同じ resolve を通り
      runnable 時のみ rewrite され、schema error は
      `load_or_none` で route arm 不活性へ fold される
      (`routing_policy.rs` route eval test + fail-open 規約
      回帰)。
- [ ] **`route resolve` 出力 shape 不変** — substitute 宣言
      route の resolve 出力に substitute echo が載らず、
      `grant` が `route.grant` anchor を指す(R11 — `route.rs`
      integration の shape assertion で pin)。
