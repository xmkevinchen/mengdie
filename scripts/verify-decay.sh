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
#   scripts/verify-decay.sh                         # report only; exits 1 if breaches > 0
#   scripts/verify-decay.sh --i-reviewed-each       # explicit approval; exits 0 regardless
#   scripts/verify-decay.sh --db-path <path>        # use non-default DB (default: ~/.mengdie/db.sqlite)
#   scripts/verify-decay.sh --db-path /tmp/test.db --i-reviewed-each
#                                                   # both flags compose
#
# Approval-gate invariant (plan 015 Step 3):
#   `--i-reviewed-each` requires a PARSEABLE structured-JSON line from
#   `mengdie`. If the binary emits no JSON (crash before flush, format
#   regression, etc.), the script exits 2 REGARDLESS of --i-reviewed-each —
#   operator cannot "approve" a breach list they cannot see. A repeated
#   exit 2 with the "transient failure" message is an escalation signal,
#   not a script bug.
#
# When the launchd daemon (BL-010 in Phase 2.2) takes over Dreaming,
# this script's interactive gate is replaced by a threshold alarm on
# `decay_floor_breaches` in the structured-JSON output. See plan 013's
# "Plan-level revisit trigger" section and BL-decay-threshold-mode.
set -euo pipefail

APPROVED=0
DB_PATH=""   # empty = let mengdie use its compiled-in default (~/.mengdie/db.sqlite)
while [[ $# -gt 0 ]]; do
  case "$1" in
    --i-reviewed-each) APPROVED=1; shift ;;
    --db-path)
      if [[ $# -lt 2 ]]; then
        echo "error: --db-path requires a value" >&2
        exit 2
      fi
      DB_PATH="$2"
      shift 2
      ;;
    --help|-h)
      sed -n '2,28p' "$0"
      exit 0
      ;;
    *) echo "unknown arg: $1" >&2; exit 2 ;;
  esac
done

if ! command -v mengdie >/dev/null 2>&1; then
  echo "mengdie binary not on PATH. Build via 'cargo build --release' and ensure the binary is reachable." >&2
  exit 2
fi

# P1 (plan 015 review) — refuse to run the approval gate against a DB that
# doesn't exist. rusqlite's `Connection::open` silently creates the file if
# it's missing; without this check, a typoed `--db-path /tmp/typo.db` would
# operate against an empty corpus, produce 0 breaches, and exit 0 — silently
# approving nothing. The whole point of the approval gate is to prevent
# silent approval of unseen state.
if [[ -n "$DB_PATH" ]] && [[ ! -f "$DB_PATH" ]]; then
  echo "error: --db-path \"$DB_PATH\" does not exist." >&2
  echo "       Refusing to create an empty DB for the approval gate." >&2
  echo "       Verify the path is correct, or run 'mengdie import' to" >&2
  echo "       populate the corpus before running the approval gate." >&2
  exit 2
fi

# Capture both streams separately: stdout has the human line; stderr
# carries the structured-JSON event + any tracing noise.
# ${VAR:-} in the trap is defensive (handles the edge where the script exits
# between the mktemp calls and the trap install — unlikely under current
# `set -e` ordering, but cheap belt-and-braces).
TMP_OUT=$(mktemp)
TMP_ERR=$(mktemp)
trap 'rm -f "${TMP_OUT:-}" "${TMP_ERR:-}"' EXIT

# Build the mengdie invocation arg array. --db-path is a GLOBAL arg on the
# binary (src/bin/cli.rs:17-18) and must precede the `dream` subcommand.
# Empty DB_PATH → omit the flag so mengdie's compiled default applies.
# Single invocation via arg array removes the dual-branch duplication from
# the initial Step 4 draft (plan 015 code-review P2).
MENGDIE_ARGS=()
if [[ -n "$DB_PATH" ]]; then
  MENGDIE_ARGS+=("--db-path" "$DB_PATH")
fi
MENGDIE_ARGS+=("dream" "--decay-dry-run")

# RUST_LOG=info ensures the tracing::info! structured line is emitted.
RUST_LOG="${RUST_LOG:-info}" mengdie "${MENGDIE_ARGS[@]}" \
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
  # Plan 015 Step 3: no --i-reviewed-each bypass here. An operator cannot
  # approve a breach list they cannot see; the silent exit-0 path has been
  # removed. Distinguish two failure causes via $TMP_ERR's presence:
  if [[ -s "$TMP_ERR" ]]; then
    # Binary emitted SOMETHING but no dreaming_pass line — format regression.
    echo "ERROR: mengdie emitted stderr but no dreaming_pass JSON line." >&2
    echo "       This is likely a schema contract regression — verify the mengdie" >&2
    echo "       binary's output format against docs/schemas/dreaming_pass.json." >&2
  else
    # Binary exited 0 with empty stderr — it died before emitting the event
    # (OOM, SIGPIPE during flush, disk-full, etc.). Transient, not a script bug.
    echo "ERROR: mengdie produced no stderr output." >&2
    echo "       Binary may have crashed before emitting the dreaming_pass event." >&2
    echo "       Retry the script; if it persists, escalate (BL-010 daemon will" >&2
    echo "       replace this interactive gate with a threshold alarm)." >&2
  fi
  echo "Raw stderr:" >&2
  cat "$TMP_ERR" >&2
  # Approval-gate invariant: exit 2 regardless of --i-reviewed-each.
  exit 2
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
