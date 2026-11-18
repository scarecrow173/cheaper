# gamma — commands run and files touched (verbatim extracts)

## Files created
- `scripts/api-inventory.mjs` — Node script (no deps) scanning `docs/*.md`, extracting `### <METHOD> /<path>` headings plus following description line, domain from filename prefix. Deterministic: no timestamps, stable sort, byte-identical re-runs. Emits "Duplicate definitions" section when the same method+path appears in two files.
- `reports/api-inventory.md` — 63 endpoints across 6 domains (billing 12, auth 12, notify 10, ledger 9, catalog 10, shipping 10), grouped in requested order, sorted by path, rows carry method, path, description, source-file link.

## Files edited
- `package.json` — added `"inventory": "node scripts/api-inventory.mjs"`.
- `README.md` — regeneration note `npm run inventory`.

## Commands run
- `grep -P '\b(GET|POST|PUT|PATCH|DELETE|HEAD|OPTIONS)\s+/' docs` — ground truth: 63 endpoints in uniform heading format.
- `npm run inventory` — generated the report.
- Cross-check: extracted METHOD+path pairs docs vs report → diff IDENTICAL (63/63).
- Re-ran `npm run inventory` → sha256 byte-identical.
