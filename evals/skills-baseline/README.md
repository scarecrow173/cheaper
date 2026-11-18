# Skills baseline harness

A repeatable baseline for evaluating changes to the consumer-facing
skills in `packages/apm/.apm/skills/` (`cheaper-usage`,
`cheaper-bulk-read`, `cheaper-code-write`, `cheaper-doc-write`,
`cheaper-tool-operate`, `cheaper-workflow-discover`,
`cheaper-workflow-create`). A skill change is "better" only if a blind
run of the same task scores higher against the same rubric.

## Variant under test

The current skill set as deployed to a consumer workspace
(`.devin/skills/`). Two arms, identical everything else:

- **arm A** — workspace carries `.devin/skills/` (copied from
  `packages/apm/.apm/skills/`) plus one `AGENTS.md` line pointing at
  them, as an organic repo would.
- **arm B** — control; identical corpus and prompt, no skills, no
  pointer line.

## Harness layout

| Piece | Path | Sees it |
|---|---|---|
| Setup script | `evals/skills-baseline/setup.sh` | operator only |
| Candidate prompt | `evals/skills-baseline/prompt.md` | verbatim to all candidates |
| Rubric | `evals/skills-baseline/rubric.md` | judge only |
| Workspaces | `G:\repositry\wsp-{marble,falcon,cobalt,ember}/` (outside the repo — the path must not leak the product name) | candidates |
| Config isolation | `~/.cache/wsp-cfg/<wsp>/` + `~/bin/cheaper{,.cmd}` | candidates (transparent) |
| Baseline report | `evals/skills-baseline/results/` | operator + judge |

Arm mapping (operator-only; also in `setup.sh` header): inventory A =
`wsp-marble`, `wsp-falcon`, B = `wsp-cobalt`, `wsp-ember`; incidents A
= `wsp-onyx`, `wsp-larch`, B = `wsp-flint`, `wsp-cinder`. The judge
sees names only.

Two scenarios exist because the first was non-discriminative — see
`results/baseline-2026-11-18.md` §1. `inventory` measures nothing
about the skills (both arms' correct answer is a deterministic
script); `incidents` discriminates on delegation/workflow process.

## Safety isolation

- `~/bin/cheaper` shadows the mise shim **only** when `cwd` contains a
  `wsp-*` workspace name; it exports `CHEAPER_CONFIG_DIR=~/.cache/wsp-cfg/<wsp>`
  and execs the real binary. Everywhere else it passes through
  untouched.
- Effect: candidates get a real `cheaper` CLI in an unconfigured
  consumer state — local commands (`count-tokens`, `workflow
  interface`, `route resolve`) work; `run`/worker aliases fail at
  provider resolution. Real credentials are unreachable and
  `~/.config/cheaper/usage.jsonl` is never written.
- **Cleanup:** delete `~/bin/cheaper` and `~/bin/cheaper.cmd` after a
  run. `rm -rf G:/repositry/wsp-* ~/.cache/wsp-cfg` to drop workspaces
  and isolated config state.

## Run

```bash
bash evals/skills-baseline/setup.sh inventory   # wsp-marble/falcon/cobalt/ember
bash evals/skills-baseline/setup.sh incidents   # wsp-onyx/larch/flint/cinder
```

Each run takes ~2 min on Windows (fork-heavy). Then spawn one
subagent per workspace with the verbatim prompt (`prompt.md` for
inventory; the incidents prompt is in
`results/2026-11-18-incidents/` judge brief). After all four
complete, one blind judge scores every workspace's artifacts against
`rubric.md`.

## Known deviations from the eval playbook (Devin)

- **Model held constant across arms and judge.** The variant under
  test is the skill set, not the model; the judge also runs
  `subagent_general` (parent model) — `pstack:poteto-agent`
  (Claude-pinned) was attempted for family separation but rejected by
  the session's model policy (inherit-parent). Blinding relies on
  sanitized labels + artifact-only judging instead of family
  separation.
- **No transcript verification.** Devin exposes no stable
  `agent-transcripts/` path. Chain-following is graded from workspace
  artifacts (`reports/`, drafted workflows, `.cheaper/`), the isolated
  `wsp-cfg/<wsp>/` state (history/usage records), and the candidate's
  command report — never self-assessed compliance.
- **Skills are read, not loaded.** Devin loads project skills at
  session start; a subagent cannot get a different skill registry. Arm
  A therefore plants `.devin/skills/` as files plus an `AGENTS.md`
  pointer — this measures *following* skills once found, slightly
  weaker than measuring auto-firing. The `AGENTS.md` pointer keeps
  discoverability deterministic across arms.
