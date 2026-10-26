---
id: "TASK-034MCBY6d6KRQNWwa96Qoo"
type: "task"
status: "done"
title: "Synchronize Node boundary records after issue 188"
created: "2026-10-26"
updated: "2026-10-26"
owners: []
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
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
  implemented-by: []
  depends-on:
    - "docs/plans/remove-ambient-node-from-rust-tests-issue-188.md"
    - "docs/tasks/remove-ambient-node-from-cheaper-tools-fixtures-issue-188.md"
    - "docs/tasks/remove-ambient-node-from-workflow-fixture-issue-188.md"
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
    - "docs/test-specs/ambient-node-free-rust-test-contract-issue-188.md"
  references: []
  defers: []
  deferred-by: []
---

# Synchronize Node boundary records after issue 188

## Work

After the cheaper-tools and workflow fixtures pass, synchronize the accepted
parent boundary design, historical inventory, accepted ADR, and repository
guidance in `AGENTS.md` with the verified F1 coverage. Preserve the original
ADR decision rationale and historical inventory facts with dated context.
Update canonical generated indexes and the manual implementation-record index.

## Classification

- `normal-plan-task`

## Upstream Documents

- `docs/plans/remove-ambient-node-from-rust-tests-issue-188.md`
- `docs/specs/remove-ambient-node-test-coupling-issue-188.md`
- `docs/designs/remove-ambient-node-from-rust-tests-issue-188.md`
- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`

## Dependencies

- `TASK-034MCBEcBex2dohPIO7AKU` and `TASK-034MCBOZ7xsFWR2eROLiV5` must be
  complete before this documentation task starts.

## Affected Files

- `docs/designs/node-dev-tooling-boundary-contract-issue-182.md`
- `docs/discovery/node-dev-tooling-inventory-issue-182.md`
- `AGENTS.md`
- `docs/adr/keep-node-js-as-the-pinned-dev-tooling-runtime.md`
- `docs/specs/README.md`, `docs/designs/README.md`, `docs/plans/README.md`
- `docs/test-specs/README.md`, `docs/tasks/README.md`, `docs/impl/ir/README.md`
- `docs/impl/ir/synchronize-node-boundary-records-after-issue-188.md`

## Done When

- [x] Boundary docs and `AGENTS.md` describe actual post-change three-site coverage, six
  runtime declaration translation, and bare-name test ownership accurately.
- [x] ADR accepted decision and historical rationale remain intact; dated note
  records the F1 implementation result and limitation.
- [x] Generated indexes and manual IR index are synchronized; focused doc and
  IR audits report no target-file blockers.

## Verification

- [x] Run `node .agents/skills/doc-driven-dev-graph/scripts/inspect_graph.js --cwd . --focus docs/plans/remove-ambient-node-from-rust-tests-issue-188.md --format json` and inspect target graph warnings (task graph issues: none).
- [x] Run `node .agents/skills/doc-status/scripts/doc_status.js audit --type all --json` and `.agents/skills/impl-doc/scripts/audit_impl_record.js --json`; filter target issue188 docs for relation/index/IR findings (0 blocking, 0 target findings).
- [x] Review all matches from the stale-claim `rg` command in the plan; historical matches must be explicitly dated.
- [x] `git diff --exit-code e5e26284de8a21ac9433e6470811ffd049448853 -- crates/cheaper-tools/tests/fixtures/process.json crates/cheaper-tools/tests/fixtures/runtime.json` exits 0.
