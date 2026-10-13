---
id: "IMPL-034EVH2F3b1QJYE8LSAfkT"
type: "impl"
status: "completed"
title: "tool-operator worker definition schemas and tool-operate alias"
created: "2026-10-13"
updated: "2026-10-13"
owners: []
relations:
  source: []
  changes:
    added:
      - {"type":"code","path":"crates/cheaper-cli/resources/workers/tool-operator.md"}
      - {"type":"code","path":"crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json"}
      - {"type":"code","path":"crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json"}
      - {"type":"code","path":"crates/cheaper-cli/tests/builtin_workers.rs"}
      - {"type":"doc","path":"docs/impl/ir/tool-operator-worker-definition-schemas-and-tool-operate-alias.md"}
    modified:
      - {"type":"code","path":"crates/cheaper-cli/src/args.rs"}
      - {"type":"code","path":"crates/cheaper-cli/src/cli.rs"}
      - {"type":"code","path":"crates/cheaper-cli/tests/args.rs"}
      - {"type":"doc","path":"docs/impl/ir/README.md"}
      - {"type":"doc","path":"docs/tasks/tool-operator-worker-and-alias.md"}
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "docs/tasks/tool-operator-worker-and-alias.md"
  implemented-by: []
  depends-on: []
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by: []
  references: []
  defers: []
  deferred-by: []
metadata:
  experiments:
    adopted: []
    rejected: []
---

# tool-operator worker definition schemas and tool-operate alias

## Summary

T3。built-in Worker `tool-operator` の定義 file
(`resources/workers/tool-operator.md`)と input/output JSON Schema、
CLI alias `tool-operate` を追加した(design §(a)(g)(h))。worker は
`tools: [read, grep, find, ls, count_tokens, process, command]` +
`cheaper.optionalTools: [process, command]` を宣言し、builtin
default policy(`process.tasks: {}` / `command.grants: {}`)でも
T2 の縮退機構経由で model session まで到達する。

## Context

TASK-034EUvKoVCWZdud4HyTGSK。issue #102 の tool-operator built-in
worker の第 3 task で、design §(a) frontmatter + 英語 body、
§(h) schema shape、§(g) CLI alias を実装する。T1(df540ad)が
`optionalTools` の parse/validate、T2(c9921f1)が registry degrade
path と prompt surfacing を着地済み。

## Implementation

### Changes

- `crates/cheaper-cli/resources/workers/tool-operator.md`(新規):
  design §(a) verbatim の frontmatter(`tools` 7 名 +
  `cheaper.optionalTools: [process, command]` + `model.tier: low` +
  `workload.outputPerInput: 0.3` + input/output schema file 参照、
  `writeScope` なし)。body は既存 worker と同じ英語 system
  prompt で、`## Procedure` / `## Output rules` /
  `## Evidence rules` / `## Prohibitions` 構成。`process` /
  `command` の言及は常に「"Available tools" に列挙されている
  場合に限る」条件付き表現(Risk #3 の prompt-injection
  consistency 対策 — degraded 注記と矛盾しない)。evidence の
  `kind` discriminator と payload field 名(`location` =
  `path` + `lines`・`commit`・`exitCode`・`truncation` =
  `truncated`・`note` = `detail`)、non-success 時の
  `unresolved` / `nextActions` populated + `summary` 非空の
  field mapping、blocked semantics(権限昇格・迂回禁止)、
  caps 遵守、mutation の honest reporting(子 process 副作用を
  audited change として主張しない)を固定した。
- `crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json`
  (新規): design §(h) verbatim — `task`(required、minLength 1、
  maxLength 4000)+ `context` / `constraints` / `output` hints
  (`focus`・`maxFindings`)。`$defs` なし、全 object
  `additionalProperties: false`。
- `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`
  (新規): design §(h) verbatim — `status`(`success`/`partial`/
  `failed`/`blocked`)+ `summary` + `findings[]`(`evidence`
  required)+ `unresolved` + `nextActions` を required、
  top-level `evidence[]` optional。`$defs.evidence` は `kind`
  discriminator の単一 object(`anyOf` / `if`/`then` 不使用 —
  strict-mode-safe。conditional-required ではなく non-success を
  含む全 status で `unresolved` / `nextActions` を required とする
  design の解決に従う)。`$defs.path` / `lines` は bulk-reader
  schema と同じ workspace-relative pattern / `{start,end}` 形。
- `crates/cheaper-cli/src/args.rs`: `alias_worker()` に
  `"tool-operate" => Some("tool-operator")`、command-required
  error enumeration に `tool-operate` を追加。
- `crates/cheaper-cli/src/cli.rs`: HELP に
  `  tool-operate           Alias for: run tool-operator` を追加。
- `crates/cheaper-cli/tests/args.rs`: `run_alias` table に
  `("tool-operate", "tool-operator")` を追加し、loop 内で各
  alias の extra positional reject(`unexpected arguments: extra`)
  も併せて assert する既存 pattern を追加。
- `crates/cheaper-cli/tests/builtin_workers.rs`(新規):
  `tests/common::builtin_dir()` 経由で `resources/workers/*.md`
  全件を loop し、`resolve_worker`(worker parse + input/output
  schema compile を含む)が全 builtin worker で成功することを
  assert。`tool-operator` が一覧に存在することも pin する。

### Decisions During Implementation

- `$defs.path` は design §(h) の「input と同じ」指示を、bulk-reader
  input/output schema が共有する workspace-relative path pattern
  (`^(?!/)(?![A-Za-z]:)(?!\.\.(/|$))(?!.*/\.\.(/|$)).+$`)として
  解釈した — tool-operator の input schema 自体は `$defs` を持た
  ないため、参照先は同規約の兄弟 worker schema。
- builtin resolve test は named worker 単発 assert ではなく
  `*.md` 全件 loop とした(task / plan の指定 — builtin class 全体を
  pin し、`optionalTools` key 未実装では strict-key reject で
  fail する T3→T1 ordering を機械的に enforce する)。
- 新規 test は独立 file `tests/builtin_workers.rs` とした
  (`tests/args.rs` は parse table 専用であり、worker resolve は
  別責務)。

## Related Experiments

### Adopted

なし。task/design が確定済みで追加 experiment は起こさなかった。

### Rejected

なし。

## Validation

- `mise exec -- cargo fmt --all -- --check` — exit 0。
- `mise exec -- cargo build --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam` — exit 0。
- `mise exec -- cargo test -p cheaper-cli --features
  cheaper-cli/dev-seam` — exit 0。`--test args` の 30 test
  (`run_alias` に `tool-operate` + positional reject を含む)と
  `builtin_workers` の loop test(`tool-operator` を含む全
  builtin worker の `resolve_worker` 成功)を含む全 pass。
- `mise exec -- cargo test -p cheaper-core` — exit 0
  (worker_validate 等の golden は不変 — 新規 worker resource は
  builtin dir 追加のみで golden corpus に触れない)。
- `mise exec -- cargo clippy --workspace --all-targets --features
  cheaper-runtime/dev-seam,cheaper-cli/dev-seam -- -D warnings` —
  exit 0。

## Risks

- worker body の normative 挙動(degraded run の完走・`blocked`
  返却・`UNKNOWN_TOOL` 経路)は T4 の dev-seam fixture test が
  検証する。本 task は定義 load と alias 解約までを pin する。

## Follow-ups

- T4: fixture DSL `FixtureWorker.optionalTools` knob + dev-seam /
  regression / golden test(degraded / granted / capability-denied /
  mixed / scripted `UNKNOWN_TOOL` / blocked status)。
- T5: 公開面 docs(README・workers.md・usage.md・command.md)+
  APM skill `cheaper-tool-operate`。
