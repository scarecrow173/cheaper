#!/bin/sh
# PATH 上の Cargo install 済み cheaper を起動する thin launcher。
# 契約: ADR-034Dmb3kNAO4Ub6FxuQCtC。
set -eu
if ! command -v cheaper >/dev/null 2>&1; then
  printf '%s\n' \
    "cheaper launcher: 'cheaper' CLI is not installed or not on PATH." \
    "install: cargo install --git https://github.com/scarecrow173/cheaper.git --locked cheaper-cli" \
    "then ensure ~/.cargo/bin (POSIX) or %USERPROFILE%\.cargo\bin (Windows) is on the IDE/agent PATH and restart it." \
    >&2
  exit 1
fi
exec cheaper "$@"
