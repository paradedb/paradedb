import { sleep } from "k6";
import exec from "k6/execution";
import { Counter } from "k6/metrics";
import db from "k6/x/database";
import { options as searchOptions } from "./search.js";

export { collectMetrics, paradedbSearch } from "./search.js";

const MILLISECONDS_PER_SECOND = 1000;
const GRACE_SECONDS = 5;
const durationSeconds = Number(__ENV.DURATION_SECONDS || 60);
const warmupSeconds = Number(__ENV.WARMUP_SECONDS || 10);
const updatesPerSecond = Number(__ENV.UPDATES_PER_SECOND || 1000);
if (!Number.isSafeInteger(updatesPerSecond) || updatesPerSecond < 1) {
  throw new Error("UPDATES_PER_SECOND must be a positive integer");
}

const backends = db.backends({
  datasetPath: "..",
  backends: [
    {
      type: "paradedb",
      alias: "updates",
      connection: __ENV.PARADEDB_URL,
      container: "",
    },
  ],
});
const completed = new Counter("benchmark_writer_completed");
const updated = new Counter("benchmark_updated_rows");
const skipped = new Counter("benchmark_skipped_update_slots");

export const options = {
  ...searchOptions,
  scenarios: {
    ...searchOptions.scenarios,
    updates: {
      executor: "per-vu-iterations",
      vus: 1,
      iterations: 1,
      startTime: `${warmupSeconds}s`,
      maxDuration: `${durationSeconds + GRACE_SECONDS}s`,
      gracefulStop: "0s",
      exec: "update",
      tags: { backend: "updates", chart: "random_updates" },
    },
  },
  thresholds: {
    ...searchOptions.thresholds,
    benchmark_writer_completed: ["count==1"],
    benchmark_updated_rows: ["count>0"],
  },
};

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
    const result = backends.get("updates").query(updateSQL);
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
