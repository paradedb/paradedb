window.BENCHMARK_DATA = {
  "lastUpdate": 1755715451336,
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
      },
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "f816692c9dd7d6faf21fccfd39aa05c498fa324a",
          "message": "chore: Fix triggers of cherry-pick workflow (#3002)\n\n## What\n\nAttempt to fix the triggers of [the cherry-pick\nworkflow](https://github.com/paradedb/paradedb/actions/workflows/cherry-pick.yml)\nso that it will actually run for a labeled PR.\n\n## Tests\n\nNone! Don't think that there is a way to test this.",
          "timestamp": "2025-08-20T18:13:09Z",
          "url": "https://github.com/paradedb/paradedb/commit/f816692c9dd7d6faf21fccfd39aa05c498fa324a"
        },
        "date": 1755715422036,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1162.6878606159319,
            "unit": "median tps",
            "extra": "avg tps: 1155.3722936269817, max tps: 1167.9343047346053, count: 55231"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2843.830276680142,
            "unit": "median tps",
            "extra": "avg tps: 2804.0848300007947, max tps: 2851.9283117369077, count: 55231"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1151.0065816402027,
            "unit": "median tps",
            "extra": "avg tps: 1146.1170965665438, max tps: 1153.891016188779, count: 55231"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1004.5746533709743,
            "unit": "median tps",
            "extra": "avg tps: 993.2911211559402, max tps: 1012.735325886529, count: 55231"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 162.12949942297377,
            "unit": "median tps",
            "extra": "avg tps: 161.31456872114617, max tps: 163.69407453786187, count: 110462"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 142.18153908276003,
            "unit": "median tps",
            "extra": "avg tps: 142.02986276537774, max tps: 152.43018629418535, count: 55231"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 47.094914826155176,
            "unit": "median tps",
            "extra": "avg tps: 52.837871470249326, max tps: 664.8746079733093, count: 55231"
          }
        ]
      },
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755715436960,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1217.475730917366,
            "unit": "median tps",
            "extra": "avg tps: 1215.240557921323, max tps: 1230.6982033635647, count: 55099"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2780.1117902708056,
            "unit": "median tps",
            "extra": "avg tps: 2765.816630729908, max tps: 2810.2185000234135, count: 55099"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1268.7229714541638,
            "unit": "median tps",
            "extra": "avg tps: 1262.8097455546767, max tps: 1274.6844554633963, count: 55099"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1054.8883289594573,
            "unit": "median tps",
            "extra": "avg tps: 1044.1330939698348, max tps: 1059.8597090152662, count: 55099"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 178.5516853793432,
            "unit": "median tps",
            "extra": "avg tps: 181.18751664006533, max tps: 191.5890441161113, count: 110198"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.31724143822922,
            "unit": "median tps",
            "extra": "avg tps: 152.48426975306273, max tps: 154.5186720356855, count: 55099"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.55842851199229,
            "unit": "median tps",
            "extra": "avg tps: 49.05609402960742, max tps: 867.716603757213, count: 55099"
          }
        ]
      },
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
          "id": "707c55b0a36223c016d33a5e6db16abdbc9a93c6",
          "message": "chore: Upgrade to `0.17.4` (#2976)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T14:06:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/707c55b0a36223c016d33a5e6db16abdbc9a93c6"
        },
        "date": 1755715446463,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1244.2922152013737,
            "unit": "median tps",
            "extra": "avg tps: 1242.1228332046906, max tps: 1267.3932401326927, count: 55183"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2633.008264535682,
            "unit": "median tps",
            "extra": "avg tps: 2624.487771500032, max tps: 2664.7198652100897, count: 55183"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1185.2814399845097,
            "unit": "median tps",
            "extra": "avg tps: 1184.8611840253259, max tps: 1214.7395614944787, count: 55183"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1047.8483463672392,
            "unit": "median tps",
            "extra": "avg tps: 1040.7394068776596, max tps: 1055.7212425433822, count: 55183"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 162.20104535697016,
            "unit": "median tps",
            "extra": "avg tps: 174.85557994528193, max tps: 191.79265747394686, count: 110366"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 137.47579599739996,
            "unit": "median tps",
            "extra": "avg tps: 137.1767647275256, max tps: 150.32382457642606, count: 55183"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 250.86569596715174,
            "unit": "median tps",
            "extra": "avg tps: 205.48229741327762, max tps: 761.5415428527042, count: 55183"
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
      },
      {
        "commit": {
          "author": {
            "name": "Stu Hood",
            "username": "stuhood",
            "email": "stuhood@paradedb.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "f816692c9dd7d6faf21fccfd39aa05c498fa324a",
          "message": "chore: Fix triggers of cherry-pick workflow (#3002)\n\n## What\n\nAttempt to fix the triggers of [the cherry-pick\nworkflow](https://github.com/paradedb/paradedb/actions/workflows/cherry-pick.yml)\nso that it will actually run for a labeled PR.\n\n## Tests\n\nNone! Don't think that there is a way to test this.",
          "timestamp": "2025-08-20T18:13:09Z",
          "url": "https://github.com/paradedb/paradedb/commit/f816692c9dd7d6faf21fccfd39aa05c498fa324a"
        },
        "date": 1755715425306,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.743593240602186, max cpu: 9.504951, count: 55231"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.9609375,
            "unit": "median mem",
            "extra": "avg mem: 91.58824315714907, max mem: 145.57421875, count: 55231"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.690070799589131, max cpu: 9.467456, count: 55231"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83.81640625,
            "unit": "median mem",
            "extra": "avg mem: 84.26868763409588, max mem: 136.38671875, count: 55231"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.749162456815798, max cpu: 9.571285, count: 55231"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 90.33203125,
            "unit": "median mem",
            "extra": "avg mem: 91.3088342880357, max mem: 144.34375, count: 55231"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.093089690227481, max cpu: 4.743083, count: 55231"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 91.25390625,
            "unit": "median mem",
            "extra": "avg mem: 92.64457403903604, max mem: 145.59375, count: 55231"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.611603852938297, max cpu: 28.235296, count: 110462"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.4453125,
            "unit": "median mem",
            "extra": "avg mem: 101.63635518464494, max mem: 159.90234375, count: 110462"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7646,
            "unit": "median block_count",
            "extra": "avg block_count: 7751.887545038113, max block_count: 14472.0, count: 55231"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.611160399051258, max segment_count: 36.0, count: 55231"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 6.740220057625452, max cpu: 19.257774, count: 55231"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 106.515625,
            "unit": "median mem",
            "extra": "avg mem: 107.09344566740145, max mem: 163.54296875, count: 55231"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.388270378986754, max cpu: 9.320388, count: 55231"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.3671875,
            "unit": "median mem",
            "extra": "avg mem: 84.30926925096414, max mem: 135.890625, count: 55231"
          }
        ]
      },
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
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T17:19:49Z",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755715440114,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.724949680051945, max cpu: 9.458128, count: 55099"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.609375,
            "unit": "median mem",
            "extra": "avg mem: 59.70736601673806, max mem: 82.50390625, count: 55099"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.636141322317165, max cpu: 9.204219, count: 55099"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.4765625,
            "unit": "median mem",
            "extra": "avg mem: 53.82671532151219, max mem: 78.65625, count: 55099"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.735309301205046, max cpu: 9.4395275, count: 55099"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 61.98828125,
            "unit": "median mem",
            "extra": "avg mem: 61.16344656323164, max mem: 85.05078125, count: 55099"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.489461522593634, max cpu: 4.6829267, count: 55099"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 60.59375,
            "unit": "median mem",
            "extra": "avg mem: 59.63628862933084, max mem: 84.03125, count: 55099"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 7.255784393257529, max cpu: 18.916256, count: 110198"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 73.00390625,
            "unit": "median mem",
            "extra": "avg mem: 73.35650980933184, max mem: 103.0078125, count: 110198"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3843,
            "unit": "median block_count",
            "extra": "avg block_count: 3783.9804715149094, max block_count: 6818.0, count: 55099"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.890016152743245, max segment_count: 28.0, count: 55099"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1094536593954585, max cpu: 18.461538, count: 55099"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.00390625,
            "unit": "median mem",
            "extra": "avg mem: 77.20159869904626, max mem: 105.8984375, count: 55099"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.317881804680662, max cpu: 4.64666, count: 55099"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 61.92578125,
            "unit": "median mem",
            "extra": "avg mem: 59.91036333463402, max mem: 84.99609375, count: 55099"
          }
        ]
      },
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
          "id": "707c55b0a36223c016d33a5e6db16abdbc9a93c6",
          "message": "chore: Upgrade to `0.17.4` (#2976)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-18T14:06:23Z",
          "url": "https://github.com/paradedb/paradedb/commit/707c55b0a36223c016d33a5e6db16abdbc9a93c6"
        },
        "date": 1755715449765,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.736707832237801, max cpu: 9.571285, count: 55183"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.87890625,
            "unit": "median mem",
            "extra": "avg mem: 59.62295347129098, max mem: 83.0, count: 55183"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.61458010364591, max cpu: 9.284333, count: 55183"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.58984375,
            "unit": "median mem",
            "extra": "avg mem: 52.99249875131381, max mem: 75.44921875, count: 55183"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.782903485710618, max cpu: 9.619239, count: 55183"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 61.22265625,
            "unit": "median mem",
            "extra": "avg mem: 60.97361768173622, max mem: 85.0390625, count: 55183"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2980698390839995, max cpu: 4.7151275, count: 55183"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.73828125,
            "unit": "median mem",
            "extra": "avg mem: 59.669003163338346, max mem: 83.0703125, count: 55183"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 7.491760605578794, max cpu: 28.91566, count: 110366"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 75.046875,
            "unit": "median mem",
            "extra": "avg mem: 75.17659572751118, max mem: 104.39453125, count: 110366"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3683,
            "unit": "median block_count",
            "extra": "avg block_count: 3692.824547414965, max block_count: 6637.0, count: 55183"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.92231303118714, max segment_count: 27.0, count: 55183"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.173730587157698, max cpu: 14.243324, count: 55183"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.53515625,
            "unit": "median mem",
            "extra": "avg mem: 77.67199908995977, max mem: 105.15234375, count: 55183"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 2.589938720104196, max cpu: 9.29332, count: 55183"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 57.53125,
            "unit": "median mem",
            "extra": "avg mem: 57.18876291441658, max mem: 83.578125, count: 55183"
          }
        ]
      }
    ]
  }
}