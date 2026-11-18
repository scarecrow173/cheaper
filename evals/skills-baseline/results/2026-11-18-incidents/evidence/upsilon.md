# upsilon — commands run and files touched (verbatim extracts)

## Files created
- `reports/root-cause-digest.md` — digest: 5 patterns, each with representative + all member incident IDs, symptoms, fix directions. Re-run procedure section included.
- `.cheaper/workflows/root-cause-digest.json` — Workflow JSON v1: `bulk-reader` (pattern extraction) + `doc-writer` (report generation) independent jobs. Inputs `incidentPaths`/`reportPath` optional with defaults; outputs `patterns`/`analysisSummary`/`reportSummary`.
- `.cheaper/workflows/catalog.json` — catalog registration (`root-cause-digest`).

## Commands run
- `cat`/`grep`/`awk` — extracted titles, symptoms, root-cause sections of all 30 files.
- `cheaper bulk-read` (attempt) → failed: `MODEL_TIER_UNAVAILABLE` (exit 5). No provider configured (`cheaper auth status` → all configured:false; `cheaper models` empty).
- `cheaper workflow interface .cheaper/workflows/root-cause-digest.json` → `ok: true` (static validation pass). Workflow NOT executed.
