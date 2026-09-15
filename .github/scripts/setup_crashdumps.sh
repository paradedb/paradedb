#!/usr/bin/env bash
# Configures the Linux runner to write process core dumps to disk on crash,
# while excluding Postgres shared memory (shared_buffers) to keep dumps small.
set -euo pipefail

COREDUMP_DIR="${COREDUMP_DIR:-/tmp/coredumps}"

# World-writable directory with sticky bit so any user (runner, postgres) can write cores
echo "Creating core dump directory at ${COREDUMP_DIR}..."
sudo mkdir -p -m 1777 "${COREDUMP_DIR}"

# Set absolute core pattern (bypasses apport / systemd-coredump pipe filters).
# fs.suid_dumpable=2 allows processes that changed UIDs or dropped privileges to dump core.
echo "Configuring core dump pattern and dumpable setting..."
sudo sysctl -w "kernel.core_pattern=${COREDUMP_DIR}/core.%e.%p.%t"
sudo sysctl -w fs.suid_dumpable=2

# Raise system-wide soft and hard core limits to unlimited
if [ -d /etc/security/limits.d ]; then
  sudo tee /etc/security/limits.d/99-core.conf >/dev/null <<'EOF'
* soft core unlimited
* hard core unlimited
root soft core unlimited
root hard core unlimited
EOF
fi

# coredump_filter bitmask 0x31:
# - Included: bit 0 (anon private / heap & stack), bit 4 (ELF headers), bit 5 (private huge pages)
# - Excluded: bit 1 (anon shared / shared_buffers), bit 3 (file-backed shared), bit 6 (shared huge pages)
sudo tee /etc/profile.d/crashdumps.sh >/dev/null <<'EOF'
ulimit -c unlimited 2>/dev/null || true
echo 0x31 > /proc/self/coredump_filter 2>/dev/null || true
EOF

# Export BASH_ENV so each subsequent non-interactive GitHub Actions step
# automatically sources /etc/profile.d/crashdumps.sh on launch
if [ -n "${GITHUB_ENV:-}" ]; then
  echo "BASH_ENV=/etc/profile.d/crashdumps.sh" >>"${GITHUB_ENV}"
fi

# Apply immediately to the current shell and all active runner processes
ulimit -c unlimited || true
echo 0x31 >/proc/self/coredump_filter 2>/dev/null || true
for pid in $(pgrep -f "Runner|runner" 2>/dev/null || true); do
  sudo prlimit --core=unlimited:unlimited --pid "$pid" 2>/dev/null || true
  sudo tee "/proc/$pid/coredump_filter" >/dev/null <<<"0x31" 2>/dev/null || true
done

echo "Crash dumps enabled: pattern=$(sysctl -n kernel.core_pattern), ulimit -c=$(ulimit -c)"
