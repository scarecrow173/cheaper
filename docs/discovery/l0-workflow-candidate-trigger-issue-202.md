---
id: "DISC-034Zk080rxFtqiKnk4VQHE"
type: "discovery"
status: "resolved"
title: "L0 workflow candidate trigger rule for issue 202"
created: "2026-11-17"
updated: "2026-11-17"
owners: ["scarecrow173"]
relations:
  source:
    - "https://github.com/scarecrow173/cheaper/issues/202"
    - "https://github.com/scarecrow173/cheaper/issues/200"
    - "https://github.com/scarecrow173/cheaper/issues/203"
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
    - "./cross-agent-hook-distribution-and-contract-matrix.md"
    - "../designs/cross-agent-distribution-verification.md"
    - "../specs/synchronize-distributed-cheaper-usage-skill-and-parent-agent-guide.md"
  refines: []
  refined-by: []
  derives-from: []
  derived-by:
    - "../specs/l0-workflow-candidate-trigger-issue-202.md"
    - "../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md"
  verifies: []
  verified-by: []
  references:
    - "packages/apm/.apm/skills/cheaper-usage/SKILL.md"
    - "packages/apm/apm.yml"
    - "docs/guides/parent-agent-integration.md"
    - "docs/guides/workflow-usage.md"
    - "https://cursor.com/docs/skills"
    - "https://developers.openai.com/codex/skills/"
    - "https://docs.github.com/en/copilot/how-tos/copilot-cli/customize-copilot/add-skills"
    - "https://docs.windsurf.com/windsurf/cascade/skills"
  defers:
    - "https://github.com/scarecrow173/cheaper/issues/203"
  deferred-by: []
---

# L0 workflow candidate trigger rule for issue 202

## Exploration Goal

Issue #202 asks for an extremely small, always-on parent-agent judgment
rule ("L0") that notices workflow-candidate signals during normal work and
decides only whether the L1 skill (`cheaper-workflow-discover`, #203) is
worth loading. The questions to settle: which existing distribution
surface carries the rule, what each APM target can actually guarantee for
skill discoverability and auto-firing, and what the fallback is where
auto-firing cannot be guaranteed.

## Key Issues and Assumptions

- Confirmed: `packages/apm` distributes five skills; `cheaper-usage` is the
  hub CLI-contract skill whose front-matter `description` is the
  auto-invocation surface on skill-capable runtimes. Its body is read on
  demand, so a short L0 section inside it does not add per-turn cost.
- Confirmed: issue #202 forbids a standalone L0 skill body, a resident
  monitor, new hook events, and per-turn catalog/`history`/`usage`/file-scan
  calls. The rule must live in skill metadata (description/short firing
  rules), a consumer-facing guide instruction, or an opt-in paste template.
- Confirmed (apm 0.33 target map + `apm-usage` reference): skill deploy
  destinations are `.agents/skills/` for `agent-skills`/`codex`/`cursor`/
  `copilot`/`windsurf` targets and `.claude/skills/` for `claude`
  (`--legacy-skill-paths` restores per-client dirs such as
  `.github/skills/` and `.cursor/skills/`).
- Confirmed (upstream docs, checked 2026-11-17): Codex CLI scans
  `.agents/skills/` from the cwd up to the repo root (implicit invocation
  by `description`, explicit via `$name`/`/skills`); Cursor ≥2.4 scans
  `.agents/skills/` plus `.cursor/skills/`/`.claude/skills/`/
  `.codex/skills/` compat dirs (implicit + `/` menu); Copilot CLI scans
  `.agents/skills/`/`.github/skills/`/`.claude/skills/` (IDE agent-mode
  support is version-dependent); Windsurf Cascade scans `.agents/skills/`
  for cross-agent compatibility alongside `.devin/skills/`/
  `.windsurf/skills/`; Claude Code natively scans `.claude/skills/`; Devin
  CLI loads `.agents/skills/` (dogfooded in this repo).
- Confirmed: description-matched implicit invocation is model judgment on
  every runtime — never a deterministic guarantee. The issue explicitly
  forbids claiming always-on auto-firing across all targets.
- Assumption: `cheaper-workflow-discover` does not exist yet (#203 owns
  it). L0 text must not instruct the parent agent to invoke an unavailable
  capability — the handoff must be conditional ("if the skill is
  installed") with a today-possible fallback.

## Alternatives and Comparison

| Option | Pros | Cons | Lean |
| --- | --- | --- | --- |
| A. `cheaper-usage` skill metadata: compact trigger clause in `description` + short L0 section at the end of `SKILL.md` | Reuses the hub skill every consumer already deploys; description is the firing surface; body section is read on demand so unrelated tasks never load the rule | Widens `cheaper-usage` implicit firing to non-CLI contexts (intended by the issue); a triggered read pulls the whole SKILL.md (the L0 section still bounds the required action) | yes |
| B. New minimal standalone L0 skill | Cleanest separation; only candidacy text is loaded | Issue forbids an independent L0 skill body; would collide with the `cheaper-workflow-discover` skill #203 owns | no |
| C. Consumer guide instruction only (`parent-agent-integration.md`) | Zero runtime cost; documents the contract | Guides are not loaded into agent context — cannot fire by itself | no (required as complement) |
| D. Opt-in minimal instruction template users paste into their own `AGENTS.md`/rules | Always-on where pasted, independent of skill auto-firing | Opt-in means most consumers never see it; duplicates rule text outside the package | yes (as complement in the guide) |

## Tentative Conclusions and Hypotheses

Confirmed: option A is the carrier (ADR-034Zk1CgFRRcnAwV6sZWqQ), with
option C's guide section and option D's template as complements that cover
environments where skill auto-firing is absent or unguaranteed.

Confirmed: the fallback that works today is explicit skill invocation —
the user asks the agent to read the `cheaper-usage` skill's L0 section;
explicit invocation works even where implicit firing does not. Once #203
ships, the same explicit path reaches `cheaper-workflow-discover`. Before
then, L0's handoff degrades to reporting the observed pattern to the user.

Confirmed: the matrix the acceptance criteria require is per-target deploy
destination × runtime discovery × firing character. Every runtime that
reads the deployed location supports description-matched implicit
invocation, but none guarantees it; runtime version support for Agent
Skills is the binding constraint.

## Open Questions

- None blocking. Whether `cheaper-workflow-discover` lands under exactly
  that name is tracked in #203; L0 text references the skill name, so a
  rename there requires a one-line sync here.

## Promotion Candidates

- [x] spec-doc needed: L0 rule content, delivery surfaces, suppression
  conditions, and per-target firing limits formalized →
  `../specs/l0-workflow-candidate-trigger-issue-202.md`
- [x] adr-doc needed: which surface carries L0 (existing skill metadata vs
  new skill vs guide/template only) →
  `../adr/l0-workflow-trigger-lives-in-cheaper-usage-skill-metadata.md`

## Sources and Relations

Issue #202 is the primary product source; epic #200 fixes the L0/L1/L2
layering and #203 owns the L1 skill contract. Deploy-destination facts come
from the installed apm 0.33 target registry (`apm targets --all --json`)
and the `apm-usage` skill's cross-client skills section. Runtime
discoverability facts come from official docs: Cursor skills doc, OpenAI
Codex skills doc, GitHub Copilot add-skills doc, and Windsurf Cascade
skills doc (`.agents/skills/` cross-agent compatibility). The per-target
hook contract matrix (DISC-034r05iHy2gdnpdai9KGpW) is the precedent for
recording per-target verification honestly rather than claiming universal
behavior.
