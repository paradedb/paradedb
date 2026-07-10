#!/bin/bash
#
# Antithesis "anytime" command: heal every fault, then require that the Stressgres
# processes running alongside us reconnect and make progress before faults resume. The
# failure condition is "the database was provably reachable and stressgres still could
# not make progress", which no fault schedule can fake.
#
# We poke the already-running drivers rather than starting our own, because Stressgres
# spends its first 60s waiting for the cluster and then builds its schema.
# See https://github.com/paradedb/paradedb/issues/5501.

set -Eeuo pipefail

# Shared with the `--reconnect-grace-file` passed by every singleton driver. Both
# commands run in the Stressgres container, so this is a plain shared file.
GRACE_FILE=/tmp/stressgres-reconnect-grace
LOCK_FILE=/tmp/stressgres-recovery-liveness.lock

# How long to hold faults off, and how much of that window Stressgres gets to recover
# in. The recovery window has to cover the worst fault we inject: a killed container,
# restarted by K8s, whose Postgres then has to finish crash recovery and, for the CNPG
# primary, be promoted back into service before a connection can succeed. Too short and
# the liveness check itself becomes the flake.
QUIET_SECONDS=90
RECOVER_SECONDS=75

# Antithesis schedules anytime commands aggressively, and a quiet period suppresses
# exactly the faults this test exists to inject. Fire on a small fraction of
# invocations so most of the run is spent under chaos.
TRIGGER_PERCENT=5
sample=$(od -An -N2 -tu2 < /dev/urandom | tr -d '[:space:]')
(( sample % 100 < TRIGGER_PERCENT )) || exit 0

# Overlapping quiet periods merge into the longest interval, but overlapping pokes
# would race: the first to finish would restore the baseline while the second is still
# counting down, silently disarming the check.
#
# Check for flock(1) separately from taking the lock: a missing binary exits 127,
# which `|| exit 0` would otherwise report as "another instance holds the lock" and
# skip every check for the whole run.
if ! command -v flock >/dev/null 2>&1; then
  echo "recovery liveness: flock(1) not found, refusing to run without single-flight" >&2
  exit 1
fi
exec 9>"${LOCK_FILE}"
flock -n 9 || exit 0

# Absent when dry-running the test template locally, where there is no fault
# injector to pause and therefore nothing to assert.
if [ -z "${ANTITHESIS_STOP_FAULTS:-}" ]; then
  echo "ANTITHESIS_STOP_FAULTS unset, skipping the recovery liveness check"
  exit 0
fi

# Write via rename so a reader never observes a half-written window. Removing the file
# restores whatever `--reconnect-grace` the driver was started with, so the baseline
# lives in exactly one place.
poke() {
  printf '%s' "$1" > "${GRACE_FILE}.tmp"
  mv "${GRACE_FILE}.tmp" "${GRACE_FILE}"
}
restore() { rm -f "${GRACE_FILE}" "${GRACE_FILE}.tmp"; }

# Restore the baseline no matter how we leave, or the next fault would be measured
# against a window that is no longer in force and fail the run. The EXIT trap covers a
# normal exit, a `set -e` failure, and SIGTERM. Only SIGKILL would strand the short
# window, and the stressgres container is named in
# `container_faults_{stop,kill}_exclusion_patterns`, so the run never injects one.
trap restore EXIT

echo "recovery liveness: pausing faults for ${QUIET_SECONDS}s; stressgres must recover within ${RECOVER_SECONDS}s"
"${ANTITHESIS_STOP_FAULTS}" "${QUIET_SECONDS}"

poke "$(( RECOVER_SECONDS * 1000 ))"
sleep "${RECOVER_SECONDS}"
restore

echo "recovery liveness: stressgres survived the quiet period"

# A nonzero exit here would report *this* command as the failure. The assertion
# belongs to the drivers: if one of them could not reconnect inside the window,
# it exits nonzero and Antithesis attributes the failure to the workload.
exit 0
