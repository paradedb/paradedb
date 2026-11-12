const assert = require("node:assert/strict");
const { readFileSync } = require("node:fs");
const { join } = require("node:path");
const { test } = require("node:test");
const vm = require("node:vm");

const source = readFileSync(join(__dirname, "search.js"), "utf8")
  .replace(/^import .*;$/gm, "")
  .replace(/^export /gm, "");

function load(point) {
  let connection;
  const queries = [];
  const context = {
    __ENV: {
      OPERATING_POINTS: "points",
      OPERATING_POINT: point.name,
      QUERY_VECTORS: "vectors",
      PARADEDB_URL: "postgresql://localhost/hn",
    },
    open: (path) =>
      JSON.stringify(path === "points" ? [point] : Array(100).fill("[1,2,3]")),
    db: {
      backends: (config) => {
        connection = config.backends[0].connection;
        return {
          addMetricsCollector: () => () => {},
          get: () => ({
            query: (...args) => {
              queries.push(args);
              return { hits: 10 };
            },
          }),
        };
      },
    },
    exec: { scenario: { iterationInTest: 0 } },
    check: () => true,
  };
  vm.runInNewContext(source + "\nsearch();", context);
  return { options: new URL(connection).searchParams.get("options"), queries };
}

for (const prefilter of ["on", "off"]) {
  const point = {
    name: "author@r90",
    reached: true,
    engine: "vectorchord",
    backend: "postgres",
    session: { "vchordrq.prefilter": prefilter, "vchordrq.epsilon": "1.9" },
    param: "vchordrq.probes",
    probe: "23678",
    neighbors: 10,
    plan_mode:
      prefilter === "on" ? "vchordrq-prefilter" : "vchordrq-postfilter",
    sql: "SELECT id FROM hn_items WHERE by = 'GoodIntentions' ORDER BY emb <=> $1 LIMIT 10",
  };
  test(`k6 preserves ${prefilter} and the selected probes in connection startup options`, () => {
    const result = load(point);
    assert.match(
      result.options,
      new RegExp(`-c vchordrq.prefilter=${prefilter}( |$)`),
    );
    assert.match(result.options, /-c vchordrq.probes=23678( |$)/);
    assert.match(result.options, /-c vchordrq.epsilon=1.9( |$)/);
    assert.deepEqual(result.queries, [[point.sql, "[1,2,3]"]]);
  });
  test(`k6 rejects a ${prefilter} setting with a mismatched plan label`, () => {
    assert.throws(
      () => load({ ...point, plan_mode: "wrong" }),
      /differs from its verified plan/,
    );
  });
  test(`k6 rejects an implicit prefilter setting for ${prefilter}`, () => {
    assert.throws(() => load({ ...point, session: {} }), /explicit prefilter/);
  });
}
