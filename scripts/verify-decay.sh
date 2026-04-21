#!/usr/bin/env bash
# scripts/verify-decay.sh — BL-008 Step 5 approval gate
#
# Runs `mengdie dream --decay-dry-run` against the live DB, inspects the
# structured-JSON line for per-memory breach detail, and requires the
# operator to pass `--i-reviewed-each` before any live `mengdie dream`
# can be authorized.
#
# Pass criterion: operator reviewed each breached memory, NOT "zero
# demotions" (see plan 013 AC7 — hard-zero tied correctness to corpus
# cleanliness; the approval gate is the durable signal).
#
# Usage:
#   scripts/verify-decay.sh                       # report only; exits 1 if breaches > 0
#   scripts/verify-decay.sh --i-reviewed-each     # explicit approval; exits 0 regardless
#
# When the launchd daemon (BL-010 in Phase 2.2) takes over Dreaming,
# this script's interactive gate is replaced by a threshold alarm on
# `decay_floor_breaches` in the structured-JSON output. See plan 013's
# "Plan-level revisit trigger" section.
set -euo pipefail

APPROVED=0
for arg in "$@"; do
  case "$arg" in
    --i-reviewed-each) APPROVED=1 ;;
    --help|-h)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $arg" >&2; exit 2 ;;
  esac
done

if ! command -v mengdie >/dev/null 2>&1; then
  echo "mengdie binary not on PATH. Build via 'cargo build --release' and ensure the binary is reachable." >&2
  exit 2
fi

# Capture both streams separately: stdout has the human line; stderr
# carries the structured-JSON event + any tracing noise.
TMP_OUT=$(mktemp)
TMP_ERR=$(mktemp)
trap 'rm -f "$TMP_OUT" "$TMP_ERR"' EXIT

# RUST_LOG=info ensures the tracing::info! structured line is emitted.
RUST_LOG="${RUST_LOG:-info}" mengdie dream --decay-dry-run \
  >"$TMP_OUT" 2>"$TMP_ERR" || {
  echo "mengdie dream --decay-dry-run failed. stderr follows:" >&2
  cat "$TMP_ERR" >&2
  exit 2
}

echo "=== Human output ==="
cat "$TMP_OUT"

# Extract the single JSON object from stderr. The CLI emits a bare
# `{...}` line via `eprintln!` (NOT tracing — tracing's default formatter
# wraps the JSON in a log line with ISO timestamp). We find the line that
# contains the unique event marker regardless of key order (serde_json
# sorts keys alphabetically, so "event" may appear mid-object).
JSON_LINE=$(grep -E '^\{.*"event":"dreaming_pass".*\}$' "$TMP_ERR" | head -n1 || true)

if [[ -z "$JSON_LINE" ]]; then
  echo "WARNING: could not parse structured JSON line from stderr. Falling back to human line only." >&2
  echo "Raw stderr:" >&2
  cat "$TMP_ERR" >&2
  if [[ $APPROVED -eq 1 ]]; then
    echo "(Proceeding anyway: --i-reviewed-each was passed.)" >&2
    exit 0
  fi
  exit 1
fi

# Parse with jq if available; fallback to crude grep.
if command -v jq >/dev/null 2>&1; then
  BREACHES=$(echo "$JSON_LINE" | jq -r '.decay_floor_breaches')
  BREACHED_IDS=$(echo "$JSON_LINE" | jq -r '.breaches[]?')
  BEFORE=$(echo "$JSON_LINE" | jq -r '.avg_effective_before')
  AFTER=$(echo "$JSON_LINE" | jq -r '.avg_effective_after')
else
  BREACHES=$(echo "$JSON_LINE" | sed -n 's/.*"decay_floor_breaches":\([0-9]*\).*/\1/p')
  BREACHED_IDS=$(echo "$JSON_LINE" | sed -n 's/.*"breaches":\[\([^]]*\)\].*/\1/p' | tr ',' '\n' | tr -d '"')
  BEFORE=$(echo "$JSON_LINE" | sed -n 's/.*"avg_effective_before":\([0-9.]*\).*/\1/p')
  AFTER=$(echo "$JSON_LINE" | sed -n 's/.*"avg_effective_after":\([0-9.]*\).*/\1/p')
fi

echo ""
echo "=== Structured summary ==="
echo "decay_floor_breaches=$BREACHES"
echo "avg_effective_before=$BEFORE"
echo "avg_effective_after=$AFTER"

if [[ "${BREACHES:-0}" -gt 0 ]]; then
  echo ""
  echo "=== Breached memories ==="
  if [[ -n "$BREACHED_IDS" ]]; then
    echo "$BREACHED_IDS" | while read -r id; do
      [[ -z "$id" ]] && continue
      echo "  - $id"
    done
  fi
  echo ""
  if [[ $APPROVED -eq 1 ]]; then
    echo "Operator approval recorded (--i-reviewed-each). Safe to run live 'mengdie dream'."
    exit 0
  fi
  echo "Review each breached memory above, then re-run with --i-reviewed-each to approve."
  echo "See docs/operations/dreaming-decay.md for the approval procedure."
  exit 1
fi

echo "No breaches — 0 would-demote. Safe to proceed to live 'mengdie dream'."
exit 0
