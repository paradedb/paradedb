import db from "k6/x/database";
import { Counter } from "k6/metrics";

const manifest = JSON.parse(open("./fixtures.json"));
if (!__ENV.SCENARIO)
  throw new Error(
    "SCENARIO must select one workload; run scenarios sequentially",
  );
const selected = manifest.scenarios.filter(
  (s) => !__ENV.SCENARIO || s.name === __ENV.SCENARIO,
);
if (selected.length === 0)
  throw new Error(`Unknown scenario: ${__ENV.SCENARIO}`);
const backends = db.backends({
  backends: [
    {
      type: "paradedb",
      alias: "paradedb",
      connection: __ENV.PARADEDB_URL,
      ...(__ENV.PARADEDB_NATIVE === "1" ? { container: "" } : {}),
    },
  ],
});
backends.setTimeout(120);
const errors = new Counter("benchmark_query_errors");
const emptyResults = new Counter("benchmark_empty_results");
const timer = db.timer({ duration: __ENV.DURATION || "30s", gap: "2s" });
const scenarios = {};
for (const entry of selected) {
  scenarios[entry.name] = {
    executor: "constant-vus",
    vus: 1,
    duration: timer.duration(),
    startTime: timer.advanceAndGet(),
    gracefulStop: "120s",
    exec: "topk",
    env: { WORKLOAD: entry.name },
    tags: { chart: entry.name },
  };
}
export const collectMetrics = backends.addDockerMetricsCollector(
  scenarios,
  timer,
);
export const options = {
  scenarios,
  setupTimeout: "5m",
  thresholds: {
    benchmark_query_errors: ["count==0"],
    benchmark_empty_results: ["count==0"],
  },
};
const byName = new Map(selected.map((s) => [s.name, s]));

export function setup() {
  if (__ENV.PREFLIGHT_DONE === "1") return;
  const client = backends.get("paradedb");
  for (const entry of selected) {
    for (const fixture of entry.fixtures) {
      const result = client.prewarm(entry.sql, ...fixture.params);
      if (result.error || result.hits !== Math.min(10, fixture.matches)) {
        throw new Error(
          `Preflight failed: ${entry.name} ${JSON.stringify(fixture.params)}: ${JSON.stringify(result)}`,
        );
      }
    }
  }
}

export function topk() {
  const entry = byName.get(__ENV.WORKLOAD);
  const fixture = entry.fixtures[__ITER % entry.fixtures.length];
  const result = backends.get("paradedb").query(entry.sql, ...fixture.params);
  errors.add(result.error ? 1 : 0);
  emptyResults.add(!result.error && result.hits === 0 ? 1 : 0);
  if (result.error || result.hits === 0)
    throw new Error(JSON.stringify(result));
}
