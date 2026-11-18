# alpha — commands run and files touched (verbatim extracts)

## Files created
- `reports/api-inventory.md` — 63 endpoints across 24 `docs/*.md` files, grouped into the 6 requested domains in the specified order (billing 12, auth 12, notify 10, ledger 9, catalog 10, shipping 10). Rows: `Method | Path | Description | Source doc`, sorted by path within domain.
- `scripts/build-api-inventory.mjs` — deterministic Node generator (no deps). Scans `docs/*.md` for `### <METHOD> <path>` headings plus following description line, derives domain from filename prefix. No timestamps; unknown domain prefixes sort alphabetically after the six known ones.

## Files edited
- `package.json` — added `"api-inventory": "node scripts/build-api-inventory.mjs"`.

## Commands run
- `node scripts/build-api-inventory.mjs` (equivalent `npm run api-inventory`)
- `grep -rhoE '^### (GET|POST|...) [^ ]+' docs` — format + coverage verification
- Report verified: method+path set diff-matched against independent grep extraction (63/63); second run byte-identical.
