#!/usr/bin/env bash
set -euo pipefail

target="${HBCI4RUST_UPSTREAM_TARGET:-target/reference/hbci4java}"
upstream_dir="$target/src/main/java/org/kapott/hbci/GV"
registry_file="${HBCI4RUST_REGISTRY_FILE:-src/gv/mod.rs}"
expected_missing="${HBCI4RUST_EXPECTED_MISSING_GV:-DauerDel,DauerEdit,DauerList,DauerNew,Donation,Last,LastCOR1SEPA,MultiLast,MultiLastCOR1SEPA,MultiUeb,StornoLast,Template,TermUeb,TermUebDel,TermUebEdit,TermUebList,Ueb,UebBZU,UebEil,UebGar,Umb}"

join_csv() {
  awk 'BEGIN { first = 1 } { if (!first) printf ","; printf "%s", $0; first = 0 }'
}

if [[ ! -d "$upstream_dir" ]]; then
  echo "upstream reference checkout missing: $upstream_dir" >&2
  echo "run scripts/fetch-upstream.sh first" >&2
  exit 2
fi

if [[ ! -f "$registry_file" ]]; then
  echo "Rust registry file missing: $registry_file" >&2
  exit 2
fi

upstream_jobs="$(
  find "$upstream_dir" -maxdepth 1 -type f -name 'GV*.java' -print \
    | sed -E 's#.*/GV([^/]+)\.java#\1#' \
    | sort
)
"

rust_jobs="$(
  awk '/^    "[^"]+",$/ { gsub(/^[[:space:]]*"|",$/, ""); print }' "$registry_file" \
    | sort
)
"

upstream_count="$(printf "%s\n" "$upstream_jobs" | awk 'NF { count++ } END { print count + 0 }')"
rust_count="$(printf "%s\n" "$rust_jobs" | awk 'NF { count++ } END { print count + 0 }')"

missing="$(
  comm -23 \
    <(printf "%s\n" "$upstream_jobs" | awk 'NF') \
    <(printf "%s\n" "$rust_jobs" | awk 'NF') \
    | join_csv
)"
extra="$(
  comm -13 \
    <(printf "%s\n" "$upstream_jobs" | awk 'NF') \
    <(printf "%s\n" "$rust_jobs" | awk 'NF') \
    | join_csv
)"

echo "upstream=$upstream_count"
echo "rust=$rust_count"
echo "missing=${missing:-<none>}"
echo "extra=${extra:-<none>}"

if [[ "$missing" != "$expected_missing" ]]; then
  echo "unexpected missing GV jobs; expected: $expected_missing" >&2
  exit 1
fi

if [[ -n "$extra" ]]; then
  echo "unexpected Rust registry jobs without upstream GV class" >&2
  exit 1
fi
