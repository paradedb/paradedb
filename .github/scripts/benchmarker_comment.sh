#!/usr/bin/env bash
# Prints the Benchmarker PR comment as Markdown.
#
# Visible: the header, the context file, the baselines, the regressions and the ratio footnote.
# Collapsed: the full results table and the report links.
#
# Reads from the working directory the files publish-benchmarker-results writes: _latency_baseline.json,
# _throughput_baseline.json, _buffer_baseline.json, _buffer_metrics.json, _index_baseline.json,
# _index_metrics.json and _ghpages_html/. Reads DATASET, COMMIT_SHA, GITHUB_REPOSITORY_OWNER,
# EVENT_REPOSITORY_NAME, METRICS_FILE, THROUGHPUT_METRICS_FILE, CONTEXT_FILE and SWEEP_REPORTS_DIR from
# the environment.
set -euo pipefail
shopt -s nullglob

# A ratio above this marks a regression.
THRESHOLD=1.15

table_header() {
  printf '| Benchmark | Current | Previous | Ratio |\n|---|---:|---:|---:|\n'
}

BASE="https://${GITHUB_REPOSITORY_OWNER}.github.io/${EVENT_REPOSITORY_NAME}/benchmarker-runs/${COMMIT_SHA}/${DATASET}"
OVERTIME="https://${GITHUB_REPOSITORY_OWNER}.github.io/${EVENT_REPOSITORY_NAME}/benchmarker/"
links=""
for h in _ghpages_html/*.html; do
  nm="$(basename "$h" .html)"
  links="${links}- [${nm}](${BASE}/${nm}.html)"$'\n'
done

# One JSON line per results-table row: the Markdown row and whether its ratio is a regression.
rows="$(jq -nc \
  --argjson threshold "$THRESHOLD" \
  --slurpfile latency "${METRICS_FILE}" \
  --slurpfile throughput "${THROUGHPUT_METRICS_FILE}" \
  --slurpfile latency_baseline _latency_baseline.json \
  --slurpfile index _index_metrics.json \
  --slurpfile index_baseline _index_baseline.json \
  --slurpfile buffers _buffer_metrics.json \
  --slurpfile buffer_baseline _buffer_baseline.json \
  --slurpfile throughput_baseline _throughput_baseline.json '
  def display: "\(.value * 1000 | round / 1000) \(.unit)";
  (($latency[0] | map(. + {baseline: $latency_baseline[0]})) +
   ($throughput[0] | map(. + {baseline: $throughput_baseline[0]})) +
   ($buffers[0] | map(. + {baseline: $buffer_baseline[0]})) +
   ($index[0] | map(. + {baseline: $index_baseline[0]})))
  | sort_by((.name | sub(" (p[0-9]+ latency|QPS|shared hits/query)$"; "")), .unit == "queries/s", .name)
  | .[] as $current
  | ([$current.baseline.benches[]? | select(.name == $current.name)][0]) as $previous
  | (if $previous != null and $previous.value > 0 then
       if $current.unit == "queries/s" then
         if $current.value > 0 then $previous.value / $current.value else null end
       else $current.value / $previous.value end
     else null end) as $ratio
  | ($ratio != null and $ratio > $threshold) as $regression
  | (if $ratio == null then "n/a" else
       "\($ratio * 100 | round / 100)x" + (if $regression then " ⚠️" else "" end)
     end) as $ratio_text
  | {
      regression: $regression,
      row: "| `\($current.name)` | \($current | display) | \(if $previous == null then "n/a" else $previous | display end) | \($ratio_text) |"
    }
')"
total="$(jq -s 'length' <<<"$rows")"
regressions="$(jq -s 'map(select(.regression)) | length' <<<"$rows")"

# shellcheck disable=SC2016 # The backticks are Markdown code spans.
printf '### 📊 Benchmarker results (%s): `%s`\n\n' "${DATASET}" "${COMMIT_SHA:0:7}"
if [ -n "$CONTEXT_FILE" ]; then
  cat "$CONTEXT_FILE"
  printf '\n'
fi
jq -nr --slurpfile latency _latency_baseline.json --slurpfile throughput _throughput_baseline.json \
  --slurpfile buffers _buffer_baseline.json --slurpfile buffer_metrics _buffer_metrics.json \
  --slurpfile index _index_baseline.json --slurpfile index_metrics _index_metrics.json '
  "Baselines: latency `\(($latency[0].commit.id // "n/a") | .[0:7])`, QPS `\(($throughput[0].commit.id // "n/a") | .[0:7])`" +
  (if ($buffer_metrics[0] | length) > 0 then ", shared hits/query `\(($buffers[0].commit.id // "n/a") | .[0:7])`" else "" end) +
  (if ($index_metrics[0] | length) > 0 then ", index build `\(($index[0].commit.id // "n/a") | .[0:7])`" else "" end) + ".\n"
'

# Regressions
if [ "$regressions" -gt 0 ]; then
  noun="regressions"
  if [ "$regressions" -eq 1 ]; then
    noun="regression"
  fi
  printf '**⚠️ %s %s above %sx**\n\n' "$regressions" "$noun" "$THRESHOLD"
  table_header
  jq -r 'select(.regression) | .row' <<<"$rows"
else
  printf '✅ No regressions above %sx.\n' "$THRESHOLD"
fi
footnote="Ratio: current/previous for latency, shared hits/query, build time, and index size;"
footnote+=" previous/current for QPS. Below 1 is better. ⚠️ marks regressions above ${THRESHOLD}x."
footnote+=" n/a means no comparable baseline."
printf '\n%s\n\n' "$footnote"

# All results. GitHub renders Markdown inside <details> only with a blank line after the summary.
printf '<details><summary>All results (%s rows)</summary>\n\n' "$total"
table_header
jq -r '.row' <<<"$rows"
printf '\n</details>\n\n'

# Reports
printf '<details><summary>Reports</summary>\n\n'
if [ -n "$SWEEP_REPORTS_DIR" ]; then
  printf '##### Sweep\n\n'
  if [ -f "$SWEEP_REPORTS_DIR/metadata.json" ]; then
    jq -r '"Compared with [main `\(.baseline_commit[0:7])`](\(.baseline_url)) at matched recall.\n"' \
      "$SWEEP_REPORTS_DIR/metadata.json"
  fi
  sweep_htmls=(_ghpages_html/sweep/*.html)
  for h in "${sweep_htmls[@]}"; do
    nm="$(basename "$h" .html)"
    printf -- '- [%s](%s/sweep/%s.html)\n' "$nm" "$BASE" "$nm"
  done
  if [ -f "$SWEEP_REPORTS_DIR/metadata.json" ]; then
    jq -r '.queries | to_entries[] | select(.value.skipped) |
      "- `\(.key)`: skipped (\(.value.skipped))."' "$SWEEP_REPORTS_DIR/metadata.json"
  elif [ ${#sweep_htmls[@]} -eq 0 ]; then
    printf 'No comparable mainline sweep results are available for this run.\n'
  fi
  printf '\n'
fi
printf '##### Search\n\n'
printf 'Interactive per-script graphs (latency timeline, throughput, and resource metrics when available):\n\n'
printf '%s\n' "$links"
printf 'Over-time tracking (latency, QPS, and available buffer and index build metrics across commits): [%s](%s)\n\n' "$OVERTIME" "$OVERTIME"
printf '_gh-pages may take ~a minute to deploy after this run._\n\n'
printf '</details>\n'
