#!/usr/bin/env bash
# extract-incident-causes.sh — 四半期ごとのインシデント根本原因棚卸し用ヘルパー。
#
# docs/incidents/ の各レポートから「根本原因」節を決定的に抽出する。
# 根本原因を記述する節見出しはファイルごとに揺れる（Root cause /
# What went wrong / Postmortem notes / Analysis 等）が、常にファイル末尾の
# `## ` 節なので、最後の `## ` 見出し直後の最初の非空行を原因文として採用する。
#
# Usage:
#   scripts/extract-incident-causes.sh [--hist] [incidents_dir]
#
#   既定: 全インシデントを TSV (id / service / date / section / cause) で出力。
#   --hist: 同一原因文でグルーピングし、件数順にインシデント番号一覧を出力。
#   incidents_dir: 対象ディレクトリ（既定 docs/incidents、カレントからの相対）。
#
# モデル・ネットワーク不要の決定的処理（awk 一発で全件走査）。
# 分類の判断自体は digest 作成者が行い、本スクリプトはその根拠データを
# 漏れなく供給する。

set -euo pipefail

MODE="tsv"
DIR="docs/incidents"
for arg in "$@"; do
  case "$arg" in
    --hist) MODE="hist" ;;
    -h|--help) sed -n '2,21p' "$0"; exit 0 ;;
    *) DIR="$arg" ;;
  esac
done

if [[ ! -d "$DIR" ]]; then
  echo "error: incidents dir not found: $DIR" >&2
  exit 2
fi

shopt -s nullglob
files=("$DIR"/*.md)
if (( ${#files[@]} == 0 )); then
  echo "error: no .md files under $DIR" >&2
  exit 2
fi

TSV="$(
  awk '
    function emit() {
      svc = title
      sub(/^# /, "", svc)
      sub(/ — incident report.*/, "", svc)
      dt = ""
      if (match(title, /\([0-9]{4}-[0-9]{2}-[0-9]{2}\)/))
        dt = substr(title, RSTART + 1, RLENGTH - 2)
      if (section == "")
        printf "warn: %s has no ## section\n", id > "/dev/stderr"
      else if (cause == "")
        printf "warn: %s has empty cause section (%s)\n", id, section > "/dev/stderr"
      printf "%s\t%s\t%s\t%s\t%s\n", id, svc, dt, section, cause
    }
    FNR == 1 { if (NR != 1) emit(); id = FILENAME; sub(/.*\//, "", id); sub(/\.md$/, "", id); title = ""; section = ""; cause = "" }
    /^# / && title == "" { title = $0 }
    /^## / { section = $0; sub(/^## /, "", section); cause = ""; next }
    section != "" && cause == "" && $0 !~ /^[[:space:]]*$/ { cause = $0 }
    END { if (NR > 0) emit() }
  ' "${files[@]}"
)"

if [[ "$MODE" == "tsv" ]]; then
  printf 'id\tservice\tdate\tsection\tcause\n'
  printf '%s\n' "$TSV"
else
  # --hist: 同一原因文をキーに件数 + インシデント番号を集約（件数降順）。
  # ソートが崩れないよう 1 行 1 グループで sort してから整形する。
  printf '%s\n' "$TSV" | awk -F '\t' '
    { count[$5]++; members[$5] = members[$5] " " $1; n++ }
    END {
      for (c in count) printf "%d\t%s\t%s\n", count[c], substr(members[c], 2), c
      printf "total\t%d incidents\t%d distinct cause statements\n", n, length(count)
    }
  ' | sort -t$'\t' -k1,1 -rn | awk -F '\t' '
    $1 == "total" { print "----"; print $2 ", " $3; next }
    { print $1 "\t" $2; print "  cause: " $3 }
  '
fi
