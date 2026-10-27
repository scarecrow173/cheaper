---
id: "ADR-034N3bHdJMNN63xqMHvKH9"
type: "adr"
status: "proposed"
title: "Use a read-only embedded resource boundary for route check"
created: "2026-10-27"
updated: "2026-10-27"
owners: ["cheaper maintainers"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/185"
    - "../discovery/explore-route-check-diagnostics-issue-185.md"
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../specs/route-check-diagnostics-issue-185.md"
  implemented-by: []
  depends-on:
    - "./pretooluse-auto-routing-via-route-resolver-and-workflow-delegation.md"
    - "./route-grant-set-projection.md"
    - "./unified-exec-grant-model.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related:
    - "../specs/route-check-diagnostics-issue-185.md"
  refines: []
  refined-by: []
  derives-from:
    - "../discovery/explore-route-check-diagnostics-issue-185.md"
  derived-by: []
  verifies: []
  verified-by: []
  references:
    - "crates/cheaper-cli/src/embedded.rs"
    - "crates/cheaper-cli/src/route/supply.rs"
    - "crates/cheaper-cli/src/route/routes.rs"
    - "crates/cheaper-cli/src/route_cmd.rs"
    - "crates/cheaper-cli/src/cli.rs"
    - "crates/cheaper-observability/src/sink.rs"
    - "crates/cheaper-core/src/policy/load.rs"
  defers: []
  deferred-by: []
metadata:
  adr:
    decision-makers: ["cheaper maintainers"]
    consulted: []
    informed: []
---

# Use a read-only embedded resource boundary for route check

## Context and Problem Statement

`cheaper route check` は builtin/user/project の route layer、policy、workflow、
grant、executable readiness を副作用なしで診断する必要がある。現行の共有
loader は `route::supply::load` が `embedded::builtin_dir()` を呼び、
`<temp>/cheaper-builtin-<version>-<fingerprint>/` へ embedded resource を
materialize してから path-based の routes/policy/workflow loader を使う。
これは route resolve/run/hook 経路では成立するが、`route check` の
no-filesystem-mutation 契約とは両立しない。

同時に、CLI 共通の observability 初期化は logging が Off 以外に解決した場合
`<log_root>/.cheaper/logs` を生成する。`route` は `--cwd` を log root の基準に
含むため、`route check --cwd <workspace>` が診断対象 workspace 内へ file sink を
作り得る。`route check` は ambient logging 設定の有無に関わらず、file logging
sink を起動してはならない。

loader を `route check` 用に複製すると、route merge、schema、grant、
denylist、workflow validation の policy 判定が drift する。必要なのは、
診断用に builtin resource を temp filesystem へ materialize せず供給する
read-only boundary と、command-scoped に file logging を抑止する dispatch
契約である。

## Decision Drivers

- `route check` は実行、設定変更、network/auth、全 filesystem mutation、
  secret/raw argv/env 漏出を一切行わない。
- builtin routes/policy/workflows は通常経路と同一 schema・merge・policy
  判定を使い、fail-open hook loader や policy predicate の複製を避ける。
- layer ごとの read/parse/schema failure は、その layer だけの診断結果として
  残り、他 layer の route diagnostics を消さない。
- clean install や temp cache 未作成環境でも builtin diagnostics が成立する。
- 通常の `route resolve`、`route run`、hook 経路の既存 temp-cache 挙動は
  変更しない。
- ambient `CHEAPER_LOG`/`--log` が有効でも、`route check` 実行中に file
  logging sink を作らない。

## Considered Options

- Allow the builtin temp cache as a documented exception
- Introduce a read-only embedded resource boundary for diagnostics
- Duplicate route/policy/workflow loading inside route check

## Decision Outcome

Chosen option: "Introduce a read-only embedded resource boundary for
diagnostics", because it is the only option that preserves the strict
no-filesystem-mutation contract without forking policy semantics.

`route check` は `embedded::builtin_dir()` と `materialize_builtin_dir()` を
呼ばない。`embedded.rs` は read-only `BuiltinResourceSource`（仮称）を提供し、
binary embedded な `routes.yaml`、`policy.yaml`、`workflows/**`、worker
resource metadata を logical path/content として参照できるようにする。
`route check` 側は、この source を消費する read-only adapter 経由で既存の
route schema/merge/viability、policy、workflow validation contract を使う。
必要であれば loader 側に in-memory/read-only source を受ける内部 entry point
を追加するが、validation semantics 自体は共有する。

通常経路の `route resolve`、`route run`、hook evaluation は既存の temp-cache
materialization 経路を維持する。この ADR は runtime 全体の embedded resource
戦略を置き換えない。

`route check` は command dispatch 時点で file logging sink を無効化できる
command-scoped contract を必要とする。stderr/diagnostics output は対象外だが、
ambient logging によって `.cheaper/logs` やその他 file sink が生成されては
ならない。

### Consequences

- Good, because `route check` が clean install でも temp cache を作らずに
  builtin diagnostics を提供できる。
- Good, because route/policy/workflow 判定を共有 predicate から消費でき、
  diagnostics と実処理の乖離を抑えられる。
- Good, because strict dry-run を ambient logging も含めて検証可能になる。
- Bad, because path-based loader 群に read-only source adapter を導入する
  変更が必要になり、単純な `supply::load` 呼び出しでは完了しない。
- Bad, because `embedded.rs` の embedded resource contract が
  materialize-only から materialize + read-only view の二経路になる。
- Bad, because CLI observability の command-scoped suppression を導入するまで
  `route check` は logging enabled environment で契約を満たせない。

### Confirmation

以下を実装・検証で確認する。

- `route check` 実行経路が `embedded::builtin_dir()` /
  `materialize_builtin_dir()` を呼ばない。
- `CHEAPER_LOG=debug` または file logging を有効化する設定の下でも、
  `route check --cwd <fixture>` が `.cheaper/logs`・temp cache・History・
  Usage・RouteLogs・credential を作成・更新しない。
- builtin `cargo-test` の no-grant、grant 付き route、project suppression、
  layer failure、shared policy failure が独立した diagnostics として報告される。
- route/policy/workflow validation が既存 contract と一致し、diagnostic
  専用の predicate 二重実装になっていない。

## Implementation Plan

- Affected paths:
  - `crates/cheaper-cli/src/embedded.rs`
  - `crates/cheaper-cli/src/route/supply.rs`
  - `crates/cheaper-cli/src/route/routes.rs`
  - `crates/cheaper-cli/src/route_cmd.rs`
  - `crates/cheaper-cli/src/args.rs`
  - `crates/cheaper-cli/src/cli.rs`
  - `crates/cheaper-cli/src/workflow_context.rs`
  - `crates/cheaper-core/src/policy/load.rs`
  - `crates/cheaper-workflow/src/loader.rs`
  - `crates/cheaper-cli/tests/`
  - `docs/guides/parent-agent-integration.md`
  - `docs/guides/configuration.md`
- Patterns to follow:
  - Keep `BuiltinResourceSource` internal to the CLI/runtime boundary; do not
    expose it as a product configuration surface.
  - Add read-only entry points or adapters beside existing loaders while
    preserving a single schema/validation/merge/policy implementation.
  - Load each route layer independently so layer errors remain per-layer
    diagnostics and do not discard successful layers.
  - Keep route resolution viability pure; sample-derived raw argv/args/env must
    not be echoed through diagnostics.
  - Disable file logging sinks for `route check` through an explicit
    command-scoped observability contract rather than mutating user logging
    configuration.
- Patterns to avoid:
  - Do not call `load_or_none` or other fail-open hook loading behavior.
  - Do not materialize embedded resources to temp or workspace paths from
    `route check`.
  - Do not duplicate grant, denylist, policy, workflow, renderer capability, or
    executable readiness predicates in the command implementation.
  - Do not write History, Usage, RouteLogs, tracing file logs, credentials, or
    diagnostic cache files.
- Dependencies or configuration: no new external dependency is expected.
  Existing `include_dir` embedded resources may be reused as the read-only
  source.

## Verification

- [ ] A clean-temp fixture runs `route check` with no pre-existing builtin cache
      and produces builtin route diagnostics without creating the temp cache.
- [ ] A fixture with ambient logging enabled creates no `.cheaper/logs` or other
      file sink under the diagnosed workspace or user config directory.
- [ ] Layer fixtures prove absent user/project files are normal empty layers,
      malformed files are per-layer diagnostics, and valid sibling layers remain
      reported.
- [ ] A shared policy failure fixture leaves valid route rows visible with
      policy-dependent status `unknown`/`unverified`.
- [ ] `builtin cargo-test` no-grant and grant-enabled fixtures produce the same
      policy result as `route resolve` without spawns.
- [ ] Diagnostics contain no raw sample argv, command, environment value, or
      secret-bearing error detail.
- [ ] `cargo test -p cheaper-cli --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam --test integration` passes for route-check coverage.
- [ ] `mise run check` passes.

## Pros and Cons

### Allow the builtin temp cache as a documented exception

- Good, because existing `supply::load` can be reused with minimal change.
- Good, because path-based routes/policy/workflow loaders remain unchanged.
- Bad, because it contradicts the accepted strict dry-run wording and makes the
  "no filesystem mutation" acceptance criterion unsatisfiable.
- Bad, because cache writes still occur on clean environments even when the
  command only inspects configuration.

### Introduce a read-only embedded resource boundary for diagnostics

- Good, because builtin resources remain accessible without filesystem writes.
- Good, because diagnostics can still share the same route/policy/workflow
  contract while avoiding runtime cache mutation.
- Good, because the normal `route resolve`/`route run` materialization path is
  isolated and does not need behavioral regression coverage.
- Bad, because loader adapters and shared validation boundaries require more
  refactoring than direct `supply::load` reuse.
- Bad, because an incomplete boundary can leave workflow/workflow-file access
  still path-dependent; design must cover all builtin resources consumed by
  diagnostics.

### Duplicate route/policy/workflow loading inside route check

- Good, because it avoids changing shared loader APIs.
- Bad, because it duplicates schema, merge, policy, grant, and workflow
  predicates and creates drift between diagnostics and real resolution.
- Bad, because later route contract changes can silently make `route check`
  report a different readiness answer than the runtime.

## More Information

- Issue: <https://github.com/scarecrow173/cheaper/issues/185>
- Spec: `../specs/route-check-diagnostics-issue-185.md`
- Discovery: `../discovery/explore-route-check-diagnostics-issue-185.md`
- Existing materialization: `crates/cheaper-cli/src/embedded.rs`
- Existing strict loader: `crates/cheaper-cli/src/route/supply.rs`
- Existing layer merge: `crates/cheaper-cli/src/route/routes.rs`
- Existing CLI logging init: `crates/cheaper-cli/src/cli.rs`,
  `crates/cheaper-observability/src/sink.rs`
