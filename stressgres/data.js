window.BENCHMARK_DATA = {
  "lastUpdate": 1755715370986,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1755715366715,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1116.4887244249749,
            "unit": "median tps",
            "extra": "avg tps: 1115.1909350751087, max tps: 1119.7814567276114, count: 55310"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2456.5874448060495,
            "unit": "median tps",
            "extra": "avg tps: 2452.060805170511, max tps: 2598.528907822059, count: 55310"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1207.608238549296,
            "unit": "median tps",
            "extra": "avg tps: 1199.9283244493497, max tps: 1214.1541553935713, count: 55310"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1008.083315094636,
            "unit": "median tps",
            "extra": "avg tps: 1001.9484392304518, max tps: 1014.7259384910247, count: 55310"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.78190326345612,
            "unit": "median tps",
            "extra": "avg tps: 156.9076067934636, max tps: 162.11418622133576, count: 110620"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.18108691163968,
            "unit": "median tps",
            "extra": "avg tps: 156.6596247253203, max tps: 160.0890634072344, count: 55310"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 57.40801504088193,
            "unit": "median tps",
            "extra": "avg tps: 54.859603289900086, max tps: 583.3573968259524, count: 55310"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Ming",
            "username": "rebasedming",
            "email": "ming.ying.nyc@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba",
          "message": "chore: upgrade to `0.18.0` (#2980)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\nNote: `cargo.toml` is already on `0.18.0` so this is docs-only\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T19:09:25Z",
          "url": "https://github.com/paradedb/paradedb/commit/60c4cf138fc2eeb08d4326b839ba91c1e8e0fbba"
        },
        "date": 1755715369422,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.793445491765856, max cpu: 9.504951, count: 55310"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 54.94140625,
            "unit": "median mem",
            "extra": "avg mem: 54.443803886616344, max mem: 73.23046875, count: 55310"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.692518999191779, max cpu: 9.476802, count: 55310"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 49.734375,
            "unit": "median mem",
            "extra": "avg mem: 49.60782782555144, max mem: 68.859375, count: 55310"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.735427073532366, max cpu: 9.476802, count: 55310"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 54.625,
            "unit": "median mem",
            "extra": "avg mem: 54.99081575720937, max mem: 74.66015625, count: 55310"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.473963577664858, max cpu: 4.673807, count: 55310"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 54.21484375,
            "unit": "median mem",
            "extra": "avg mem: 54.611305101360514, max mem: 73.8203125, count: 55310"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.495478373512194, max cpu: 28.430405, count: 110620"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 64.37890625,
            "unit": "median mem",
            "extra": "avg mem: 64.61561154600209, max mem: 91.08203125, count: 110620"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3109,
            "unit": "median block_count",
            "extra": "avg block_count: 3133.2996384017356, max block_count: 5537.0, count: 55310"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.617790634604955, max segment_count: 28.0, count: 55310"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.6186586931913265, max cpu: 18.60465, count: 55310"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 71.9140625,
            "unit": "median mem",
            "extra": "avg mem: 72.55760147351293, max mem: 93.52734375, count: 55310"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 5.908369591653062, max cpu: 9.230769, count: 55310"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 55.99609375,
            "unit": "median mem",
            "extra": "avg mem: 54.15287279933556, max mem: 74.12109375, count: 55310"
          }
        ]
      }
    ]
  }
}