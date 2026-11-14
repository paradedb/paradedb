#!/usr/bin/env bash
set -euo pipefail

if [ "$#" -lt 4 ] || [ "$3" != -- ]; then
  echo "Usage: $0 CONTAINER OUTPUT_DIRECTORY -- COMMAND [ARGS...]" >&2
  exit 1
fi

container="$1"
output="$2"
shift 3
readonly events=cpu-clock:u,branch-misses:u
readonly sample_frequency=99
readonly stack_bytes=16384
readonly startup_timeout_seconds=30
readonly callgraph_min_percent=0.5

mkdir -p "$output"
output="$(realpath "$output")"
if [ -e "$output/perf.data" ]; then
  echo "Refusing to overwrite $output/perf.data" >&2
  exit 1
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
  printf 'pids=%s\nevents=%s\nfrequency=%s\nstack_bytes=%s\n' "$pids" "$events" "$sample_frequency" "$stack_bytes"
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

run_perf() {
  sudo -n perf "$@"
}

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
  --event "$events" --freq "$sample_frequency" \
  --call-graph "dwarf,$stack_bytes" --inherit --namespaces --pid "$pids" \
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
