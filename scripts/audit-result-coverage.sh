#!/usr/bin/env bash
set -euo pipefail

target="${HBCI4RUST_UPSTREAM_TARGET:-target/reference/hbci4java}"
upstream_dir="$target/src/main/java/org/kapott/hbci/GV_Result"
result_file="${HBCI4RUST_RESULT_FILE:-src/gv_result/mod.rs}"
expected_missing="${HBCI4RUST_EXPECTED_MISSING_GVR:-WPStammData}"

join_csv() {
  awk 'BEGIN { first = 1 } { if (!first) printf ","; printf "%s", $0; first = 0 }'
}

normalize_upstream_result_name() {
  awk '
    $0 == "DauerLastList" { print "DauerList"; next }
    $0 == "DauerLastNew" { print "DauerNew"; next }
    $0 == "LastB2BSEPA" || $0 == "LastCOR1SEPA" || $0 == "LastSEPA" {
      print "LastSepa"; next
    }
    $0 == "InstUebSEPA" { print "InstUebSepa"; next }
    $0 == "TANList" { print "TanList"; next }
    $0 == "TANMediaList" { print "TanMediaList"; next }
    { print }
  '
}

if [[ ! -d "$upstream_dir" ]]; then
  echo "upstream reference checkout missing: $upstream_dir" >&2
  echo "run scripts/fetch-upstream.sh first" >&2
  exit 2
fi

if [[ ! -f "$result_file" ]]; then
  echo "Rust result file missing: $result_file" >&2
  exit 2
fi

upstream_results="$(
  find "$upstream_dir" -maxdepth 1 -type f -name 'GVR*.java' -print \
    | sed -E 's#.*/GVR([^/]+)\.java#\1#' \
    | sort
)"
normalized_upstream_results="$(
  printf "%s\n" "$upstream_results" \
    | awk 'NF' \
    | normalize_upstream_result_name \
    | sort -u
)"
rust_results="$(
  awk '
    /^pub enum HbciJobResultData \{/ { inside = 1; next }
    inside && /^\}/ { inside = 0 }
    inside && /^[[:space:]]*[A-Za-z0-9]+\(Gvr/ {
      name = $1
      sub(/\(.*/, "", name)
      print name
    }
  ' "$result_file" \
    | sort
)"

upstream_count="$(printf "%s\n" "$upstream_results" | awk 'NF { count++ } END { print count + 0 }')"
normalized_count="$(
  printf "%s\n" "$normalized_upstream_results" | awk 'NF { count++ } END { print count + 0 }'
)"
rust_count="$(printf "%s\n" "$rust_results" | awk 'NF { count++ } END { print count + 0 }')"

missing="$(
  comm -23 \
    <(printf "%s\n" "$normalized_upstream_results" | awk 'NF') \
    <(printf "%s\n" "$rust_results" | awk 'NF') \
    | join_csv
)"
extra="$(
  comm -13 \
    <(printf "%s\n" "$normalized_upstream_results" | awk 'NF') \
    <(printf "%s\n" "$rust_results" | awk 'NF') \
    | join_csv
)"

echo "upstream_raw=$upstream_count"
echo "upstream_normalized=$normalized_count"
echo "rust=$rust_count"
echo "missing=${missing:-<none>}"
echo "extra=${extra:-<none>}"

if [[ "$missing" != "$expected_missing" ]]; then
  echo "unexpected missing GVR result shapes; expected: $expected_missing" >&2
  exit 1
fi

if [[ -n "$extra" ]]; then
  echo "unexpected Rust result variants without normalized upstream GVR class" >&2
  exit 1
fi
