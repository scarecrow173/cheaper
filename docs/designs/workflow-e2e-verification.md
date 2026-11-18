---
id: "DESIGN-034ZwWdAPNPIYtwcSU1r5S"
type: "design"
status: "approved"
title: "Workflow E2E and APM distribution verification approach"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/205"
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
    - "./workflow-catalog-manifest-and-read-only-list.md"
    - "./l0-workflow-candidate-trigger-distribution.md"
    - "./workflow-discover-skill-structure.md"
    - "./workflow-create-skill.md"
  refines: []
  refined-by: []
  derives-from:
    - "../specs/workflow-e2e-verification-issue-205.md"
  derived-by:
    - "../plans/verify-workflow-e2e-issue-205.md"
    - "../test-specs/workflow-e2e-verification-contract.md"
  verifies: []
  verified-by:
    - "../test-specs/workflow-e2e-verification-contract.md"
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md"
    - "crates/cheaper-workflow/src/catalog.rs"
    - "crates/cheaper-cli/resources/policy.yaml"
  defers: []
  deferred-by: []
---

# Workflow E2E and APM distribution verification approach

## Context

Epic #200's four slices are on main but only slice-locally tested.
#205 needs evidence that the assembled conduit — L0 trigger text → L1
read-only inspection → L2 authoring+registration → explicit-path
execution — works against the real binary and the real APM payload,
without billable calls or side effects. The design problem is choosing
fixtures and isolation that exercise real behavior while remaining
reproducible and cost-free.

## Scope

### In Scope

- CLI surface verification with `target/debug/cheaper.exe` (same code
  as the distributed binary) in a scratch workspace.
- Scratch `CHEAPER_CONFIG_DIR` so all run artifacts (history records)
  land outside the real user config and stay measurable.
- A tool-only fixture workflow (`count_tokens` + `grep`, both in the
  builtin `tools.allow`) proving `workflow run` end-to-end.
- Invalid-definition fixtures covering every message in the L2 skill's
  error table; catalog fixtures covering both fail-closed manifest
  errors and per-entry diagnostics.
- Scratch APM consumers (`path:` dep on `packages/apm`) for
  `agent-skills` + harness targets (`windsurf`, `claude` as the
  differing deploy roots).
- Static verification: skill bodies vs CLI help vs guides vs doc
  indexes; L0/L1/L2 skill text invariants that no CLI can enforce.

### Out of Scope

- Worker-job workflows (model-backed) — billing + already covered by
  engine tests / `CHEAPER_E2E_OLLAMA`.
- `command`/`process` jobs succeeding — verified to fail closed;
  supplying a grant would change the permission surface under test.
- Consumer-agent runtime auto-firing (unverifiable here; documented).
- Repo `.cheaper/` writes — none are performed.

## Components and Boundaries

- **Scratch workspace** (`$TEMP/cheaper-e2e-205/ws`): holds
  `src/*.rs` fixtures and `.cheaper/workflows/` for authored
  definitions + `catalog.json`. Nothing is committed.
- **Isolated config dir** (`$TEMP/cheaper-e2e-205/config`): the only
  place run artifacts may appear; also the probe for "read-only
  commands write nothing".
- **Debug binary**: `target/debug/cheaper.exe`, built via
  `mise exec -- cargo build -p cheaper-cli`.
- **Fixture workflows**: `file-inspect.json` (valid tool-only),
  `run-git.json` (valid statically, fails closed at run), and a set of
  deliberately invalid definitions (malformed JSON, v1 shape, unknown
  names, binding violations, cycle, impossible pointer).
- **Catalog fixtures**: absent, valid, mixed-status entries
  (duplicate/missing/out-of-workspace/malformed), corrupted manifest
  (bad JSON, wrong schemaVersion, non-array), field-cap violation.
- **Scratch APM consumers**: three `apm.yml` variants —
  `agent-skills` only, `windsurf`, `claude` — each with a single
  `path:` dependency on `packages/apm`.
- **Doc-chain artifacts**: the six generated docs + test-spec +
  impl-record + experiment log under `docs/`.

## Data and Control Flow

1. Absent catalog → `workflow list` → `present:false`, empty list.
2. Author `file-inspect.json` → `workflow interface` → `ok:true` with
   public `{inputs,outputs}` projection only.
3. Append catalog entry → `workflow list` → `status:"ok"`;
   `--format text` and `--limit` truncation verified.
4. `workflow run` with `--input <file>` and `--input -` → `ok:true`,
   `result` = bound public outputs only, `jobs[]` metadata without
   private inputs/results.
5. Negative battery → each invalid definition/manifest reproduces its
   documented `INVALID_REQUEST`/`INVALID_INPUT` + exit 2.
6. Permission probe → `run-git.json` `command` job → `POLICY_VIOLATION`
   at run (exit 6) while `interface` passes; `workflow run <id>` →
   `cannot read workflow file` (no named lookup); inline `--input`
   JSON → `cannot read input file`.
7. Read-only probe → history record count unchanged across
   `interface`/`list`/`count-tokens`/`route check`; workspace
   `.cheaper/` untouched.
8. APM consumers → `apm install` per target; deployed tree + SKILL.md
   front matter + hook bundle roots compared to the guide matrix.
9. Docs pass → `doc_status lint`/`audit`, index-vs-front-matter scan,
   guide claims vs `--help` and `catalog.rs` constants.
10. Evidence → every step's measured output appended to the
    experiment log.

## Non-Functional Constraints

- Zero billing: no provider calls anywhere; model-backed paths are
  explicitly excluded.
- Zero persistent side effects outside scratch dirs: no grant/policy/
  Route writes, no repo `.cheaper/` writes, no commits of scratch.
- Reproducible: all fixtures are inline here-documents or tiny JSON;
  the exp log records the commands and outcomes.

## Risks and Trade-offs

- Debug binary may differ from a release build — accepted: the
  contract surface (envelope/diagnostics/exit codes) is shared code;
  no performance claims are made from debug timings.
- Windows-only verification — accepted: APM deploy paths are
  target-keyed directories, and POSIX launchers are static files
  verified by content, not execution.
- Skill-text invariants cannot be machine-enforced — accepted:
  verified by reading the shipped SKILL.md against the issue's
  expectations and recording quotes in the evidence.

## Deferred Design Concerns

- Live consumer-runtime firing — same deferral as the spec.

## Verification Notes

- [x] Plan matches the executed steps and the recorded experiment log.
