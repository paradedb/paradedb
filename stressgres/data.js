window.BENCHMARK_DATA = {
  "lastUpdate": 1752252244082,
  "repoUrl": "https://github.com/paradedb/paradedb",
  "entries": {
    "pg_search wide-table.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237074771,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.825549325934574,
            "unit": "median tps",
            "extra": "avg tps: 38.10476012239314, max tps: 39.32428486805184, count: 59161"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 202.7817098654163,
            "unit": "median tps",
            "extra": "avg tps: 206.11763802798248, max tps: 243.09219735024135, count: 59161"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238150078,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.76214151689536,
            "unit": "median tps",
            "extra": "avg tps: 37.02609090387134, max tps: 38.15774112652048, count: 59180"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 202.66915644497536,
            "unit": "median tps",
            "extra": "avg tps: 206.94870979720113, max tps: 252.9890260121063, count: 59180"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239187807,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.82234014574883,
            "unit": "median tps",
            "extra": "avg tps: 38.186195096009584, max tps: 39.58086860017298, count: 59153"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 201.74464204799588,
            "unit": "median tps",
            "extra": "avg tps: 205.23224918969623, max tps: 233.7369525946125, count: 59153"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240190472,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.322322974767935,
            "unit": "median tps",
            "extra": "avg tps: 36.81398351471555, max tps: 38.37935597937283, count: 59171"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 200.01435827198972,
            "unit": "median tps",
            "extra": "avg tps: 204.28389298607237, max tps: 238.33243225119554, count: 59171"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242269730,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.19904665199496,
            "unit": "median tps",
            "extra": "avg tps: 37.510360712348565, max tps: 38.69780589761047, count: 59165"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 214.78443922622802,
            "unit": "median tps",
            "extra": "avg tps: 216.85950065619, max tps: 246.3335642076657, count: 59165"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242270591,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.55215050040496,
            "unit": "median tps",
            "extra": "avg tps: 37.86920875539607, max tps: 39.12103939840368, count: 59156"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 200.95279863069567,
            "unit": "median tps",
            "extra": "avg tps: 205.07650664493136, max tps: 238.30423395993574, count: 59156"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242297691,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.530638674566504,
            "unit": "median tps",
            "extra": "avg tps: 38.54578320184089, max tps: 39.69305130939306, count: 59169"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 203.3153258696447,
            "unit": "median tps",
            "extra": "avg tps: 204.12094986004436, max tps: 236.93919046235746, count: 59169"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242298112,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 37.09741977565106,
            "unit": "median tps",
            "extra": "avg tps: 37.636850036873874, max tps: 39.2388578805394, count: 59111"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 200.2565596201875,
            "unit": "median tps",
            "extra": "avg tps: 204.49374794218056, max tps: 222.20521665144904, count: 59111"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242309745,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.2119626806918,
            "unit": "median tps",
            "extra": "avg tps: 38.47800874983463, max tps: 39.821689152547016, count: 59164"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 204.7102159695084,
            "unit": "median tps",
            "extra": "avg tps: 208.70117700234437, max tps: 224.13753765138432, count: 59164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752245033538,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 36.288242763496385,
            "unit": "median tps",
            "extra": "avg tps: 36.766143239941414, max tps: 38.0815200998229, count: 59174"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 204.17603081686661,
            "unit": "median tps",
            "extra": "avg tps: 207.86482160704657, max tps: 235.55131576811144, count: 59174"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0e0a552a073d66f6de8ad45fc0587a4f2ba39dac",
          "message": "chore: Run benchmarks on `benchmark`-labeled PRs. (#2826)\n\n## What\n\nAdjust benchmarks jobs to automatically run when the `benchmark` label\nis applied.\n\n## Why\n\n#2820 failed to actually filter to `perf:`-titled PRs, but additionally,\nin practice that would have been too noisy, since they would have re-run\non every push to the PR.\n\n## Tests\n\nManually tested adding/removing the label.",
          "timestamp": "2025-07-11T09:27:20-07:00",
          "tree_id": "0d8f5a0d7fda145130d823ec6bc08e217a9677d0",
          "url": "https://github.com/paradedb/paradedb/commit/0e0a552a073d66f6de8ad45fc0587a4f2ba39dac"
        },
        "date": 1752252243234,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 38.96194088043833,
            "unit": "median tps",
            "extra": "avg tps: 39.091641575236, max tps: 40.244620676149026, count: 59159"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 201.55868289259345,
            "unit": "median tps",
            "extra": "avg tps: 204.59735368339355, max tps: 238.58220830567413, count: 59159"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237088919,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 390.61062538021366,
            "unit": "median tps",
            "extra": "avg tps: 394.6734848646864, max tps: 587.3638830351085, count: 58418"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2516.447798620683,
            "unit": "median tps",
            "extra": "avg tps: 2452.522804628353, max tps: 2783.7517386232435, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 391.61871424174217,
            "unit": "median tps",
            "extra": "avg tps: 397.22787805247185, max tps: 592.9112646606859, count: 58418"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 386.9820676683288,
            "unit": "median tps",
            "extra": "avg tps: 387.3600215614747, max tps: 486.7151347332137, count: 58418"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 265.8198623483887,
            "unit": "median tps",
            "extra": "avg tps: 264.24797005831863, max tps: 275.9731733626332, count: 116836"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 244.3693135727048,
            "unit": "median tps",
            "extra": "avg tps: 241.1945628713284, max tps: 247.34549920140702, count: 58418"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 9.438228530430472,
            "unit": "median tps",
            "extra": "avg tps: 14.39237952297812, max tps: 1467.1490652793304, count: 58418"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238143945,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 424.8040573830263,
            "unit": "median tps",
            "extra": "avg tps: 430.26262891612987, max tps: 625.4019075870007, count: 58429"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2669.4671215626186,
            "unit": "median tps",
            "extra": "avg tps: 2585.349611022355, max tps: 3103.1823057924116, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 425.5931240671708,
            "unit": "median tps",
            "extra": "avg tps: 431.9001976013556, max tps: 636.9263162649956, count: 58429"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 428.99259209280535,
            "unit": "median tps",
            "extra": "avg tps: 430.5479095073583, max tps: 560.229864554346, count: 58429"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 293.9350646225956,
            "unit": "median tps",
            "extra": "avg tps: 292.02900262617015, max tps: 303.6400647409563, count: 116858"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 269.3806150354518,
            "unit": "median tps",
            "extra": "avg tps: 266.1746204223828, max tps: 275.8882036791982, count: 58429"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.308464214984275,
            "unit": "median tps",
            "extra": "avg tps: 20.10877557684736, max tps: 1580.3180864244357, count: 58429"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239192222,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 419.9507849883561,
            "unit": "median tps",
            "extra": "avg tps: 425.04714770413767, max tps: 630.6280458892587, count: 58487"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2504.301465772778,
            "unit": "median tps",
            "extra": "avg tps: 2359.1891961968727, max tps: 3045.2768244477948, count: 58487"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 415.51375897220925,
            "unit": "median tps",
            "extra": "avg tps: 421.2410175397073, max tps: 608.7548401627973, count: 58487"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 418.02264178386986,
            "unit": "median tps",
            "extra": "avg tps: 420.50438026908574, max tps: 523.1861176539895, count: 58487"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 287.0577202647603,
            "unit": "median tps",
            "extra": "avg tps: 291.91354072135937, max tps: 321.6306651395404, count: 116974"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 259.018805817646,
            "unit": "median tps",
            "extra": "avg tps: 253.54578824946495, max tps: 281.59551880155203, count: 58487"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 14.454366628858304,
            "unit": "median tps",
            "extra": "avg tps: 20.92424510888961, max tps: 1789.8276932879673, count: 58487"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240199154,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 422.78737942877837,
            "unit": "median tps",
            "extra": "avg tps: 428.9491122020344, max tps: 617.6369137335371, count: 58484"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2648.23794528854,
            "unit": "median tps",
            "extra": "avg tps: 2535.02234211775, max tps: 3028.8331553893854, count: 58484"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 423.59314899254184,
            "unit": "median tps",
            "extra": "avg tps: 429.1871024007199, max tps: 625.2070998518259, count: 58484"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 423.78290965200836,
            "unit": "median tps",
            "extra": "avg tps: 425.7119938468826, max tps: 535.8722819151196, count: 58484"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.705323484711,
            "unit": "median tps",
            "extra": "avg tps: 287.24118177604964, max tps: 301.3746025700164, count: 116968"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 271.73480830906306,
            "unit": "median tps",
            "extra": "avg tps: 263.54524606254665, max tps: 276.5806397735441, count: 58484"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 11.892687396008617,
            "unit": "median tps",
            "extra": "avg tps: 18.462332175309673, max tps: 1662.546322696916, count: 58484"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242267577,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 420.91687397936823,
            "unit": "median tps",
            "extra": "avg tps: 424.99157028125353, max tps: 623.0053840875207, count: 58510"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2653.7520182228814,
            "unit": "median tps",
            "extra": "avg tps: 2520.5699511161424, max tps: 3096.183607226404, count: 58510"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 421.3652384755941,
            "unit": "median tps",
            "extra": "avg tps: 427.5806729862549, max tps: 630.8541623415919, count: 58510"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 426.0759813743613,
            "unit": "median tps",
            "extra": "avg tps: 427.0602445313631, max tps: 542.8882534562845, count: 58510"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.01794812443103,
            "unit": "median tps",
            "extra": "avg tps: 286.1915647693137, max tps: 294.84872362983583, count: 117020"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 264.115255095766,
            "unit": "median tps",
            "extra": "avg tps: 259.1327778774077, max tps: 277.2598176966288, count: 58510"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 11.763587604842462,
            "unit": "median tps",
            "extra": "avg tps: 18.397809351287904, max tps: 1605.4121654923076, count: 58510"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242275510,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 421.0846198323314,
            "unit": "median tps",
            "extra": "avg tps: 426.6024206149527, max tps: 610.0215167131444, count: 58509"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2555.3081517484957,
            "unit": "median tps",
            "extra": "avg tps: 2402.861885873489, max tps: 3015.913578334383, count: 58509"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 421.33914273846483,
            "unit": "median tps",
            "extra": "avg tps: 426.2360583639731, max tps: 621.5880177582862, count: 58509"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 421.34124513049187,
            "unit": "median tps",
            "extra": "avg tps: 423.90369642819525, max tps: 557.1057642452083, count: 58509"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 289.1995655225524,
            "unit": "median tps",
            "extra": "avg tps: 297.7753920246677, max tps: 331.7102176499733, count: 117018"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 285.77452956413526,
            "unit": "median tps",
            "extra": "avg tps: 281.92246793622684, max tps: 296.44102987387447, count: 58509"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.969513450717152,
            "unit": "median tps",
            "extra": "avg tps: 19.127201812216185, max tps: 1420.277010828192, count: 58509"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242287493,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 426.7746932092187,
            "unit": "median tps",
            "extra": "avg tps: 433.01058402052513, max tps: 634.626643475137, count: 58467"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2726.771190589186,
            "unit": "median tps",
            "extra": "avg tps: 2654.5192862854824, max tps: 3102.5710283182502, count: 58467"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 427.36646488181594,
            "unit": "median tps",
            "extra": "avg tps: 433.97764499350416, max tps: 624.5086873061583, count: 58467"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 429.4603086713481,
            "unit": "median tps",
            "extra": "avg tps: 431.60982949016335, max tps: 540.6456595912189, count: 58467"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 292.8062642973656,
            "unit": "median tps",
            "extra": "avg tps: 291.3301503282136, max tps: 306.1528143403611, count: 116934"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 271.52702802451535,
            "unit": "median tps",
            "extra": "avg tps: 269.2492972962839, max tps: 277.68593806235344, count: 58467"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 12.636123097237364,
            "unit": "median tps",
            "extra": "avg tps: 19.1632419984933, max tps: 1590.3206722603547, count: 58467"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242300333,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 429.22984855556956,
            "unit": "median tps",
            "extra": "avg tps: 435.46898607315154, max tps: 625.4298346164885, count: 58520"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2824.351115567359,
            "unit": "median tps",
            "extra": "avg tps: 2791.8626056279327, max tps: 3116.1674863131448, count: 58520"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 427.8219076000739,
            "unit": "median tps",
            "extra": "avg tps: 433.9641590739696, max tps: 637.1425581578122, count: 58520"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 430.11195443547155,
            "unit": "median tps",
            "extra": "avg tps: 432.4492332657866, max tps: 549.9135948264129, count: 58520"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 295.9500114239412,
            "unit": "median tps",
            "extra": "avg tps: 293.533000021387, max tps: 305.7479194777693, count: 117040"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 274.0168310858391,
            "unit": "median tps",
            "extra": "avg tps: 270.8722098516149, max tps: 277.45126989182813, count: 58520"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 19.057231501226926,
            "unit": "median tps",
            "extra": "avg tps: 24.232086089574388, max tps: 1669.1063437724808, count: 58520"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242310778,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 421.6242857582653,
            "unit": "median tps",
            "extra": "avg tps: 425.33345426764777, max tps: 623.1012971220861, count: 58517"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2606.589693763638,
            "unit": "median tps",
            "extra": "avg tps: 2483.77266598526, max tps: 3123.3352222837634, count: 58517"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 421.7229307025697,
            "unit": "median tps",
            "extra": "avg tps: 425.92747223300324, max tps: 616.1836317596243, count: 58517"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 429.3841726562211,
            "unit": "median tps",
            "extra": "avg tps: 430.5376819449203, max tps: 543.4003242687096, count: 58517"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.13684866542326,
            "unit": "median tps",
            "extra": "avg tps: 286.57712906538285, max tps: 297.8622849327923, count: 117034"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 265.68227415044765,
            "unit": "median tps",
            "extra": "avg tps: 261.54181257271244, max tps: 272.2117785860457, count: 58517"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.216902839826679,
            "unit": "median tps",
            "extra": "avg tps: 18.804213195048224, max tps: 1537.8156558860662, count: 58517"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752244967409,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 421.2826013107481,
            "unit": "median tps",
            "extra": "avg tps: 428.325355520748, max tps: 616.2326010612538, count: 58492"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2736.17931266536,
            "unit": "median tps",
            "extra": "avg tps: 2617.8567420999116, max tps: 3091.7522449634653, count: 58492"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 423.0311407474487,
            "unit": "median tps",
            "extra": "avg tps: 429.7644639397119, max tps: 625.2291059432195, count: 58492"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 422.5294389352778,
            "unit": "median tps",
            "extra": "avg tps: 427.4669617590523, max tps: 549.6119904231209, count: 58492"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 290.3436499725682,
            "unit": "median tps",
            "extra": "avg tps: 289.142625824739, max tps: 306.39042894660054, count: 116984"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 267.6721357924853,
            "unit": "median tps",
            "extra": "avg tps: 264.93818993107095, max tps: 279.0778420400053, count: 58492"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 13.157395946021119,
            "unit": "median tps",
            "extra": "avg tps: 18.466957270305603, max tps: 1429.6252666251123, count: 58492"
          }
        ]
      }
    ],
    "pg_search single-server.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237097892,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.885071801192753, max cpu: 23.738873, count: 58418"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.9375,
            "unit": "median mem",
            "extra": "avg mem: 91.59658617367506, max mem: 100.84765625, count: 58418"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.698972,
            "unit": "median cpu",
            "extra": "avg cpu: 4.744757178684784, max cpu: 9.538003, count: 58418"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83,
            "unit": "median mem",
            "extra": "avg mem: 76.55455082337636, max mem: 86.0, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.880863164202392, max cpu: 24.242424, count: 58418"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 98.99609375,
            "unit": "median mem",
            "extra": "avg mem: 92.26902158795234, max mem: 101.1953125, count: 58418"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.692082,
            "unit": "median cpu",
            "extra": "avg cpu: 4.477421035393748, max cpu: 4.7832584, count: 58418"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.23828125,
            "unit": "median mem",
            "extra": "avg mem: 88.48916001917217, max mem: 96.58203125, count: 58418"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 6.611805182822304, max cpu: 19.783615, count: 116836"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 97.65625,
            "unit": "median mem",
            "extra": "avg mem: 92.39853566109761, max mem: 106.796875, count: 116836"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7699,
            "unit": "median block_count",
            "extra": "avg block_count: 6908.741500907255, max block_count: 7699.0, count: 58418"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.43637235098771, max segment_count: 369.0, count: 58418"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 5.526272854948317, max cpu: 14.328358, count: 58418"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 102.63671875,
            "unit": "median mem",
            "extra": "avg mem: 97.33222670292119, max mem: 110.59765625, count: 58418"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.552238,
            "unit": "median cpu",
            "extra": "avg cpu: 13.272652448144251, max cpu: 32.844578, count: 58418"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 77.2421875,
            "unit": "median mem",
            "extra": "avg mem: 72.8856530200452, max mem: 83.0703125, count: 58418"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238182794,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.606133071552365, max cpu: 28.152493, count: 58429"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 82.921875,
            "unit": "median mem",
            "extra": "avg mem: 81.84320072224409, max mem: 87.34375, count: 58429"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759502174083395, max cpu: 9.595202, count: 58429"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 70.9296875,
            "unit": "median mem",
            "extra": "avg mem: 69.13551115456366, max mem: 73.1796875, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.606299078939965, max cpu: 23.952095, count: 58429"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 82.3984375,
            "unit": "median mem",
            "extra": "avg mem: 81.69680937548135, max mem: 87.35546875, count: 58429"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 4.259818066665957, max cpu: 4.797601, count: 58429"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 77.6796875,
            "unit": "median mem",
            "extra": "avg mem: 77.43897762134385, max mem: 83.1796875, count: 58429"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 6.396690470046422, max cpu: 24.615385, count: 116858"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 88.1796875,
            "unit": "median mem",
            "extra": "avg mem: 87.42538590506213, max mem: 96.4296875, count: 116858"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 5727,
            "unit": "median block_count",
            "extra": "avg block_count: 5675.860754077598, max block_count: 6064.0, count: 58429"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.52431155761694, max segment_count: 265.0, count: 58429"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 5.324219863529958, max cpu: 14.180207, count: 58429"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 89.73828125,
            "unit": "median mem",
            "extra": "avg mem: 89.05469044160434, max mem: 96.11328125, count: 58429"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.138439,
            "unit": "median cpu",
            "extra": "avg cpu: 14.647324940570195, max cpu: 28.828829, count: 58429"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 70.91796875,
            "unit": "median mem",
            "extra": "avg mem: 68.47138748256431, max mem: 72.2734375, count: 58429"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239203936,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.646965346156069, max cpu: 24.767801, count: 58487"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 86.234375,
            "unit": "median mem",
            "extra": "avg mem: 87.36479454034658, max mem: 100.41796875, count: 58487"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.74947941634801, max cpu: 9.624061, count: 58487"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 75,
            "unit": "median mem",
            "extra": "avg mem: 73.65216201891019, max mem: 86.5, count: 58487"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.649376795385369, max cpu: 28.276878, count: 58487"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 87.03125,
            "unit": "median mem",
            "extra": "avg mem: 87.83397565912082, max mem: 102.2265625, count: 58487"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7058825,
            "unit": "median cpu",
            "extra": "avg cpu: 4.588910140144389, max cpu: 4.804805, count: 58487"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 82.76953125,
            "unit": "median mem",
            "extra": "avg mem: 82.6927249858943, max mem: 94.91015625, count: 58487"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 6.316505723093951, max cpu: 23.564064, count: 116974"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 92.6328125,
            "unit": "median mem",
            "extra": "avg mem: 93.05821866664814, max mem: 108.53125, count: 116974"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6223,
            "unit": "median block_count",
            "extra": "avg block_count: 6380.314685314685, max block_count: 7676.0, count: 58487"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.81597619983928, max segment_count: 402.0, count: 58487"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.487807291598191, max cpu: 14.43609, count: 58487"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 94.49609375,
            "unit": "median mem",
            "extra": "avg mem: 93.35109224325491, max mem: 110.828125, count: 58487"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.740849,
            "unit": "median cpu",
            "extra": "avg cpu: 16.081865130268024, max cpu: 28.828829, count: 58487"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 76.58203125,
            "unit": "median mem",
            "extra": "avg mem: 74.4542117799682, max mem: 89.26953125, count: 58487"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240203023,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.596913468053925, max cpu: 24.390244, count: 58484"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.72265625,
            "unit": "median mem",
            "extra": "avg mem: 90.57149671811949, max mem: 100.46875, count: 58484"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.713052404153525, max cpu: 9.481482, count: 58484"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 81.94921875,
            "unit": "median mem",
            "extra": "avg mem: 75.25564530737039, max mem: 82.94921875, count: 58484"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.5411560787201415, max cpu: 24.390244, count: 58484"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.80078125,
            "unit": "median mem",
            "extra": "avg mem: 90.43691777612253, max mem: 101.48046875, count: 58484"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.712813,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3996947243326785, max cpu: 4.7904196, count: 58484"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.52734375,
            "unit": "median mem",
            "extra": "avg mem: 86.33824609081117, max mem: 94.609375, count: 58484"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.365506951255694, max cpu: 24.767801, count: 116968"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 98.16796875,
            "unit": "median mem",
            "extra": "avg mem: 94.01243535892296, max mem: 109.1640625, count: 116968"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7720,
            "unit": "median block_count",
            "extra": "avg block_count: 6761.632104507215, max block_count: 7720.0, count: 58484"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 118,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.6186820326927, max segment_count: 352.0, count: 58484"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 5.489308406597192, max cpu: 19.512194, count: 58484"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 106.81640625,
            "unit": "median mem",
            "extra": "avg mem: 99.48099713970915, max mem: 113.51171875, count: 58484"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.138439,
            "unit": "median cpu",
            "extra": "avg cpu: 14.504301650257142, max cpu: 28.785606, count: 58484"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 75.1640625,
            "unit": "median mem",
            "extra": "avg mem: 73.02810644524571, max mem: 80.1875, count: 58484"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242288445,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.610271424460606, max cpu: 29.135054, count: 58510"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 84.18359375,
            "unit": "median mem",
            "extra": "avg mem: 84.38099637081268, max mem: 99.97265625, count: 58510"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.662249800874041, max cpu: 9.4395275, count: 58510"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 72.75390625,
            "unit": "median mem",
            "extra": "avg mem: 71.59764656949666, max mem: 81.00390625, count: 58510"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.667693636464605, max cpu: 28.276878, count: 58510"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 84.8984375,
            "unit": "median mem",
            "extra": "avg mem: 84.93446018842933, max mem: 100.59765625, count: 58510"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.593727807438234, max cpu: 4.804805, count: 58510"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 83.2109375,
            "unit": "median mem",
            "extra": "avg mem: 81.38928194699623, max mem: 94.59765625, count: 58510"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.414213479338727, max cpu: 23.952095, count: 117020"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 86.19921875,
            "unit": "median mem",
            "extra": "avg mem: 86.92594074463767, max mem: 105.65625, count: 117020"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 5970,
            "unit": "median block_count",
            "extra": "avg block_count: 6003.556862074859, max block_count: 7676.0, count: 58510"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.91196376687746, max segment_count: 277.0, count: 58510"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.472635432277946, max cpu: 19.21922, count: 58510"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 92.9609375,
            "unit": "median mem",
            "extra": "avg mem: 92.39285131548026, max mem: 105.4609375, count: 58510"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.159292,
            "unit": "median cpu",
            "extra": "avg cpu: 14.210967599513943, max cpu: 28.360415, count: 58510"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 75.6796875,
            "unit": "median mem",
            "extra": "avg mem: 74.25170490354213, max mem: 77.23828125, count: 58510"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242317418,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.637184258229063, max cpu: 24.615385, count: 58467"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 94.94921875,
            "unit": "median mem",
            "extra": "avg mem: 91.91918825469924, max mem: 97.609375, count: 58467"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.793934864531813, max cpu: 9.60961, count: 58467"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83.44921875,
            "unit": "median mem",
            "extra": "avg mem: 78.11979159985547, max mem: 83.44921875, count: 58467"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.589710423313957, max cpu: 29.135054, count: 58467"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 95.1796875,
            "unit": "median mem",
            "extra": "avg mem: 92.19890981504524, max mem: 98.10546875, count: 58467"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.712813,
            "unit": "median cpu",
            "extra": "avg cpu: 4.53573482552681, max cpu: 4.776119, count: 58467"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 90.71484375,
            "unit": "median mem",
            "extra": "avg mem: 87.00079425145809, max mem: 92.296875, count: 58467"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.48395045557315, max cpu: 24.767801, count: 116934"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 95.59765625,
            "unit": "median mem",
            "extra": "avg mem: 93.45675592107514, max mem: 102.37109375, count: 116934"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7270,
            "unit": "median block_count",
            "extra": "avg block_count: 6920.844236919972, max block_count: 7270.0, count: 58467"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.6504523919476, max segment_count: 281.0, count: 58467"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.3739551851160865, max cpu: 18.991098, count: 58467"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 99.0234375,
            "unit": "median mem",
            "extra": "avg mem: 95.23472582557254, max mem: 104.640625, count: 58467"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 11.455848,
            "unit": "median cpu",
            "extra": "avg cpu: 12.591184729436383, max cpu: 29.003021, count: 58467"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 78.37890625,
            "unit": "median mem",
            "extra": "avg mem: 75.23847480202507, max mem: 80.6328125, count: 58467"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242327295,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.551353490927696, max cpu: 34.042553, count: 58509"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.36328125,
            "unit": "median mem",
            "extra": "avg mem: 94.07202157200174, max mem: 102.5390625, count: 58509"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.694829622717155, max cpu: 9.595202, count: 58509"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 78,
            "unit": "median mem",
            "extra": "avg mem: 79.20176810405236, max mem: 87.5, count: 58509"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.633696489004427, max cpu: 29.17933, count: 58509"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 93.69140625,
            "unit": "median mem",
            "extra": "avg mem: 95.04932981731443, max mem: 102.8828125, count: 58509"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.712813,
            "unit": "median cpu",
            "extra": "avg cpu: 4.234391175822738, max cpu: 4.8780484, count: 58509"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 88.984375,
            "unit": "median mem",
            "extra": "avg mem: 89.69197340366439, max mem: 96.94921875, count: 58509"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 6.185642336854058, max cpu: 23.59882, count: 117018"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 98.703125,
            "unit": "median mem",
            "extra": "avg mem: 99.3022446666859, max mem: 110.05078125, count: 117018"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7080,
            "unit": "median block_count",
            "extra": "avg block_count: 7306.942094378643, max block_count: 7959.0, count: 58509"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.13572270932677, max segment_count: 333.0, count: 58509"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.333852683744226, max cpu: 14.222222, count: 58509"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 103.4609375,
            "unit": "median mem",
            "extra": "avg mem: 101.82232260741937, max mem: 109.1484375, count: 58509"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 15.274464,
            "unit": "median cpu",
            "extra": "avg cpu: 16.10857933592802, max cpu: 28.785606, count: 58509"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 77.3515625,
            "unit": "median mem",
            "extra": "avg mem: 77.09715445754073, max mem: 81.39453125, count: 58509"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242345765,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.610174950628639, max cpu: 24.353119, count: 58520"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 87.62890625,
            "unit": "median mem",
            "extra": "avg mem: 86.70489576213261, max mem: 90.53515625, count: 58520"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.70858916074842, max cpu: 9.509658, count: 58520"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 78.75,
            "unit": "median mem",
            "extra": "avg mem: 75.61956596035543, max mem: 79.0, count: 58520"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.554840994557351, max cpu: 23.952095, count: 58520"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.56640625,
            "unit": "median mem",
            "extra": "avg mem: 88.04120112514953, max mem: 92.1640625, count: 58520"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.529323248331103, max cpu: 4.84115, count: 58520"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 85.953125,
            "unit": "median mem",
            "extra": "avg mem: 83.37138358146787, max mem: 86.22265625, count: 58520"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.441110792061556, max cpu: 19.753086, count: 117040"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 95.078125,
            "unit": "median mem",
            "extra": "avg mem: 93.63217449696685, max mem: 100.79296875, count: 117040"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6660,
            "unit": "median block_count",
            "extra": "avg block_count: 6507.844634313055, max block_count: 6660.0, count: 58520"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 116.82067669172932, max segment_count: 373.0, count: 58520"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.336997145323234, max cpu: 19.482496, count: 58520"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 98.30859375,
            "unit": "median mem",
            "extra": "avg mem: 97.0669074808826, max mem: 104.55859375, count: 58520"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 22.857143,
            "unit": "median cpu",
            "extra": "avg cpu: 18.712631230288668, max cpu: 28.91566, count: 58520"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 78.90625,
            "unit": "median mem",
            "extra": "avg mem: 74.22372459468986, max mem: 80.26953125, count: 58520"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242375236,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.616296357989693, max cpu: 23.66864, count: 58517"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.08984375,
            "unit": "median mem",
            "extra": "avg mem: 85.39837308228805, max mem: 91.5703125, count: 58517"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.748394834250682, max cpu: 9.60961, count: 58517"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 77.5,
            "unit": "median mem",
            "extra": "avg mem: 73.58967052309585, max mem: 78.5, count: 58517"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.7477746,
            "unit": "median cpu",
            "extra": "avg cpu: 6.657813692774051, max cpu: 24.279211, count: 58517"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.52734375,
            "unit": "median mem",
            "extra": "avg mem: 85.7954805734872, max mem: 92.359375, count: 58517"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7197638,
            "unit": "median cpu",
            "extra": "avg cpu: 4.806011116004302, max cpu: 9.453471, count: 58517"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 84.87890625,
            "unit": "median mem",
            "extra": "avg mem: 80.52497511406942, max mem: 86.17578125, count: 58517"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.508431412623098, max cpu: 23.529411, count: 117034"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 94.2421875,
            "unit": "median mem",
            "extra": "avg mem: 91.55150583243972, max mem: 101.08203125, count: 117034"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 6587,
            "unit": "median block_count",
            "extra": "avg block_count: 6135.231300305894, max block_count: 6587.0, count: 58517"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.08373635012047, max segment_count: 299.0, count: 58517"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.383905985788223, max cpu: 14.349776, count: 58517"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 95.65234375,
            "unit": "median mem",
            "extra": "avg mem: 92.86450091426423, max mem: 101.41796875, count: 58517"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 14.222222,
            "unit": "median cpu",
            "extra": "avg cpu: 15.098894780552191, max cpu: 28.444445, count: 58517"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 80.1875,
            "unit": "median mem",
            "extra": "avg mem: 76.71651093378847, max mem: 82.73828125, count: 58517"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752244995162,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.686958657605573, max cpu: 24.353119, count: 58492"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 96.34375,
            "unit": "median mem",
            "extra": "avg mem: 92.33571785190709, max mem: 101.5546875, count: 58492"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.721218687853342, max cpu: 9.4395275, count: 58492"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.70703125,
            "unit": "median mem",
            "extra": "avg mem: 80.738690160086, max mem: 87.95703125, count: 58492"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.740741,
            "unit": "median cpu",
            "extra": "avg cpu: 6.6703436773161044, max cpu: 24.353119, count: 58492"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.31640625,
            "unit": "median mem",
            "extra": "avg mem: 92.94400075544519, max mem: 101.5625, count: 58492"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 4.651776512012357, max cpu: 4.8484845, count: 58492"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 92.12109375,
            "unit": "median mem",
            "extra": "avg mem: 87.0725895875718, max mem: 94.3125, count: 58492"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 6.4395120814270035, max cpu: 24.806202, count: 116984"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 97.56640625,
            "unit": "median mem",
            "extra": "avg mem: 94.62648017036518, max mem: 107.06640625, count: 116984"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7451,
            "unit": "median block_count",
            "extra": "avg block_count: 6972.813718115298, max block_count: 7852.0, count: 58492"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 117,
            "unit": "median segment_count",
            "extra": "avg segment_count: 117.04651918211037, max segment_count: 410.0, count: 58492"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.7267356,
            "unit": "median cpu",
            "extra": "avg cpu: 5.466018142675612, max cpu: 18.768328, count: 58492"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 100.35546875,
            "unit": "median mem",
            "extra": "avg mem: 96.77958858155816, max mem: 107.27734375, count: 58492"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 15.888955219830907, max cpu: 28.318584, count: 58492"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 87.7578125,
            "unit": "median mem",
            "extra": "avg mem: 82.37771137505983, max mem: 90.6875, count: 58492"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237110637,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.388316394131813, max cpu: 47.97601, count: 59161"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.7109375,
            "unit": "median mem",
            "extra": "avg mem: 171.8246386845853, max mem: 175.78125, count: 59161"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23776,
            "unit": "median block_count",
            "extra": "avg block_count: 21748.436115008197, max block_count: 27509.0, count: 59161"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.45557039265732, max segment_count: 169.0, count: 59161"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.715757226466351, max cpu: 28.828829, count: 59161"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.546875,
            "unit": "median mem",
            "extra": "avg mem: 157.37672840057218, max mem: 173.796875, count: 59161"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238165895,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.4761867534322, max cpu: 47.83259, count: 59180"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 178.55078125,
            "unit": "median mem",
            "extra": "avg mem: 177.66108792824858, max mem: 178.55078125, count: 59180"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23773,
            "unit": "median block_count",
            "extra": "avg block_count: 21655.293612706995, max block_count: 27284.0, count: 59180"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 74.97571814802298, max segment_count: 186.0, count: 59180"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.702628536523862, max cpu: 33.48281, count: 59180"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.51171875,
            "unit": "median mem",
            "extra": "avg mem: 157.74049443963332, max mem: 173.6640625, count: 59180"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239194558,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.906942,
            "unit": "median cpu",
            "extra": "avg cpu: 19.369188323011837, max cpu: 48.1203, count: 59153"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.265625,
            "unit": "median mem",
            "extra": "avg mem: 168.6761738362805, max mem: 176.1796875, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24145,
            "unit": "median block_count",
            "extra": "avg block_count: 21959.312731391477, max block_count: 27383.0, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.63308707926902, max segment_count: 170.0, count: 59153"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.453471,
            "unit": "median cpu",
            "extra": "avg cpu: 9.671659018677241, max cpu: 34.408604, count: 59153"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.81640625,
            "unit": "median mem",
            "extra": "avg mem: 157.25052690427788, max mem: 174.2734375, count: 59153"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240232279,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.290300920641997, max cpu: 47.128128, count: 59171"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 177.74609375,
            "unit": "median mem",
            "extra": "avg mem: 177.22146604227578, max mem: 178.49609375, count: 59171"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23611,
            "unit": "median block_count",
            "extra": "avg block_count: 21568.428689729768, max block_count: 27332.0, count: 59171"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 71,
            "unit": "median segment_count",
            "extra": "avg segment_count: 74.72275269980227, max segment_count: 171.0, count: 59171"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.584573042850904, max cpu: 33.432835, count: 59171"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.63671875,
            "unit": "median mem",
            "extra": "avg mem: 157.96731236321423, max mem: 174.125, count: 59171"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242305524,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.318624264237442, max cpu: 47.90419, count: 59165"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 178.7890625,
            "unit": "median mem",
            "extra": "avg mem: 177.99237883461507, max mem: 178.7890625, count: 59165"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23343,
            "unit": "median block_count",
            "extra": "avg block_count: 21533.576827516266, max block_count: 28043.0, count: 59165"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.30558607284712, max segment_count: 171.0, count: 59165"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.425626,
            "unit": "median cpu",
            "extra": "avg cpu: 9.136747636809142, max cpu: 33.03835, count: 59165"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.19140625,
            "unit": "median mem",
            "extra": "avg mem: 158.3868387137666, max mem: 177.0078125, count: 59165"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242310425,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.851252,
            "unit": "median cpu",
            "extra": "avg cpu: 19.373407485359014, max cpu: 47.97601, count: 59156"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.15625,
            "unit": "median mem",
            "extra": "avg mem: 168.24412953145074, max mem: 172.23046875, count: 59156"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24183,
            "unit": "median block_count",
            "extra": "avg block_count: 21977.2720603151, max block_count: 27129.0, count: 59156"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.39847183717627, max segment_count: 186.0, count: 59156"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.425626,
            "unit": "median cpu",
            "extra": "avg cpu: 9.630570022365104, max cpu: 33.58321, count: 59156"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.953125,
            "unit": "median mem",
            "extra": "avg mem: 158.03873966450993, max mem: 175.37890625, count: 59156"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242351579,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.49354657261162, max cpu: 48.04805, count: 59169"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 171.53515625,
            "unit": "median mem",
            "extra": "avg mem: 170.68180821302118, max mem: 177.328125, count: 59169"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24290,
            "unit": "median block_count",
            "extra": "avg block_count: 22062.798357247884, max block_count: 27709.0, count: 59169"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.72732342949855, max segment_count: 194.0, count: 59169"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.453471,
            "unit": "median cpu",
            "extra": "avg cpu: 9.816864018992263, max cpu: 33.532936, count: 59169"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.1328125,
            "unit": "median mem",
            "extra": "avg mem: 158.55089922510098, max mem: 174.90625, count: 59169"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242359165,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 19.32087708891552, max cpu: 47.83259, count: 59111"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.4921875,
            "unit": "median mem",
            "extra": "avg mem: 168.57609055949823, max mem: 172.12890625, count: 59111"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24188,
            "unit": "median block_count",
            "extra": "avg block_count: 22014.51173216491, max block_count: 27098.0, count: 59111"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.15866759148044, max segment_count: 182.0, count: 59111"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.384164,
            "unit": "median cpu",
            "extra": "avg cpu: 9.325402659121597, max cpu: 33.48281, count: 59111"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.125,
            "unit": "median mem",
            "extra": "avg mem: 155.44182449121146, max mem: 171.0859375, count: 59111"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242363554,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.906942,
            "unit": "median cpu",
            "extra": "avg cpu: 19.356939682761215, max cpu: 48.192772, count: 59164"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.890625,
            "unit": "median mem",
            "extra": "avg mem: 171.03011821483503, max mem: 174.62109375, count: 59164"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23994,
            "unit": "median block_count",
            "extra": "avg block_count: 21933.411770671355, max block_count: 27725.0, count: 59164"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 73,
            "unit": "median segment_count",
            "extra": "avg segment_count: 75.89427692515719, max segment_count: 170.0, count: 59164"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.456331926541239, max cpu: 33.532936, count: 59164"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 164.95703125,
            "unit": "median mem",
            "extra": "avg mem: 156.53082691755122, max mem: 173.48046875, count: 59164"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752245071205,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.435231651229767, max cpu: 48.04805, count: 59174"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.4609375,
            "unit": "median mem",
            "extra": "avg mem: 174.6064031632347, max mem: 178.47265625, count: 59174"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23974,
            "unit": "median block_count",
            "extra": "avg block_count: 21800.552624463446, max block_count: 27859.0, count: 59174"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 72,
            "unit": "median segment_count",
            "extra": "avg segment_count: 74.86713759421367, max segment_count: 188.0, count: 59174"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.4395275,
            "unit": "median cpu",
            "extra": "avg cpu: 9.522206851802565, max cpu: 32.844578, count: 59174"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.078125,
            "unit": "median mem",
            "extra": "avg mem: 157.11647885103676, max mem: 173.6953125, count: 59174"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237116603,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.933620044579112,
            "unit": "median tps",
            "extra": "avg tps: 6.8816419883614675, max tps: 10.173375863220695, count: 59115"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 6.89333235698411,
            "unit": "median tps",
            "extra": "avg tps: 6.281018090359702, max tps: 7.689324986980393, count: 59115"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238177595,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.42721053621521,
            "unit": "median tps",
            "extra": "avg tps: 7.294552663247997, max tps: 10.752947595238485, count: 59181"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.218829222460416,
            "unit": "median tps",
            "extra": "avg tps: 6.61514504053846, max tps: 8.057049558883858, count: 59181"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239184790,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.237446921617035,
            "unit": "median tps",
            "extra": "avg tps: 7.1568245283804215, max tps: 10.65030554696505, count: 59183"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.313414407949339,
            "unit": "median tps",
            "extra": "avg tps: 6.652802067942109, max tps: 8.091755012063107, count: 59183"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240198547,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.398615870216046,
            "unit": "median tps",
            "extra": "avg tps: 7.2644077472018695, max tps: 10.740905180962095, count: 59173"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.092793967833161,
            "unit": "median tps",
            "extra": "avg tps: 6.4988869915825385, max tps: 7.900682228346816, count: 59173"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242054392,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 4.755065893413038,
            "unit": "median tps",
            "extra": "avg tps: 4.793428025161008, max tps: 8.303483377353672, count: 33377"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.692314898414298,
            "unit": "median tps",
            "extra": "avg tps: 5.418939368461245, max tps: 6.99685331685685, count: 33377"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242272085,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.397412922095858,
            "unit": "median tps",
            "extra": "avg tps: 7.3144380247885135, max tps: 10.85325206277412, count: 59185"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.171085098704748,
            "unit": "median tps",
            "extra": "avg tps: 6.575405241031336, max tps: 7.999462532147821, count: 59185"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242307756,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.224287892425368,
            "unit": "median tps",
            "extra": "avg tps: 7.127329222062369, max tps: 10.529553794925889, count: 59169"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.08106903276883,
            "unit": "median tps",
            "extra": "avg tps: 6.476812767400267, max tps: 7.9144223436467644, count: 59169"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242315552,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.235162930478962,
            "unit": "median tps",
            "extra": "avg tps: 7.166231960410151, max tps: 10.691732116504953, count: 59176"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.271492030183408,
            "unit": "median tps",
            "extra": "avg tps: 6.6197614761871115, max tps: 8.018599914925678, count: 59176"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242322221,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.200580477761095,
            "unit": "median tps",
            "extra": "avg tps: 7.138237934968243, max tps: 10.635968505530807, count: 59153"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.193889762394928,
            "unit": "median tps",
            "extra": "avg tps: 6.5537700445783695, max tps: 7.975111543932936, count: 59153"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752245044578,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.340158361083036,
            "unit": "median tps",
            "extra": "avg tps: 7.221470988613443, max tps: 10.714219704023472, count: 59163"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.111459649088347,
            "unit": "median tps",
            "extra": "avg tps: 6.477631564345534, max tps: 7.948731874010775, count: 59163"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0e0a552a073d66f6de8ad45fc0587a4f2ba39dac",
          "message": "chore: Run benchmarks on `benchmark`-labeled PRs. (#2826)\n\n## What\n\nAdjust benchmarks jobs to automatically run when the `benchmark` label\nis applied.\n\n## Why\n\n#2820 failed to actually filter to `perf:`-titled PRs, but additionally,\nin practice that would have been too noisy, since they would have re-run\non every push to the PR.\n\n## Tests\n\nManually tested adding/removing the label.",
          "timestamp": "2025-07-11T09:27:20-07:00",
          "tree_id": "0d8f5a0d7fda145130d823ec6bc08e217a9677d0",
          "url": "https://github.com/paradedb/paradedb/commit/0e0a552a073d66f6de8ad45fc0587a4f2ba39dac"
        },
        "date": 1752252233941,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.31399874568713,
            "unit": "median tps",
            "extra": "avg tps: 7.215933433430299, max tps: 10.689965606957953, count: 59168"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 7.234799631696866,
            "unit": "median tps",
            "extra": "avg tps: 6.602329185498556, max tps: 8.082235874900915, count: 59168"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752237161360,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 22.829966,
            "unit": "median cpu",
            "extra": "avg cpu: 20.096259615936148, max cpu: 43.11377, count: 59115"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.07421875,
            "unit": "median mem",
            "extra": "avg mem: 227.26424243265245, max mem: 230.93359375, count: 59115"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.55626161246322, max cpu: 33.532936, count: 59115"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.1953125,
            "unit": "median mem",
            "extra": "avg mem: 159.38718434143195, max mem: 166.140625, count: 59115"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23229,
            "unit": "median block_count",
            "extra": "avg block_count: 21557.03264822803, max block_count: 24786.0, count: 59115"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.01409117821196, max segment_count: 85.0, count: 59115"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752238191382,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.047619,
            "unit": "median cpu",
            "extra": "avg cpu: 19.708163573241134, max cpu: 43.90244, count: 59181"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.2109375,
            "unit": "median mem",
            "extra": "avg mem: 227.3669952930417, max mem: 231.79296875, count: 59181"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.31500893965218, max cpu: 33.633633, count: 59181"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.5,
            "unit": "median mem",
            "extra": "avg mem: 159.75227803528583, max mem: 165.20703125, count: 59181"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23627,
            "unit": "median block_count",
            "extra": "avg block_count: 21966.33490478363, max block_count: 25082.0, count: 59181"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 56.20224396343421, max segment_count: 88.0, count: 59181"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752239212554,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.070322,
            "unit": "median cpu",
            "extra": "avg cpu: 19.715288637380066, max cpu: 43.570347, count: 59183"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.171875,
            "unit": "median mem",
            "extra": "avg mem: 227.43512805355846, max mem: 232.26171875, count: 59183"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.564064,
            "unit": "median cpu",
            "extra": "avg cpu: 21.31953668499028, max cpu: 33.73494, count: 59183"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.921875,
            "unit": "median mem",
            "extra": "avg mem: 159.42872530963282, max mem: 166.0, count: 59183"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23345,
            "unit": "median block_count",
            "extra": "avg block_count: 21820.544784819966, max block_count: 25084.0, count: 59183"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.86335603129277, max segment_count: 87.0, count: 59183"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752240212272,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.093079,
            "unit": "median cpu",
            "extra": "avg cpu: 19.779726273119223, max cpu: 43.373497, count: 59173"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 229.2265625,
            "unit": "median mem",
            "extra": "avg mem: 228.30338471251667, max mem: 231.94140625, count: 59173"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.564064,
            "unit": "median cpu",
            "extra": "avg cpu: 21.362313705218238, max cpu: 33.633633, count: 59173"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.58984375,
            "unit": "median mem",
            "extra": "avg mem: 160.2041791785316, max mem: 166.10546875, count: 59173"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23598,
            "unit": "median block_count",
            "extra": "avg block_count: 21979.04052523955, max block_count: 25298.0, count: 59173"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 56.05233805958799, max segment_count: 87.0, count: 59173"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242080249,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.391813,
            "unit": "median cpu",
            "extra": "avg cpu: 20.8986680917799, max cpu: 43.50453, count: 33377"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.83203125,
            "unit": "median mem",
            "extra": "avg mem: 227.027518130936, max mem: 231.1328125, count: 33377"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.460411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.75369085463789, max cpu: 33.03835, count: 33377"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.16796875,
            "unit": "median mem",
            "extra": "avg mem: 159.05190571445158, max mem: 160.1640625, count: 33377"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 20570,
            "unit": "median block_count",
            "extra": "avg block_count: 19589.798334182222, max block_count: 23175.0, count: 33377"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 52.59238397699014, max segment_count: 84.0, count: 33377"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242297855,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.070322,
            "unit": "median cpu",
            "extra": "avg cpu: 19.719417704969846, max cpu: 43.243244, count: 59185"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.00390625,
            "unit": "median mem",
            "extra": "avg mem: 227.1227772952395, max mem: 231.3984375, count: 59185"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.564064,
            "unit": "median cpu",
            "extra": "avg cpu: 21.303288006818153, max cpu: 33.633633, count: 59185"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.14453125,
            "unit": "median mem",
            "extra": "avg mem: 162.00160810646702, max mem: 166.74609375, count: 59185"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23526,
            "unit": "median block_count",
            "extra": "avg block_count: 21991.665624735997, max block_count: 25337.0, count: 59185"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 56.20635296105432, max segment_count: 88.0, count: 59185"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242370604,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.104477,
            "unit": "median cpu",
            "extra": "avg cpu: 19.93511785434141, max cpu: 43.11377, count: 59169"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.9375,
            "unit": "median mem",
            "extra": "avg mem: 228.02301702802566, max mem: 232.203125, count: 59169"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.408911545629426, max cpu: 33.185184, count: 59169"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.5078125,
            "unit": "median mem",
            "extra": "avg mem: 160.09258358473187, max mem: 166.28125, count: 59169"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23344,
            "unit": "median block_count",
            "extra": "avg block_count: 21770.78546198178, max block_count: 24854.0, count: 59169"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.497405736111816, max segment_count: 87.0, count: 59169"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242372086,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.002375,
            "unit": "median cpu",
            "extra": "avg cpu: 19.708215296829977, max cpu: 43.17841, count: 59153"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.37890625,
            "unit": "median mem",
            "extra": "avg mem: 227.4203882569354, max mem: 231.1328125, count: 59153"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.44152748326559, max cpu: 33.532936, count: 59153"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.76953125,
            "unit": "median mem",
            "extra": "avg mem: 160.31958306214392, max mem: 165.8515625, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23606,
            "unit": "median block_count",
            "extra": "avg block_count: 21834.401399759943, max block_count: 25112.0, count: 59153"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 53,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.63952800365155, max segment_count: 87.0, count: 59153"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr",
            "email": "eebbrr@gmail.com"
          },
          "committer": {
            "name": "GitHub",
            "username": "web-flow",
            "email": "noreply@github.com"
          },
          "id": "e350be5d171ddb28a700c02d8adc32a1a9f4b084",
          "message": "feat: custom fsm (#2765)\n\nA custom FSM implementation that allows us to internally track\nfree/reusable blocks without generating full read+write cycles on every\npage that is returned to the FSM.\n\nThere's a new UDF called `paradedb.fsm_info()` that returns a table of\nFSM block numbers in use and the free block numbers they contain.\n\n## Why\n\nTo reduce I/O during segment merging and garbage collection, with an aim of reducing WAL traffic for enterprise.",
          "timestamp": "2025-07-09T15:42:21Z",
          "url": "https://github.com/paradedb/paradedb/commit/e350be5d171ddb28a700c02d8adc32a1a9f4b084"
        },
        "date": 1752242383453,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.047619,
            "unit": "median cpu",
            "extra": "avg cpu: 19.700743482818737, max cpu: 43.768997, count: 59176"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.38671875,
            "unit": "median mem",
            "extra": "avg mem: 227.56543536442138, max mem: 231.34375, count: 59176"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.305813993371864, max cpu: 33.532936, count: 59176"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.58984375,
            "unit": "median mem",
            "extra": "avg mem: 160.45589474354045, max mem: 163.42578125, count: 59176"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23537,
            "unit": "median block_count",
            "extra": "avg block_count: 21873.933368257403, max block_count: 25307.0, count: 59176"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.921741922400976, max segment_count: 88.0, count: 59176"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "b7d4bc24bfd70b39d516d876904a317cd3a2f5f4",
          "message": "chore: Avoid impersonating Phil (#2822)\n\n## What\n\nUse the same github token for the stressgres job as for the benchmarks\njob.\n\n## Why\n\nTo avoid impersonating Phil: using the release token results in comments\nfrom him on `perf` PRs, commits, and gh-pages pushes.",
          "timestamp": "2025-07-11T07:27:20-07:00",
          "tree_id": "da11aeca2a89a309d5985def5237c0cb6b676df0",
          "url": "https://github.com/paradedb/paradedb/commit/b7d4bc24bfd70b39d516d876904a317cd3a2f5f4"
        },
        "date": 1752245106221,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.047619,
            "unit": "median cpu",
            "extra": "avg cpu: 19.808235043851543, max cpu: 43.243244, count: 59163"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.51171875,
            "unit": "median mem",
            "extra": "avg mem: 227.7336423841759, max mem: 233.4375, count: 59163"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.564064,
            "unit": "median cpu",
            "extra": "avg cpu: 21.40547352319237, max cpu: 33.532936, count: 59163"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.89453125,
            "unit": "median mem",
            "extra": "avg mem: 159.88038349823793, max mem: 163.328125, count: 59163"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23731,
            "unit": "median block_count",
            "extra": "avg block_count: 21886.875226070348, max block_count: 25031.0, count: 59163"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.99342494464446, max segment_count: 87.0, count: 59163"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "stuhood@paradedb.com",
            "name": "Stu Hood",
            "username": "stuhood"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0e0a552a073d66f6de8ad45fc0587a4f2ba39dac",
          "message": "chore: Run benchmarks on `benchmark`-labeled PRs. (#2826)\n\n## What\n\nAdjust benchmarks jobs to automatically run when the `benchmark` label\nis applied.\n\n## Why\n\n#2820 failed to actually filter to `perf:`-titled PRs, but additionally,\nin practice that would have been too noisy, since they would have re-run\non every push to the PR.\n\n## Tests\n\nManually tested adding/removing the label.",
          "timestamp": "2025-07-11T09:27:20-07:00",
          "tree_id": "0d8f5a0d7fda145130d823ec6bc08e217a9677d0",
          "url": "https://github.com/paradedb/paradedb/commit/0e0a552a073d66f6de8ad45fc0587a4f2ba39dac"
        },
        "date": 1752252239631,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 19.047619,
            "unit": "median cpu",
            "extra": "avg cpu: 19.700339172375795, max cpu: 43.70258, count: 59168"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.71484375,
            "unit": "median mem",
            "extra": "avg mem: 228.0810571962463, max mem: 232.52734375, count: 59168"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.529411,
            "unit": "median cpu",
            "extra": "avg cpu: 21.310630948730296, max cpu: 33.633633, count: 59168"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.18359375,
            "unit": "median mem",
            "extra": "avg mem: 159.9395260767011, max mem: 161.16796875, count: 59168"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23631,
            "unit": "median block_count",
            "extra": "avg block_count: 21897.07831936182, max block_count: 25066.0, count: 59168"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 54,
            "unit": "median segment_count",
            "extra": "avg segment_count: 55.796106003245, max segment_count: 87.0, count: 59168"
          }
        ]
      }
    ]
  }
}