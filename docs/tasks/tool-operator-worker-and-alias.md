---
id: "TASK-034EUvKoVCWZdud4HyTGSK"
type: "task"
status: "todo"
title: "T3: tool-operator worker 定義・schema・tool-operate alias"
created: "2026-10-13"
updated: "2026-10-13"
owners: ["scarecrow173"]
relations:
  source: []
  changes:
    added: []
    modified: []
    deleted: []
    renamed: []
    moved: []
    generated: []
  implements:
    - "../plans/implement-tool-operator-built-in-worker.md"
  implemented-by: []
  depends-on:
    - "../plans/implement-tool-operator-built-in-worker.md"
    - "docs/tasks/optional-tools-core-machinery.md"
    - "docs/tasks/registry-degrade-and-prompt-surfacing.md"
  blocks: []
  supersedes: []
  superseded-by: []
  related: []
  refines: []
  refined-by: []
  derives-from: []
  derived-by: []
  verifies: []
  verified-by:
    - "../test-specs/tool-operator-built-in-worker.md"
  references: []
  defers: []
  deferred-by: []
---

# T3: tool-operator worker 定義・schema・tool-operate alias

## Scope

built-in Worker `tool-operator` の定義 file と input/output JSON Schema、
CLI alias `tool-operate` を追加する（design §(a)(g)(h)）。worker は
`tools: [read, grep, find, ls, count_tokens, process, command]` +
`cheaper.optionalTools: [process, command]` を宣言し、builtin default
policy（`process.tasks: {}` / `command.grants: {}`）でも T2 の縮退機構経由で
model session まで到達して evidence 付き compact structured result を親へ
返す。

- In: `tool-operator.md`（frontmatter + 英語 body）、input/output schema
  file、args.rs alias + error enumeration、cli.rs HELP、tests/args.rs の
  alias test と builtin worker resolve test。
- Out: positional-sugar `cheaper tool-operate "<task>"` の parser 変更
  （canonical invocation は `--input` / stdin JSON のみ — parser 規約ごと
  別 spec）、per-worker execution 制限 override、`write` / `edit`
  capability、fixture DSL・test（T4）、docs/APM（T5）。

### 所有ファイル

- `crates/cheaper-cli/resources/workers/tool-operator.md`（新規）— design
  §(a) の frontmatter（`tools` + `cheaper.optionalTools: [process, command]`
  + `model.tier: low` + `workload.outputPerInput: 0.3` + input/output
  schema file 参照。`writeScope` なし）と英語 body（role / tool usage
  discipline（`process` / `command` 等の gated tool は「"Available tools"
  に列挙されている場合に限る」条件付き表現でのみ言及 — Risk #3 の
  prompt-injection consistency 対策）/ evidence requirements / never dump
  large output / blocked semantics / partial・failed の必須 field mapping /
  respect caps / honesty about mutations）。
- `crates/cheaper-cli/resources/workers/schemas/tool-operator.input.schema.json`
  （新規）— design §(h) の shape — `task`（required、maxLength 4000）+
  `context` / `constraints` / `output` hints。`$defs` なし、全 object
  `additionalProperties: false`（strict-mode-safe）。
- `crates/cheaper-cli/resources/workers/schemas/tool-operator.output.schema.json`
  （新規）— design §(h) の shape — `status`（`success` / `partial` /
  `failed` / `blocked`）+ `summary` + `findings[]`（`evidence` required）+
  `unresolved` + `nextActions` required、top-level `evidence[]` optional、
  `$defs.evidence` は `kind` discriminator 単一 object（`anyOf` 不使用）。
- `crates/cheaper-cli/src/args.rs` — `alias_worker()`（:39-46）に
  `"tool-operate" => Some("tool-operator")` + error enumeration（:483 の
  `command required: run | bulk-read | code-write | doc-write | ...`）へ
  `tool-operate` 追加。
- `crates/cheaper-cli/src/cli.rs` — HELP（:516-518）に
  `  tool-operate           Alias for: run tool-operator` 追加。
- `crates/cheaper-cli/tests/args.rs` — `run_alias` table（:77-79）へ
  `("tool-operate", "tool-operator")` + extra positional reject の既存
  pattern。
- `crates/cheaper-cli/tests/`（builtin resolve test）—
  `tests/common::builtin_dir()` 経由で `resources/workers/*.md` 全件を
  loop し `resolve_worker` が各 builtin worker を parse + input/output
  schema compile できることを assert する cli test（`tool-operator` を
  含む builtin class 全体を pin し、T3→T1 依存 — `optionalTools` key
  未実装だと strict-key reject で load 不能 — を機械的に enforce する）。

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/implement-tool-operator-built-in-worker.md` — 実装対象 plan（T3）
- `docs/designs/tool-operator-built-in-worker.md` — §(a) worker 定義・
  §(g) CLI alias・§(h) schema shape
- `docs/specs/tool-operator-built-in-worker.md` — AC1
- `docs/test-specs/tool-operator-built-in-worker.md` — 検証 spec
  （worker 定義 load・alias 解約・positional reject の行為保証）

## Dependencies

- `docs/tasks/optional-tools-core-machinery.md`（T1 — `optionalTools` key
  の parse/validate が無いと worker file が strict-key reject で load 不能）
- `docs/tasks/registry-degrade-and-prompt-surfacing.md`（T2 — worker の
  normative 挙動である縮退が成立していること）

## Steps

1. `tool-operator.input.schema.json` / `tool-operator.output.schema.json`
   を design §(h) verbatim shape で作成する（strict-mode-safe:
   `additionalProperties: false`・`anyOf` 不使用の `kind` discriminator）。
2. `tool-operator.md` を design §(a) の frontmatter + 英語 body で作成する
   （gated tool 言及は条件付き表現、evidence field 名・blocked semantics・
   partial/failed 必須 field mapping・caps・honest mutations）。
3. `args.rs` の `alias_worker()` と error enumeration、`cli.rs` の HELP、
   `tests/args.rs` の `run_alias` table + positional reject test を追加する。
4. `builtin_dir()` loop で全 builtin worker の `resolve_worker`（worker
   parse + input/output schema compile を含む）を assert する cli test を
   追加する。
5. `cargo test -p cheaper-cli --features cheaper-cli/dev-seam` が pass
   することを確認する。

## Done When

- [ ] `tool-operator` が worker discovery で load され input/output schema
  が解決する（`resolve_worker` が parse + schema compile に成功）
- [ ] `cheaper tool-operate` が `cheaper run tool-operator` と等価に解約
  され、extra positional を reject する（canonical invocation は
  `--input` / stdin JSON のみ）
- [ ] `builtin_dir()` loop test が `tool-operator` を含む全 builtin
  worker で pass — `optionalTools` key 未実装（T1）では strict-key
  reject で fail するため T3→T1 ordering が機械的に enforce される

## Rollback

- task 単位の coherent commit を `git revert` — worker file + schema file
  + alias（args.rs・cli.rs・args test）を unit として除去する。T3 以降に
  T1/T2 を revert する場合は本 task の worker file 除去を必ず伴わせる
  （T1 の Rollback 参照）。
- revert 後に
  `mise exec -- cargo build --workspace --all-targets --features cheaper-runtime/dev-seam,cheaper-cli/dev-seam`
  が pass すること。

## Verification

- [ ] `mise exec -- cargo test -p cheaper-cli --features cheaper-cli/dev-seam` exits 0（`--test args` の alias table + positional reject + builtin resolve loop test を含む）
