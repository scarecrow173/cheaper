# usage / history

Inspect cheaper's own run telemetry — two stores under
`CHEAPER_CONFIG_DIR`: `usage.jsonl` (usage-bearing records:
`run`/`count`/`route`) and `history/<uuid>/{started,terminal}.json`
(execution lifecycle: `workflow`/`job`/`run`). Read views:
`cheaper usage`/`cheaper history`. Utility commands themselves are
never recorded.

## Sub-features

- `history list` — run records, `--limit`/`--cursor`/`--kind`/`--status`
- `usage` — aggregates; `measured` offload block, `--compare-model`
  estimated savings (clearly labeled estimated)
- Record taxonomy: `run`, `count` (count-tokens), `route`, `workflow`,
  `job`, `route-refetch`
- Non-recording commands: `workflow list`, `workflow interface`,
  `route check`, `route resolve`, `count-tokens` failure, auth,
  `history`, `usage` itself

## How to get to it (user POV)

`cheaper history list --cwd proj` to see what ran;
`cheaper usage --cwd proj` for totals.

## Driving it with the CLI

```bash
wc -l "$CFG/usage.jsonl" 2>/dev/null || echo 0   # baseline

"$BIN" workflow list --cwd "$WS"                  # read-only, records nothing
"$BIN" count-tokens "$WS/main.rs" --cwd "$WS"     # kind:"count" in usage.jsonl
"$BIN" history list                               # NOTE: no --cwd flag exists
"$BIN" usage                                      # aggregates (no --cwd needed)

wc -l "$CFG/usage.jsonl"                          # +1 (the count-tokens only)
find "$CFG/history" -name terminal.json | wc -l   # lifecycle records
```

## Gotchas

- Both stores live under `CHEAPER_CONFIG_DIR`, not the workspace —
  `.cheaper/` in `$WS` must stay free of telemetry.
- `history list` **rejects `--cwd`** (`INVALID_REQUEST: --cwd is not
  valid for 'history list'`) — it reads the config dir, so isolation is
  entirely `CHEAPER_CONFIG_DIR`. `usage --cwd` IS accepted (scopes
  workspace records); `history list` prints human text (`ok history` /
  `records: N`), not the JSON envelope.
- `workflow list`/`interface`/`route check`/`history`/`usage` write no
  records to either store — tool-only `workflow run` writes history/
  records but no usage.jsonl line (no model usage).
- `usage` records cheaper's own runs only — it cannot observe parent
  agent token counts. Claims must stay `measured`/`estimated`/`unknown`
  labeled.
