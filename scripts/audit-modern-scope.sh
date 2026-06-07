#!/usr/bin/env bash
set -euo pipefail

registry_source="src/gv/mod.rs"

modern_jobs=(
  AccInfo
  CardList
  ChangePIN
  CustomMsg
  DauerLastSEPAList
  DauerLastSEPANew
  DauerSEPADel
  DauerSEPAEdit
  DauerSEPAList
  DauerSEPANew
  FestCondList
  FestList
  FestListAll
  InfoList
  InfoOrder
  InstUebSEPA
  KUmsAll
  KUmsAllCamt
  KUmsNew
  KUmsZeitSEPA
  Kontoauszug
  KontoauszugPdf
  LastB2BSEPA
  LastSEPA
  MultiLastB2BSEPA
  MultiLastSEPA
  MultiUebSEPA
  Receipt
  SEPAInfo
  SaldoReq
  SaldoReqAll
  Status
  TAN2Step
  TANList
  TANMediaList
  TermMultiUebSEPA
  TermUebSEPA
  TermUebSEPADel
  TermUebSEPAEdit
  TermUebSEPAList
  UebSEPA
  UmbSEPA
  VoP
  VoPAuth
  WPDepotList
  WPDepotUms
)

legacy_jobs=(
  UebForeign
)

tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

registry_file="$tmpdir/registry"
modern_file="$tmpdir/modern"
legacy_file="$tmpdir/legacy"
classified_file="$tmpdir/classified"
duplicates_file="$tmpdir/duplicates"
unclassified_file="$tmpdir/unclassified"
stale_file="$tmpdir/stale"

sed -n '/pub const PINTAN_JOB_NAMES/,/];/p' "$registry_source" \
  | sed -n 's/^[[:space:]]*"\([^"]*\)",[[:space:]]*$/\1/p' \
  | sort >"$registry_file"

printf '%s\n' "${modern_jobs[@]}" | sort >"$modern_file"
printf '%s\n' "${legacy_jobs[@]}" | sort >"$legacy_file"
cat "$modern_file" "$legacy_file" | sort >"$classified_file"
cat "$modern_file" "$legacy_file" | sort | uniq -d >"$duplicates_file"

comm -23 "$registry_file" "$classified_file" >"$unclassified_file"
comm -13 "$registry_file" "$classified_file" >"$stale_file"

registry_count="$(wc -l <"$registry_file" | tr -d ' ')"
modern_count="$(wc -l <"$modern_file" | tr -d ' ')"
legacy_count="$(wc -l <"$legacy_file" | tr -d ' ')"

echo "registry=$registry_count"
echo "modern=$modern_count"
echo "legacy=$legacy_count"

if [[ -s "$duplicates_file" ]]; then
  echo "duplicates=$(paste -sd, "$duplicates_file")"
else
  echo "duplicates=<none>"
fi

if [[ -s "$unclassified_file" ]]; then
  echo "unclassified=$(paste -sd, "$unclassified_file")"
else
  echo "unclassified=<none>"
fi

if [[ -s "$stale_file" ]]; then
  echo "stale=$(paste -sd, "$stale_file")"
else
  echo "stale=<none>"
fi

if [[ -s "$duplicates_file" || -s "$unclassified_file" || -s "$stale_file" ]]; then
  exit 1
fi
