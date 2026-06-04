#!/usr/bin/env bash
set -euo pipefail

repo_url="${HBCI4RUST_UPSTREAM_REPO:-https://github.com/hbci4j/hbci4java.git}"
ref="${1:-${HBCI4RUST_UPSTREAM_REF:-hbci4j-core-4.1.11}}"
target="${2:-${HBCI4RUST_UPSTREAM_TARGET:-target/reference/hbci4java}}"

if [[ -e "$target" && ! -d "$target/.git" ]]; then
  echo "target exists but is not a git checkout: $target" >&2
  exit 2
fi

mkdir -p "$(dirname "$target")"

if [[ -d "$target/.git" ]]; then
  git -C "$target" fetch --tags origin
else
  git clone --filter=blob:none --no-checkout "$repo_url" "$target"
fi

git -C "$target" checkout --detach "$ref"
git -C "$target" rev-parse HEAD
