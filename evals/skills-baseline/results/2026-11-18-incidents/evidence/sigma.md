# sigma — commands run and files touched (verbatim extracts)

## Files created
- `reports/root-cause-digest.md` — digest: 5 patterns, per-pattern member + representative incident IDs, fixes, cross-cutting observations.
- `scripts/extract-incident-causes.sh` — deterministic extractor (id/service/date/section/cause TSV; `--hist` groups identical causes). Uses the last `##` section; warns on template-breaking files. ~0.2s runtime.
- `docs/runbooks/quarterly-root-cause-digest.md` — quarterly procedure: script path (baseline) + a delegated workflow path.
- `.cheaper/workflows/incident-root-cause-digest.json` — Workflow JSON v1: `bulk-reader` job → `doc-writer` job. Optional inputs `incidentGlobs`, `reportPath`.
- `.cheaper/workflows/catalog.json` — catalog registration (`incident-root-cause-digest`).

## Commands run
- `scripts/extract-incident-causes.sh` and `--hist` — output verified against manual reads.
- `cheaper workflow interface .cheaper/workflows/incident-root-cause-digest.json --cwd .` → `ok: true`.
- Script edge cases: missing dir → exit 2 with error; `--help` works.
