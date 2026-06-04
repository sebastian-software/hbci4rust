#!/usr/bin/env bash
set -euo pipefail

target="${HBCI4RUST_UPSTREAM_TARGET:-target/reference/hbci4java}"
out="${HBCI4RUST_GOLDENS_DIR:-tests/fixtures/goldens}"

if [[ ! -d "$target/.git" ]]; then
  scripts/fetch-upstream.sh
fi

mkdir -p "$out"

cat > "$out/README.md" <<'EOF'
# Golden Fixtures

Golden fixtures are generated from the pinned hbci4java reference checkout.

The concrete generators will be added as individual port slices land. They must
stay offline and deterministic: SEPA XML, CAMT parse summaries, MT940 parse
summaries, BPD/message parser outputs, and PinTAN replay transcripts.
EOF

echo "prepared $out"
