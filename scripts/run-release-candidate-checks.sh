#!/usr/bin/env bash
set -euo pipefail

include_package=0

usage() {
  cat <<'USAGE'
Usage: scripts/run-release-candidate-checks.sh [--package]

Runs the v1 release-candidate gates and writes full command logs under
target/release-gates/<timestamp>/.

Options:
  --package  Also run package-list and local package verification checks.
  -h, --help Show this help.
USAGE
}

while (($#)); do
  case "$1" in
    --package)
      include_package=1
      ;;
    -h | --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
log_dir="${HBCI4RUST_RELEASE_GATE_DIR:-target/release-gates/$timestamp}"
summary_file="$log_dir/SUMMARY.md"

mkdir -p "$log_dir"

cat >"$summary_file" <<SUMMARY
# Release Candidate Check Summary

UTC timestamp: $timestamp

Full command logs live next to this file.

SUMMARY

run_check() {
  local name="$1"
  shift

  local log_file="$log_dir/$name.log"
  local command_line="$*"

  echo "==> $name"
  echo "    $command_line"

  {
    echo "## $name"
    echo
    echo '```text'
    echo "$ $command_line"
  } >>"$summary_file"

  if "$@" >"$log_file" 2>&1; then
    echo "    PASS"
    {
      echo "exit=0"
      echo "log=$log_file"
      echo '```'
      echo
    } >>"$summary_file"
  else
    local status=$?
    echo "    FAIL (exit $status)"
    echo
    tail -n 80 "$log_file" || true
    {
      echo "exit=$status"
      echo "log=$log_file"
      echo '```'
      echo
    } >>"$summary_file"
    exit "$status"
  fi
}

run_check cargo-fmt cargo fmt --check
run_check cargo-clippy cargo clippy --all-targets
run_check cargo-test cargo test
run_check cargo-test-list cargo test -- --list
run_check audit-modern-scope scripts/audit-modern-scope.sh
run_check audit-job-coverage scripts/audit-job-coverage.sh
run_check audit-result-coverage scripts/audit-result-coverage.sh
run_check git-diff-check git diff --check

if ((include_package)); then
  run_check cargo-package-list cargo package --list
  run_check cargo-package cargo package
fi

echo
echo "release candidate checks passed"
echo "summary: $summary_file"
