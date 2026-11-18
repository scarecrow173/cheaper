# logistics-gateway

Internal service workspace. API specifications for each service domain
live under `docs/` — one set of files per domain (billing, auth,
notify, ledger, catalog, shipping). Generated reports go under
`reports/`.

After adding or changing docs, regenerate the endpoint inventory with
`npm run inventory` (verifies freshness with `npm run inventory:check`).
