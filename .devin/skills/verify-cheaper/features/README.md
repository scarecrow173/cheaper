# Feature map — verify-cheaper

User-facing surface of the `cheaper` CLI that this skill drives. Each
file names the user journey, the drive commands, and the observable end
state that proves it. Drive the journey as written; when a journey
fails, report it — never weaken it.

| Feature | What the user does | Proof anchor |
|---|---|---|
| [workflow-list](workflow-list.md) | list the project workflow catalog | `result.workflows[].status` + `diagnostics[]` |
| [workflow-interface](workflow-interface.md) | statically validate + read a workflow's public contract | `result.inputs/outputs`, exit 0 / `error.code` exit 2 |
| [workflow-run](workflow-run.md) | execute a workflow file by explicit path | `result.jobs[].status` + `result.outputs` + `usage.jsonl` |
| [usage-history](usage-history.md) | inspect recorded delegation telemetry | `result.runs`/`result.totals`, `kind:"run"` records |
| [skill-claims](skill-claims.md) | (verification target) SKILL.md command/table claims vs real binary | every claim passes on `target/debug/cheaper` |
