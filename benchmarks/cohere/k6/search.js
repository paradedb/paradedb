import { check } from "k6";
import exec from "k6/execution";
import db from "k6/x/database";

const points = JSON.parse(open(__ENV.OPERATING_POINTS));
const point = points.find((item) => item.name === __ENV.OPERATING_POINT);
if (!point?.reached)
  throw new Error("A reached recall operating point is required");

const vectors = JSON.parse(open(__ENV.QUERY_VECTORS));
if (vectors.length !== 100)
  throw new Error("Expected the 100 pdbench query vectors");

const session = {
  ...point.session,
  [point.param || "paradedb.vector_cluster_max_probe"]: point.probe,
};
if (point.engine === "vectorchord") {
  const prefilter = session["vchordrq.prefilter"];
  if (!["on", "off"].includes(prefilter))
    throw new Error("VectorChord requires an explicit prefilter setting");
  const mode =
    prefilter === "on" ? "vchordrq-prefilter" : "vchordrq-postfilter";
  if (point.plan_mode !== mode)
    throw new Error("VectorChord filter mode differs from its verified plan");
}
const settings = Object.entries(session)
  .map(([key, value]) => `-c ${key}=${value}`)
  .join(" ");
const url = `${__ENV.PARADEDB_URL}?default_query_exec_mode=exec&options=${encodeURIComponent(settings)}`;
const backends = db.backends({
  backends: [
    {
      type: point.backend || "paradedb",
      connection: url,
      container: "paradedb",
    },
  ],
});
const warmup = Number(__ENV.WARMUP_SECONDS || 10);
const duration = Number(__ENV.DURATION_SECONDS || 50);
const scenarios = {
  search: {
    executor: "ramping-vus",
    startVUs: 1,
    stages: [
      { duration: `${warmup}s`, target: 1 },
      { duration: `${duration}s`, target: 1 },
    ],
    gracefulRampDown: "0s",
    exec: "search",
    tags: { warmup: "true" },
  },
};
const requiredChecks =
  point.require_full_neighbors === false
    ? "checks{check:query succeeds}"
    : "checks";
export const collectMetrics = backends.addMetricsCollector(
  scenarios,
  `${warmup + duration}s`,
);
export const options = {
  scenarios,
  thresholds: { [requiredChecks]: ["rate==1"] },
};

export function search() {
  const vector = vectors[exec.scenario.iterationInTest % vectors.length];
  const result = backends
    .get(point.backend || "paradedb")
    .query(point.sql, vector);
  if (result.deadlineReached) return;
  check(result, {
    "query succeeds": (r) => !r.error,
    [`returns ${point.neighbors} neighbors`]: (r) => r.hits === point.neighbors,
  });
}
