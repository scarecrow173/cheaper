# phi — commands run and files touched (verbatim extracts)

## Files created
- `reports/root-cause-digest.md` — digest: 5 root-cause patterns, each with representative incident ID, all member IDs, summary, common fixes.
- `scripts/extract-incident-root-causes.sh` — re-run script. Extracts root cause from each report's last `##` section; `--count` mode emits verbatim-cluster tallies (count + ID list).
- `docs/procedures/quarterly-root-cause-digest.md` — quarterly procedure doc: prerequisites, 4-step procedure, digest template structure, cautions.

## Commands run
- `awk`/`grep`/`sed`/`head` — root-cause extraction and frequency counts across all 30 `docs/incidents/*.md`.
- `bash scripts/extract-incident-root-causes.sh --count` — verbatim clusters verified (5 clusters × 6 each).
- `bash scripts/extract-incident-root-causes.sh` — runs from other cwd; no empty fields.
