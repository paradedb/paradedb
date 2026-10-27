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
  "paradedb.vector_cluster_max_probe": point.probe,
};
const settings = Object.entries(session)
  .map(([key, value]) => `-c ${key}=${value}`)
  .join(" ");
const url = `${__ENV.PARADEDB_URL}?default_query_exec_mode=exec&options=${encodeURIComponent(settings)}`;
const backends = db.backends({
  backends: [
    {
      type: "paradedb",
      connection: url,
      container: "",
    },
  ],
});
const warmup = Number(__ENV.WARMUP_SECONDS || 30);
const duration = Number(__ENV.DURATION_SECONDS || 300);
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
export const options = { scenarios, thresholds: { checks: ["rate==1"] } };

export function search() {
  const vector = vectors[exec.scenario.iterationInTest % vectors.length];
  const result = backends.get("paradedb").query(point.sql, vector);
  if (result.deadlineReached) return;
  check(result, {
    "query succeeds": (r) => !r.error,
    "returns 10 neighbors": (r) => r.hits === 10,
  });
}
