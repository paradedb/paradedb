import { sleep } from "k6";
import exec from "k6/execution";
import { Counter } from "k6/metrics";
import db from "k6/x/database";
import { loadQueries, paradedbRequest } from "./queries.js";

if (!__ENV.PARADEDB_URL) {
  throw new Error("PARADEDB_URL is required");
}

const MILLISECONDS_PER_SECOND = 1000;
const GRACE_SECONDS = 5;
const updatesPerSecond = Number(__ENV.UPDATES_PER_SECOND || 0);
if (!Number.isSafeInteger(updatesPerSecond) || updatesPerSecond < 0) {
  throw new Error("UPDATES_PER_SECOND must be a non-negative integer");
}

const vus = Number(__ENV.VUS || 8);
const warmupSeconds = Number(__ENV.WARMUP_SECONDS || 10);
const durationSeconds = Number(__ENV.DURATION_SECONDS || 60);
const topK = Number(__ENV.TOP_K || 10);
const workload = __ENV.WORKLOAD || "topk";

if (!["topk", "count"].includes(workload)) {
  throw new Error("WORKLOAD must be topk or count");
}

if (![vus, durationSeconds, topK].every(Number.isSafeInteger)) {
  throw new Error("VUS, DURATION_SECONDS, and TOP_K must be integers");
}
if (vus < 1 || durationSeconds < 1 || topK < 1) {
  throw new Error("VUS, DURATION_SECONDS, and TOP_K must be positive");
}
if (!Number.isSafeInteger(warmupSeconds) || warmupSeconds < 0) {
  throw new Error("WARMUP_SECONDS must be a non-negative integer");
}

const queries = loadQueries(
  JSON.parse(open(__ENV.QUERIES || "./queries.json")),
  __ENV.QUERY_STYLE || "disjunction",
);
const paradedbRequests = queries.map((query) =>
  paradedbRequest(query, topK, workload),
);

const backends = db.backends({
  datasetPath: "..",
  backends: [
    {
      type: "paradedb",
      connection: __ENV.PARADEDB_URL,
      container: __ENV.PARADEDB_CONTAINER || "",
    },
  ],
});

const updateBackends =
  updatesPerSecond > 0
    ? db.backends({
        datasetPath: "..",
        backends: [
          {
            type: "paradedb",
            alias: "updates",
            connection: __ENV.PARADEDB_URL,
            container: "",
          },
        ],
      })
    : null;
const completed = new Counter("benchmark_writer_completed");
const updated = new Counter("benchmark_updated_rows");
const skipped = new Counter("benchmark_skipped_update_slots");

const timer = db.timer({
  duration: `${warmupSeconds + durationSeconds}s`,
  gap: "0s",
});
const stages = [
  ...(warmupSeconds > 0
    ? [{ duration: `${warmupSeconds}s`, target: vus }]
    : []),
  { duration: `${durationSeconds}s`, target: vus },
];
const scenario = (backend, startTime) => ({
  executor: "ramping-vus",
  startVUs: vus,
  stages,
  startTime,
  gracefulRampDown: "0s",
  exec: `${backend}Search`,
  tags: {
    backend,
    chart: `${workload}_${__ENV.QUERY_STYLE || "disjunction"}`,
    ...(warmupSeconds > 0 ? { warmup: "true" } : {}),
  },
});

const scenarios = {
  paradedb: scenario("paradedb", timer.get()),
};
if (updatesPerSecond > 0) {
  scenarios.updates = {
    executor: "per-vu-iterations",
    vus: 1,
    iterations: 1,
    startTime: `${warmupSeconds}s`,
    maxDuration: `${durationSeconds + GRACE_SECONDS}s`,
    gracefulStop: "0s",
    exec: "update",
    tags: { backend: "updates", chart: "random_updates" },
  };
}

const queryErrors = new Counter("benchmark_query_errors");
export const collectMetrics = backends.addMetricsCollector(scenarios, timer);
export const options = {
  scenarios,
  thresholds: {
    benchmark_query_errors: ["count==0"],
    ...(updatesPerSecond > 0
      ? {
          benchmark_writer_completed: ["count==1"],
          benchmark_updated_rows: ["count>0"],
        }
      : {}),
  },
};

export function paradedbSearch() {
  const index = Number(exec.scenario.iterationInTest) % queries.length;
  exec.vu.metrics.tags.query_id = queries[index].id;
  const result = backends.get("paradedb").query(...paradedbRequests[index]);
  queryErrors.add(result?.error && !result?.deadlineReached ? 1 : 0, {
    backend: "paradedb",
  });
}

// The scalar bounds preserve a TID range scan instead of scanning the heap.
const updateSQL = `
WITH bounds AS MATERIALIZED (
  SELECT floor(random() * GREATEST(
    pg_relation_size('documents') / current_setting('block_size')::bigint, 1
  ))::bigint AS block
), candidate AS MATERIALIZED (
  SELECT ctid FROM documents
  WHERE ctid >= (SELECT format('(%s,0)', block)::tid FROM bounds)
    AND ctid < (SELECT format('(%s,0)', block + 1)::tid FROM bounds)
  LIMIT 1
)
UPDATE documents SET body = body || ' '
WHERE ctid = (SELECT ctid FROM candidate)
RETURNING id`;

export function update() {
  completed.add(0);
  updated.add(0);
  const start = exec.scenario.startTime;
  const deadline = start + durationSeconds * MILLISECONDS_PER_SECOND;
  const interval = MILLISECONDS_PER_SECOND / updatesPerSecond;
  let next = start;
  while (next < deadline) {
    const delay = next - Date.now();
    if (delay > 0) sleep(delay / MILLISECONDS_PER_SECOND);
    if (Date.now() >= deadline) break;
    const result = updateBackends.get("updates").query(updateSQL);
    if (result.error) exec.test.abort(`Update failed: ${result.error}`);
    updated.add(result.hits);
    next += interval;
    const now = Date.now();
    if (next < now) {
      const missed = Math.ceil((now - next) / interval);
      skipped.add(missed);
      next += missed * interval;
    }
  }
  completed.add(1);
}
