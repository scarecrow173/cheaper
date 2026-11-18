# logistics-gateway

Internal service workspace. Postmortem and incident reports for each
service live under `docs/incidents/` — one file per incident.
Generated reports go under `reports/`.

## Quarterly root-cause digest

`reports/root-cause-digest.md` clusters all incident reports by their
root-cause statement and lists the recurring patterns. Regenerate it
each quarter:

```bash
npm run digest
```

This runs `scripts/digest-incidents.mjs`, which reads every
`docs/incidents/INC-*.md` file, takes the first line of the last
`## ` section as the root cause, and groups incidents by that text.
Options: `--max <n>` changes the pattern cap (default 5), `--out <path>`
changes the output file. If a known cause is worded differently in a
new report it shows up as a separate pattern — normalize the wording
in that report and re-run.
