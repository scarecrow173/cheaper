#!/usr/bin/env bash
# Build the four sanitized candidate workspaces for the skills baseline.
# Idempotent: deletes and recreates only the wsp-* dirs it owns, the
# per-workspace cheaper config dirs, and the ~/bin/cheaper shims.
#
# Arm mapping (recorded here only — never visible to candidates):
#   inventory:  A=wsp-marble,wsp-falcon  B=wsp-cobalt,wsp-ember
#   incidents:  A=wsp-onyx,wsp-larch     B=wsp-flint,wsp-cinder
# (A = skills deployed at .devin/skills/ + AGENTS.md pointer; B = control)
set -euo pipefail

REPO="$(cd "$(dirname "$0")/../.." && pwd)"
SKILLS_SRC="$REPO/packages/apm/.apm/skills"
# Workspaces live OUTSIDE the repo so the path never leaks the product
# name to a candidate; isolated config dirs live under ~/.cache.
TARGET="$(dirname "$REPO")"
CFG_ROOT="$HOME/.cache/wsp-cfg"
REAL_CHEAPER="/c/Users/akiyoshi/AppData/Local/mise/shims/cheaper.exe"

MODE="${1:-inventory}"
case "$MODE" in
  inventory)
    ARMS_A=(wsp-marble wsp-falcon); ARMS_B=(wsp-cobalt wsp-ember)
    DOC_GEN=write_domain_docs ;;
  incidents)
    ARMS_A=(wsp-onyx wsp-larch); ARMS_B=(wsp-flint wsp-cinder)
    DOC_GEN=write_incident_docs ;;
  *) echo "usage: $0 [inventory|incidents]" >&2; exit 2 ;;
esac
# shim covers every wsp name ever issued, so reruns keep older
# workspaces isolated too.
SHIM_NAMES=(wsp-marble wsp-falcon wsp-cobalt wsp-ember \
            wsp-onyx wsp-larch wsp-flint wsp-cinder)

# --- shared docs corpus -----------------------------------------------------

pad() {  # prose padding so each file lands ~6-9KB; rotates paragraphs
  local n="$1" i
  for i in $(seq "$n"); do
    case $((i % 3)) in
    0) cat <<'TXT'
All endpoints in this section require an authenticated principal with
the `api:read` or `api:write` scope. Responses follow the common
envelope: `{"ok": true, "result": ...}` on success and
`{"ok": false, "error": {"code": ..., "message": ...}}` on failure.
Rate limits apply per workspace: 600 requests per minute for read
methods and 120 per minute for write methods.
TXT
    ;;
    1) cat <<'TXT'
Idempotency keys are accepted on all POST endpoints via the
`Idempotency-Key` header and are retained for 24 hours. Pagination uses
cursor-based continuation with `limit` (default 50, max 200) and
`cursor` query parameters; the `result.nextCursor` field is null at the
end of a list.
TXT
    ;;
    2) cat <<'TXT'
Timestamps are RFC 3339 UTC. Monetary amounts are integer minor units
in the workspace settlement currency. Enum fields are extensible —
clients must ignore unrecognized values. Webhook signatures use HMAC
SHA-256 over the raw request body with a per-endpoint secret.
TXT
    ;;
    esac
  done
}

ep() {  # endpoint entry
  printf '### %s %s\n\n%s\n\n| Param | In | Type | Notes |\n|---|---|---|---|\n| workspace | header | string | required |\n\n' "$1" "$2" "$3"
}

write_domain_docs() {
  local dir="$1"
  declare -A docs
  docs[billing-overview.md]="
$(ep 'POST' '/v1/invoices' 'Create a draft invoice for a subscription or one-off charge.')
$(ep 'GET' '/v1/invoices/{id}' 'Fetch a single invoice with line items.')
$(ep 'POST' '/v1/invoices/{id}/void' 'Void an open invoice; emits billing.invoice.voided.')
$(ep 'GET' '/v1/subscriptions' 'List subscriptions for the workspace.')"
  docs[billing-payments.md]="
$(ep 'POST' '/v1/payments' 'Capture a payment against an invoice or customer balance.')
$(ep 'GET' '/v1/payments/{id}' 'Fetch a payment with its attempt history.')
$(ep 'POST' '/v1/refunds' 'Refund a captured payment, full or partial.')"
  docs[billing-subscriptions.md]="
$(ep 'PATCH' '/v1/subscriptions/{id}' 'Change plan, quantity, or billing anchor.')
$(ep 'GET' '/v1/credit-notes' 'List credit notes issued in the period.')"
  docs[billing-payouts.md]="
$(ep 'POST' '/v1/payouts' 'Trigger a payout to the default settlement account.')
$(ep 'GET' '/v1/tax-rates' 'List tax rates applicable to the workspace.')
$(ep 'POST' '/v1/dunning/retry' 'Retry dunning for a past-due invoice.')"
  docs[auth-oauth.md]="
$(ep 'POST' '/v1/oauth/token' 'Exchange a code or refresh token for an access token.')
$(ep 'POST' '/v1/oauth/revoke' 'Revoke an access or refresh token.')
$(ep 'GET' '/v1/oauth/userinfo' 'Return claims for the presented access token.')"
  docs[auth-sessions.md]="
$(ep 'POST' '/v1/sessions' 'Create an interactive session from credentials.')
$(ep 'DELETE' '/v1/sessions/{id}' 'Terminate a session and its refresh chain.')"
  docs[auth-mfa.md]="
$(ep 'POST' '/v1/mfa/enable' 'Start TOTP enrollment; returns provisioning URI.')
$(ep 'POST' '/v1/mfa/verify' 'Verify a TOTP code and activate MFA.')"
  docs[auth-api-keys.md]="
$(ep 'GET' '/v1/api-keys' 'List API keys for the workspace.')
$(ep 'POST' '/v1/api-keys' 'Create an API key with scoped permissions.')
$(ep 'DELETE' '/v1/api-keys/{id}' 'Revoke an API key immediately.')
$(ep 'POST' '/v1/password/reset' 'Send a password reset email.')
$(ep 'POST' '/v1/password/change' 'Change password with current credential proof.')"
  docs[notify-send.md]="
$(ep 'POST' '/v1/notifications' 'Send a notification over one or more channels.')
$(ep 'GET' '/v1/notifications/{id}' 'Fetch delivery state per channel.')"
  docs[notify-templates.md]="
$(ep 'POST' '/v1/templates' 'Create a reusable message template.')
$(ep 'GET' '/v1/templates/{slug}' 'Fetch a template by slug.')"
  docs[notify-channels.md]="
$(ep 'POST' '/v1/channels/email/test' 'Send a test message through the email channel.')
$(ep 'GET' '/v1/subscriptions/topics' 'List topic subscriptions for the workspace.')"
  docs[notify-broadcasts.md]="
$(ep 'POST' '/v1/topics' 'Create a broadcast topic.')
$(ep 'POST' '/v1/broadcasts' 'Broadcast a message to a topic.')
$(ep 'GET' '/v1/delivery/{id}' 'Fetch per-recipient delivery detail.')
$(ep 'POST' '/v1/webhooks' 'Register a webhook endpoint for delivery events.')"
  docs[ledger-entries.md]="
$(ep 'POST' '/v1/entries' 'Append a double-entry transaction.')
$(ep 'GET' '/v1/entries/{id}' 'Fetch an entry with its postings.')"
  docs[ledger-accounts.md]="
$(ep 'GET' '/v1/accounts' 'List ledger accounts.')
$(ep 'POST' '/v1/accounts' 'Open a ledger account.')
$(ep 'GET' '/v1/accounts/{id}/balance' 'Compute the current account balance.')"
  docs[ledger-transfers.md]="
$(ep 'POST' '/v1/transfers' 'Move funds between accounts atomically.')"
  docs[ledger-reports.md]="
$(ep 'GET' '/v1/journals' 'List journal entries for a period.')
$(ep 'POST' '/v1/reconciliations' 'Start an account reconciliation run.')
$(ep 'GET' '/v1/reports/trial-balance' 'Fetch the trial balance report.')"
  docs[catalog-products.md]="
$(ep 'GET' '/v1/products' 'List products with filters.')
$(ep 'POST' '/v1/products' 'Create a product.')
$(ep 'GET' '/v1/products/{sku}' 'Fetch a product by SKU.')
$(ep 'PATCH' '/v1/products/{sku}' 'Update product fields.')"
  docs[catalog-categories.md]="
$(ep 'DELETE' '/v1/products/{sku}' 'Archive a product.')
$(ep 'GET' '/v1/categories' 'List categories.')
$(ep 'POST' '/v1/categories' 'Create a category.')"
  docs[catalog-search.md]="
$(ep 'GET' '/v1/search' 'Full-text search across products and categories.')"
  docs[catalog-import.md]="
$(ep 'POST' '/v1/import/jobs' 'Start a CSV bulk import.')
$(ep 'GET' '/v1/import/jobs/{id}' 'Check import job progress.')"
  docs[shipping-shipments.md]="
$(ep 'POST' '/v1/shipments' 'Create a shipment from an order.')
$(ep 'GET' '/v1/shipments/{id}' 'Fetch a shipment with items.')
$(ep 'POST' '/v1/shipments/{id}/cancel' 'Cancel a shipment before pickup.')"
  docs[shipping-carriers.md]="
$(ep 'GET' '/v1/carriers' 'List enabled carriers and service levels.')"
  docs[shipping-rates.md]="
$(ep 'POST' '/v1/rates/quote' 'Quote shipping rates for a parcel.')
$(ep 'GET' '/v1/tracking/{tracking}' 'Fetch tracking events for a label.')"
  docs[shipping-ops.md]="
$(ep 'POST' '/v1/labels' 'Purchase and render a shipping label.')
$(ep 'GET' '/v1/pickups' 'List scheduled carrier pickups.')
$(ep 'POST' '/v1/pickups' 'Schedule a carrier pickup.')
$(ep 'GET' '/v1/addresses/{id}/validate' 'Validate a stored address.')"

  for name in "${!docs[@]}"; do
    local domain="${name%%-*}"
    {
      printf '# %s — %s\n\n' "$domain" "${name#*-}" | sed 's/.md//'
      printf 'Service domain documentation file.\n\n'
      pad 6
      printf '## Endpoints\n\n'
      printf '%s' "${docs[$name]}"
      pad 3
    } > "$dir/docs/$name"
  done
}

write_workspace() {
  local name="$1" with_skills="$2"
  local dir="$TARGET/$name"
  rm -rf "$dir"
  mkdir -p "$dir/docs"

  if [ "$MODE" = "inventory" ]; then
    cat > "$dir/README.md" <<'MD'
# logistics-gateway

Internal service workspace. API specifications for each service domain
live under `docs/` — one set of files per domain (billing, auth,
notify, ledger, catalog, shipping). Generated reports go under
`reports/`.
MD
  else
    cat > "$dir/README.md" <<'MD'
# logistics-gateway

Internal service workspace. Postmortem and incident reports for each
service live under `docs/incidents/` — one file per incident.
Generated reports go under `reports/`.
MD
  fi
  cat > "$dir/package.json" <<'JSON'
{
  "name": "logistics-gateway",
  "private": true,
  "version": "0.0.0",
  "scripts": { "test": "echo \"no tests yet\"" }
}
JSON

  if [ "$with_skills" = "1" ]; then
    cat > "$dir/AGENTS.md" <<'MD'
# AGENTS.md

Agent work happens inside this directory only.

このリポジトリにはエージェント向けの補助ガイドが `.devin/skills/`
配下にある。大量ファイルの読み込み・機械的な文書やコードの生成・
繰り返し手順の定型化を行う前に、該当する SKILL.md を読むこと。
MD
    mkdir -p "$dir/.devin/skills"
    cp -r "$SKILLS_SRC/." "$dir/.devin/skills/"
  else
    cat > "$dir/AGENTS.md" <<'MD'
# AGENTS.md

Agent work happens inside this directory only.
MD
  fi

  $DOC_GEN "$dir"
  mkdir -p "$CFG_ROOT/$name"
}

# --- incident corpus (scenario: incidents) ----------------------------------
# 30 unstructured postmortem reports across 6 services. Five latent
# root-cause categories are planted (6 incidents each) with varied
# phrasing so the digest cannot be produced by pattern-matching.
# Categories: pool-exhaustion, cert-expiry, queue-backlog,
# config-drift, upstream-timeout.

incident_prose() {
  local svc="$1" date="$2" symptom="$3"
  cat <<EOF
# $svc — incident report ($date)

## Summary

$symptom The incident was detected by synthetic monitoring and paged
the on-call rotation. Customer-visible impact lasted until mitigation
completed; no data loss was observed.

## Timeline

- Alert fired and on-call acknowledged.
- Triage narrowed the blast radius to the $svc path.
- Mitigation applied; error rates returned to baseline.
- Follow-up items filed for the owning team.

## Impact

Affected requests saw elevated latency and intermittent failures for
the duration. Retry-able operations eventually succeeded; interactive
flows surfaced user-facing errors. Roughly one third of traffic
through $svc was degraded at peak.

## Detection

Synthetic probes caught the failure before customer reports
accumulated. Dashboards showed the anomaly clearly once the right
panel was consulted; alert routing could have been faster.

## Remediation

Short-term mitigation was applied during the incident. Permanent
prevention items were ticketed for the next sprint, including better
runbook coverage and tighter alerting thresholds.
EOF
  pad 3
}

write_incident_docs() {
  local dir="$1"
  mkdir -p "$dir/docs/incidents"
  local services=(checkout search billing-sync notify-relay ledger-writer gateway-edge)
  local i=0 svc date
  # pool-exhaustion (6)
  for svc in checkout search billing-sync notify-relay ledger-writer gateway-edge; do
    i=$((i+1)); date="2026-0$(( (i%9)+1 ))-1$((i%9))"
    { incident_prose "$svc" "$date" "Requests began timing out under normal load."
      printf '\n## Root cause\n\n'
      printf 'The database connection pool was exhausted: pool size stayed at its default while concurrency grew, so requests queued on pool acquisition until they timed out. Raising the pool limit and adding an acquisition-timeout alert resolved it.\n'
      pad 2; } > "$dir/docs/incidents/INC-0$((100+i)).md"
  done
  # cert-expiry (6)
  for svc in checkout search billing-sync notify-relay ledger-writer gateway-edge; do
    i=$((i+1)); date="2026-0$(( (i%9)+1 ))-2$((i%9))"
    { incident_prose "$svc" "$date" "TLS handshakes started failing between internal services."
      printf '\n## What went wrong\n\n'
      printf 'An internal certificate had quietly reached its expiry date. Renewal automation covered only the public-facing certificates, not the service-mesh ones. Manual renewal restored mTLS; coverage was added to the renewal job.\n'
      pad 2; } > "$dir/docs/incidents/INC-0$((100+i)).md"
  done
  # queue-backlog (6)
  for svc in checkout search billing-sync notify-relay ledger-writer gateway-edge; do
    i=$((i+1)); date="2026-0$(( (i%9)+1 ))-0$((i%9+1))"
    { incident_prose "$svc" "$date" "Message processing lagged hours behind real time."
      printf '\n## Postmortem notes\n\n'
      printf 'Consumers could not keep up after one poison message forced repeated redeliveries; the backlog grew until the queue depth alarm fired. Dead-letter routing for unprocessable messages stopped the retry storm.\n'
      pad 2; } > "$dir/docs/incidents/INC-0$((100+i)).md"
  done
  # config-drift (6)
  for svc in checkout search billing-sync notify-relay ledger-writer gateway-edge; do
    i=$((i+1)); date="2026-0$(( (i%9)+1 ))-1$((i%9))"
    { incident_prose "$svc" "$date" "Behavior diverged between staging and production with no code change."
      printf '\n## Analysis\n\n'
      printf 'The deployed configuration had drifted from the declared manifest — a manual hotfix months earlier changed a feature flag that was never written back. Reconciling the manifest and adding a drift check to deploys closed the gap.\n'
      pad 2; } > "$dir/docs/incidents/INC-0$((100+i)).md"
  done
  # upstream-timeout (6)
  for svc in checkout search billing-sync notify-relay ledger-writer gateway-edge; do
    i=$((i+1)); date="2026-0$(( (i%9)+1 ))-2$((i%9))"
    { incident_prose "$svc" "$date" "Cascading failures rolled in from an external dependency."
      printf '\n## Root cause\n\n'
      printf 'An upstream partner API began exceeding its latency SLO; our client had no circuit breaker, so slow responses exhausted worker capacity and the failure cascaded inward. A bulkhead plus breaker now isolates the dependency.\n'
      pad 2; } > "$dir/docs/incidents/INC-0$((100+i)).md"
  done
}

# --- per-workspace cheaper config isolation ---------------------------------

write_shims() {
  mkdir -p "$HOME/bin"
  {
    echo '#!/usr/bin/env bash'
    echo '# evals/skills-baseline: redirect CHEAPER_CONFIG_DIR per wsp dir;'
    echo '# pass through untouched outside candidate workspaces.'
    for n in "${SHIM_NAMES[@]}"; do
      printf 'case "$PWD" in *%s*) export CHEAPER_CONFIG_DIR="%s";; esac\n' \
        "$n" "$CFG_ROOT/$n"
    done
    printf 'exec "%s" "$@"\n' "$REAL_CHEAPER"
  } > "$HOME/bin/cheaper"
  chmod +x "$HOME/bin/cheaper"

  {
    echo '@echo off'
    echo 'rem evals/skills-baseline: redirect CHEAPER_CONFIG_DIR per wsp dir'
    for n in "${SHIM_NAMES[@]}"; do
      printf 'echo %%CD%%|findstr /i /c:"%s" >nul && set "CHEAPER_CONFIG_DIR=%s"\r\n' \
        "$n" "$USERPROFILE\\.cache\\wsp-cfg\\$n"
    done
    printf '"C:\\Users\\akiyoshi\\AppData\\Local\\mise\\shims\\cheaper.exe" %%*\r\n'
  } > "$HOME/bin/cheaper.cmd"
}

# --- main -------------------------------------------------------------------

for n in "${ARMS_A[@]}"; do write_workspace "$n" 1; done
for n in "${ARMS_B[@]}"; do write_workspace "$n" 0; done
write_shims

echo "workspaces:"
for n in "${ARMS_A[@]}" "${ARMS_B[@]}"; do
  printf '  %s  files=%s  bytes=%s  skills=%s\n' "$n" \
    "$(find "$TARGET/$n" -type f | wc -l)" \
    "$(du -sb "$TARGET/$n" | cut -f1)" \
    "$([ -d "$TARGET/$n/.devin/skills" ] && echo yes || echo no)"
done
