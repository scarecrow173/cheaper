#!/usr/bin/env bash
# Extract the root-cause statement from each incident report.
#
# Incident reports keep their root cause in the LAST "##" section of the
# file. The heading text is not stable across reports ("Root cause",
# "What went wrong", "Postmortem notes", "Analysis"), so extraction keys
# on position, not on the heading name.
#
# Usage:
#   scripts/extract-incident-root-causes.sh [--count] [INCIDENTS_DIR]
#
#   INCIDENTS_DIR  directory of incident .md files
#                  (default: <repo root>/docs/incidents)
#   --count        aggregate verbatim-identical root causes:
#                  output "<count>\t<incident ids>\t<root cause>"
#                  sorted by count descending.
#
# Default output: "<incident id>\t<root cause>" per file, sorted by id.
# Empty root-cause fields mean the file needs a manual look (e.g. a new
# trailing boilerplate block the exclusion list does not cover yet).
set -euo pipefail

count=0
dir=""
for arg in "$@"; do
  case "$arg" in
    --count) count=1 ;;
    -h|--help) sed -n '2,20p' "$0"; exit 0 ;;
    *) dir="$arg" ;;
  esac
done

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
dir="${dir:-$repo_root/docs/incidents}"

if [ ! -d "$dir" ]; then
  echo "error: incidents directory not found: $dir" >&2
  exit 1
fi

# Boilerplate lines appended after the root cause in the current report
# template (the API-conventions paragraph). Compared by exact match;
# when the template changes, leaked lines are visible in the output and
# this list can be extended.
BOILERPLATE='Idempotency keys are accepted on all POST endpoints via the
`Idempotency-Key` header and are retained for 24 hours. Pagination uses
cursor-based continuation with `limit` (default 50, max 200) and
`cursor` query parameters; the `result.nextCursor` field is null at the
end of a list.
Timestamps are RFC 3339 UTC. Monetary amounts are integer minor units
in the workspace settlement currency. Enum fields are extensible —
clients must ignore unrecognized values. Webhook signatures use HMAC
SHA-256 over the raw request body with a per-endpoint secret.'

emit() {
  local f id last cause
  shopt -s nullglob
  for f in "$dir"/*.md; do
    id="$(basename "$f" .md)"
    last="$(grep -n '^## ' "$f" | tail -n 1 | cut -d: -f1)"
    if [ -z "$last" ]; then
      printf '%s\t%s\n' "$id" "(no section found)"
      continue
    fi
    cause="$(tail -n "+$((last + 1))" "$f" | awk -v bp="$BOILERPLATE" '
      BEGIN { n = split(bp, a, "\n"); for (i = 1; i <= n; i++) B[a[i]] = 1 }
      /^[[:space:]]*$/ { if (out != "") exit; next }
      ($0 in B) { exit }
      { out = (out == "") ? $0 : out " " $0 }
      END { print out }
    ')"
    printf '%s\t%s\n' "$id" "$cause"
  done | sort
}

if [ "$count" -eq 1 ]; then
  emit | awk -F'\t' '
    { count[$2]++; ids[$2] = ids[$2] (ids[$2] ? ", " : "") $1 }
    END { for (k in count) printf "%d\t%s\t%s\n", count[k], ids[k], k }
  ' | sort -rn
else
  emit
fi
