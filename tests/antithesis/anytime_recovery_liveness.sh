#!/bin/bash
#
# Antithesis "anytime" command: heal every fault, then require that the driver running alongside
# makes progress before faults resume. "The database was provably reachable and the workload still
# made no progress" is a failure no fault schedule can fake; the rationale lives in
# tests/src/fixtures/fault_grace.rs. Mirrors stressgres/suites/antithesis/anytime_recovery_liveness.sh.

set -Eeuo pipefail

# The poke channel, shared via the proptests container's filesystem. The driver is launched with
# this same path in PARADEDB_QGEN_RECONNECT_GRACE_FILE.
GRACE_FILE=/tmp/qgen-reconnect-grace
LOCK_FILE=/tmp/qgen-recovery-liveness.lock

# RECOVER < QUIET so the window is judged before faults resume, and QUIET must comfortably exceed
# the 60s statement_timeout or slow-query detection never fires. 90s of recovery allows a few
# attempts past sqlx's 30s pool-acquire timeout.
QUIET_SECONDS="${QGEN_LIVENESS_QUIET_SECONDS:-120}"
RECOVER_SECONDS="${QGEN_LIVENESS_RECOVER_SECONDS:-90}"

# Minimum chaos between pauses; without it back-to-back pauses would suppress the faults this
# test exists to inject.
COOLDOWN_SECONDS="${QGEN_LIVENESS_COOLDOWN_SECONDS:-300}"
COOLDOWN_FILE=/tmp/qgen-recovery-liveness.last

# Fire on a small fraction of invocations so most of the run is spent under chaos.
TRIGGER_PERCENT="${QGEN_LIVENESS_TRIGGER_PERCENT:-10}"
sample=$(od -An -N2 -tu2 < /dev/urandom | tr -d '[:space:]')
(( sample % 100 < TRIGGER_PERCENT )) || exit 0

# Overlapping pokes would race: the first to finish would restore the baseline while the second is
# still counting down, silently disarming the check. Check for flock(1) separately from taking the
# lock, because a missing binary exits 127, which `|| exit 0` would misreport as "another instance
# holds the lock" and skip every check for the whole run.
if ! command -v flock >/dev/null 2>&1; then
  echo "qgen recovery liveness: flock(1) not found, refusing to run without single-flight" >&2
  exit 1
fi
exec 9>"${LOCK_FILE}"
flock -n 9 || exit 0

# Enforce the cooldown under the lock, so two invocations cannot both pass. Clock faults make the
# arithmetic inexact, and that is fine: overshoot means more chaos, and a backwards jump goes
# negative, firing early rather than never.
last=$(stat -c %Y "${COOLDOWN_FILE}" 2>/dev/null || echo 0)
now=$(date +%s)
elapsed=$(( now - last ))
if (( elapsed >= 0 && elapsed < COOLDOWN_SECONDS )); then
  echo "qgen recovery liveness: ${elapsed}s since the last pause, under the ${COOLDOWN_SECONDS}s cooldown; leaving faults alone"
  exit 0
fi
touch "${COOLDOWN_FILE}"

# Atomic rename so a reader never sees a half-written declaration. Format:
# '<unpause_epoch_ms> <window_ms>' (parsed by fault_grace.rs; the deadline makes a stranded file
# self-voiding). Removing the file restores retry-forever.
poke() {
  printf '%s %s' "$1" "$2" > "${GRACE_FILE}.tmp"
  mv "${GRACE_FILE}.tmp" "${GRACE_FILE}"
}
restore() { rm -f "${GRACE_FILE}" "${GRACE_FILE}.tmp"; }

# Restore on every exit path (normal, set -e, SIGTERM), or the next fault is judged against a
# stale window. SIGKILL would strand it, but this container is in the fault exclusion patterns.
trap restore EXIT

echo "qgen recovery liveness: pausing faults for ${QUIET_SECONDS}s; qgen must finish a case within ${RECOVER_SECONDS}s"
"${ANTITHESIS_STOP_FAULTS}" "${QUIET_SECONDS}"

UNPAUSE_AT_MS=$(( ($(date +%s) + QUIET_SECONDS) * 1000 ))
poke "${UNPAUSE_AT_MS}" "$(( RECOVER_SECONDS * 1000 ))"
sleep "${RECOVER_SECONDS}"
restore

echo "qgen recovery liveness: qgen survived the quiet period"

# Exit 0 either way: the assertion belongs to the driver, whose failing case carries the repro.
exit 0
