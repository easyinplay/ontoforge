#!/usr/bin/env bash
set -euo pipefail

tree=$(cargo tree --workspace --locked --features ontoforge-storage/embedded-pg -e normal,build,dev --prefix none)
while read -r package _; do
  case "$package" in
    openssl | native-tls)
      printf '::error::Forbidden TLS dependency: %s\n' "$package" >&2
      exit 1
      ;;
  esac
done <<< "$tree"
