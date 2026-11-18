# logistics-gateway

Internal service workspace. API specifications for each service domain
live under `docs/` — one set of files per domain (billing, auth,
notify, ledger, catalog, shipping). Generated reports go under
`reports/`.

Regenerate the API endpoint inventory after docs change:

```bash
npm run inventory   # writes reports/api-inventory.md
```
