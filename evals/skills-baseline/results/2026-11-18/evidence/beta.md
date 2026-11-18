# beta — commands run and files touched (verbatim extracts)

## Files created
- `reports/api-inventory.md` — generated inventory: one table per domain (Method / Path / Description / Source file), sorted by path then method.
- `scripts/build-api-inventory.mjs` — generator. Scans `docs/**/*.md`, derives domain from file H1 (filename prefix fallback), groups in specified domain order (unknown domains appended alphabetically), sorts deterministically, warns on duplicate endpoint declarations.

## Files edited
- `package.json` — added `"inventory": "node scripts/build-api-inventory.mjs"`.
- `README.md` — one line noting `npm run inventory` regenerates the report.

## Commands run
- `npm run inventory` (twice) — wrote the report, then re-ran and diffed to confirm byte-identical output.
