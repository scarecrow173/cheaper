# tau — commands run and files touched (verbatim extracts)

## Files created
- `scripts/digest-incidents.mjs` — automation script. Scans `docs/incidents/INC-*.md`, takes first line of the last `##` section as the root cause, clusters by normalized text, writes `reports/root-cause-digest.md`. `--max <n>` (default 5) caps patterns; `--out <path>` changes output. Overflow listed under "Further patterns".
- `reports/root-cause-digest.md` — this run's digest (script-generated; covers 30/30 incidents).

## Files edited
- `package.json` — added `"digest": "node scripts/digest-incidents.mjs"`.
- `README.md` — "Quarterly root-cause digest" section with re-run instructions.

## Commands run
- `node scripts/digest-incidents.mjs` / `npm run digest` — digest generation (30 incidents → 5 patterns).
- `node scripts/digest-incidents.mjs --max 3 --out /tmp/digest-test.md` — verified `--max` and overflow appendix.
- `ls`, `grep -n '^## '`, `tail`/`grep -m1`, `node --version` — extraction checks.
