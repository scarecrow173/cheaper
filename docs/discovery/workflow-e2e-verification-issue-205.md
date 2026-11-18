---
id: "DISC-034ZwWbWY4BGMQ7jf5Jjyd"
type: "discovery"
status: "resolved"
title: "Workflow discovery+authoring E2E, APM distribution, and docs verification for issue 205"
created: "2026-11-18"
updated: "2026-11-18"
owners: []
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/205"
    - "https://github.com/scarecrow173/cheaper/issues/200"
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
    - "./workflow-catalog-issue-201.md"
    - "./l0-workflow-candidate-trigger-issue-202.md"
    - "./workflow-discover-skill-issue-203.md"
    - "./workflow-create-skill-issue-204.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/workflow-e2e-verification-issue-205.md"
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-workflow-discover/SKILL.md"
    - "packages/apm/.apm/skills/cheaper-workflow-create/SKILL.md"
    - "packages/apm/apm.yml"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
    - "docs/guides/usage.md"
    - "docs/guides/distribution.md"
    - "crates/cheaper-workflow/src/catalog.rs"
    - "crates/cheaper-cli/resources/policy.yaml"
  defers: []
  deferred-by: []
---

# Workflow discovery+authoring E2E, APM distribution, and docs verification for issue 205

## Exploration Goal

Issue #205 is the closing verification issue of epic #200: prove, in an
environment close to a real APM consumer, that the whole
discovery→proposal→approval→creation→reuse path works — the #201
catalog + `workflow list`, the #202 L0 judgment rule in `cheaper-usage`,
the #203 L1 `cheaper-workflow-discover` skill, and the #204 L2
`cheaper-workflow-create` skill — and that the APM package distributes
all of it consistently with the guides. The questions to settle: which
parts can be measured end-to-end with the real CLI vs which can only be
verified statically (skill bodies are parent-agent instructions, not
executable code), what fixtures avoid billable model calls and side
effects, and whether guide/skill/index text has drifted from the
shipped behavior.

## Key Issues and Assumptions

- Confirmed: all five deliverables are on main — `workflow list` +
  catalog manifest (#201), L0 section in `cheaper-usage` (#202),
  `cheaper-workflow-discover` (#203), `cheaper-workflow-create` (#204).
  `packages/apm/.apm/skills/` carries exactly 7 skills matching the
  "skills（7 件）" claim in `parent-agent-integration.md`.
- Confirmed: the builtin policy (`crates/cheaper-cli/resources/
  policy.yaml`) allows `read`/`grep`/`find`/`ls`/`write`/`edit`/
  `count_tokens` without grants; `command`/`process` resolve through
  the unified `exec` grant namespace and fail closed with no grant.
  A tool-only workflow (`grep` + `count_tokens`) is therefore a real
  end-to-end `workflow run` fixture with no billable calls, no grants,
  and no side effects.
- Confirmed: `workflow interface` runs the real compiler's static
  validation plus the public-interface projection — the same code path
  the L2 skill instructs — so `interface` acceptance is the correct
  authoring gate, and every documented error message can be exercised
  without execution.
- Confirmed: `workflow run` writes execution history records
  (`kind:"workflow"`/`kind:"job"`) under `<userConfigDir>/history/`;
  `workflow list`/`workflow interface`/`count-tokens`/`route check`
  write none. Pointing `CHEAPER_CONFIG_DIR` at a scratch dir isolates
  all run artifacts from the real user config.
- Confirmed: `catalog.json` manifest-level failures (bad JSON,
  `schemaVersion` ≠ 1, non-array `workflows`, >64 KiB, >256 entries,
  field caps) fail closed with `INVALID_INPUT` + exit 2; entry shape
  violations (`malformed-entry`/`duplicate-id`/`missing-path`/
  `path-out-of-workspace`/`denied-path`/`not-regular-file`) are
  per-entry diagnostics with `ok:true`. Both tiers are documented in
  `workflow-usage.md` and match `catalog.rs` exactly.
- Confirmed: `apm install` from a scratch consumer with a `path:` dep
  on `packages/apm` resolves the transitive `path:` hook packages,
  deploys all 7 skills to the target's skill root, and deploys only
  the active target's hook bundle — non-matching hook packages are
  skipped by per-dependency `targets:` non-overlap.
- Confirmed: skill bodies are the only surface the parent agent reads;
  L0's negative cases ("don't propose on one-shot work", "don't run
  catalog/history scans just for the check") are judgment-rule text,
  not enforceable by a CLI — they are verified by reading the shipped
  SKILL.md and by the absence of any hook/CLI mechanism, not by a
  runtime test.
- Assumption: an ADR is not warranted — the issue verifies existing
  behavior and records evidence; it introduces no new architectural
  decision. The verification boundary is a design concern only.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. Real CLI in a scratch workspace + scratch `CHEAPER_CONFIG_DIR`, tool-only workflow fixture, `path:`-dep APM consumers | Exercises the exact shipped binary and package a consumer gets; zero billing/side effects; every envelope and diagnostic is measured | Requires building `target/debug/cheaper.exe`; Windows-only evidence for hook launchers | yes |
| B. Rust integration tests only | Deterministic, CI-able | #201-#204 already ship contract tests; the issue asks for cross-cutting verification of the *assembled* path and the APM artifact, which tests in one crate cannot cover | no |
| C. Real APM consumer agents (Claude/Codex/Cursor UI) | Proves actual skill auto-firing | Not reproducible in this environment; runtimes' auto-firing is model-judgment-dependent by design — the guide already documents the limit and the manual fallback | no |
| D. Model-backed `workflow run` (worker jobs) | Would prove worker jobs end-to-end | Billable provider calls; worker jobs are already covered by `cheaper-workflow` engine tests and the Ollama e2e gate; out of scope for this verification | no |

## Tentative Conclusions and Hypotheses

- Confirmed: the assembled path is exercisable end-to-end without
  billing — catalog list → JSON v1 authoring → `interface` validation →
  catalog registration → `workflow list` status → tool-only
  `workflow run` — using only `CHEAPER_CONFIG_DIR` isolation.
- Confirmed: the approval boundary is observable as CLI behavior —
  `interface`/`list`/`route check`/`route resolve`/`count-tokens` are
  read-only (no workspace `.cheaper/` writes, no history records),
  while `command`-job workflows fail closed at run with
  `POLICY_VIOLATION` absent a grant.
- Hypothesis (to be measured): every error message in the create
  skill's invalid-table maps to a real `INVALID_REQUEST`/
  `INVALID_INPUT` envelope with exit 2 — confirmed during execution,
  with one nuance: `{"$from":"x"}` missing `path` is caught by the v1
  JSON schema (`invalid Workflow JSON v1`) in `outputs[].value`
  position and by `invalid binding wrapper` inside job `input`.
- Confirmed: doc index rows for the #201-#204 chain were already
  synced to front matter by commits `d80ad03`/`069b741` on main —
  `doc_status lint` reports no `index-metadata-mismatch`, so no repair
  is needed, only re-verification.

## Open Questions

- None blocking. Per-target agent runtimes' actual skill auto-firing
  stays unverifiable in this environment (documented limitation, same
  as issue's own caveat).

## Promotion Candidates

- [x] spec-doc needed: the issue's scenario matrix + acceptance criteria
  formalized as the verification contract
- [ ] adr-doc needed: none — verification-only work, no new decision

## Sources and Relations

Issue #205 (parent: #200; depends: #201 #202 #203 #204). Prior chain
docs for the four landed issues are listed in `related`; inspected
files in `references`. Promotes to
`docs/specs/workflow-e2e-verification-issue-205.md`.
