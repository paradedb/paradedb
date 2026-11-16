#!/usr/bin/env bash
set -euo pipefail
export LC_ALL=C

check_only=false
if [ "$#" -eq 2 ] && [ "$1" = --check ]; then
  check_only=true
  output="$2"
elif [ "$#" -ge 4 ] && [ "$3" = -- ]; then
  container="$1"
  output="$2"
  shift 3
else
  echo "Usage: $0 CONTAINER OUTPUT_DIRECTORY -- COMMAND [ARGS...] | --check OUTPUT_DIRECTORY" >&2
  exit 1
fi

readonly events=cpu-clock:u,branch-misses:u
readonly sample_frequency=99
readonly stack_bytes=16384
readonly startup_timeout_seconds=30
readonly callgraph_min_percent=0.5
readonly preflight_seconds=2

record_args=(--event "$events" --freq "$sample_frequency" --call-graph "dwarf,$stack_bytes")
if [ "$(uname -m)" = aarch64 ]; then
  # The software clock PMU rejects the SVE vg register selected by perf's hardware probe.
  printf -v user_regs '%s,' x{0..29} lr sp pc
  record_args+=("--user-regs=${user_regs%,}")
fi

run_perf() {
  sudo -n perf "$@"
}

validate_samples() {
  if ! awk '
    /^# Samples: [1-9]/ {
      split($0, fields, "\047")
      split(fields[2], event, ":")
      seen[event[1]] = 1
    }
    END { exit !(seen["cpu-clock"] && seen["branch-misses"]) }
  ' "$1"; then
    echo "perf must record nonzero cpu-clock and branch-misses samples: $1" >&2
    return 1
  fi
}

mkdir -p "$output"
output="$(realpath "$output")"
if [ -e "$output/perf.data" ]; then
  echo "Refusing to overwrite $output/perf.data" >&2
  exit 1
fi
if "$check_only"; then
  {
    uname -a
    perf version
    printf 'record_args: '
    printf '%q ' "${record_args[@]}"
    printf '\n'
  } >"$output/metadata.txt"
  record_status=0
  run_perf record --output "$output/perf.data" "${record_args[@]}" -- \
    python3 -c 'import sys, time
deadline = time.monotonic() + float(sys.argv[1])
while time.monotonic() < deadline:
    pass' "$preflight_seconds" >"$output/record.log" 2>&1 || record_status=$?
  if [ -e "$output/perf.data" ]; then
    sudo -n chown "$(id -u):$(id -g)" "$output/perf.data"
  fi
  if [ "$record_status" -ne 0 ]; then
    cat "$output/record.log" >&2
    exit "$record_status"
  fi
  run_perf report --stdio --stdio-color never --header --input "$output/perf.data" \
    --show-nr-samples --no-children --call-graph none \
    >"$output/report.txt" 2>"$output/report.log"
  validate_samples "$output/report.txt"
  exit 0
fi
docker top "$container" -eo pid,comm >"$output/processes.txt"
pids="$(awk '$2 == "postgres" {print $1}' "$output/processes.txt" | paste -sd, -)"
if [ -z "$pids" ]; then
  echo "No PostgreSQL processes in $container" >&2
  exit 1
fi
container_pid="$(docker inspect --format '{{.State.Pid}}' "$container")"
symfs="/proc/$container_pid/root"
{
  date -u +%FT%TZ
  uname -a
  perf version
  docker inspect --format 'container={{.Name}} image={{.Config.Image}} image_id={{.Image}}' "$container"
  printf 'pids=%s\n' "$pids"
  printf 'record_args: '
  printf '%q ' "${record_args[@]}"
  printf '\n'
  printf 'command: '
  printf '%q ' "$@"
  printf '\n'
} >"$output/metadata.txt"

control_dir="$(mktemp -d)"
mkfifo "$control_dir/control" "$control_dir/ack"
exec 3<>"$control_dir/control"
exec 4<>"$control_dir/ack"
perf_pid=
benchmark_pid=

# shellcheck disable=SC2329
cleanup() {
  status=$?
  trap - EXIT INT TERM HUP
  set +e
  if [ -n "$benchmark_pid" ]; then
    kill -TERM "$benchmark_pid" 2>/dev/null
    wait "$benchmark_pid"
  fi
  if [ -n "$perf_pid" ]; then
    printf 'stop\n' >&3
    wait "$perf_pid"
    record_status=$?
    cat "$output/record.log"
    if [ "$status" -eq 0 ]; then status=$record_status; fi
    report_status=0
    if [ -s "$output/perf.data" ]; then
      run_perf report --stdio --stdio-color never --header \
        --symfs "$symfs" --input "$output/perf.data" \
        --sort dso,symbol --show-nr-samples --no-children --call-graph none \
        >"$output/report.txt" 2>"$output/report.log" || report_status=$?
      run_perf report --stdio --stdio-color never --header \
        --symfs "$symfs" --input "$output/perf.data" \
        --sort comm,dso,symbol --show-nr-samples --children \
        --call-graph "graph,$callgraph_min_percent,caller" \
        >"$output/report-callgraph.txt" 2>>"$output/report.log" || report_status=$?
      echo "::group::PostgreSQL perf report: $output"
      cat "$output/report.txt" "$output/report-callgraph.txt" "$output/report.log"
      echo '::endgroup::'
      validate_samples "$output/report.txt" || report_status=$?
    else
      echo "perf produced no profile: $output/perf.data" >&2
      report_status=1
    fi
    sudo -n chown "$(id -u):$(id -g)" "$output/perf.data"
    if [ "$status" -eq 0 ]; then status=$report_status; fi
  fi
  exec 3>&-
  exec 4>&-
  rm -rf "$control_dir"
  exit "$status"
}
trap 'cleanup' EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
trap 'exit 129' HUP

run_perf record --output "$output/perf.data" \
  "${record_args[@]}" --inherit --namespaces --pid "$pids" \
  --delay=-1 --control="fifo:$control_dir/control,$control_dir/ack" \
  >"$output/record.log" 2>&1 &
perf_pid=$!
printf 'enable\n' >&3
if ! read -r -t "$startup_timeout_seconds" -u 4 ack || [ "$ack" != ack ]; then
  echo "perf did not acknowledge startup; benchmark was not started" >&2
  exit 1
fi

"$@" &
benchmark_pid=$!
status=0
wait "$benchmark_pid" || status=$?
benchmark_pid=
exit "$status"
