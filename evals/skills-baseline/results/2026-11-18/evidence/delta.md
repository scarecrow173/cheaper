# delta — commands run and files touched (verbatim extracts)

## Files created
- `reports/api-inventory.md` — 63 endpoints from all 24 files under `docs/`, grouped by domain in requested order (billing 12, auth 12, notify 10, ledger 9, catalog 10, shipping 10). Table: `Method | Path | Summary | Source`, sorted by path, links back to source doc.
- `scripts/generate-api-inventory.mjs` — regeneration script. Scans `docs/*.md`, extracts `### <METHOD> /<path>` headings plus one-line summaries, domain from filename prefix (fallback: `# <domain> —` title, then `other`). Six known domains first in canonical order; new domains appended alphabetically.

## Files edited
- `package.json` — added `npm run inventory` and `npm run inventory:check` (`--check` mode: exits 1 if report is stale).

## Commands run
- `node scripts/generate-api-inventory.mjs` (equivalent `npm run inventory`)
- `node scripts/generate-api-inventory.mjs --check` — verifies report matches docs
- Verification by runner: set-equality diff of every `### METHOD /path` heading vs report rows → 63/63; added temporary `docs/fraud-rules.md` → `--check` failed stale, regeneration produced `## fraud` section, removal + regeneration restored state.
