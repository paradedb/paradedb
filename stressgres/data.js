window.BENCHMARK_DATA = {
  "lastUpdate": 1756145825815,
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755715493319,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1256.533949601907,
            "unit": "median tps",
            "extra": "avg tps: 1251.7578175243382, max tps: 1262.976571597613, count: 55217"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2953.7122027014025,
            "unit": "median tps",
            "extra": "avg tps: 2928.228941037981, max tps: 2968.189532060059, count: 55217"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1264.0377204656168,
            "unit": "median tps",
            "extra": "avg tps: 1259.0557652682796, max tps: 1268.1811857862056, count: 55217"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 911.8524745792212,
            "unit": "median tps",
            "extra": "avg tps: 912.994584302596, max tps: 989.9209911059633, count: 55217"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 174.3220018512833,
            "unit": "median tps",
            "extra": "avg tps: 173.72196838279933, max tps: 176.45752290097963, count: 110434"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.665897232315,
            "unit": "median tps",
            "extra": "avg tps: 153.33059768920103, max tps: 154.35617079938731, count: 55217"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 49.85233888733582,
            "unit": "median tps",
            "extra": "avg tps: 57.597267362257355, max tps: 793.9828801411384, count: 55217"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755715525169,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 299.6220425416693,
            "unit": "median tps",
            "extra": "avg tps: 301.2070678049899, max tps: 535.8897792153914, count: 55218"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2441.851324866876,
            "unit": "median tps",
            "extra": "avg tps: 2426.197242481759, max tps: 2660.075708414737, count: 55218"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 308.0274382350238,
            "unit": "median tps",
            "extra": "avg tps: 308.7554000291646, max tps: 484.9810922127007, count: 55218"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 270.69217839050253,
            "unit": "median tps",
            "extra": "avg tps: 270.5547535156572, max tps: 435.7596386437051, count: 55218"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 152.51531877818124,
            "unit": "median tps",
            "extra": "avg tps: 151.59025240758297, max tps: 156.76027218151694, count: 110436"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 141.10866810515418,
            "unit": "median tps",
            "extra": "avg tps: 139.79167372747094, max tps: 141.92587129093184, count: 55218"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.691854040091016,
            "unit": "median tps",
            "extra": "avg tps: 9.154715502252817, max tps: 1197.557461800911, count: 55218"
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
        "date": 1755724630781,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1236.9292335316463,
            "unit": "median tps",
            "extra": "avg tps: 1233.524412519379, max tps: 1253.5770036312992, count: 55222"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2629.109684444998,
            "unit": "median tps",
            "extra": "avg tps: 2624.555260620635, max tps: 2669.727683715839, count: 55222"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1232.654276024278,
            "unit": "median tps",
            "extra": "avg tps: 1228.826761757374, max tps: 1237.8518422287007, count: 55222"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 962.1956564282336,
            "unit": "median tps",
            "extra": "avg tps: 960.538969808775, max tps: 980.1198636626364, count: 55222"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 173.88419493135433,
            "unit": "median tps",
            "extra": "avg tps: 172.68319383182833, max tps: 177.0443126329157, count: 110444"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.14472299369265,
            "unit": "median tps",
            "extra": "avg tps: 152.04433761675043, max tps: 154.68931800019038, count: 55222"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 29.700515659945825,
            "unit": "median tps",
            "extra": "avg tps: 48.244038153433124, max tps: 728.3347717945077, count: 55222"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755724678921,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1196.7346449020963,
            "unit": "median tps",
            "extra": "avg tps: 1187.5956456683123, max tps: 1199.6634355633337, count: 55406"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2344.1602478820437,
            "unit": "median tps",
            "extra": "avg tps: 2342.369512406594, max tps: 2578.312813974043, count: 55406"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1191.1761791597755,
            "unit": "median tps",
            "extra": "avg tps: 1184.8377426330624, max tps: 1197.3227829464684, count: 55406"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1053.3238647193473,
            "unit": "median tps",
            "extra": "avg tps: 1041.185542385324, max tps: 1056.5833387132138, count: 55406"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 154.71034742815846,
            "unit": "median tps",
            "extra": "avg tps: 153.30064546920698, max tps: 161.7505011591701, count: 110812"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 131.53934241236516,
            "unit": "median tps",
            "extra": "avg tps: 131.7307831394826, max tps: 141.0300759913331, count: 55406"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 63.61371081441365,
            "unit": "median tps",
            "extra": "avg tps: 78.752304280384, max tps: 801.319934195607, count: 55406"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755724987564,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 304.56972497639225,
            "unit": "median tps",
            "extra": "avg tps: 305.8191110015292, max tps: 528.954264742586, count: 55305"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2427.837860900668,
            "unit": "median tps",
            "extra": "avg tps: 2426.646081436125, max tps: 2595.163158497585, count: 55305"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 303.0872330182266,
            "unit": "median tps",
            "extra": "avg tps: 304.4124755940173, max tps: 506.45055948606205, count: 55305"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 248.5751317746758,
            "unit": "median tps",
            "extra": "avg tps: 250.1883663921892, max tps: 388.18352276624546, count: 55305"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.22648019662353,
            "unit": "median tps",
            "extra": "avg tps: 155.559491785663, max tps: 159.38046010205116, count: 110610"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 137.47805275232005,
            "unit": "median tps",
            "extra": "avg tps: 136.44195468161516, max tps: 142.59301330799713, count: 55305"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 4.746050810671489,
            "unit": "median tps",
            "extra": "avg tps: 8.87791559524838, max tps: 1133.171440906809, count: 55305"
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
        "date": 1755725039390,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1084.0767931626222,
            "unit": "median tps",
            "extra": "avg tps: 1082.3479983227169, max tps: 1090.8473801144446, count: 55285"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2844.6019772452682,
            "unit": "median tps",
            "extra": "avg tps: 2806.4354406134757, max tps: 2852.5974983741553, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1182.9875738386045,
            "unit": "median tps",
            "extra": "avg tps: 1172.7220771926984, max tps: 1189.409145698208, count: 55285"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1034.3083597082002,
            "unit": "median tps",
            "extra": "avg tps: 1023.1894045701273, max tps: 1039.4448406402678, count: 55285"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 155.40121192429842,
            "unit": "median tps",
            "extra": "avg tps: 155.65064615676448, max tps: 159.50883259001134, count: 110570"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 137.299482352109,
            "unit": "median tps",
            "extra": "avg tps: 137.2478414265768, max tps: 142.51725955271812, count: 55285"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 42.47563402926256,
            "unit": "median tps",
            "extra": "avg tps: 51.37275116939526, max tps: 749.9242576499773, count: 55285"
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
        "date": 1755725056203,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1253.4582174287848,
            "unit": "median tps",
            "extra": "avg tps: 1248.0184449338062, max tps: 1257.7005136141838, count: 55283"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2829.772465511342,
            "unit": "median tps",
            "extra": "avg tps: 2820.5896456797573, max tps: 2873.5278340592004, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1242.479811588259,
            "unit": "median tps",
            "extra": "avg tps: 1238.4990908769014, max tps: 1246.8223702643456, count: 55283"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1060.933563140538,
            "unit": "median tps",
            "extra": "avg tps: 1052.1223545680352, max tps: 1064.8904284525995, count: 55283"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 179.352740445096,
            "unit": "median tps",
            "extra": "avg tps: 180.55419280309465, max tps: 185.1156494021371, count: 110566"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 158.0280477727054,
            "unit": "median tps",
            "extra": "avg tps: 156.88083125847785, max tps: 158.81009318574715, count: 55283"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 30.004653295853686,
            "unit": "median tps",
            "extra": "avg tps: 32.952479063885356, max tps: 760.9481413841646, count: 55283"
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
        "date": 1755726148818,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1280.4653763643203,
            "unit": "median tps",
            "extra": "avg tps: 1273.170546481896, max tps: 1285.302430757094, count: 55251"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2609.426016575645,
            "unit": "median tps",
            "extra": "avg tps: 2612.5897263402135, max tps: 2652.7033934260885, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1210.4284177730206,
            "unit": "median tps",
            "extra": "avg tps: 1207.7335886774235, max tps: 1215.2706222423253, count: 55251"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1075.0238907689277,
            "unit": "median tps",
            "extra": "avg tps: 1063.901582059064, max tps: 1082.3387896915565, count: 55251"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 180.55201948883212,
            "unit": "median tps",
            "extra": "avg tps: 184.2453916628613, max tps: 192.5263677725552, count: 110502"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 154.18144658445883,
            "unit": "median tps",
            "extra": "avg tps: 153.17586090240556, max tps: 155.17263370101404, count: 55251"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 36.99047645740585,
            "unit": "median tps",
            "extra": "avg tps: 43.09746490935574, max tps: 750.9091632694284, count: 55251"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755794892360,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1191.509182173793,
            "unit": "median tps",
            "extra": "avg tps: 1183.7197209703556, max tps: 1195.1304495601307, count: 55179"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2846.4374094285467,
            "unit": "median tps",
            "extra": "avg tps: 2817.3124697743765, max tps: 2852.896042300975, count: 55179"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1180.7091214787433,
            "unit": "median tps",
            "extra": "avg tps: 1171.6030953467268, max tps: 1185.9516892498154, count: 55179"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 875.4824198766612,
            "unit": "median tps",
            "extra": "avg tps: 874.0349639158709, max tps: 902.9528639368203, count: 55179"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 154.9039519571436,
            "unit": "median tps",
            "extra": "avg tps: 154.40797527133753, max tps: 159.77556250148783, count: 110358"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 140.5855423124256,
            "unit": "median tps",
            "extra": "avg tps: 140.46345315720993, max tps: 143.49211615301937, count: 55179"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 76.60163179715929,
            "unit": "median tps",
            "extra": "avg tps: 96.41325313442836, max tps: 834.8241901996648, count: 55179"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755797103980,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1171.7631288686746,
            "unit": "median tps",
            "extra": "avg tps: 1165.6925264739075, max tps: 1182.4540558681117, count: 55359"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2763.332778602086,
            "unit": "median tps",
            "extra": "avg tps: 2726.0136565309103, max tps: 2790.1215591810687, count: 55359"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1089.6544313435768,
            "unit": "median tps",
            "extra": "avg tps: 1090.9007982070177, max tps: 1108.9519590046832, count: 55359"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 939.201584699334,
            "unit": "median tps",
            "extra": "avg tps: 936.5402285197516, max tps: 948.0599296146519, count: 55359"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 162.39591794588128,
            "unit": "median tps",
            "extra": "avg tps: 162.47536074782795, max tps: 166.3045415575638, count: 110718"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 144.47734366933986,
            "unit": "median tps",
            "extra": "avg tps: 145.07211763610908, max tps: 152.53107600602416, count: 55359"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 125.92169104525382,
            "unit": "median tps",
            "extra": "avg tps: 138.2464736468193, max tps: 803.2741454167186, count: 55359"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755804853559,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1122.4053370915312,
            "unit": "median tps",
            "extra": "avg tps: 1114.596488678084, max tps: 1124.415768673027, count: 55214"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2609.304619880608,
            "unit": "median tps",
            "extra": "avg tps: 2597.3735812397463, max tps: 2615.4799198847845, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1154.053000262801,
            "unit": "median tps",
            "extra": "avg tps: 1144.5461267824182, max tps: 1158.6639591901478, count: 55214"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 942.2659595054064,
            "unit": "median tps",
            "extra": "avg tps: 935.7711658295667, max tps: 950.6558830837324, count: 55214"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 167.4811538985087,
            "unit": "median tps",
            "extra": "avg tps: 171.79959020753367, max tps: 180.50078681178323, count: 110428"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 144.60667074983422,
            "unit": "median tps",
            "extra": "avg tps: 144.16865508931446, max tps: 144.9121310410424, count: 55214"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 98.94970886983673,
            "unit": "median tps",
            "extra": "avg tps: 122.17077600678304, max tps: 792.9138871731113, count: 55214"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755811647028,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1196.0781062246558,
            "unit": "median tps",
            "extra": "avg tps: 1187.7076502822272, max tps: 1200.272224982228, count: 55131"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2506.2998080089264,
            "unit": "median tps",
            "extra": "avg tps: 2491.7782688680427, max tps: 2510.083310933078, count: 55131"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1090.3183397789826,
            "unit": "median tps",
            "extra": "avg tps: 1089.1880610243293, max tps: 1123.7069397413104, count: 55131"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 951.9179431025949,
            "unit": "median tps",
            "extra": "avg tps: 947.6524125448246, max tps: 955.1780737385359, count: 55131"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 157.05660094587174,
            "unit": "median tps",
            "extra": "avg tps: 155.76735789543704, max tps: 173.57963774762783, count: 110262"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 167.9990205972526,
            "unit": "median tps",
            "extra": "avg tps: 165.7462351054542, max tps: 169.05685215675297, count: 55131"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 80.95394161452434,
            "unit": "median tps",
            "extra": "avg tps: 95.88107638509996, max tps: 785.1512201249961, count: 55131"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755812975883,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1170.9928934435109,
            "unit": "median tps",
            "extra": "avg tps: 1160.3200944857344, max tps: 1175.466526131665, count: 55192"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2450.1156160482706,
            "unit": "median tps",
            "extra": "avg tps: 2429.8320910409184, max tps: 2454.0295179221016, count: 55192"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1165.8878541162426,
            "unit": "median tps",
            "extra": "avg tps: 1157.8632910271804, max tps: 1168.1224917944376, count: 55192"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 865.816848283649,
            "unit": "median tps",
            "extra": "avg tps: 863.5515038431444, max tps: 901.2381750792143, count: 55192"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 164.86013502103708,
            "unit": "median tps",
            "extra": "avg tps: 163.9953561541898, max tps: 166.11580007727028, count: 110384"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 159.07471588293174,
            "unit": "median tps",
            "extra": "avg tps: 157.84786194887968, max tps: 161.27305156271257, count: 55192"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 62.311205767898024,
            "unit": "median tps",
            "extra": "avg tps: 84.2950711365507, max tps: 852.9737650859073, count: 55192"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755814757121,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1168.082891826055,
            "unit": "median tps",
            "extra": "avg tps: 1161.8132316936276, max tps: 1171.4179949387292, count: 55241"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2841.9312088004604,
            "unit": "median tps",
            "extra": "avg tps: 2805.0348575455373, max tps: 2861.005020348897, count: 55241"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1166.5052805825069,
            "unit": "median tps",
            "extra": "avg tps: 1159.52435221109, max tps: 1168.949360288792, count: 55241"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1012.0264182584477,
            "unit": "median tps",
            "extra": "avg tps: 999.377609497136, max tps: 1025.4582173034357, count: 55241"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 166.42820763074363,
            "unit": "median tps",
            "extra": "avg tps: 165.74137681694708, max tps: 167.49337143049948, count: 110482"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 145.8807790887134,
            "unit": "median tps",
            "extra": "avg tps: 145.37547531922428, max tps: 148.0086180484669, count: 55241"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 83.7461571645613,
            "unit": "median tps",
            "extra": "avg tps: 99.6442024611029, max tps: 723.4016079770943, count: 55241"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755834370455,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1195.1902622835762,
            "unit": "median tps",
            "extra": "avg tps: 1188.2345465791127, max tps: 1204.1308681439502, count: 55102"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2428.432036428251,
            "unit": "median tps",
            "extra": "avg tps: 2421.525711165937, max tps: 2439.3693540527574, count: 55102"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1058.399248163393,
            "unit": "median tps",
            "extra": "avg tps: 1060.4712391840644, max tps: 1108.861623482288, count: 55102"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 1030.5946122016444,
            "unit": "median tps",
            "extra": "avg tps: 1022.7805952570455, max tps: 1041.096007665378, count: 55102"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 156.38940713225674,
            "unit": "median tps",
            "extra": "avg tps: 156.07363095711642, max tps: 161.70369481623396, count: 110204"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 138.19012923567396,
            "unit": "median tps",
            "extra": "avg tps: 137.49364275916497, max tps: 143.33522773640118, count: 55102"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 178.33280328400437,
            "unit": "median tps",
            "extra": "avg tps: 155.44381534630193, max tps: 820.2745623014936, count: 55102"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755834671963,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1188.8391967805785,
            "unit": "median tps",
            "extra": "avg tps: 1179.6654848228811, max tps: 1194.2589388762533, count: 55253"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2657.6871502861322,
            "unit": "median tps",
            "extra": "avg tps: 2627.9576157842016, max tps: 2665.230841889005, count: 55253"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1129.1244202141486,
            "unit": "median tps",
            "extra": "avg tps: 1123.7494155493462, max tps: 1131.2995141193894, count: 55253"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 954.4386125825586,
            "unit": "median tps",
            "extra": "avg tps: 946.6214090129278, max tps: 961.5137031202765, count: 55253"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 166.78275236061492,
            "unit": "median tps",
            "extra": "avg tps: 172.59092761425396, max tps: 185.04082586872187, count: 110506"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 139.68439263128332,
            "unit": "median tps",
            "extra": "avg tps: 139.92738742805193, max tps: 145.5699189022381, count: 55253"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 86.3738342263773,
            "unit": "median tps",
            "extra": "avg tps: 99.51385357124346, max tps: 782.5191482435575, count: 55253"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "74e58071d8682e5f1a784791d82619ae41994f0b",
          "message": "fix: restore the garbage list (#3021)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:21-04:00",
          "tree_id": "3455b2de8f4efa7f45573d0f6a731fe933e7cedb",
          "url": "https://github.com/paradedb/paradedb/commit/74e58071d8682e5f1a784791d82619ae41994f0b"
        },
        "date": 1755834677131,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1183.7699925538213,
            "unit": "median tps",
            "extra": "avg tps: 1175.632495165159, max tps: 1186.3792517313213, count: 55214"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2599.70518777303,
            "unit": "median tps",
            "extra": "avg tps: 2582.569125951008, max tps: 2605.892317764122, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1116.0993563497086,
            "unit": "median tps",
            "extra": "avg tps: 1112.518810061035, max tps: 1162.2464606854903, count: 55214"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 943.3919745730038,
            "unit": "median tps",
            "extra": "avg tps: 935.9187703615466, max tps: 949.4924102748823, count: 55214"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 168.70337621888308,
            "unit": "median tps",
            "extra": "avg tps: 173.70696500619937, max tps: 184.23496663347967, count: 110428"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 147.2503713006154,
            "unit": "median tps",
            "extra": "avg tps: 147.06838192874156, max tps: 155.7126117606551, count: 55214"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 76.86239329040215,
            "unit": "median tps",
            "extra": "avg tps: 93.179221524388, max tps: 723.559682096818, count: 55214"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755873798257,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1147.4193482160138,
            "unit": "median tps",
            "extra": "avg tps: 1144.0328617735486, max tps: 1150.6925001109025, count: 55289"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2655.10418370262,
            "unit": "median tps",
            "extra": "avg tps: 2633.208876824282, max tps: 2662.6287874981927, count: 55289"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1122.0927123453766,
            "unit": "median tps",
            "extra": "avg tps: 1118.872527493004, max tps: 1138.9626055984045, count: 55289"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 911.7084242915407,
            "unit": "median tps",
            "extra": "avg tps: 906.6129951885514, max tps: 918.4403632983459, count: 55289"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 167.82308116849168,
            "unit": "median tps",
            "extra": "avg tps: 174.84333283307652, max tps: 186.34538123593887, count: 110578"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 144.00596825122173,
            "unit": "median tps",
            "extra": "avg tps: 143.86264473146997, max tps: 145.02874648684585, count: 55289"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 67.48704131548008,
            "unit": "median tps",
            "extra": "avg tps: 71.67057249823436, max tps: 676.8665950090565, count: 55289"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755887390723,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1152.2956927865407,
            "unit": "median tps",
            "extra": "avg tps: 1145.66805343343, max tps: 1156.1129459421595, count: 55283"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2465.0848724183675,
            "unit": "median tps",
            "extra": "avg tps: 2456.1116637740624, max tps: 2469.7208482565593, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1147.5779294223346,
            "unit": "median tps",
            "extra": "avg tps: 1140.4031310855164, max tps: 1150.4604271661526, count: 55283"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 959.0082412914289,
            "unit": "median tps",
            "extra": "avg tps: 950.4443121281776, max tps: 965.120709893603, count: 55283"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 168.02755047940235,
            "unit": "median tps",
            "extra": "avg tps: 166.55505849679955, max tps: 169.51474302460235, count: 110566"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.33640621183721,
            "unit": "median tps",
            "extra": "avg tps: 152.80459677447993, max tps: 153.69576249280323, count: 55283"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 181.8651147817977,
            "unit": "median tps",
            "extra": "avg tps: 176.2703405504529, max tps: 755.6522790472736, count: 55283"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755890733316,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1155.0121223606893,
            "unit": "median tps",
            "extra": "avg tps: 1148.3681126264332, max tps: 1160.5271773194359, count: 55132"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2854.826476307806,
            "unit": "median tps",
            "extra": "avg tps: 2827.395062901955, max tps: 2869.511912589987, count: 55132"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1156.6165996831708,
            "unit": "median tps",
            "extra": "avg tps: 1151.9585460427425, max tps: 1159.9513358994466, count: 55132"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 841.3301938660834,
            "unit": "median tps",
            "extra": "avg tps: 840.3484076094701, max tps: 892.4298968011383, count: 55132"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 178.3425825321799,
            "unit": "median tps",
            "extra": "avg tps: 177.44667284455448, max tps: 179.90663128571325, count: 110264"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 167.32479946699448,
            "unit": "median tps",
            "extra": "avg tps: 166.07616870444835, max tps: 167.7146626936276, count: 55132"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 55.41639942779269,
            "unit": "median tps",
            "extra": "avg tps: 60.513654017218236, max tps: 793.9280383626028, count: 55132"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755891835903,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1151.5381336036542,
            "unit": "median tps",
            "extra": "avg tps: 1143.9221405696278, max tps: 1153.9585109511866, count: 55056"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2686.3169892300252,
            "unit": "median tps",
            "extra": "avg tps: 2654.6156805467886, max tps: 2694.1032867202284, count: 55056"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1121.4128515983361,
            "unit": "median tps",
            "extra": "avg tps: 1116.660151310903, max tps: 1124.5406815721465, count: 55056"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 952.0613414539908,
            "unit": "median tps",
            "extra": "avg tps: 944.9964314806024, max tps: 960.7003140507661, count: 55056"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 179.1059070188253,
            "unit": "median tps",
            "extra": "avg tps: 182.90095588024388, max tps: 191.4546190367187, count: 110112"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 156.7702842366392,
            "unit": "median tps",
            "extra": "avg tps: 155.58761250509843, max tps: 158.14499749082438, count: 55056"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 101.64109344355474,
            "unit": "median tps",
            "extra": "avg tps: 116.78609082622793, max tps: 742.8520914629208, count: 55056"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "de951c94cea485c109ffc396ae394d036babb97d",
          "message": "perf: optimize merging heuristics to prefer background merging (#3033)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:53-07:00",
          "tree_id": "f489cbf34a77726e0c2bc0f7f879b77d8c93cb47",
          "url": "https://github.com/paradedb/paradedb/commit/de951c94cea485c109ffc396ae394d036babb97d"
        },
        "date": 1755891836958,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1123.5811368385716,
            "unit": "median tps",
            "extra": "avg tps: 1121.499963315783, max tps: 1125.859482370049, count: 55154"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2833.0254166088853,
            "unit": "median tps",
            "extra": "avg tps: 2790.672946776092, max tps: 2848.1634337460323, count: 55154"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1149.3780305071045,
            "unit": "median tps",
            "extra": "avg tps: 1144.8600467988426, max tps: 1152.6537993383747, count: 55154"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 959.3019548342376,
            "unit": "median tps",
            "extra": "avg tps: 954.3075118875746, max tps: 969.2988261164733, count: 55154"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 178.43619987192955,
            "unit": "median tps",
            "extra": "avg tps: 177.87185995192485, max tps: 180.02070055414592, count: 110308"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 166.07353935617647,
            "unit": "median tps",
            "extra": "avg tps: 164.3389771270807, max tps: 167.0658896228587, count: 55154"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 69.33050260807563,
            "unit": "median tps",
            "extra": "avg tps: 81.76698570752066, max tps: 729.4776064964358, count: 55154"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a556c28327e5b7ca9b5196f4153d49bca00c90a6",
          "message": "chore: Upgrade to `0.17.18` (#3034)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T15:36:02-04:00",
          "tree_id": "9de75b3b13ca610745c645ca4046bddf6fe9eec4",
          "url": "https://github.com/paradedb/paradedb/commit/a556c28327e5b7ca9b5196f4153d49bca00c90a6"
        },
        "date": 1755892329254,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1104.6124409236872,
            "unit": "median tps",
            "extra": "avg tps: 1098.789326079968, max tps: 1109.5123892912777, count: 55205"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2643.5302401531094,
            "unit": "median tps",
            "extra": "avg tps: 2618.898755230589, max tps: 2655.848286421532, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1159.060121502336,
            "unit": "median tps",
            "extra": "avg tps: 1152.714322783695, max tps: 1164.9215594214227, count: 55205"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 937.207566546857,
            "unit": "median tps",
            "extra": "avg tps: 932.2747465385436, max tps: 950.7467124184567, count: 55205"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 176.28210130886572,
            "unit": "median tps",
            "extra": "avg tps: 183.07913158355672, max tps: 194.82578386056227, count: 110410"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 156.11559252503227,
            "unit": "median tps",
            "extra": "avg tps: 155.70040247961373, max tps: 156.90258377890964, count: 55205"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 58.606445860219445,
            "unit": "median tps",
            "extra": "avg tps: 57.68914553783416, max tps: 751.2154666249992, count: 55205"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0cad75ef84c3942edd137497c4b6864b384a3655",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3037)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests",
          "timestamp": "2025-08-24T13:29:07-04:00",
          "tree_id": "d76d1a8d15e8ca4d61db683033627e7bf4ec476d",
          "url": "https://github.com/paradedb/paradedb/commit/0cad75ef84c3942edd137497c4b6864b384a3655"
        },
        "date": 1756057510269,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1091.3089447116186,
            "unit": "median tps",
            "extra": "avg tps: 1089.3194308784002, max tps: 1094.35553198641, count: 55272"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2855.349408776735,
            "unit": "median tps",
            "extra": "avg tps: 2808.9805493081576, max tps: 2872.116905898296, count: 55272"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1173.1353447389452,
            "unit": "median tps",
            "extra": "avg tps: 1165.9781370581838, max tps: 1180.2873307803327, count: 55272"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 977.4238554121233,
            "unit": "median tps",
            "extra": "avg tps: 970.3340042748263, max tps: 987.5600908615886, count: 55272"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 162.78866379011959,
            "unit": "median tps",
            "extra": "avg tps: 162.5982143568867, max tps: 164.90386942552027, count: 110544"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 144.35615495598807,
            "unit": "median tps",
            "extra": "avg tps: 144.35725995581254, max tps: 145.9244028318527, count: 55272"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 40.82734057873256,
            "unit": "median tps",
            "extra": "avg tps: 47.02906381383853, max tps: 755.2236934818909, count: 55272"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756058190150,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1156.8721922839668,
            "unit": "median tps",
            "extra": "avg tps: 1150.8442890806518, max tps: 1161.013078580355, count: 55271"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2596.9196280700085,
            "unit": "median tps",
            "extra": "avg tps: 2584.977904634891, max tps: 2603.30336355734, count: 55271"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1155.7358342685557,
            "unit": "median tps",
            "extra": "avg tps: 1148.9034391708783, max tps: 1159.795500887032, count: 55271"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 964.0417870982945,
            "unit": "median tps",
            "extra": "avg tps: 953.6742240474941, max tps: 970.4210261975232, count: 55271"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 175.9085645247319,
            "unit": "median tps",
            "extra": "avg tps: 177.91713765344235, max tps: 183.9801403262981, count: 110542"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 153.5271786629015,
            "unit": "median tps",
            "extra": "avg tps: 152.29090594364388, max tps: 154.6829434378801, count: 55271"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 59.88747149429553,
            "unit": "median tps",
            "extra": "avg tps: 65.5500892050404, max tps: 784.5801750712202, count: 55271"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756058538356,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1161.1935255388967,
            "unit": "median tps",
            "extra": "avg tps: 1154.2916723341584, max tps: 1163.258645389186, count: 55243"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2504.293401375749,
            "unit": "median tps",
            "extra": "avg tps: 2492.5776824309983, max tps: 2596.17854219675, count: 55243"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1161.594316650474,
            "unit": "median tps",
            "extra": "avg tps: 1154.6082087497612, max tps: 1164.5224783960593, count: 55243"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 874.8188835822835,
            "unit": "median tps",
            "extra": "avg tps: 874.705631408101, max tps: 894.2258186515871, count: 55243"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 174.53840015027146,
            "unit": "median tps",
            "extra": "avg tps: 173.50799298576914, max tps: 175.3928907892119, count: 110486"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 157.89336795702442,
            "unit": "median tps",
            "extra": "avg tps: 155.92944973100995, max tps: 159.68393080093202, count: 55243"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 91.15405195220752,
            "unit": "median tps",
            "extra": "avg tps: 91.44762936245895, max tps: 721.9309631858524, count: 55243"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "30a6611e0400569e914fd4b6a05437add41330c9",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3039)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:19-04:00",
          "tree_id": "d9cb66392591c4d960645870ca1a913b68494028",
          "url": "https://github.com/paradedb/paradedb/commit/30a6611e0400569e914fd4b6a05437add41330c9"
        },
        "date": 1756058540534,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1058.737047714144,
            "unit": "median tps",
            "extra": "avg tps: 1059.8396094755565, max tps: 1132.8923077696395, count: 55344"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2821.3985199089084,
            "unit": "median tps",
            "extra": "avg tps: 2788.7148566710316, max tps: 2841.545144874957, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1182.109467517143,
            "unit": "median tps",
            "extra": "avg tps: 1173.9092270195752, max tps: 1184.9342388981363, count: 55344"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 995.1555708464607,
            "unit": "median tps",
            "extra": "avg tps: 985.5002521822523, max tps: 1001.8698344347343, count: 55344"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 164.98112600108044,
            "unit": "median tps",
            "extra": "avg tps: 164.50393251919627, max tps: 168.03209083893077, count: 110688"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 146.348097612804,
            "unit": "median tps",
            "extra": "avg tps: 146.20334565899483, max tps: 149.43688492537024, count: 55344"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 57.13400808258307,
            "unit": "median tps",
            "extra": "avg tps: 71.78800166939506, max tps: 819.8233280728003, count: 55344"
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
          "id": "63f8cdc8a048cd896087e9de9ae429c1934fbb5d",
          "message": "fix: Remove the string/numeric classification from fast fields explain (#3019)\n\n## What\n\nRemove the string/numeric classification from MixedFastFields explain.\n\n## Why\n\nThe distinction is no longer actually used in the scan implementation,\nand it can be confusing when the field being projected from is actually\ne.g. a JSON field with nested/dynamic columns for the subtypes.",
          "timestamp": "2025-08-25T10:59:27-07:00",
          "tree_id": "b67ebb1850debf3ad52031247220f30f60dfd572",
          "url": "https://github.com/paradedb/paradedb/commit/63f8cdc8a048cd896087e9de9ae429c1934fbb5d"
        },
        "date": 1756145824658,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - tps",
            "value": 1180.53937834234,
            "unit": "median tps",
            "extra": "avg tps: 1172.0373303038589, max tps: 1184.0496669959218, count: 55276"
          },
          {
            "name": "Delete values - Primary - tps",
            "value": 2786.4716145751436,
            "unit": "median tps",
            "extra": "avg tps: 2754.3372935697785, max tps: 2808.3131873581074, count: 55276"
          },
          {
            "name": "Index Only Scan - Primary - tps",
            "value": 1091.2490055115122,
            "unit": "median tps",
            "extra": "avg tps: 1090.2208102961472, max tps: 1098.961558742776, count: 55276"
          },
          {
            "name": "Index Scan - Primary - tps",
            "value": 944.2535365643239,
            "unit": "median tps",
            "extra": "avg tps: 936.322407892788, max tps: 951.60297564677, count: 55276"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 171.04081466627585,
            "unit": "median tps",
            "extra": "avg tps: 170.07097617020054, max tps: 172.5068531680193, count: 110552"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 155.05141444106388,
            "unit": "median tps",
            "extra": "avg tps: 154.85293308141624, max tps: 157.9328776340067, count: 55276"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 153.52967245702723,
            "unit": "median tps",
            "extra": "avg tps: 142.46722028036191, max tps: 789.5582500546769, count: 55276"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755715496372,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.765865028409586, max cpu: 9.628887, count: 55217"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 60.4453125,
            "unit": "median mem",
            "extra": "avg mem: 60.31556907010069, max mem: 82.87109375, count: 55217"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654006232620673, max cpu: 9.421001, count: 55217"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.00390625,
            "unit": "median mem",
            "extra": "avg mem: 53.67067922073365, max mem: 76.1015625, count: 55217"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.711898670014493, max cpu: 9.657948, count: 55217"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.3828125,
            "unit": "median mem",
            "extra": "avg mem: 61.22373692941033, max mem: 84.15625, count: 55217"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.302178877400693, max cpu: 4.8096194, count: 55217"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.87890625,
            "unit": "median mem",
            "extra": "avg mem: 59.99216889443016, max mem: 83.35546875, count: 55217"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 7.623292340933352, max cpu: 33.633633, count: 110434"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 68.6484375,
            "unit": "median mem",
            "extra": "avg mem: 68.70816831206875, max mem: 103.90234375, count: 110434"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3671,
            "unit": "median block_count",
            "extra": "avg block_count: 3683.9262727058695, max block_count: 6592.0, count: 55217"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.920477389209845, max segment_count: 28.0, count: 55217"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.673807,
            "unit": "median cpu",
            "extra": "avg cpu: 6.474361717252339, max cpu: 14.4, count: 55217"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 68.62109375,
            "unit": "median mem",
            "extra": "avg mem: 68.05046262077349, max mem: 95.48828125, count: 55217"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.997181552340484, max cpu: 9.302325, count: 55217"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.10546875,
            "unit": "median mem",
            "extra": "avg mem: 58.08749944819983, max mem: 82.7734375, count: 55217"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755715528446,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 7.494145,
            "unit": "median cpu",
            "extra": "avg cpu: 7.218016200246679, max cpu: 27.639154, count: 55218"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.26953125,
            "unit": "median mem",
            "extra": "avg mem: 96.48787886762469, max mem: 99.1953125, count: 55218"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.617936307660204, max cpu: 9.230769, count: 55218"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 88.1796875,
            "unit": "median mem",
            "extra": "avg mem: 86.70871166331631, max mem: 88.953125, count: 55218"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.7151275,
            "unit": "median cpu",
            "extra": "avg cpu: 7.123484248283452, max cpu: 23.188406, count: 55218"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.97265625,
            "unit": "median mem",
            "extra": "avg mem: 97.08497512133725, max mem: 99.68359375, count: 55218"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.585503277005196, max cpu: 4.738401, count: 55218"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.42578125,
            "unit": "median mem",
            "extra": "avg mem: 94.6939390376553, max mem: 96.19140625, count: 55218"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 7.825029671532665, max cpu: 23.59882, count: 110436"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 107.09765625,
            "unit": "median mem",
            "extra": "avg mem: 106.87385761747754, max mem: 118.6171875, count: 110436"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8295,
            "unit": "median block_count",
            "extra": "avg block_count: 8256.86323300373, max block_count: 8295.0, count: 55218"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.87002426744903, max segment_count: 376.0, count: 55218"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1509034103972775, max cpu: 18.497108, count: 55218"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 115.03125,
            "unit": "median mem",
            "extra": "avg mem: 114.806303184084, max mem: 121.16015625, count: 55218"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.479307,
            "unit": "median cpu",
            "extra": "avg cpu: 16.83223000910111, max cpu: 32.43243, count: 55218"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 98.19921875,
            "unit": "median mem",
            "extra": "avg mem: 95.3868237316183, max mem: 99.7421875, count: 55218"
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
        "date": 1755724633473,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.792893827557509, max cpu: 9.458128, count: 55222"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.9453125,
            "unit": "median mem",
            "extra": "avg mem: 60.17221496301293, max mem: 83.24609375, count: 55222"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.636582444708272, max cpu: 9.284333, count: 55222"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 54.34765625,
            "unit": "median mem",
            "extra": "avg mem: 53.41448883314168, max mem: 77.96875, count: 55222"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.756704270616309, max cpu: 9.571285, count: 55222"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 60.203125,
            "unit": "median mem",
            "extra": "avg mem: 60.92965489014795, max mem: 85.9453125, count: 55222"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.411199437695918, max cpu: 4.738401, count: 55222"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 58.578125,
            "unit": "median mem",
            "extra": "avg mem: 59.64601291152077, max mem: 82.8203125, count: 55222"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.204219,
            "unit": "median cpu",
            "extra": "avg cpu: 7.455907291560713, max cpu: 23.59882, count: 110444"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 74.4921875,
            "unit": "median mem",
            "extra": "avg mem: 74.51784094908959, max mem: 105.0703125, count: 110444"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3661,
            "unit": "median block_count",
            "extra": "avg block_count: 3698.721687008801, max block_count: 6669.0, count: 55222"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.907138459309696, max segment_count: 28.0, count: 55222"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.135763201514212, max cpu: 14.428859, count: 55222"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 78.51171875,
            "unit": "median mem",
            "extra": "avg mem: 78.49277214300913, max mem: 107.6875, count: 55222"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.298346751273304, max cpu: 4.64666, count: 55222"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.38671875,
            "unit": "median mem",
            "extra": "avg mem: 58.39812279016968, max mem: 83.40234375, count: 55222"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755724681993,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.767901891807581, max cpu: 9.657948, count: 55406"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 90.03515625,
            "unit": "median mem",
            "extra": "avg mem: 92.30945119887738, max mem: 150.4609375, count: 55406"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.681618315321331, max cpu: 9.338522, count: 55406"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.08203125,
            "unit": "median mem",
            "extra": "avg mem: 87.34204067192633, max mem: 144.66015625, count: 55406"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.773798362830636, max cpu: 9.523809, count: 55406"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.90625,
            "unit": "median mem",
            "extra": "avg mem: 94.90233669977079, max mem: 152.78125, count: 55406"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.54504335711515, max cpu: 4.733728, count: 55406"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.26171875,
            "unit": "median mem",
            "extra": "avg mem: 94.48436541168826, max mem: 152.4375, count: 55406"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.836221282638723, max cpu: 28.77123, count: 110812"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 104.78125,
            "unit": "median mem",
            "extra": "avg mem: 106.6304252923871, max mem: 171.4609375, count: 110812"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7829,
            "unit": "median block_count",
            "extra": "avg block_count: 8087.375212070895, max block_count: 15433.0, count: 55406"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.727646825253583, max segment_count: 44.0, count: 55406"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.678363,
            "unit": "median cpu",
            "extra": "avg cpu: 7.107703068317833, max cpu: 33.802814, count: 55406"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 106.16015625,
            "unit": "median mem",
            "extra": "avg mem: 107.66227202378803, max mem: 171.48828125, count: 55406"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.930217358023299, max cpu: 9.365853, count: 55406"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.8125,
            "unit": "median mem",
            "extra": "avg mem: 84.95485188314443, max mem: 142.828125, count: 55406"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755724990767,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.7244096,
            "unit": "median cpu",
            "extra": "avg cpu: 7.169595769854749, max cpu: 23.645319, count: 55305"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 110.140625,
            "unit": "median mem",
            "extra": "avg mem: 105.94309885238677, max mem: 112.26171875, count: 55305"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641287057478375, max cpu: 9.302325, count: 55305"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 91.8671875,
            "unit": "median mem",
            "extra": "avg mem: 88.38590613981557, max mem: 91.8671875, count: 55305"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.733728,
            "unit": "median cpu",
            "extra": "avg cpu: 7.214689590140456, max cpu: 23.645319, count: 55305"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 110.6796875,
            "unit": "median mem",
            "extra": "avg mem: 106.04537246575806, max mem: 112.85546875, count: 55305"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.625082354699752, max cpu: 9.275363, count: 55305"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 107.8984375,
            "unit": "median mem",
            "extra": "avg mem: 103.14105542559443, max mem: 108.2734375, count: 55305"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 7.680059757378835, max cpu: 23.622047, count: 110610"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 121.9921875,
            "unit": "median mem",
            "extra": "avg mem: 118.54258818992406, max mem: 129.33984375, count: 110610"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 10175,
            "unit": "median block_count",
            "extra": "avg block_count: 9549.855130639182, max block_count: 10175.0, count: 55305"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 119,
            "unit": "median segment_count",
            "extra": "avg segment_count: 118.65413615405478, max segment_count: 346.0, count: 55305"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 6.064587030060571, max cpu: 18.514948, count: 55305"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 119.66796875,
            "unit": "median mem",
            "extra": "avg mem: 117.79450207937799, max mem: 126.65234375, count: 55305"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.461538,
            "unit": "median cpu",
            "extra": "avg cpu: 16.589215135711523, max cpu: 28.263002, count: 55305"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 97.203125,
            "unit": "median mem",
            "extra": "avg mem: 93.37971031382786, max mem: 98.046875, count: 55305"
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
        "date": 1755725042074,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.758665140319465, max cpu: 9.476802, count: 55285"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.22265625,
            "unit": "median mem",
            "extra": "avg mem: 96.25204882936148, max mem: 153.15625, count: 55285"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622598740036425, max cpu: 9.430255, count: 55285"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.92578125,
            "unit": "median mem",
            "extra": "avg mem: 88.71810490526363, max mem: 146.38671875, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.774934073139636, max cpu: 9.402546, count: 55285"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 99.453125,
            "unit": "median mem",
            "extra": "avg mem: 97.4422381607805, max mem: 153.51953125, count: 55285"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.567626199658905, max cpu: 4.6829267, count: 55285"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 99.9609375,
            "unit": "median mem",
            "extra": "avg mem: 97.36396011859003, max mem: 154.3671875, count: 55285"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.524579182750887, max cpu: 27.906979, count: 110570"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.83984375,
            "unit": "median mem",
            "extra": "avg mem: 109.84729872761599, max mem: 172.87109375, count: 110570"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8727,
            "unit": "median block_count",
            "extra": "avg block_count: 8434.881613457539, max block_count: 15841.0, count: 55285"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.663254047209913, max segment_count: 43.0, count: 55285"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.6662999179468025, max cpu: 18.953604, count: 55285"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 118.55859375,
            "unit": "median mem",
            "extra": "avg mem: 115.42717304366012, max mem: 176.73046875, count: 55285"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6829267,
            "unit": "median cpu",
            "extra": "avg cpu: 6.510061060520302, max cpu: 13.88621, count: 55285"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 93.28515625,
            "unit": "median mem",
            "extra": "avg mem: 89.5196209132224, max mem: 146.46484375, count: 55285"
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
        "date": 1755725059364,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759207703721713, max cpu: 9.476802, count: 55283"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 59.21875,
            "unit": "median mem",
            "extra": "avg mem: 59.48304116715356, max mem: 83.22265625, count: 55283"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.616716729006913, max cpu: 9.320388, count: 55283"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.578125,
            "unit": "median mem",
            "extra": "avg mem: 53.82879173977534, max mem: 77.16015625, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.753208109511162, max cpu: 9.467456, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 61.2421875,
            "unit": "median mem",
            "extra": "avg mem: 61.121692162373606, max mem: 84.07421875, count: 55283"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.249726531361151, max cpu: 4.7244096, count: 55283"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 61.52734375,
            "unit": "median mem",
            "extra": "avg mem: 60.73168875094966, max mem: 84.15625, count: 55283"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.303011509552543, max cpu: 24.048098, count: 110566"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 63.078125,
            "unit": "median mem",
            "extra": "avg mem: 63.86547451013422, max mem: 92.80859375, count: 110566"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3764,
            "unit": "median block_count",
            "extra": "avg block_count: 3752.3775844292095, max block_count: 6732.0, count: 55283"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.939276088490132, max segment_count: 28.0, count: 55283"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.128815230704097, max cpu: 14.145383, count: 55283"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 68.0859375,
            "unit": "median mem",
            "extra": "avg mem: 67.59847412970534, max mem: 95.6953125, count: 55283"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.817697263733153, max cpu: 9.448819, count: 55283"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.66015625,
            "unit": "median mem",
            "extra": "avg mem: 59.4088214279254, max mem: 84.94140625, count: 55283"
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
        "date": 1755726152063,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.756071577172462, max cpu: 9.467456, count: 55251"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 61.1171875,
            "unit": "median mem",
            "extra": "avg mem: 60.68258754649237, max mem: 84.6796875, count: 55251"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.663668912757657, max cpu: 9.411765, count: 55251"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 53.3984375,
            "unit": "median mem",
            "extra": "avg mem: 53.30086913019674, max mem: 77.46875, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.786378217931747, max cpu: 9.458128, count: 55251"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 61.33203125,
            "unit": "median mem",
            "extra": "avg mem: 61.280318102274165, max mem: 86.0546875, count: 55251"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.187186959778631, max cpu: 4.733728, count: 55251"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 59.8359375,
            "unit": "median mem",
            "extra": "avg mem: 59.19488928933413, max mem: 82.73046875, count: 55251"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.2284016204457515, max cpu: 18.934912, count: 110502"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 74.36328125,
            "unit": "median mem",
            "extra": "avg mem: 74.66756148220168, max mem: 103.64453125, count: 110502"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 3773,
            "unit": "median block_count",
            "extra": "avg block_count: 3765.625328048361, max block_count: 6761.0, count: 55251"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 8,
            "unit": "median segment_count",
            "extra": "avg segment_count: 8.917829541546759, max segment_count: 27.0, count: 55251"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 6.069954991225789, max cpu: 14.187191, count: 55251"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 79.58203125,
            "unit": "median mem",
            "extra": "avg mem: 79.04572463903368, max mem: 106.52734375, count: 55251"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7995945105934705, max cpu: 9.29332, count: 55251"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 59.9140625,
            "unit": "median mem",
            "extra": "avg mem: 58.93570718742647, max mem: 84.39453125, count: 55251"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755794895362,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.739570589979831, max cpu: 9.514371, count: 55179"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.9140625,
            "unit": "median mem",
            "extra": "avg mem: 98.19707551219668, max mem: 153.203125, count: 55179"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.654705491408566, max cpu: 9.430255, count: 55179"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 91.828125,
            "unit": "median mem",
            "extra": "avg mem: 90.51968840897352, max mem: 144.94921875, count: 55179"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.77063254378312, max cpu: 9.514371, count: 55179"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 101.9765625,
            "unit": "median mem",
            "extra": "avg mem: 100.13572064893347, max mem: 155.546875, count: 55179"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.715072576530571, max cpu: 9.393347, count: 55179"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 100.39453125,
            "unit": "median mem",
            "extra": "avg mem: 99.44605065162925, max mem: 154.78515625, count: 55179"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.266409,
            "unit": "median cpu",
            "extra": "avg cpu: 8.678696830113436, max cpu: 28.77123, count: 110358"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 112.171875,
            "unit": "median mem",
            "extra": "avg mem: 111.36295171170191, max mem: 174.265625, count: 110358"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8908,
            "unit": "median block_count",
            "extra": "avg block_count: 8773.44228782689, max block_count: 16553.0, count: 55179"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.704398412439515, max segment_count: 41.0, count: 55179"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.669261,
            "unit": "median cpu",
            "extra": "avg cpu: 6.789611768530064, max cpu: 23.166023, count: 55179"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 117.21484375,
            "unit": "median mem",
            "extra": "avg mem: 116.12246025435401, max mem: 177.234375, count: 55179"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.567168652413453, max cpu: 9.302325, count: 55179"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 89.046875,
            "unit": "median mem",
            "extra": "avg mem: 88.23257963570833, max mem: 144.03515625, count: 55179"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755797106467,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.759531985476192, max cpu: 9.476802, count: 55359"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 97.0234375,
            "unit": "median mem",
            "extra": "avg mem: 97.73677637105078, max mem: 155.1796875, count: 55359"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7211353179137525, max cpu: 9.356726, count: 55359"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 90.54296875,
            "unit": "median mem",
            "extra": "avg mem: 90.54917123344894, max mem: 147.328125, count: 55359"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.745830375721818, max cpu: 9.504951, count: 55359"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 97.37109375,
            "unit": "median mem",
            "extra": "avg mem: 97.71399749814844, max mem: 154.53515625, count: 55359"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.796174154701137, max cpu: 9.221902, count: 55359"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 97.69140625,
            "unit": "median mem",
            "extra": "avg mem: 97.42589294988169, max mem: 152.75, count: 55359"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.248554,
            "unit": "median cpu",
            "extra": "avg cpu: 8.312841028812521, max cpu: 28.235296, count: 110718"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 109.3046875,
            "unit": "median mem",
            "extra": "avg mem: 109.47327215171653, max mem: 172.6484375, count: 110718"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8495,
            "unit": "median block_count",
            "extra": "avg block_count: 8587.801405372207, max block_count: 16475.0, count: 55359"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.690529091927239, max segment_count: 34.0, count: 55359"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.623520267125766, max cpu: 23.323614, count: 55359"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 108.7890625,
            "unit": "median mem",
            "extra": "avg mem: 112.46782090818566, max mem: 173.4375, count: 55359"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.376552556145959, max cpu: 4.678363, count: 55359"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 88.73828125,
            "unit": "median mem",
            "extra": "avg mem: 89.21277014013079, max mem: 147.65625, count: 55359"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755804856255,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.781164602908165, max cpu: 14.131501, count: 55214"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.03125,
            "unit": "median mem",
            "extra": "avg mem: 97.91913219190332, max mem: 153.8359375, count: 55214"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.644842013136253, max cpu: 9.257474, count: 55214"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 91.7578125,
            "unit": "median mem",
            "extra": "avg mem: 91.83927267654038, max mem: 149.02734375, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.754549284340318, max cpu: 9.486166, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 98.5,
            "unit": "median mem",
            "extra": "avg mem: 98.54448394825135, max mem: 153.73828125, count: 55214"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.3846503182917544, max cpu: 4.729064, count: 55214"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 97.28515625,
            "unit": "median mem",
            "extra": "avg mem: 98.02109646670229, max mem: 154.14453125, count: 55214"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.166536219731084, max cpu: 27.853, count: 110428"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 108.90234375,
            "unit": "median mem",
            "extra": "avg mem: 109.0450717888579, max mem: 172.66015625, count: 110428"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8567,
            "unit": "median block_count",
            "extra": "avg block_count: 8644.472108523201, max block_count: 16349.0, count: 55214"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.69725069728692, max segment_count: 37.0, count: 55214"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.70931876931444, max cpu: 18.75, count: 55214"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 114.265625,
            "unit": "median mem",
            "extra": "avg mem: 113.27378048366356, max mem: 173.33984375, count: 55214"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.617249665143023, max cpu: 9.320388, count: 55214"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 92.2578125,
            "unit": "median mem",
            "extra": "avg mem: 89.46111083126561, max mem: 148.05859375, count: 55214"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755811649640,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.803854728177791, max cpu: 9.514371, count: 55131"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.58984375,
            "unit": "median mem",
            "extra": "avg mem: 91.40810105986196, max mem: 144.38671875, count: 55131"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7284067038903315, max cpu: 9.311348, count: 55131"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.359375,
            "unit": "median mem",
            "extra": "avg mem: 85.01707254652555, max mem: 138.53125, count: 55131"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.775669739414616, max cpu: 9.514371, count: 55131"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.87109375,
            "unit": "median mem",
            "extra": "avg mem: 92.11093074053164, max mem: 145.5390625, count: 55131"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7044884734676935, max cpu: 4.7058825, count: 55131"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 94.2265625,
            "unit": "median mem",
            "extra": "avg mem: 91.67224592051205, max mem: 145.0546875, count: 55131"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.546025690699748, max cpu: 32.526623, count: 110262"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 101.16796875,
            "unit": "median mem",
            "extra": "avg mem: 99.13176938791243, max mem: 157.90625, count: 110262"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7902,
            "unit": "median block_count",
            "extra": "avg block_count: 7751.06176198509, max block_count: 14505.0, count: 55131"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.611724800928696, max segment_count: 31.0, count: 55131"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.716924787361683, max cpu: 19.009901, count: 55131"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 106.8828125,
            "unit": "median mem",
            "extra": "avg mem: 106.02973308744173, max mem: 160.78515625, count: 55131"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.783820461737386, max cpu: 9.320388, count: 55131"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.1640625,
            "unit": "median mem",
            "extra": "avg mem: 83.29071247630644, max mem: 134.11328125, count: 55131"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755812978349,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.714308591819078, max cpu: 9.648242, count: 55192"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 93.3203125,
            "unit": "median mem",
            "extra": "avg mem: 92.24792457466661, max mem: 146.34765625, count: 55192"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.613434815405436, max cpu: 9.504951, count: 55192"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.3046875,
            "unit": "median mem",
            "extra": "avg mem: 85.64166179258316, max mem: 139.80859375, count: 55192"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.756074602058195, max cpu: 14.117648, count: 55192"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 94.85546875,
            "unit": "median mem",
            "extra": "avg mem: 93.33320810760617, max mem: 147.59375, count: 55192"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.498046325471212, max cpu: 4.701273, count: 55192"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.5,
            "unit": "median mem",
            "extra": "avg mem: 92.1681200683523, max mem: 146.734375, count: 55192"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.41762820982373, max cpu: 28.713858, count: 110384"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 103.15625,
            "unit": "median mem",
            "extra": "avg mem: 101.15111306750752, max mem: 160.9140625, count: 110384"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8030,
            "unit": "median block_count",
            "extra": "avg block_count: 7887.8976119727495, max block_count: 14839.0, count: 55192"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.613259167995361, max segment_count: 27.0, count: 55192"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.911610585427996, max cpu: 28.235296, count: 55192"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 107.466796875,
            "unit": "median mem",
            "extra": "avg mem: 106.16671226978094, max mem: 164.4140625, count: 55192"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.296022079424996, max cpu: 9.375, count: 55192"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 88.83203125,
            "unit": "median mem",
            "extra": "avg mem: 85.94240128494619, max mem: 140.4375, count: 55192"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755814759294,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.785785024961691, max cpu: 9.67742, count: 55241"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 93.51171875,
            "unit": "median mem",
            "extra": "avg mem: 92.27627874336996, max mem: 141.328125, count: 55241"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 4.629341032048114, max cpu: 9.347614, count: 55241"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.29296875,
            "unit": "median mem",
            "extra": "avg mem: 84.12697232354591, max mem: 133.2890625, count: 55241"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.757198301710887, max cpu: 9.619239, count: 55241"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.87109375,
            "unit": "median mem",
            "extra": "avg mem: 92.65117076480784, max mem: 142.6328125, count: 55241"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.601515517532544, max cpu: 4.743083, count: 55241"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 93.5859375,
            "unit": "median mem",
            "extra": "avg mem: 92.23699484135425, max mem: 141.61328125, count: 55241"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.257474,
            "unit": "median cpu",
            "extra": "avg cpu: 8.610058160639927, max cpu: 38.63179, count: 110482"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 104.00390625,
            "unit": "median mem",
            "extra": "avg mem: 101.58871778887058, max mem: 155.19921875, count: 110482"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7989,
            "unit": "median block_count",
            "extra": "avg block_count: 7789.359352654731, max block_count: 14081.0, count: 55241"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.557412067124057, max segment_count: 28.0, count: 55241"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6829267,
            "unit": "median cpu",
            "extra": "avg cpu: 6.7509362642337605, max cpu: 23.210833, count: 55241"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 111.203125,
            "unit": "median mem",
            "extra": "avg mem: 109.38802863532068, max mem: 163.08984375, count: 55241"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 3.876947500687661, max cpu: 4.743083, count: 55241"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 87.05078125,
            "unit": "median mem",
            "extra": "avg mem: 85.06652469406781, max mem: 134.86328125, count: 55241"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755834373138,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747819714644986, max cpu: 9.542743, count: 55102"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 94.921875,
            "unit": "median mem",
            "extra": "avg mem: 92.71564595545534, max mem: 145.3359375, count: 55102"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.691807649389546, max cpu: 9.302325, count: 55102"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 89.38671875,
            "unit": "median mem",
            "extra": "avg mem: 86.78287319709356, max mem: 138.796875, count: 55102"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.729409249518492, max cpu: 9.476802, count: 55102"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 95.140625,
            "unit": "median mem",
            "extra": "avg mem: 93.8564895840895, max mem: 146.53515625, count: 55102"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.565528351350581, max cpu: 4.7244096, count: 55102"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 96.2421875,
            "unit": "median mem",
            "extra": "avg mem: 93.0405668124569, max mem: 144.34375, count: 55102"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.239654,
            "unit": "median cpu",
            "extra": "avg cpu: 8.487052319934998, max cpu: 28.742516, count: 110204"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 106.8359375,
            "unit": "median mem",
            "extra": "avg mem: 105.41996759986026, max mem: 162.8046875, count: 110204"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8243,
            "unit": "median block_count",
            "extra": "avg block_count: 8024.6505390003995, max block_count: 14620.0, count: 55102"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.602373779536133, max segment_count: 28.0, count: 55102"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.825304529248399, max cpu: 23.66864, count: 55102"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 112.5546875,
            "unit": "median mem",
            "extra": "avg mem: 110.61238206530616, max mem: 166.7265625, count: 55102"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 5.284363540057443, max cpu: 9.29332, count: 55102"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 87.1328125,
            "unit": "median mem",
            "extra": "avg mem: 85.84920749246851, max mem: 139.1328125, count: 55102"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755834674542,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.785809566256347, max cpu: 9.448819, count: 55253"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.98828125,
            "unit": "median mem",
            "extra": "avg mem: 91.28049346585253, max mem: 144.26171875, count: 55253"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.679723969838234, max cpu: 9.275363, count: 55253"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.17578125,
            "unit": "median mem",
            "extra": "avg mem: 85.43827420852713, max mem: 138.046875, count: 55253"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.770247417723291, max cpu: 9.628887, count: 55253"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 90.16796875,
            "unit": "median mem",
            "extra": "avg mem: 90.84172048633106, max mem: 142.97265625, count: 55253"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.4413726031867675, max cpu: 4.6421666, count: 55253"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 91.25390625,
            "unit": "median mem",
            "extra": "avg mem: 91.07245696783433, max mem: 142.82421875, count: 55253"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.107218796182243, max cpu: 27.906979, count: 110506"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 101.0703125,
            "unit": "median mem",
            "extra": "avg mem: 101.18564383664688, max mem: 157.70703125, count: 110506"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7744,
            "unit": "median block_count",
            "extra": "avg block_count: 7742.546250882305, max block_count: 14417.0, count: 55253"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.595261795739598, max segment_count: 32.0, count: 55253"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.875402256007984, max cpu: 23.166023, count: 55253"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 104.95703125,
            "unit": "median mem",
            "extra": "avg mem: 105.60944675798147, max mem: 162.8828125, count: 55253"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.372626629164036, max cpu: 9.213051, count: 55253"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 87.78515625,
            "unit": "median mem",
            "extra": "avg mem: 85.77296997730892, max mem: 138.22265625, count: 55253"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "74e58071d8682e5f1a784791d82619ae41994f0b",
          "message": "fix: restore the garbage list (#3021)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:21-04:00",
          "tree_id": "3455b2de8f4efa7f45573d0f6a731fe933e7cedb",
          "url": "https://github.com/paradedb/paradedb/commit/74e58071d8682e5f1a784791d82619ae41994f0b"
        },
        "date": 1755834679659,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.774356741264601, max cpu: 9.458128, count: 55214"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 90.921875,
            "unit": "median mem",
            "extra": "avg mem: 93.01796886319593, max mem: 150.72265625, count: 55214"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.657601856814395, max cpu: 9.421001, count: 55214"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 84.56640625,
            "unit": "median mem",
            "extra": "avg mem: 85.82328884544681, max mem: 142.16796875, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7608125606818765, max cpu: 9.448819, count: 55214"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 91.00390625,
            "unit": "median mem",
            "extra": "avg mem: 92.91243955337414, max mem: 150.87109375, count: 55214"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.175894358523823, max cpu: 4.6647234, count: 55214"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 91.328125,
            "unit": "median mem",
            "extra": "avg mem: 92.72617133731934, max mem: 149.72265625, count: 55214"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 8.01582682118821, max cpu: 27.799229, count: 110428"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.9453125,
            "unit": "median mem",
            "extra": "avg mem: 101.7385257532057, max mem: 165.33984375, count: 110428"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7783,
            "unit": "median block_count",
            "extra": "avg block_count: 7961.58124750969, max block_count: 15256.0, count: 55214"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.616311080523056, max segment_count: 29.0, count: 55214"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.867704093221378, max cpu: 18.622696, count: 55214"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 105.51171875,
            "unit": "median mem",
            "extra": "avg mem: 106.29618018924096, max mem: 167.98046875, count: 55214"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.505364556847967, max cpu: 4.6332045, count: 55214"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 84.62109375,
            "unit": "median mem",
            "extra": "avg mem: 84.20811637447024, max mem: 140.73828125, count: 55214"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755873801060,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.751587759932189, max cpu: 9.523809, count: 55289"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 88.83984375,
            "unit": "median mem",
            "extra": "avg mem: 89.13228445079491, max mem: 145.01171875, count: 55289"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.641061690383177, max cpu: 9.284333, count: 55289"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 81.97265625,
            "unit": "median mem",
            "extra": "avg mem: 82.4654614559406, max mem: 137.82421875, count: 55289"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.764187736520612, max cpu: 9.523809, count: 55289"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.390625,
            "unit": "median mem",
            "extra": "avg mem: 89.63206401815913, max mem: 144.8203125, count: 55289"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.395240307024582, max cpu: 4.733728, count: 55289"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 89.47265625,
            "unit": "median mem",
            "extra": "avg mem: 89.89087983990939, max mem: 144.859375, count: 55289"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.221902,
            "unit": "median cpu",
            "extra": "avg cpu: 8.244768832524178, max cpu: 28.886658, count: 110578"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.59765625,
            "unit": "median mem",
            "extra": "avg mem: 100.4378968493959, max mem: 161.09765625, count: 110578"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7446,
            "unit": "median block_count",
            "extra": "avg block_count: 7544.977409611315, max block_count: 14577.0, count: 55289"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.616361301524716, max segment_count: 26.0, count: 55289"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 6.876215926758416, max cpu: 23.143684, count: 55289"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 105.85546875,
            "unit": "median mem",
            "extra": "avg mem: 105.74396586854981, max mem: 164.87109375, count: 55289"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8536136232378935, max cpu: 4.7151275, count: 55289"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 80.83984375,
            "unit": "median mem",
            "extra": "avg mem: 81.17079678767476, max mem: 132.84375, count: 55289"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755887393067,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.751419238380519, max cpu: 9.619239, count: 55283"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 94.13671875,
            "unit": "median mem",
            "extra": "avg mem: 95.20017413241865, max mem: 153.953125, count: 55283"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.651537984555238, max cpu: 9.29332, count: 55283"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 87.06640625,
            "unit": "median mem",
            "extra": "avg mem: 88.20347546940289, max mem: 146.8671875, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.741616094630985, max cpu: 9.667674, count: 55283"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 94.421875,
            "unit": "median mem",
            "extra": "avg mem: 95.8737690468815, max mem: 153.84375, count: 55283"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.48003123106472, max cpu: 4.7197638, count: 55283"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 94.875,
            "unit": "median mem",
            "extra": "avg mem: 95.77304979322305, max mem: 154.77734375, count: 55283"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 8.329340445354937, max cpu: 29.003021, count: 110566"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 104.4453125,
            "unit": "median mem",
            "extra": "avg mem: 105.50518394414874, max mem: 171.5703125, count: 110566"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8174,
            "unit": "median block_count",
            "extra": "avg block_count: 8330.103413345874, max block_count: 16173.0, count: 55283"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 9,
            "unit": "median segment_count",
            "extra": "avg segment_count: 9.549192337608307, max segment_count: 31.0, count: 55283"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.660194,
            "unit": "median cpu",
            "extra": "avg cpu: 6.774556443607171, max cpu: 19.277107, count: 55283"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 111.59375,
            "unit": "median mem",
            "extra": "avg mem: 110.78864660022069, max mem: 172.390625, count: 55283"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.580839,
            "unit": "median cpu",
            "extra": "avg cpu: 5.78834287811914, max cpu: 9.580839, count: 55283"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 88.9140625,
            "unit": "median mem",
            "extra": "avg mem: 86.12293964973409, max mem: 144.640625, count: 55283"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755890736138,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.737850013287732, max cpu: 9.590409, count: 55132"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.76171875,
            "unit": "median mem",
            "extra": "avg mem: 90.56476027137144, max mem: 146.41796875, count: 55132"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747382034109143, max cpu: 9.504951, count: 55132"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 82.55859375,
            "unit": "median mem",
            "extra": "avg mem: 83.31995178900185, max mem: 139.9921875, count: 55132"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.752285597932011, max cpu: 9.687184, count: 55132"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 89.33203125,
            "unit": "median mem",
            "extra": "avg mem: 90.46603492527026, max mem: 147.578125, count: 55132"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.205262341904181, max cpu: 4.7524753, count: 55132"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 90.03125,
            "unit": "median mem",
            "extra": "avg mem: 90.19853105387978, max mem: 148.53515625, count: 55132"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.17782,
            "unit": "median cpu",
            "extra": "avg cpu: 7.404107350440047, max cpu: 24.120604, count: 110264"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 93.140625,
            "unit": "median mem",
            "extra": "avg mem: 94.24759377210604, max mem: 156.59765625, count: 110264"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7483,
            "unit": "median block_count",
            "extra": "avg block_count: 7580.616320830008, max block_count: 14740.0, count: 55132"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.567800914169629, max segment_count: 29.0, count: 55132"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.068139726513127, max cpu: 14.187191, count: 55132"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 97.625,
            "unit": "median mem",
            "extra": "avg mem: 97.73582861360009, max mem: 157.375, count: 55132"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.5448726944987, max cpu: 9.239654, count: 55132"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 84.171875,
            "unit": "median mem",
            "extra": "avg mem: 83.07177737917634, max mem: 138.6328125, count: 55132"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755891838099,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7113551930053665, max cpu: 9.448819, count: 55056"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.7734375,
            "unit": "median mem",
            "extra": "avg mem: 98.01672423589164, max mem: 154.21875, count: 55056"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.64189445520576, max cpu: 9.257474, count: 55056"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 93.65234375,
            "unit": "median mem",
            "extra": "avg mem: 91.957962191337, max mem: 148.6875, count: 55056"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 4.747894305508804, max cpu: 9.458128, count: 55056"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 101.17578125,
            "unit": "median mem",
            "extra": "avg mem: 98.94771183261135, max mem: 154.35546875, count: 55056"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.531710853123497, max cpu: 4.729064, count: 55056"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 98.98828125,
            "unit": "median mem",
            "extra": "avg mem: 97.8098375120332, max mem: 154.83203125, count: 55056"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.142857,
            "unit": "median cpu",
            "extra": "avg cpu: 7.211816726388126, max cpu: 23.622047, count: 110112"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 107.18359375,
            "unit": "median mem",
            "extra": "avg mem: 106.23613418184439, max mem: 167.96484375, count: 110112"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8759,
            "unit": "median block_count",
            "extra": "avg block_count: 8623.386715344377, max block_count: 15969.0, count: 55056"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.68853530950305, max segment_count: 30.0, count: 55056"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1119778777521425, max cpu: 18.532818, count: 55056"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 105.32421875,
            "unit": "median mem",
            "extra": "avg mem: 104.04042063421788, max mem: 161.75, count: 55056"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.575402095750174, max cpu: 9.257474, count: 55056"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 93.11328125,
            "unit": "median mem",
            "extra": "avg mem: 90.68922544488794, max mem: 149.41015625, count: 55056"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "de951c94cea485c109ffc396ae394d036babb97d",
          "message": "perf: optimize merging heuristics to prefer background merging (#3033)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:53-07:00",
          "tree_id": "f489cbf34a77726e0c2bc0f7f879b77d8c93cb47",
          "url": "https://github.com/paradedb/paradedb/commit/de951c94cea485c109ffc396ae394d036babb97d"
        },
        "date": 1755891843855,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.741331160840106, max cpu: 9.458128, count: 55154"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 89.20703125,
            "unit": "median mem",
            "extra": "avg mem: 91.26574476409236, max mem: 148.52734375, count: 55154"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.622634783050837, max cpu: 9.275363, count: 55154"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 83.26171875,
            "unit": "median mem",
            "extra": "avg mem: 85.23625560079505, max mem: 141.078125, count: 55154"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.7572175733144135, max cpu: 9.619239, count: 55154"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 90.296875,
            "unit": "median mem",
            "extra": "avg mem: 91.84202351314048, max mem: 150.2265625, count: 55154"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.132112059787043, max cpu: 4.7058825, count: 55154"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 90.11328125,
            "unit": "median mem",
            "extra": "avg mem: 91.99880441298455, max mem: 148.4921875, count: 55154"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.160305,
            "unit": "median cpu",
            "extra": "avg cpu: 7.341693977257425, max cpu: 24.048098, count: 110308"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 94.046875,
            "unit": "median mem",
            "extra": "avg mem: 95.8965424156113, max mem: 159.14453125, count: 110308"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7493,
            "unit": "median block_count",
            "extra": "avg block_count: 7739.373680966022, max block_count: 14995.0, count: 55154"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.637052616310694, max segment_count: 28.0, count: 55154"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 5.942285532386458, max cpu: 14.131501, count: 55154"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 99.609375,
            "unit": "median mem",
            "extra": "avg mem: 100.39817042111633, max mem: 158.87890625, count: 55154"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.619827,
            "unit": "median cpu",
            "extra": "avg cpu: 4.557278435288905, max cpu: 9.302325, count: 55154"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 83.3671875,
            "unit": "median mem",
            "extra": "avg mem: 84.47690911470609, max mem: 140.0078125, count: 55154"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a556c28327e5b7ca9b5196f4153d49bca00c90a6",
          "message": "chore: Upgrade to `0.17.18` (#3034)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T15:36:02-04:00",
          "tree_id": "9de75b3b13ca610745c645ca4046bddf6fe9eec4",
          "url": "https://github.com/paradedb/paradedb/commit/a556c28327e5b7ca9b5196f4153d49bca00c90a6"
        },
        "date": 1755892331908,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.749489382651631, max cpu: 9.402546, count: 55205"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 98.83203125,
            "unit": "median mem",
            "extra": "avg mem: 96.31521162598497, max mem: 151.16015625, count: 55205"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.670470699185967, max cpu: 9.284333, count: 55205"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 89.921875,
            "unit": "median mem",
            "extra": "avg mem: 88.31159584163572, max mem: 143.66796875, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.756081091085275, max cpu: 9.486166, count: 55205"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 99.08203125,
            "unit": "median mem",
            "extra": "avg mem: 97.50218779718776, max mem: 152.5625, count: 55205"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.422459631581886, max cpu: 4.7197638, count: 55205"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 98.3828125,
            "unit": "median mem",
            "extra": "avg mem: 96.1977783092564, max mem: 151.82421875, count: 55205"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.169055,
            "unit": "median cpu",
            "extra": "avg cpu: 7.256322360565983, max cpu: 22.900763, count: 110410"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 115.515625,
            "unit": "median mem",
            "extra": "avg mem: 113.31517776780862, max mem: 174.40234375, count: 110410"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8605,
            "unit": "median block_count",
            "extra": "avg block_count: 8348.821320532561, max block_count: 15381.0, count: 55205"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.641662892853908, max segment_count: 28.0, count: 55205"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.1465431977847755, max cpu: 14.159292, count: 55205"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 119.0546875,
            "unit": "median mem",
            "extra": "avg mem: 116.95631389536727, max mem: 173.4140625, count: 55205"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 4.496448383482808, max cpu: 9.356726, count: 55205"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 90.80859375,
            "unit": "median mem",
            "extra": "avg mem: 88.30664349073906, max mem: 140.25390625, count: 55205"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0cad75ef84c3942edd137497c4b6864b384a3655",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3037)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests",
          "timestamp": "2025-08-24T13:29:07-04:00",
          "tree_id": "d76d1a8d15e8ca4d61db683033627e7bf4ec476d",
          "url": "https://github.com/paradedb/paradedb/commit/0cad75ef84c3942edd137497c4b6864b384a3655"
        },
        "date": 1756057512881,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.809945800773907, max cpu: 9.514371, count: 55272"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 90.37890625,
            "unit": "median mem",
            "extra": "avg mem: 90.01807375626447, max mem: 146.03515625, count: 55272"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6248588447166386, max cpu: 9.284333, count: 55272"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 85.140625,
            "unit": "median mem",
            "extra": "avg mem: 83.97117903843719, max mem: 139.5078125, count: 55272"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.791214616029361, max cpu: 9.504951, count: 55272"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.9296875,
            "unit": "median mem",
            "extra": "avg mem: 91.73574478827435, max mem: 147.31640625, count: 55272"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.359668544742892, max cpu: 4.729064, count: 55272"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 92.46875,
            "unit": "median mem",
            "extra": "avg mem: 91.24187286284194, max mem: 146.875, count: 55272"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.213051,
            "unit": "median cpu",
            "extra": "avg cpu: 7.630805667087425, max cpu: 27.906979, count: 110544"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 100.68359375,
            "unit": "median mem",
            "extra": "avg mem: 100.48198751741388, max mem: 161.51953125, count: 110544"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7827,
            "unit": "median block_count",
            "extra": "avg block_count: 7743.096106527718, max block_count: 14753.0, count: 55272"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.649334201765813, max segment_count: 30.0, count: 55272"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.212508474632742, max cpu: 14.229248, count: 55272"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 111.404296875,
            "unit": "median mem",
            "extra": "avg mem: 110.90505272504613, max mem: 167.9765625, count: 55272"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.6046149290244625, max cpu: 9.467456, count: 55272"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.65625,
            "unit": "median mem",
            "extra": "avg mem: 85.10301646007473, max mem: 136.9453125, count: 55272"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756058192334,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.704294673172002, max cpu: 9.67742, count: 55271"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 94.81640625,
            "unit": "median mem",
            "extra": "avg mem: 95.03373456186337, max mem: 153.3515625, count: 55271"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.677298461660495, max cpu: 9.248554, count: 55271"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 88.15625,
            "unit": "median mem",
            "extra": "avg mem: 88.88338658892096, max mem: 148.23828125, count: 55271"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.774838298734925, max cpu: 9.687184, count: 55271"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.18359375,
            "unit": "median mem",
            "extra": "avg mem: 94.29119856592969, max mem: 153.47265625, count: 55271"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.531667950929236, max cpu: 4.804805, count: 55271"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 95.2421875,
            "unit": "median mem",
            "extra": "avg mem: 95.32547470644641, max mem: 154.66015625, count: 55271"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.186603,
            "unit": "median cpu",
            "extra": "avg cpu: 7.5088427964266575, max cpu: 24.19355, count: 110542"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 111.4921875,
            "unit": "median mem",
            "extra": "avg mem: 111.97491267459428, max mem: 177.11328125, count: 110542"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 8121,
            "unit": "median block_count",
            "extra": "avg block_count: 8231.52620723345, max block_count: 15958.0, count: 55271"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.741238624233324, max segment_count: 29.0, count: 55271"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 6.185387876840239, max cpu: 15.496368, count: 55271"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 116.625,
            "unit": "median mem",
            "extra": "avg mem: 116.04298097668759, max mem: 177.03125, count: 55271"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.650779536722073, max cpu: 9.275363, count: 55271"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 90.48828125,
            "unit": "median mem",
            "extra": "avg mem: 89.17839592360822, max mem: 145.5546875, count: 55271"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756058540978,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.788500217774131, max cpu: 13.872832, count: 55243"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.46484375,
            "unit": "median mem",
            "extra": "avg mem: 93.30826926035878, max mem: 150.765625, count: 55243"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.588525399802335, max cpu: 9.284333, count: 55243"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.421875,
            "unit": "median mem",
            "extra": "avg mem: 87.44980727196206, max mem: 145.8359375, count: 55243"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.733151857918612, max cpu: 9.448819, count: 55243"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 91.4140625,
            "unit": "median mem",
            "extra": "avg mem: 92.53086781074073, max mem: 150.0234375, count: 55243"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 4.193743421619346, max cpu: 4.7105007, count: 55243"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 92.875,
            "unit": "median mem",
            "extra": "avg mem: 94.36809916810728, max mem: 152.9140625, count: 55243"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.195402,
            "unit": "median cpu",
            "extra": "avg cpu: 7.558946810715128, max cpu: 32.526623, count: 110486"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 104.94140625,
            "unit": "median mem",
            "extra": "avg mem: 105.17650762172357, max mem: 167.859375, count: 110486"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7821,
            "unit": "median block_count",
            "extra": "avg block_count: 7968.506543815506, max block_count: 15339.0, count: 55243"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.567311695599443, max segment_count: 30.0, count: 55243"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 6.193587427651374, max cpu: 19.199999, count: 55243"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 100.22265625,
            "unit": "median mem",
            "extra": "avg mem: 100.26879253932626, max mem: 159.671875, count: 55243"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6065254,
            "unit": "median cpu",
            "extra": "avg cpu: 3.8359038618998946, max cpu: 9.239654, count: 55243"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 85.20703125,
            "unit": "median mem",
            "extra": "avg mem: 85.14559727872762, max mem: 141.6015625, count: 55243"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "30a6611e0400569e914fd4b6a05437add41330c9",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3039)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:19-04:00",
          "tree_id": "d9cb66392591c4d960645870ca1a913b68494028",
          "url": "https://github.com/paradedb/paradedb/commit/30a6611e0400569e914fd4b6a05437add41330c9"
        },
        "date": 1756058543225,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.794044124560664, max cpu: 9.648242, count: 55344"
          },
          {
            "name": "Custom Scan - Primary - mem",
            "value": 92.8515625,
            "unit": "median mem",
            "extra": "avg mem: 92.57276449344103, max mem: 148.05078125, count: 55344"
          },
          {
            "name": "Delete values - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.639222762581806, max cpu: 9.302325, count: 55344"
          },
          {
            "name": "Delete values - Primary - mem",
            "value": 86.0234375,
            "unit": "median mem",
            "extra": "avg mem: 86.63564880847517, max mem: 141.62109375, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.777098650453381, max cpu: 9.657948, count: 55344"
          },
          {
            "name": "Index Only Scan - Primary - mem",
            "value": 92.6484375,
            "unit": "median mem",
            "extra": "avg mem: 93.2278635951955, max mem: 148.5625, count: 55344"
          },
          {
            "name": "Index Scan - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.2256222000828165, max cpu: 4.729064, count: 55344"
          },
          {
            "name": "Index Scan - Primary - mem",
            "value": 92.6171875,
            "unit": "median mem",
            "extra": "avg mem: 92.91090201173569, max mem: 149.0234375, count: 55344"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.230769,
            "unit": "median cpu",
            "extra": "avg cpu: 7.582250156025421, max cpu: 28.973843, count: 110688"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 97.43359375,
            "unit": "median mem",
            "extra": "avg mem: 97.41823117117484, max mem: 158.421875, count: 110688"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 7930,
            "unit": "median block_count",
            "extra": "avg block_count: 7977.274826539462, max block_count: 15068.0, count: 55344"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 10,
            "unit": "median segment_count",
            "extra": "avg segment_count: 10.592205117085863, max segment_count: 33.0, count: 55344"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 4.655674,
            "unit": "median cpu",
            "extra": "avg cpu: 6.135394748395965, max cpu: 18.550726, count: 55344"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 100.76953125,
            "unit": "median mem",
            "extra": "avg mem: 100.41188485596903, max mem: 158.01171875, count: 55344"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 4.478666493585596, max cpu: 4.7105007, count: 55344"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 86.98046875,
            "unit": "median mem",
            "extra": "avg mem: 85.58143561463302, max mem: 140.91015625, count: 55344"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - TPS": [
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
        "date": 1755716036756,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.756446977464457,
            "unit": "median tps",
            "extra": "avg tps: 5.803493442503363, max tps: 8.776875796286348, count: 57767"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.8675198469742424,
            "unit": "median tps",
            "extra": "avg tps: 5.256345765480045, max tps: 6.615598829208859, count: 57767"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755716163778,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.776908165368656,
            "unit": "median tps",
            "extra": "avg tps: 5.836842792102271, max tps: 8.790577583553445, count: 57510"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.571663166294946,
            "unit": "median tps",
            "extra": "avg tps: 4.982886432593411, max tps: 6.317742235279659, count: 57510"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755716196855,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.71847561443096,
            "unit": "median tps",
            "extra": "avg tps: 5.765804920524814, max tps: 8.60127126911043, count: 57942"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.902979378878519,
            "unit": "median tps",
            "extra": "avg tps: 5.280683886069507, max tps: 6.7054643082124885, count: 57942"
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
        "date": 1755725306998,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.771200626675449,
            "unit": "median tps",
            "extra": "avg tps: 5.79199760248553, max tps: 8.669853127851368, count: 57907"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.919417332534481,
            "unit": "median tps",
            "extra": "avg tps: 5.30032750803755, max tps: 6.696960299404023, count: 57907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755725354427,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.788251074941238,
            "unit": "median tps",
            "extra": "avg tps: 5.826339693157543, max tps: 8.695569688198225, count: 57920"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.773949040940641,
            "unit": "median tps",
            "extra": "avg tps: 5.170705907826859, max tps: 6.521927069976282, count: 57920"
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
        "date": 1755725714962,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.70638872906968,
            "unit": "median tps",
            "extra": "avg tps: 5.7469084425852115, max tps: 8.588556230369807, count: 57918"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.910028671869697,
            "unit": "median tps",
            "extra": "avg tps: 5.2971521027980675, max tps: 6.711607562876859, count: 57918"
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
        "date": 1755726827225,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.902447324541406,
            "unit": "median tps",
            "extra": "avg tps: 5.900711347199286, max tps: 8.806789600765168, count: 57747"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.753408181129046,
            "unit": "median tps",
            "extra": "avg tps: 5.1668942879502895, max tps: 6.530891170148713, count: 57747"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755795575040,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.725057285366136,
            "unit": "median tps",
            "extra": "avg tps: 5.757325840457212, max tps: 8.606235202500832, count: 57880"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.888186454103314,
            "unit": "median tps",
            "extra": "avg tps: 5.273427334351231, max tps: 6.64321957527865, count: 57880"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755797785100,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.748924161782917,
            "unit": "median tps",
            "extra": "avg tps: 5.79062394215111, max tps: 8.6414469209464, count: 57733"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.803365907745763,
            "unit": "median tps",
            "extra": "avg tps: 5.205432038263172, max tps: 6.57126333550232, count: 57733"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755805535430,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.622367150187077,
            "unit": "median tps",
            "extra": "avg tps: 5.695066935323378, max tps: 8.545488642539713, count: 57713"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.75076722313134,
            "unit": "median tps",
            "extra": "avg tps: 5.155225844911593, max tps: 6.512659816886702, count: 57713"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755812332039,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.750632011401726,
            "unit": "median tps",
            "extra": "avg tps: 5.815933820777128, max tps: 8.717529746464027, count: 57491"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.742868989737723,
            "unit": "median tps",
            "extra": "avg tps: 5.143136354588341, max tps: 6.466997973109033, count: 57491"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755813659041,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.798292948451294,
            "unit": "median tps",
            "extra": "avg tps: 5.819962986444398, max tps: 8.686044075894594, count: 57686"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.761309054827524,
            "unit": "median tps",
            "extra": "avg tps: 5.179435689178295, max tps: 6.495068239888188, count: 57686"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755815439706,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.940452690665845,
            "unit": "median tps",
            "extra": "avg tps: 5.956810865729291, max tps: 8.909224164909922, count: 57465"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.647523742408952,
            "unit": "median tps",
            "extra": "avg tps: 5.043956965423059, max tps: 6.4204229880084185, count: 57465"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755835051233,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.862472965209608,
            "unit": "median tps",
            "extra": "avg tps: 5.885013025631855, max tps: 8.805737297828076, count: 57754"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.758756648778571,
            "unit": "median tps",
            "extra": "avg tps: 5.156711148327658, max tps: 6.49844099663835, count: 57754"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755835355972,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.85810925920981,
            "unit": "median tps",
            "extra": "avg tps: 5.875905791786639, max tps: 8.782568771497829, count: 57186"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.725182496460671,
            "unit": "median tps",
            "extra": "avg tps: 5.130871448806732, max tps: 6.4700576567791135, count: 57186"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755874483006,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.82289623878645,
            "unit": "median tps",
            "extra": "avg tps: 5.848144906434408, max tps: 8.745214456938537, count: 57735"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.754817655720525,
            "unit": "median tps",
            "extra": "avg tps: 5.1578249804193375, max tps: 6.492082597046589, count: 57735"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755888077008,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 6.698737998363695,
            "unit": "median tps",
            "extra": "avg tps: 5.747763168897072, max tps: 8.613129992232622, count: 57890"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.759795689873016,
            "unit": "median tps",
            "extra": "avg tps: 5.160932447188653, max tps: 6.5040380204866945, count: 57890"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755891423614,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.26568396997451,
            "unit": "median tps",
            "extra": "avg tps: 7.054716110467783, max tps: 10.904767720639486, count: 57524"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.428302337530054,
            "unit": "median tps",
            "extra": "avg tps: 4.9037378123836515, max tps: 6.005010258200556, count: 57524"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755892524156,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.242358987040967,
            "unit": "median tps",
            "extra": "avg tps: 7.015981981947876, max tps: 10.811479463729357, count: 57757"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.486706902046442,
            "unit": "median tps",
            "extra": "avg tps: 4.967084571373688, max tps: 6.095110410943558, count: 57757"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0cad75ef84c3942edd137497c4b6864b384a3655",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3037)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests",
          "timestamp": "2025-08-24T13:29:07-04:00",
          "tree_id": "d76d1a8d15e8ca4d61db683033627e7bf4ec476d",
          "url": "https://github.com/paradedb/paradedb/commit/0cad75ef84c3942edd137497c4b6864b384a3655"
        },
        "date": 1756058201519,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.602084478520775,
            "unit": "median tps",
            "extra": "avg tps: 6.538961881894259, max tps: 10.199816839344647, count: 57867"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.688525123104231,
            "unit": "median tps",
            "extra": "avg tps: 5.11452501591372, max tps: 6.313834313207874, count: 57867"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756058884829,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 8.004134266811894,
            "unit": "median tps",
            "extra": "avg tps: 6.831519477946425, max tps: 10.546885861822863, count: 57807"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.504793344321183,
            "unit": "median tps",
            "extra": "avg tps: 4.972771887229804, max tps: 6.1571026710471815, count: 57807"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756059231711,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 7.903677691683938,
            "unit": "median tps",
            "extra": "avg tps: 6.753852745914065, max tps: 10.415776187714847, count: 57917"
          },
          {
            "name": "Count Query - Primary - tps",
            "value": 5.563748064454779,
            "unit": "median tps",
            "extra": "avg tps: 5.015313956178831, max tps: 6.2021124768392255, count: 57917"
          }
        ]
      }
    ],
    "pg_search bulk-updates.toml Performance - Other Metrics": [
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
        "date": 1755716039503,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.14626637700689, max cpu: 55.92233, count: 57767"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 238.0234375,
            "unit": "median mem",
            "extra": "avg mem: 236.94477538808923, max mem: 243.484375, count: 57767"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.311381815931945, max cpu: 33.3996, count: 57767"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.625,
            "unit": "median mem",
            "extra": "avg mem: 159.8865327252151, max mem: 162.5, count: 57767"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22575,
            "unit": "median block_count",
            "extra": "avg block_count: 20737.431457406477, max block_count: 23515.0, count: 57767"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.62102930739003, max segment_count: 96.0, count: 57767"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755716166626,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 21.322116744740892, max cpu: 42.772278, count: 57510"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.1640625,
            "unit": "median mem",
            "extra": "avg mem: 226.48976319335767, max mem: 237.0234375, count: 57510"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.33255297882187, max cpu: 33.267326, count: 57510"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.44921875,
            "unit": "median mem",
            "extra": "avg mem: 161.18733997348286, max mem: 163.4453125, count: 57510"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22503,
            "unit": "median block_count",
            "extra": "avg block_count: 20730.061171970094, max block_count: 23506.0, count: 57510"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.65798991479743, max segment_count: 97.0, count: 57510"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755716199603,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.43332124488102, max cpu: 42.814667, count: 57942"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 227.4140625,
            "unit": "median mem",
            "extra": "avg mem: 226.93131163221412, max mem: 232.1640625, count: 57942"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.17182429279233, max cpu: 33.333336, count: 57942"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.05078125,
            "unit": "median mem",
            "extra": "avg mem: 158.99376228329623, max mem: 161.25, count: 57942"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 21371,
            "unit": "median block_count",
            "extra": "avg block_count: 19849.484122053087, max block_count: 21486.0, count: 57942"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.35295640468054, max segment_count: 95.0, count: 57942"
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
        "date": 1755725309634,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 21.37084736472876, max cpu: 42.985077, count: 57907"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 226.95703125,
            "unit": "median mem",
            "extra": "avg mem: 226.1433216056565, max mem: 231.2265625, count: 57907"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.081449747023605, max cpu: 33.23442, count: 57907"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.7421875,
            "unit": "median mem",
            "extra": "avg mem: 158.85206080709153, max mem: 159.93359375, count: 57907"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22392,
            "unit": "median block_count",
            "extra": "avg block_count: 20729.29866855475, max block_count: 23686.0, count: 57907"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.58177767800093, max segment_count: 98.0, count: 57907"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755725356966,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.218911768885796, max cpu: 51.014496, count: 57920"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.04296875,
            "unit": "median mem",
            "extra": "avg mem: 235.1811462065133, max mem: 240.7890625, count: 57920"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.26704603189405, max cpu: 33.168808, count: 57920"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.9140625,
            "unit": "median mem",
            "extra": "avg mem: 161.34936584135446, max mem: 163.671875, count: 57920"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22435,
            "unit": "median block_count",
            "extra": "avg block_count: 20885.423705110497, max block_count: 23735.0, count: 57920"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.66740331491712, max segment_count: 96.0, count: 57920"
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
        "date": 1755725717685,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 22.284980025947643, max cpu: 46.198265, count: 57918"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.76171875,
            "unit": "median mem",
            "extra": "avg mem: 234.912577169986, max mem: 241.828125, count: 57918"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.164569649440132, max cpu: 33.20158, count: 57918"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 162.2578125,
            "unit": "median mem",
            "extra": "avg mem: 161.59178291399047, max mem: 164.375, count: 57918"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22333,
            "unit": "median block_count",
            "extra": "avg block_count: 20735.324527780656, max block_count: 23580.0, count: 57918"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.43093684174177, max segment_count: 95.0, count: 57918"
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
        "date": 1755726829838,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 21.54306344496765, max cpu: 42.772278, count: 57747"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 228.57421875,
            "unit": "median mem",
            "extra": "avg mem: 227.4350219221345, max mem: 231.64453125, count: 57747"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.353333667071077, max cpu: 33.432835, count: 57747"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.37109375,
            "unit": "median mem",
            "extra": "avg mem: 159.3239910596005, max mem: 160.4375, count: 57747"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22691,
            "unit": "median block_count",
            "extra": "avg block_count: 20948.924498242333, max block_count: 23887.0, count: 57747"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.88875612585935, max segment_count: 99.0, count: 57747"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755795577576,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.231754928937104, max cpu: 65.11628, count: 57880"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.4453125,
            "unit": "median mem",
            "extra": "avg mem: 235.30394181658173, max mem: 241.58203125, count: 57880"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.229046122090065, max cpu: 33.23442, count: 57880"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.81640625,
            "unit": "median mem",
            "extra": "avg mem: 160.61876417264168, max mem: 162.1171875, count: 57880"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22265,
            "unit": "median block_count",
            "extra": "avg block_count: 20778.52303040774, max block_count: 23672.0, count: 57880"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.50722183828611, max segment_count: 95.0, count: 57880"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755797787793,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.19870326034441, max cpu: 47.61905, count: 57733"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 237.0703125,
            "unit": "median mem",
            "extra": "avg mem: 236.27487670883636, max mem: 243.12890625, count: 57733"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.164122560292522, max cpu: 33.103447, count: 57733"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.2265625,
            "unit": "median mem",
            "extra": "avg mem: 161.06626639551902, max mem: 162.69921875, count: 57733"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22074,
            "unit": "median block_count",
            "extra": "avg block_count: 20715.235186115395, max block_count: 23602.0, count: 57733"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.48817833821211, max segment_count: 95.0, count: 57733"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755805537485,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.041644535584457, max cpu: 50.623203, count: 57713"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 237.8984375,
            "unit": "median mem",
            "extra": "avg mem: 236.9529358230598, max mem: 243.4921875, count: 57713"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.170203438398996, max cpu: 33.168808, count: 57713"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.2265625,
            "unit": "median mem",
            "extra": "avg mem: 160.24442093852338, max mem: 161.765625, count: 57713"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22466,
            "unit": "median block_count",
            "extra": "avg block_count: 20816.09127926117, max block_count: 23607.0, count: 57713"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.29800911406443, max segment_count: 96.0, count: 57713"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755812334144,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.099520737258167, max cpu: 47.43083, count: 57491"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 237.07421875,
            "unit": "median mem",
            "extra": "avg mem: 235.85208133120835, max mem: 242.51953125, count: 57491"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.303162175156494, max cpu: 33.267326, count: 57491"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.0390625,
            "unit": "median mem",
            "extra": "avg mem: 160.50896709648032, max mem: 162.58203125, count: 57491"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22107,
            "unit": "median block_count",
            "extra": "avg block_count: 20777.328973230593, max block_count: 23701.0, count: 57491"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.73601085387278, max segment_count: 97.0, count: 57491"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755813661238,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.405347098981984, max cpu: 65.30612, count: 57686"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.6796875,
            "unit": "median mem",
            "extra": "avg mem: 234.63195366130864, max mem: 240.37890625, count: 57686"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.333545503333497, max cpu: 33.136093, count: 57686"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.97265625,
            "unit": "median mem",
            "extra": "avg mem: 159.9890391245276, max mem: 161.6015625, count: 57686"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22440,
            "unit": "median block_count",
            "extra": "avg block_count: 20832.477810907327, max block_count: 23624.0, count: 57686"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.58287972818361, max segment_count: 96.0, count: 57686"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755815441847,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.034480048489474, max cpu: 55.976677, count: 57465"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 237.140625,
            "unit": "median mem",
            "extra": "avg mem: 236.1066821244453, max mem: 243.8671875, count: 57465"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.323614,
            "unit": "median cpu",
            "extra": "avg cpu: 22.339480656455105, max cpu: 33.432835, count: 57465"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.22265625,
            "unit": "median mem",
            "extra": "avg mem: 160.98299032563298, max mem: 162.65625, count: 57465"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22550,
            "unit": "median block_count",
            "extra": "avg block_count: 20899.51128513008, max block_count: 23764.0, count: 57465"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.94031149395285, max segment_count: 95.0, count: 57465"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755835053438,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.016054059759195, max cpu: 42.985077, count: 57754"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 237.26171875,
            "unit": "median mem",
            "extra": "avg mem: 236.18887422840842, max mem: 242.875, count: 57754"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.307242580239638, max cpu: 33.267326, count: 57754"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.1953125,
            "unit": "median mem",
            "extra": "avg mem: 160.3087786668456, max mem: 162.5625, count: 57754"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22472,
            "unit": "median block_count",
            "extra": "avg block_count: 20842.14452678602, max block_count: 23736.0, count: 57754"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.84555182325033, max segment_count: 96.0, count: 57754"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755835358165,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 22.11732743267086, max cpu: 47.38401, count: 57186"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.265625,
            "unit": "median mem",
            "extra": "avg mem: 234.6369654094752, max mem: 242.0625, count: 57186"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.245575839086126, max cpu: 33.23442, count: 57186"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 159.27734375,
            "unit": "median mem",
            "extra": "avg mem: 159.34331897776553, max mem: 160.4375, count: 57186"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22652,
            "unit": "median block_count",
            "extra": "avg block_count: 20912.312681425523, max block_count: 23791.0, count: 57186"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.80705067673907, max segment_count: 98.0, count: 57186"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755874485208,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.04206592719476, max cpu: 46.966736, count: 57735"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 236.12109375,
            "unit": "median mem",
            "extra": "avg mem: 235.27101763445052, max mem: 242.54296875, count: 57735"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.24929588337496, max cpu: 33.20158, count: 57735"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.06640625,
            "unit": "median mem",
            "extra": "avg mem: 160.7694593969213, max mem: 162.8671875, count: 57735"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22434,
            "unit": "median block_count",
            "extra": "avg block_count: 20851.509829392915, max block_count: 23701.0, count: 57735"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.69851909586906, max segment_count: 97.0, count: 57735"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755888079276,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.245455743929845, max cpu: 57.086224, count: 57890"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.9375,
            "unit": "median mem",
            "extra": "avg mem: 235.13470158922092, max mem: 241.25, count: 57890"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.17511596270286, max cpu: 33.300297, count: 57890"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 158.9765625,
            "unit": "median mem",
            "extra": "avg mem: 158.73184393353773, max mem: 160.421875, count: 57890"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 22151,
            "unit": "median block_count",
            "extra": "avg block_count: 20648.298721713596, max block_count: 23686.0, count: 57890"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 67,
            "unit": "median segment_count",
            "extra": "avg segment_count: 68.48201761962342, max segment_count: 97.0, count: 57890"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755891425837,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 19.661939170610555, max cpu: 42.899704, count: 57524"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 233.53515625,
            "unit": "median mem",
            "extra": "avg mem: 232.68117186685123, max mem: 237.8203125, count: 57524"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.498464217601754, max cpu: 33.103447, count: 57524"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.10546875,
            "unit": "median mem",
            "extra": "avg mem: 160.062798041361, max mem: 163.328125, count: 57524"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 24416,
            "unit": "median block_count",
            "extra": "avg block_count: 22492.318041165425, max block_count: 26405.0, count: 57524"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 38,
            "unit": "median segment_count",
            "extra": "avg segment_count: 38.09437799874835, max segment_count: 53.0, count: 57524"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755892531431,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.991098,
            "unit": "median cpu",
            "extra": "avg cpu: 19.869592779020646, max cpu: 47.19764, count: 57757"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 232.25390625,
            "unit": "median mem",
            "extra": "avg mem: 231.0563492303963, max mem: 235.40234375, count: 57757"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.255816,
            "unit": "median cpu",
            "extra": "avg cpu: 22.262423024167468, max cpu: 33.20158, count: 57757"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.02734375,
            "unit": "median mem",
            "extra": "avg mem: 160.04122849827726, max mem: 161.8515625, count: 57757"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23351,
            "unit": "median block_count",
            "extra": "avg block_count: 21842.117821216474, max block_count: 25320.0, count: 57757"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 37,
            "unit": "median segment_count",
            "extra": "avg segment_count: 37.59852485413023, max segment_count: 54.0, count: 57757"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "0cad75ef84c3942edd137497c4b6864b384a3655",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3037)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests",
          "timestamp": "2025-08-24T13:29:07-04:00",
          "tree_id": "d76d1a8d15e8ca4d61db683033627e7bf4ec476d",
          "url": "https://github.com/paradedb/paradedb/commit/0cad75ef84c3942edd137497c4b6864b384a3655"
        },
        "date": 1756058203661,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.121387,
            "unit": "median cpu",
            "extra": "avg cpu: 20.60518136822052, max cpu: 42.899704, count: 57867"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 235.05078125,
            "unit": "median mem",
            "extra": "avg mem: 231.61742788149982, max mem: 240.2578125, count: 57867"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 22.31298669023521, max cpu: 33.333336, count: 57867"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 161.06640625,
            "unit": "median mem",
            "extra": "avg mem: 160.49562088496035, max mem: 162.54296875, count: 57867"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 25479,
            "unit": "median block_count",
            "extra": "avg block_count: 23268.020806331762, max block_count: 26375.0, count: 57867"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.691309381858396, max segment_count: 71.0, count: 57867"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756058886982,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.010548,
            "unit": "median cpu",
            "extra": "avg cpu: 20.10252118135072, max cpu: 42.899704, count: 57807"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 232.36328125,
            "unit": "median mem",
            "extra": "avg mem: 229.70637409288668, max mem: 239.65234375, count: 57807"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.30097,
            "unit": "median cpu",
            "extra": "avg cpu: 22.300623684158435, max cpu: 33.3996, count: 57807"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.91796875,
            "unit": "median mem",
            "extra": "avg mem: 160.82079551892505, max mem: 163.39453125, count: 57807"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23332,
            "unit": "median block_count",
            "extra": "avg block_count: 21726.916013631566, max block_count: 25126.0, count: 57807"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 48.67221962738077, max segment_count: 71.0, count: 57807"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756059233960,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 23.054754,
            "unit": "median cpu",
            "extra": "avg cpu: 20.52370158410105, max cpu: 42.72997, count: 57917"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 230.87890625,
            "unit": "median mem",
            "extra": "avg mem: 228.31202234987137, max mem: 233.51953125, count: 57917"
          },
          {
            "name": "Count Query - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 22.18117471693188, max cpu: 33.23442, count: 57917"
          },
          {
            "name": "Count Query - Primary - mem",
            "value": 160.01953125,
            "unit": "median mem",
            "extra": "avg mem: 159.83655773024327, max mem: 161.7890625, count: 57917"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 23688,
            "unit": "median block_count",
            "extra": "avg block_count: 22034.631196367216, max block_count: 25324.0, count: 57917"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 50,
            "unit": "median segment_count",
            "extra": "avg segment_count: 49.155394789094736, max segment_count: 71.0, count: 57917"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - TPS": [
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
        "date": 1755716686075,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.47798649638836,
            "unit": "median tps",
            "extra": "avg tps: 28.228372308112576, max tps: 28.603561939181894, count: 57683"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 122.41983291779356,
            "unit": "median tps",
            "extra": "avg tps: 121.78283495851011, max tps: 124.0763733444903, count: 57683"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755716811710,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 29.310844705620088,
            "unit": "median tps",
            "extra": "avg tps: 29.15897198316339, max tps: 29.45261508816826, count: 57611"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 142.278591044247,
            "unit": "median tps",
            "extra": "avg tps: 141.25687421479807, max tps: 144.97181674347308, count: 57611"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755716844710,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.36828747735019,
            "unit": "median tps",
            "extra": "avg tps: 27.402531391546894, max tps: 30.131720435964127, count: 56618"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 131.93315904069212,
            "unit": "median tps",
            "extra": "avg tps: 131.5839916147656, max tps: 141.53827636008072, count: 56618"
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
        "date": 1755725960262,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 29.1524880649853,
            "unit": "median tps",
            "extra": "avg tps: 28.88060258721591, max tps: 29.309166949872175, count: 57552"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 138.31465973590366,
            "unit": "median tps",
            "extra": "avg tps: 137.73380767050494, max tps: 140.199756237339, count: 57552"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755726008274,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.65277117932587,
            "unit": "median tps",
            "extra": "avg tps: 27.525939512813824, max tps: 27.807969364587233, count: 57901"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 121.87635268790595,
            "unit": "median tps",
            "extra": "avg tps: 121.08808709076678, max tps: 123.68287900340925, count: 57901"
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
        "date": 1755726369201,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.605926938859938,
            "unit": "median tps",
            "extra": "avg tps: 27.51511685225382, max tps: 27.796150463475772, count: 57911"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 125.63255786603655,
            "unit": "median tps",
            "extra": "avg tps: 124.30101917813028, max tps: 127.42199761021526, count: 57911"
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
        "date": 1755727481049,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.06820658131072,
            "unit": "median tps",
            "extra": "avg tps: 27.078805535396377, max tps: 27.724941843183146, count: 56104"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 130.16978737667642,
            "unit": "median tps",
            "extra": "avg tps: 129.64204277392818, max tps: 137.67630363567616, count: 56104"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755796230455,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.914330088704904,
            "unit": "median tps",
            "extra": "avg tps: 27.77285738772965, max tps: 28.090710330444978, count: 57490"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 125.72014156602968,
            "unit": "median tps",
            "extra": "avg tps: 124.838775510516, max tps: 127.78227829890407, count: 57490"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755798442308,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.751970543497457,
            "unit": "median tps",
            "extra": "avg tps: 27.59256961049825, max tps: 27.8750426412571, count: 57950"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 125.36456912011604,
            "unit": "median tps",
            "extra": "avg tps: 124.60452251155385, max tps: 127.20675589559362, count: 57950"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755806191494,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 25.62718114148919,
            "unit": "median tps",
            "extra": "avg tps: 25.637460767297092, max tps: 26.10059515803176, count: 56374"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 112.48527316617279,
            "unit": "median tps",
            "extra": "avg tps: 112.30971693449584, max tps: 116.56825229991374, count: 56374"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755812992961,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.9210612058077,
            "unit": "median tps",
            "extra": "avg tps: 27.803490539484862, max tps: 28.123781872539574, count: 57893"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 127.15367848468138,
            "unit": "median tps",
            "extra": "avg tps: 126.17567665894045, max tps: 129.08489386483387, count: 57893"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755814317063,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.619673863515462,
            "unit": "median tps",
            "extra": "avg tps: 28.49081606306439, max tps: 28.784795387112712, count: 57684"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 123.96564836493808,
            "unit": "median tps",
            "extra": "avg tps: 123.44596352825539, max tps: 126.32218546027934, count: 57684"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755816097544,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.9436151942169,
            "unit": "median tps",
            "extra": "avg tps: 27.88375425333968, max tps: 28.181820982158705, count: 57464"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 124.25021067120949,
            "unit": "median tps",
            "extra": "avg tps: 123.56730289600678, max tps: 126.17359771359985, count: 57464"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755835706848,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 25.752467788216798,
            "unit": "median tps",
            "extra": "avg tps: 25.693557835615987, max tps: 25.960941905089875, count: 56401"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 114.32151162372178,
            "unit": "median tps",
            "extra": "avg tps: 113.86465307069838, max tps: 118.2122881486496, count: 56401"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755836015446,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 27.823085509435614,
            "unit": "median tps",
            "extra": "avg tps: 27.707246536370008, max tps: 28.021036390280727, count: 57478"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 124.02018036986799,
            "unit": "median tps",
            "extra": "avg tps: 123.3486993263159, max tps: 126.15408212910008, count: 57478"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755875145865,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 25.805338368965753,
            "unit": "median tps",
            "extra": "avg tps: 25.80049308633475, max tps: 26.223755060605296, count: 56334"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 119.35594005286413,
            "unit": "median tps",
            "extra": "avg tps: 118.76549921801025, max tps: 120.69977704137285, count: 56334"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755888741323,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.601555167559386,
            "unit": "median tps",
            "extra": "avg tps: 28.377896764544943, max tps: 28.707820898469627, count: 57710"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 120.42065172850022,
            "unit": "median tps",
            "extra": "avg tps: 119.8796351502216, max tps: 121.9327453728469, count: 57710"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755892085784,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 29.845226071746392,
            "unit": "median tps",
            "extra": "avg tps: 29.773872825684155, max tps: 30.124201135648036, count: 57911"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 157.9882824128506,
            "unit": "median tps",
            "extra": "avg tps: 157.28077785633104, max tps: 159.88683765811697, count: 57911"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755893195269,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 30.03997078630587,
            "unit": "median tps",
            "extra": "avg tps: 29.860781300093084, max tps: 30.31027173816866, count: 57785"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 155.83451237742153,
            "unit": "median tps",
            "extra": "avg tps: 155.11473170611237, max tps: 158.6554674013907, count: 57785"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756059550896,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 28.96357850701081,
            "unit": "median tps",
            "extra": "avg tps: 28.873496638355483, max tps: 29.242707008014694, count: 58006"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 152.8361356970769,
            "unit": "median tps",
            "extra": "avg tps: 152.06425933162836, max tps: 155.35744728717785, count: 58006"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756059901635,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - tps",
            "value": 29.831186207259034,
            "unit": "median tps",
            "extra": "avg tps: 29.74027749499618, max tps: 30.110756622711456, count: 57815"
          },
          {
            "name": "Single Update - Primary - tps",
            "value": 151.10806278493897,
            "unit": "median tps",
            "extra": "avg tps: 150.17614688167515, max tps: 153.1682632865189, count: 57815"
          }
        ]
      }
    ],
    "pg_search wide-table.toml Performance - Other Metrics": [
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
        "date": 1755716688816,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.608533923891983, max cpu: 101.53847, count: 57683"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.39453125,
            "unit": "median mem",
            "extra": "avg mem: 172.98578362830037, max mem: 180.23046875, count: 57683"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18255,
            "unit": "median block_count",
            "extra": "avg block_count: 16728.99977463031, max block_count: 21557.0, count: 57683"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.08966246554444, max segment_count: 117.0, count: 57683"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.792525672820176, max cpu: 171.26326, count: 57683"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.953125,
            "unit": "median mem",
            "extra": "avg mem: 156.4320274035461, max mem: 174.7890625, count: 57683"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755716814461,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 20.245597187303883, max cpu: 47.43083, count: 57611"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.6953125,
            "unit": "median mem",
            "extra": "avg mem: 173.8543355435594, max mem: 178.59375, count: 57611"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18424,
            "unit": "median block_count",
            "extra": "avg block_count: 17043.88104702227, max block_count: 22537.0, count: 57611"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 43.079724358195485, max segment_count: 119.0, count: 57611"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.384164,
            "unit": "median cpu",
            "extra": "avg cpu: 11.341203239543852, max cpu: 37.426903, count: 57611"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.28515625,
            "unit": "median mem",
            "extra": "avg mem: 151.8968074944455, max mem: 169.765625, count: 57611"
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
          "id": "309944e7eb5d08d60af4a4b78822d7da10f12323",
          "message": "chore: Upgrade to `0.16.5` (#2928)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-03T18:49:51Z",
          "url": "https://github.com/paradedb/paradedb/commit/309944e7eb5d08d60af4a4b78822d7da10f12323"
        },
        "date": 1755716847451,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.730506195329546, max cpu: 60.523766, count: 56618"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.0390625,
            "unit": "median mem",
            "extra": "avg mem: 171.1407422882299, max mem: 176.578125, count: 56618"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 16962,
            "unit": "median block_count",
            "extra": "avg block_count: 15356.353491822389, max block_count: 16962.0, count: 56618"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 82,
            "unit": "median segment_count",
            "extra": "avg segment_count: 83.79557737821894, max segment_count: 173.0, count: 56618"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.218379585551643, max cpu: 36.994217, count: 56618"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 161.3203125,
            "unit": "median mem",
            "extra": "avg mem: 151.71469686374033, max mem: 168.57421875, count: 56618"
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
        "date": 1755725962951,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 20.200463266256484, max cpu: 49.72111, count: 57552"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.671875,
            "unit": "median mem",
            "extra": "avg mem: 169.92523276504292, max mem: 177.36328125, count: 57552"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18607,
            "unit": "median block_count",
            "extra": "avg block_count: 17084.51417848207, max block_count: 22705.0, count: 57552"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.327946900194604, max segment_count: 116.0, count: 57552"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.448819,
            "unit": "median cpu",
            "extra": "avg cpu: 11.601608477240424, max cpu: 37.28155, count: 57552"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.03515625,
            "unit": "median mem",
            "extra": "avg mem: 156.42472290005125, max mem: 177.5625, count: 57552"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a",
          "message": "chore: Upgrade to `0.17.5` (#3005)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-20T13:19:49-04:00",
          "tree_id": "26f07dc326d4e53d8df249d9268a911b9d59d86b",
          "url": "https://github.com/paradedb/paradedb/commit/8cd6a2c7cdf969cf43bd66f12beca6ddedd6889a"
        },
        "date": 1755726011142,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.779343,
            "unit": "median cpu",
            "extra": "avg cpu: 20.946357666378937, max cpu: 124.61539, count: 57901"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.3671875,
            "unit": "median mem",
            "extra": "avg mem: 172.72370336328387, max mem: 178.7890625, count: 57901"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19108,
            "unit": "median block_count",
            "extra": "avg block_count: 17198.649211585293, max block_count: 21362.0, count: 57901"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.570870969413306, max segment_count: 122.0, count: 57901"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.435803937249748, max cpu: 157.83366, count: 57901"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.515625,
            "unit": "median mem",
            "extra": "avg mem: 158.260034099152, max mem: 176.32421875, count: 57901"
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
        "date": 1755726372018,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.786694,
            "unit": "median cpu",
            "extra": "avg cpu: 20.986929191245167, max cpu: 47.66634, count: 57911"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.59375,
            "unit": "median mem",
            "extra": "avg mem: 174.5593554926957, max mem: 180.859375, count: 57911"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18696,
            "unit": "median block_count",
            "extra": "avg block_count: 16969.1119476438, max block_count: 21706.0, count: 57911"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.31988741344477, max segment_count: 118.0, count: 57911"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.159468347197958, max cpu: 166.15385, count: 57911"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 169.45703125,
            "unit": "median mem",
            "extra": "avg mem: 158.74774998219252, max mem: 177.35546875, count: 57911"
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
        "date": 1755727483704,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 20.876677091331103, max cpu: 55.54484, count: 56104"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.41015625,
            "unit": "median mem",
            "extra": "avg mem: 168.66011538003085, max mem: 172.14453125, count: 56104"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19717,
            "unit": "median block_count",
            "extra": "avg block_count: 17824.590938257523, max block_count: 22223.0, count: 56104"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.834058177670045, max segment_count: 139.0, count: 56104"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 12.564867272659752, max cpu: 37.573387, count: 56104"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 162.11328125,
            "unit": "median mem",
            "extra": "avg mem: 153.68710014270374, max mem: 169.0078125, count: 56104"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755796232996,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 20.834232540489097, max cpu: 88.115944, count: 57490"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.328125,
            "unit": "median mem",
            "extra": "avg mem: 171.75086740628805, max mem: 176.7109375, count: 57490"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18816,
            "unit": "median block_count",
            "extra": "avg block_count: 17028.12830057401, max block_count: 21523.0, count: 57490"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.80172203861541, max segment_count: 115.0, count: 57490"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.264299057690092, max cpu: 139.80582, count: 57490"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.88671875,
            "unit": "median mem",
            "extra": "avg mem: 158.06920028211428, max mem: 175.9453125, count: 57490"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755798444969,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.768328,
            "unit": "median cpu",
            "extra": "avg cpu: 20.874412288161633, max cpu: 51.6129, count: 57950"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.3828125,
            "unit": "median mem",
            "extra": "avg mem: 173.1825626887403, max mem: 179.06640625, count: 57950"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19433,
            "unit": "median block_count",
            "extra": "avg block_count: 17425.868559102673, max block_count: 21883.0, count: 57950"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.05304572907679, max segment_count: 125.0, count: 57950"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.677044,
            "unit": "median cpu",
            "extra": "avg cpu: 19.391354590469533, max cpu: 174.88016, count: 57950"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.7421875,
            "unit": "median mem",
            "extra": "avg mem: 157.56318391393444, max mem: 177.12890625, count: 57950"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "2bb7c02ed7b398872104e7879aed251dd18a6414",
          "message": "fix: don't ERROR if `paradedb.terms_with_operator` is missing (#3013)\n\n\n## What\n\nIt's possible that when upgrading pg_search from `v0.15.26` to `v0.17.x`\nwe could generate an ERROR complaining that\n`paradedb.terms_with_operator(...)` doesn't exist if the user upgraded\nthe extension binary but has not yet run `ALTER EXTENSION pg_search\nUPDATE;`.\n\nThis fixes that by inspecting the catalogs directly when it's necessary\nto lookup that function and plumbing through an `Option<pg_sys::Oid>`\nreturn value.\n\n## Why\n\nTo help users/customers/partners better deal with upgrading from really\nold pg_search versions.\n\n## How\n\n## Tests\n\nExisting tests pass, especially the ones added in #2730, and a new\nregress test has been added that explicitly removes the function from\nthe schema and ensures the query still works (tho with a different plan,\nof course).",
          "timestamp": "2025-08-21T15:17:35-04:00",
          "tree_id": "4c5e053bc28da403546f6c1b9ad61b9b62a75bdf",
          "url": "https://github.com/paradedb/paradedb/commit/2bb7c02ed7b398872104e7879aed251dd18a6414"
        },
        "date": 1755806194114,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.897638,
            "unit": "median cpu",
            "extra": "avg cpu: 21.277215141388183, max cpu: 106.87318, count: 56374"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.86328125,
            "unit": "median mem",
            "extra": "avg mem: 171.0025393327376, max mem: 176.53515625, count: 56374"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17950,
            "unit": "median block_count",
            "extra": "avg block_count: 16300.251144144464, max block_count: 21247.0, count: 56374"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 39,
            "unit": "median segment_count",
            "extra": "avg segment_count: 40.73335225458545, max segment_count: 110.0, count: 56374"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 22.966507,
            "unit": "median cpu",
            "extra": "avg cpu: 21.332558583916704, max cpu: 184.61539, count: 56374"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 168.84765625,
            "unit": "median mem",
            "extra": "avg mem: 158.25303421402154, max mem: 176.80859375, count: 56374"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ori@eigenstate.org",
            "name": "Ori Bernstein",
            "username": "oridb"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "97ad25718b14ab34440c0587cb8bb9968598bf7d",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:11:27-04:00",
          "tree_id": "74eee4eb0e1503ed631c849c97ccf32653c4e203",
          "url": "https://github.com/paradedb/paradedb/commit/97ad25718b14ab34440c0587cb8bb9968598bf7d"
        },
        "date": 1755812995011,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.713451,
            "unit": "median cpu",
            "extra": "avg cpu: 20.916341767860903, max cpu: 97.391304, count: 57893"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 176.16015625,
            "unit": "median mem",
            "extra": "avg mem: 174.6017860400869, max mem: 180.69140625, count: 57893"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18833,
            "unit": "median block_count",
            "extra": "avg block_count: 17060.453940890955, max block_count: 21347.0, count: 57893"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.087782633479, max segment_count: 115.0, count: 57893"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 19.35328746684494, max cpu: 161.53847, count: 57893"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.09765625,
            "unit": "median mem",
            "extra": "avg mem: 155.77951707514293, max mem: 174.23046875, count: 57893"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7f2428be0ee6a082b53176d781e62ee2519ccccc",
          "message": "fix: fsm: we have space if any slots are invalid, no need for all (#3015) (#3016)\n\n## What\n\nWhen walking the free space map looking for blocks with open slots, we\nwant to pick a block with any empty slots, not all open slots, so that\nwe don't end up with a long, low occupancy list.\n\n## Why\n\nPerformance.\n\n## How\n\nAdd a function to query whether any slots in the fsm block are empty,\nand then use it.\n\n## Tests\n\nRan unit tests.\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests\n\nCo-authored-by: Ori Bernstein <ori@eigenstate.org>\nCo-authored-by: Ori Bernstein <ori@paradedb.com>",
          "timestamp": "2025-08-21T17:33:35-04:00",
          "tree_id": "b3df08f37cb0652f7b23674356d6a786d632a136",
          "url": "https://github.com/paradedb/paradedb/commit/7f2428be0ee6a082b53176d781e62ee2519ccccc"
        },
        "date": 1755814319174,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 20.667697050803728, max cpu: 88.97562, count: 57684"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.53125,
            "unit": "median mem",
            "extra": "avg mem: 171.80926205977826, max mem: 178.55859375, count: 57684"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19085,
            "unit": "median block_count",
            "extra": "avg block_count: 17233.65760002774, max block_count: 21528.0, count: 57684"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 41,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.79708411344567, max segment_count: 121.0, count: 57684"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 19.89305525329344, max cpu: 162.16216, count: 57684"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.2421875,
            "unit": "median mem",
            "extra": "avg mem: 156.3475453277772, max mem: 174.9453125, count: 57684"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4f929495a112d17ce5992a41cbd1d3b05aa17cbb",
          "message": "chore: Upgrade to `0.17.6` (#3017)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-21T18:03:19-04:00",
          "tree_id": "f3d6f5fb6a9f1748954b75791999f0080ecf1fb5",
          "url": "https://github.com/paradedb/paradedb/commit/4f929495a112d17ce5992a41cbd1d3b05aa17cbb"
        },
        "date": 1755816099638,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.75,
            "unit": "median cpu",
            "extra": "avg cpu: 20.85337372878651, max cpu: 57.657658, count: 57464"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.4765625,
            "unit": "median mem",
            "extra": "avg mem: 174.02873925686083, max mem: 179.75390625, count: 57464"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18937,
            "unit": "median block_count",
            "extra": "avg block_count: 17087.595851315607, max block_count: 21224.0, count: 57464"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40.5,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.45518933593206, max segment_count: 122.0, count: 57464"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.54253263874948, max cpu: 171.76016, count: 57464"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.474609375,
            "unit": "median mem",
            "extra": "avg mem: 156.56022778935508, max mem: 175.0625, count: 57464"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755835709046,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.879055,
            "unit": "median cpu",
            "extra": "avg cpu: 21.245632555659252, max cpu: 56.03113, count: 56401"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.25390625,
            "unit": "median mem",
            "extra": "avg mem: 171.56733292904823, max mem: 176.953125, count: 56401"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17721,
            "unit": "median block_count",
            "extra": "avg block_count: 16154.310012233826, max block_count: 20948.0, count: 56401"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 39,
            "unit": "median segment_count",
            "extra": "avg segment_count: 40.722256697576285, max segment_count: 108.0, count: 56401"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 22.922636,
            "unit": "median cpu",
            "extra": "avg cpu: 21.315465469610682, max cpu: 166.79536, count: 56401"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.3984375,
            "unit": "median mem",
            "extra": "avg mem: 155.58824928026542, max mem: 174.921875, count: 56401"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755836017557,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.87900809459248, max cpu: 74.05979, count: 57478"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 174.50390625,
            "unit": "median mem",
            "extra": "avg mem: 173.02437542679374, max mem: 178.484375, count: 57478"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18665,
            "unit": "median block_count",
            "extra": "avg block_count: 16902.924579839244, max block_count: 21168.0, count: 57478"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.413688715682525, max segment_count: 117.0, count: 57478"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.640776,
            "unit": "median cpu",
            "extra": "avg cpu: 19.582184132665056, max cpu: 172.25995, count: 57478"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.1171875,
            "unit": "median mem",
            "extra": "avg mem: 156.34653897469465, max mem: 174.8046875, count: 57478"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "5fbe8d2ee63335d61db9665e1eec916cc530f7a0",
          "message": "chore: Upgrade to `0.17.7` (#3024)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-22T10:27:16-04:00",
          "tree_id": "d8a53236b167b06e2b4e1beaeaf72fb424454256",
          "url": "https://github.com/paradedb/paradedb/commit/5fbe8d2ee63335d61db9665e1eec916cc530f7a0"
        },
        "date": 1755875148142,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.916256,
            "unit": "median cpu",
            "extra": "avg cpu: 21.26245671983156, max cpu: 102.52427, count: 56334"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 173.43359375,
            "unit": "median mem",
            "extra": "avg mem: 172.23893409452995, max mem: 178.421875, count: 56334"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 17648,
            "unit": "median block_count",
            "extra": "avg block_count: 16173.668601555011, max block_count: 21020.0, count: 56334"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 41.29935740405439, max segment_count: 110.0, count: 56334"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.934912,
            "unit": "median cpu",
            "extra": "avg cpu: 21.148021573147453, max cpu: 148.98157, count: 56334"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 169.51953125,
            "unit": "median mem",
            "extra": "avg mem: 158.25033560993361, max mem: 177.59765625, count: 56334"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755888743631,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.731707,
            "unit": "median cpu",
            "extra": "avg cpu: 20.578128790380923, max cpu: 107.91789, count: 57710"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 175.5703125,
            "unit": "median mem",
            "extra": "avg mem: 174.9033572360726, max mem: 182.8046875, count: 57710"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18792,
            "unit": "median block_count",
            "extra": "avg block_count: 17030.820516374977, max block_count: 21511.0, count: 57710"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 40,
            "unit": "median segment_count",
            "extra": "avg segment_count: 42.08840755501646, max segment_count: 113.0, count: 57710"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 19.9401921439003, max cpu: 157.68115, count: 57710"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 166.6953125,
            "unit": "median mem",
            "extra": "avg mem: 157.3265480148588, max mem: 175.328125, count: 57710"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755892088890,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 20.369457964669223, max cpu: 47.151276, count: 57911"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 172.05078125,
            "unit": "median mem",
            "extra": "avg mem: 170.45196238624786, max mem: 174.93359375, count: 57911"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19435,
            "unit": "median block_count",
            "extra": "avg block_count: 17908.333321821414, max block_count: 24595.0, count: 57911"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 38,
            "unit": "median segment_count",
            "extra": "avg segment_count: 37.660686225414864, max segment_count: 83.0, count: 57911"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 10.777405502542925, max cpu: 33.005894, count: 57911"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 169.890625,
            "unit": "median mem",
            "extra": "avg mem: 159.34976515687865, max mem: 179.546875, count: 57911"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "8386fa71d8c2979fb63993f4045d140aff8768fe",
          "message": "perf: optimize merging heuristics to prefer background merging (#3032)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-22T12:27:56-07:00",
          "tree_id": "fa86b284fe7c52c4cf1226cd910b7b9f1c0507d2",
          "url": "https://github.com/paradedb/paradedb/commit/8386fa71d8c2979fb63993f4045d140aff8768fe"
        },
        "date": 1755893197449,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 20.24578380371751, max cpu: 46.60194, count: 57785"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 171.98046875,
            "unit": "median mem",
            "extra": "avg mem: 170.37683871246864, max mem: 176.0859375, count: 57785"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18831,
            "unit": "median block_count",
            "extra": "avg block_count: 17897.816388336072, max block_count: 24866.0, count: 57785"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 38,
            "unit": "median segment_count",
            "extra": "avg segment_count: 37.70466384009691, max segment_count: 78.0, count: 57785"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 11.211352317045256, max cpu: 37.907207, count: 57785"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 167.8359375,
            "unit": "median mem",
            "extra": "avg mem: 158.7233802430345, max mem: 180.42578125, count: 57785"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "ming.ying.nyc@gmail.com",
            "name": "Ming",
            "username": "rebasedming"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "6832efa7662fee49675d88dd2ceecb31dfffdd45",
          "message": "chore: Upgrade to `0.17.9` (#3041)",
          "timestamp": "2025-08-24T13:40:28-04:00",
          "tree_id": "4b3ff40f76aaaa9a2242a4bee88214604e92587a",
          "url": "https://github.com/paradedb/paradedb/commit/6832efa7662fee49675d88dd2ceecb31dfffdd45"
        },
        "date": 1756059553069,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 20.541934383510064, max cpu: 67.46988, count: 58006"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 170.453125,
            "unit": "median mem",
            "extra": "avg mem: 168.5941158026756, max mem: 172.73046875, count: 58006"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 18896,
            "unit": "median block_count",
            "extra": "avg block_count: 17485.880874392304, max block_count: 23633.0, count: 58006"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 35,
            "unit": "median segment_count",
            "extra": "avg segment_count: 38.073009688652895, max segment_count: 110.0, count: 58006"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.329447,
            "unit": "median cpu",
            "extra": "avg cpu: 11.091927233411086, max cpu: 36.78161, count: 58006"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 165.875,
            "unit": "median mem",
            "extra": "avg mem: 156.13785319622107, max mem: 175.71875, count: 58006"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "21990816+philippemnoel@users.noreply.github.com",
            "name": "Philippe Noël",
            "username": "philippemnoel"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "4b08446356edfe8faed3798c0521304b8789d3e5",
          "message": "fix: make IndexLayerSizes calculations more accurate (#3040)\n\n## What\n\nThe `From<&PgSearchRelation> for IndexLayerSizes` `from()` ctor was\nimproperly calculating both the total `index_byte_size` and the\n`target_segment_byte_size`.\n\nFor the former, it needs to only consider SegmentMetaEntries that are\nactually visible. In almost every case, at least after a CREATE INDEX of\na large index, the index could actually have 2x+ total segments as there\nare actually visible segments.\n\nSecondly, we need to adjust the `target_segment_byte_size` down by 1/3\nto account for how `LayeredMergePolicy` does the same thing. Failure to\ndo this can cause the background worker to merge segments together that\ndon't actually need to be.\n\n## Why\n\nThese two miscalculations are what contributed to the recent query\nbenchmark regressions we saw after #3031 was merged.\n\n## How\n\n## Tests\n\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>",
          "timestamp": "2025-08-24T13:46:13-04:00",
          "tree_id": "1335b21c3132607da548eff65e5a684aa86ebecf",
          "url": "https://github.com/paradedb/paradedb/commit/4b08446356edfe8faed3798c0521304b8789d3e5"
        },
        "date": 1756059903773,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Bulk Update - Primary - cpu",
            "value": 18.695229,
            "unit": "median cpu",
            "extra": "avg cpu: 20.321867986001525, max cpu: 55.813957, count: 57815"
          },
          {
            "name": "Bulk Update - Primary - mem",
            "value": 169.33984375,
            "unit": "median mem",
            "extra": "avg mem: 167.7000993200294, max mem: 171.74609375, count: 57815"
          },
          {
            "name": "Monitor Index Size - Primary - block_count",
            "value": 19219,
            "unit": "median block_count",
            "extra": "avg block_count: 17716.72282279685, max block_count: 23783.0, count: 57815"
          },
          {
            "name": "Monitor Index Size - Primary - segment_count",
            "value": 36,
            "unit": "median segment_count",
            "extra": "avg segment_count: 38.29142955980282, max segment_count: 102.0, count: 57815"
          },
          {
            "name": "Single Update - Primary - cpu",
            "value": 9.356726,
            "unit": "median cpu",
            "extra": "avg cpu: 11.308228710152761, max cpu: 41.901066, count: 57815"
          },
          {
            "name": "Single Update - Primary - mem",
            "value": 170.296875,
            "unit": "median mem",
            "extra": "avg mem: 158.8315215423117, max mem: 179.31640625, count: 57815"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - TPS": [
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
        "date": 1755717334889,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.28947427224411,
            "unit": "median tps",
            "extra": "avg tps: 36.608473136456524, max tps: 38.08890291277521, count: 55662"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 261.3398652030008,
            "unit": "median tps",
            "extra": "avg tps: 300.1069684460285, max tps: 2514.2766785707904, count: 55662"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 117.78034924175007,
            "unit": "median tps",
            "extra": "avg tps: 117.93236808364414, max tps: 136.4354036757948, count: 55662"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 66.3431770657506,
            "unit": "median tps",
            "extra": "avg tps: 61.842086830854726, max tps: 99.7781783858007, count: 111324"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.856713639862562,
            "unit": "median tps",
            "extra": "avg tps: 16.881873775753494, max tps: 18.520684752683295, count: 55662"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755717485697,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 5.184261383528083,
            "unit": "median tps",
            "extra": "avg tps: 19.533561348710574, max tps: 605.0007913410351, count: 55419"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 157.3911713164023,
            "unit": "median tps",
            "extra": "avg tps: 154.19016396267634, max tps: 160.86508559138392, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1959.0284332389115,
            "unit": "median tps",
            "extra": "avg tps: 1948.3051795032209, max tps: 1987.2217585809615, count: 55419"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 3.315667044685341,
            "unit": "median tps",
            "extra": "avg tps: 8.74885366511506, max tps: 79.14797756190453, count: 166257"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.3148368099816434,
            "unit": "median tps",
            "extra": "avg tps: 0.6467406041314335, max tps: 4.553118345383784, count: 55419"
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
        "date": 1755726636139,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 263.3277849804827,
            "unit": "median tps",
            "extra": "avg tps: 258.0076716738847, max tps: 604.4180742266462, count: 55421"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 157.5710673413365,
            "unit": "median tps",
            "extra": "avg tps: 152.8721966892048, max tps: 162.96444221942065, count: 55421"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1987.4201190509352,
            "unit": "median tps",
            "extra": "avg tps: 1949.1446103854707, max tps: 2076.284880568145, count: 55421"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 18.82033166444295,
            "unit": "median tps",
            "extra": "avg tps: 22.747706170916047, max tps: 71.5806309482768, count: 166263"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.2941229697597172,
            "unit": "median tps",
            "extra": "avg tps: 0.5695409641449162, max tps: 4.755439143914976, count: 55421"
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
        "date": 1755727027208,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.331521060022936,
            "unit": "median tps",
            "extra": "avg tps: 36.19278080826215, max tps: 37.93155594822544, count: 55589"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 257.75364673533227,
            "unit": "median tps",
            "extra": "avg tps: 295.9258763760244, max tps: 2507.0351698186573, count: 55589"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 117.78087504535272,
            "unit": "median tps",
            "extra": "avg tps: 117.92662377140532, max tps: 130.49698596514907, count: 55589"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 73.29904338082919,
            "unit": "median tps",
            "extra": "avg tps: 65.38934976816272, max tps: 104.14948548139961, count: 111178"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.649137119503973,
            "unit": "median tps",
            "extra": "avg tps: 15.828507014210839, max tps: 20.56346146391899, count: 55589"
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
        "date": 1755728162135,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - tps",
            "value": 4.931276016159726,
            "unit": "median tps",
            "extra": "avg tps: 18.924921871200674, max tps: 599.3305628052716, count: 55521"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 124.1012896544035,
            "unit": "median tps",
            "extra": "avg tps: 124.22389626298568, max tps: 133.79153802426728, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - tps",
            "value": 1742.8239330277252,
            "unit": "median tps",
            "extra": "avg tps: 1732.1405919793835, max tps: 1882.319251836727, count: 55521"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 3.2533857987154478,
            "unit": "median tps",
            "extra": "avg tps: 8.61114300970805, max tps: 74.20474521716052, count: 166563"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 0.3052511907782593,
            "unit": "median tps",
            "extra": "avg tps: 0.6349216648576634, max tps: 4.695055690242096, count: 55521"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755796888261,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 34.11576004581176,
            "unit": "median tps",
            "extra": "avg tps: 34.35852236039567, max tps: 36.13738812469355, count: 55616"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 252.27815268977758,
            "unit": "median tps",
            "extra": "avg tps: 283.4026083146275, max tps: 2442.9968306980495, count: 55616"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 129.00891024272158,
            "unit": "median tps",
            "extra": "avg tps: 127.95354528855977, max tps: 129.23869691217422, count: 55616"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 69.31321114505909,
            "unit": "median tps",
            "extra": "avg tps: 64.69598773951684, max tps: 106.83679980263165, count: 111232"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.994418576601404,
            "unit": "median tps",
            "extra": "avg tps: 16.32806258010327, max tps: 17.975016434782216, count: 55616"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755799100511,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 33.85994644854997,
            "unit": "median tps",
            "extra": "avg tps: 34.0635734489934, max tps: 36.90008351595902, count: 55609"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 261.0351727562691,
            "unit": "median tps",
            "extra": "avg tps: 300.54352735378876, max tps: 2544.273354984615, count: 55609"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 128.9352690013566,
            "unit": "median tps",
            "extra": "avg tps: 128.29975211978052, max tps: 129.27243912177678, count: 55609"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 69.99873505590342,
            "unit": "median tps",
            "extra": "avg tps: 65.10708107780643, max tps: 100.74379742400765, count: 111218"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.405288963627125,
            "unit": "median tps",
            "extra": "avg tps: 16.39208373813903, max tps: 20.02412145718976, count: 55609"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755836365616,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 34.21160935012933,
            "unit": "median tps",
            "extra": "avg tps: 34.74928934031507, max tps: 37.41445996525833, count: 55641"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 259.93713306770314,
            "unit": "median tps",
            "extra": "avg tps: 296.73814421569017, max tps: 2515.2013272823083, count: 55641"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 127.51077280261191,
            "unit": "median tps",
            "extra": "avg tps: 126.96677009119453, max tps: 127.95119230565243, count: 55641"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 70.43807006499993,
            "unit": "median tps",
            "extra": "avg tps: 65.79231432985486, max tps: 101.46283480800678, count: 111282"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.181602865774448,
            "unit": "median tps",
            "extra": "avg tps: 16.285367139693385, max tps: 18.183681182608435, count: 55641"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755836677066,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 37.117277068142286,
            "unit": "median tps",
            "extra": "avg tps: 37.406807120393374, max tps: 38.71385873556994, count: 55630"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 255.0932948959213,
            "unit": "median tps",
            "extra": "avg tps: 291.05789517377514, max tps: 2485.6743493178215, count: 55630"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 119.30494973895983,
            "unit": "median tps",
            "extra": "avg tps: 119.0115045911294, max tps: 124.2307430715724, count: 55630"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 65.93934770014886,
            "unit": "median tps",
            "extra": "avg tps: 62.0549995227974, max tps: 106.43794845090592, count: 111260"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 16.86339663695926,
            "unit": "median tps",
            "extra": "avg tps: 16.94173624106889, max tps: 20.412088218269236, count: 55630"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755889403882,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.59927110952076,
            "unit": "median tps",
            "extra": "avg tps: 36.534305708693076, max tps: 37.3244242964263, count: 55496"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 255.86980395612505,
            "unit": "median tps",
            "extra": "avg tps: 288.8880872047884, max tps: 2409.8606154040726, count: 55496"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 122.13961820263259,
            "unit": "median tps",
            "extra": "avg tps: 121.46552247506666, max tps: 125.61753882401037, count: 55496"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 67.43322041311897,
            "unit": "median tps",
            "extra": "avg tps: 63.255440752625816, max tps: 101.91021130581525, count: 110992"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 17.164112466054192,
            "unit": "median tps",
            "extra": "avg tps: 17.08633202100464, max tps: 19.468746009211266, count: 55496"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755892753849,
        "tool": "customBiggerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - tps",
            "value": 36.20140430843666,
            "unit": "median tps",
            "extra": "avg tps: 36.48588432577562, max tps: 37.96322219005072, count: 55532"
          },
          {
            "name": "Delete value - Primary - tps",
            "value": 255.89317790380696,
            "unit": "median tps",
            "extra": "avg tps: 288.0784993260113, max tps: 2428.5374990698033, count: 55532"
          },
          {
            "name": "Insert value - Primary - tps",
            "value": 151.75246157904013,
            "unit": "median tps",
            "extra": "avg tps: 151.62958733783327, max tps: 154.0939488131042, count: 55532"
          },
          {
            "name": "Update random values - Primary - tps",
            "value": 68.56893719435283,
            "unit": "median tps",
            "extra": "avg tps: 71.9208443035584, max tps: 129.40768076621055, count: 111064"
          },
          {
            "name": "Vacuum - Primary - tps",
            "value": 15.921470310186516,
            "unit": "median tps",
            "extra": "avg tps: 16.221796138576032, max tps: 35.00996707505151, count: 55532"
          }
        ]
      }
    ],
    "pg_search background-merge.toml Performance - Other Metrics": [
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
        "date": 1755717337746,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.898860047168508, max cpu: 41.458733, count: 55662"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.9140625,
            "unit": "median mem",
            "extra": "avg mem: 154.215805890913, max mem: 155.9140625, count: 55662"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.549145027935271, max cpu: 28.152493, count: 55662"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 145.64453125,
            "unit": "median mem",
            "extra": "avg mem: 141.15512280033954, max mem: 146.0234375, count: 55662"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.37756362778912, max cpu: 70.03892, count: 55662"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 156.125,
            "unit": "median mem",
            "extra": "avg mem: 130.07757712005048, max mem: 164.19921875, count: 55662"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 20610,
            "unit": "median block_count",
            "extra": "avg block_count: 20801.353149365816, max block_count: 41505.0, count: 55662"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.624277,
            "unit": "median cpu",
            "extra": "avg cpu: 4.393961969898859, max cpu: 4.678363, count: 55662"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 98.94140625,
            "unit": "median mem",
            "extra": "avg mem: 87.96878642240218, max mem: 128.63671875, count: 55662"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.249559843340162, max segment_count: 45.0, count: 55662"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.949552997490088, max cpu: 75.22037, count: 111324"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 161.22265625,
            "unit": "median mem",
            "extra": "avg mem: 146.77198463915013, max mem: 170.0859375, count: 111324"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.93998,
            "unit": "median cpu",
            "extra": "avg cpu: 14.038776097183273, max cpu: 28.070175, count: 55662"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 161.00390625,
            "unit": "median mem",
            "extra": "avg mem: 157.9711239125211, max mem: 162.66796875, count: 55662"
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
          "id": "885295995a921682849cc27e412c5c2c7ddf78c4",
          "message": "chore: upgrade to `0.17.3` (#2940)\n\n# Ticket(s) Closed\n\n- Closes #\n\n## What\n\n## Why\n\n## How\n\n## Tests",
          "timestamp": "2025-08-05T20:49:37Z",
          "url": "https://github.com/paradedb/paradedb/commit/885295995a921682849cc27e412c5c2c7ddf78c4"
        },
        "date": 1755717488936,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 23.233301,
            "unit": "median cpu",
            "extra": "avg cpu: 23.360471602753343, max cpu: 32.74854, count: 55419"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 204.7265625,
            "unit": "median mem",
            "extra": "avg mem: 203.0932820450793, max mem: 204.7265625, count: 55419"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 10.47693568186829, max cpu: 27.87996, count: 55419"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 165.12109375,
            "unit": "median mem",
            "extra": "avg mem: 160.77828586258775, max mem: 165.49609375, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 36176,
            "unit": "median block_count",
            "extra": "avg block_count: 37719.760948411196, max block_count: 51549.0, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 3.5986386535692145, max cpu: 4.655674, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 131.28125,
            "unit": "median mem",
            "extra": "avg mem: 116.0684077634701, max mem: 142.56640625, count: 55419"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.565943088110576, max segment_count: 63.0, count: 55419"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.188406,
            "unit": "median cpu",
            "extra": "avg cpu: 23.146062513084804, max cpu: 32.78049, count: 166257"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 220.53515625,
            "unit": "median mem",
            "extra": "avg mem: 223.8057640759111, max mem: 452.13671875, count: 166257"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 9.29332,
            "unit": "median cpu",
            "extra": "avg cpu: 13.154616711205062, max cpu: 32.589718, count: 55419"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 191.2265625,
            "unit": "median mem",
            "extra": "avg mem: 189.84395440868656, max mem: 250.1796875, count: 55419"
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
        "date": 1755726638766,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.495837918303824, max cpu: 32.589718, count: 55421"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.109375,
            "unit": "median mem",
            "extra": "avg mem: 203.09900706072608, max mem: 205.109375, count: 55421"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.284333,
            "unit": "median cpu",
            "extra": "avg cpu: 10.151527720396277, max cpu: 27.961164, count: 55421"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 167.11328125,
            "unit": "median mem",
            "extra": "avg mem: 160.2108393873712, max mem: 167.11328125, count: 55421"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 41137,
            "unit": "median block_count",
            "extra": "avg block_count: 42227.42211436098, max block_count: 59119.0, count: 55421"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.150864595091473, max cpu: 4.655674, count: 55421"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 117.8046875,
            "unit": "median mem",
            "extra": "avg mem: 105.85507855494758, max mem: 138.05859375, count: 55421"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 26,
            "unit": "median segment_count",
            "extra": "avg segment_count: 26.03054798722506, max segment_count: 50.0, count: 55421"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.076923,
            "unit": "median cpu",
            "extra": "avg cpu: 20.468771338290313, max cpu: 32.78049, count: 166263"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 219.5703125,
            "unit": "median mem",
            "extra": "avg mem: 236.67707041141145, max mem: 458.25, count: 166263"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 18.408438,
            "unit": "median cpu",
            "extra": "avg cpu: 15.409615347658875, max cpu: 32.65306, count: 55421"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 193.15625,
            "unit": "median mem",
            "extra": "avg mem: 191.8934858430243, max mem: 223.53125, count: 55421"
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
        "date": 1755727030419,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.032154681926293, max cpu: 41.458733, count: 55589"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 157.47265625,
            "unit": "median mem",
            "extra": "avg mem: 144.08315536234687, max mem: 158.9609375, count: 55589"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.595660437611333, max cpu: 28.042841, count: 55589"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 147.76953125,
            "unit": "median mem",
            "extra": "avg mem: 142.67729669707586, max mem: 147.76953125, count: 55589"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 19.28131568777339, max cpu: 78.68852, count: 55589"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 158.46484375,
            "unit": "median mem",
            "extra": "avg mem: 132.27768350640414, max mem: 167.93359375, count: 55589"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 20821,
            "unit": "median block_count",
            "extra": "avg block_count: 21064.21471873932, max block_count: 41735.0, count: 55589"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.628737,
            "unit": "median cpu",
            "extra": "avg cpu: 4.522273636342635, max cpu: 4.6829267, count: 55589"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 99.51953125,
            "unit": "median mem",
            "extra": "avg mem: 88.52805165196801, max mem: 128.859375, count: 55589"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.727787871701235, max segment_count: 49.0, count: 55589"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 20.45267066724675, max cpu: 73.84615, count: 111178"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 165.86328125,
            "unit": "median mem",
            "extra": "avg mem: 151.9808231928754, max mem: 175.4140625, count: 111178"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.994169,
            "unit": "median cpu",
            "extra": "avg cpu: 15.101645594171952, max cpu: 32.24568, count: 55589"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.421875,
            "unit": "median mem",
            "extra": "avg mem: 152.2003761001502, max mem: 155.796875, count: 55589"
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
        "date": 1755728165201,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Delete value - Primary - cpu",
            "value": 23.27837,
            "unit": "median cpu",
            "extra": "avg cpu: 24.288847324592247, max cpu: 32.78049, count: 55521"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 205.03515625,
            "unit": "median mem",
            "extra": "avg mem: 203.48465459578358, max mem: 205.03515625, count: 55521"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 13.872832,
            "unit": "median cpu",
            "extra": "avg cpu: 12.179518016475585, max cpu: 18.768328, count: 55521"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 156.875,
            "unit": "median mem",
            "extra": "avg mem: 153.74706874707317, max mem: 163.0625, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 38986,
            "unit": "median block_count",
            "extra": "avg block_count: 39519.44705606887, max block_count: 48721.0, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.316087601594983, max cpu: 4.6829267, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 121.75,
            "unit": "median mem",
            "extra": "avg mem: 110.53004690792673, max mem: 138.73828125, count: 55521"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 24,
            "unit": "median segment_count",
            "extra": "avg segment_count: 24.444552511662256, max segment_count: 56.0, count: 55521"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 23.210833,
            "unit": "median cpu",
            "extra": "avg cpu: 23.19497199208204, max cpu: 32.876713, count: 166563"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 210.6328125,
            "unit": "median mem",
            "extra": "avg mem: 221.93051667199498, max mem: 400.65234375, count: 166563"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 14.748091795174721, max cpu: 32.621357, count: 55521"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 193.1953125,
            "unit": "median mem",
            "extra": "avg mem: 191.63622224586192, max mem: 223.66796875, count: 55521"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "7008e501092f0d14807ba6c7a99a221724fc6c4c",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3008)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables",
          "timestamp": "2025-08-21T09:32:07-07:00",
          "tree_id": "cc49836a78f32b14f927d373fd12024625747a67",
          "url": "https://github.com/paradedb/paradedb/commit/7008e501092f0d14807ba6c7a99a221724fc6c4c"
        },
        "date": 1755796890706,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.202061226369096, max cpu: 41.69884, count: 55616"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.65625,
            "unit": "median mem",
            "extra": "avg mem: 141.67252011842365, max mem: 155.65625, count: 55616"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6421666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.617424348241332, max cpu: 28.09756, count: 55616"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.3671875,
            "unit": "median mem",
            "extra": "avg mem: 141.77816284320159, max mem: 146.3671875, count: 55616"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.24821219791151, max cpu: 64.990326, count: 55616"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.2734375,
            "unit": "median mem",
            "extra": "avg mem: 129.7324947800543, max mem: 163.4921875, count: 55616"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21397,
            "unit": "median block_count",
            "extra": "avg block_count: 21658.449852560414, max block_count: 43135.0, count: 55616"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6647234,
            "unit": "median cpu",
            "extra": "avg cpu: 4.547461626984374, max cpu: 4.6875, count: 55616"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100,
            "unit": "median mem",
            "extra": "avg mem: 89.17004211917434, max mem: 128.140625, count: 55616"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 31,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.59383990218642, max segment_count: 49.0, count: 55616"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.66621699718695, max cpu: 74.635574, count: 111232"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 165.1484375,
            "unit": "median mem",
            "extra": "avg mem: 150.69878033636903, max mem: 174.53515625, count: 111232"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.953489,
            "unit": "median cpu",
            "extra": "avg cpu: 14.493547409079737, max cpu: 32.40116, count: 55616"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.84765625,
            "unit": "median mem",
            "extra": "avg mem: 153.57560338414305, max mem: 157.1875, count: 55616"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "218ef2ba755f2e58e10e888347f8778b3cd4cd9f",
          "message": "fix: fixed empty table aggregate errors in aggregate custom scan (#3012)\n\n# Ticket(s) Closed\n\n- Closes #2996\n\n## What\n\nFix aggregate pushdown queries over empty tables that were throwing\nerrors instead of returning proper empty results.\n\n## Why\n\nWhen `paradedb.enable_aggregate_custom_scan` is enabled and aggregate\nqueries are executed on empty tables, two critical errors were\noccurring:\n\n1. **Simple aggregates** (COUNT, SUM, AVG, MIN, MAX): `ERROR: unexpected\naggregate result collection type`\n2. **GROUP BY aggregates**: `ERROR: missing bucket results`\n\nThis happened because the aggregate execution pipeline was returning\n`null` for empty tables, but the aggregate scan state expected proper\nJSON object structures.\n\n## How\n\nFixed `execute_aggregate` function**:\n- Changed return value from `serde_json::Value::Null` to\n`serde_json::json!({})` when no segments exist\n- This ensures a valid JSON object is always returned\n\nImproved `json_to_aggregate_results` function:\n- Added null-safety checks before calling `.as_object()`\n- Implemented proper empty result handling using\n`result_from_aggregate_with_doc_count` with `doc_count = 0`\n- This leverages existing logic that correctly handles COUNT (returns 0)\nvs other aggregates (returns NULL) for empty result sets\n\nUpdated `extract_bucket_results` function:\n- Added graceful handling for missing bucket structures in GROUP BY\nqueries\n- Returns empty result set instead of panicking when buckets are missing\n\n## Tests\n\nAdded regression test `empty_aggregate.sql` covering:\n- Simple SQL aggregates (COUNT, SUM, AVG, MIN, MAX) on empty tables\n- GROUP BY aggregates with single and multiple grouping columns  \n- JSON aggregates using `paradedb.aggregate()` function\n- JSON bucket aggregations (terms, histogram, range) with nested\nsub-aggregations\n- Edge cases with HAVING, FILTER clauses, and complex expressions\n\n**Expected behavior after fix:**\n- COUNT returns 0 for empty tables\n- SUM/AVG/MIN/MAX return NULL for empty tables  \n- GROUP BY queries return empty result sets (0 rows)\n- JSON aggregates return empty objects `{}` instead of `null`\n- No errors or panics when querying empty tables\n\nCo-authored-by: Moe <mdashti@gmail.com>",
          "timestamp": "2025-08-21T10:08:57-07:00",
          "tree_id": "99b6bb355bb88f5cdd62370c6319ed49287e5163",
          "url": "https://github.com/paradedb/paradedb/commit/218ef2ba755f2e58e10e888347f8778b3cd4cd9f"
        },
        "date": 1755799102571,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.255462834001683, max cpu: 41.860466, count: 55609"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 156.44921875,
            "unit": "median mem",
            "extra": "avg mem: 143.16774586285493, max mem: 157.19921875, count: 55609"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.553359266377503, max cpu: 27.988338, count: 55609"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 147.2578125,
            "unit": "median mem",
            "extra": "avg mem: 142.19979103539896, max mem: 147.65234375, count: 55609"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.885819676562466, max cpu: 70.03892, count: 55609"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 158.9453125,
            "unit": "median mem",
            "extra": "avg mem: 133.83780320519162, max mem: 167.71484375, count: 55609"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21323,
            "unit": "median block_count",
            "extra": "avg block_count: 21632.556762394575, max block_count: 43063.0, count: 55609"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.7657580446151333, max cpu: 4.655674, count: 55609"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 103.015625,
            "unit": "median mem",
            "extra": "avg mem: 91.35004169738711, max mem: 130.46875, count: 55609"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.55642072326422, max segment_count: 52.0, count: 55609"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.670683150867546, max cpu: 82.20742, count: 111218"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 166.28125,
            "unit": "median mem",
            "extra": "avg mem: 151.128251813162, max mem: 174.79296875, count: 111218"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.953489,
            "unit": "median cpu",
            "extra": "avg cpu: 14.291703316282831, max cpu: 28.235296, count: 55609"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.16015625,
            "unit": "median mem",
            "extra": "avg mem: 152.1437523040335, max mem: 155.4921875, count: 55609"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a010144b9900b25aad51e82ec239c3eeaa3a3fac",
          "message": "fix: restore the garbage list (#3020)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\n---------\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:30:11-04:00",
          "tree_id": "7aaa1f192dbf4add4a2c522a9f32de6bbaa0ff9b",
          "url": "https://github.com/paradedb/paradedb/commit/a010144b9900b25aad51e82ec239c3eeaa3a3fac"
        },
        "date": 1755836367808,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.343028639581956, max cpu: 46.421665, count: 55641"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.96875,
            "unit": "median mem",
            "extra": "avg mem: 142.45047449834655, max mem: 156.34375, count: 55641"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 7.587151055425364, max cpu: 28.042841, count: 55641"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.125,
            "unit": "median mem",
            "extra": "avg mem: 141.53839155535036, max mem: 146.125, count: 55641"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.532818,
            "unit": "median cpu",
            "extra": "avg cpu: 18.449426521908766, max cpu: 70.5191, count: 55641"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 158.0078125,
            "unit": "median mem",
            "extra": "avg mem: 133.26407649878237, max mem: 166.76171875, count: 55641"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21427,
            "unit": "median block_count",
            "extra": "avg block_count: 21632.365288186767, max block_count: 43044.0, count: 55641"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.64666,
            "unit": "median cpu",
            "extra": "avg cpu: 4.587890788426305, max cpu: 4.701273, count: 55641"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 100.76171875,
            "unit": "median mem",
            "extra": "avg mem: 89.92844537133139, max mem: 128.59375, count: 55641"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.530112686687875, max segment_count: 47.0, count: 55641"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 20.917598386484965, max cpu: 75.59055, count: 111282"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.236328125,
            "unit": "median mem",
            "extra": "avg mem: 149.3211890035001, max mem: 172.9296875, count: 111282"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 13.977682712442611, max cpu: 27.988338, count: 55641"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.671875,
            "unit": "median mem",
            "extra": "avg mem: 152.6935585101364, max mem: 156.453125, count: 55641"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "41898282+github-actions[bot]@users.noreply.github.com",
            "name": "github-actions[bot]",
            "username": "github-actions[bot]"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "a1edf576f60871a1e6ef4ef8eaa7749994ac4a64",
          "message": "fix: restore the garbage list (#3022)\n\n## What\n\nThis brings back our segment meta entry garbage list. It's necessary to\nhelp avoid potential query cancel situations in parallel workers.\n\n## Why\n\n## How\n\n## Tests\n\nSigned-off-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Eric Ridge <eebbrr@gmail.com>\nCo-authored-by: Stu Hood <stuhood@paradedb.com>",
          "timestamp": "2025-08-21T23:35:10-04:00",
          "tree_id": "561a6e7f3af192d0668156c2bc359a32eabf7664",
          "url": "https://github.com/paradedb/paradedb/commit/a1edf576f60871a1e6ef4ef8eaa7749994ac4a64"
        },
        "date": 1755836679230,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.533597381646537, max cpu: 41.65863, count: 55630"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.12890625,
            "unit": "median mem",
            "extra": "avg mem: 140.8750469058961, max mem: 156.4609375, count: 55630"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6376815,
            "unit": "median cpu",
            "extra": "avg cpu: 7.6452245265492245, max cpu: 28.180038, count: 55630"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.74609375,
            "unit": "median mem",
            "extra": "avg mem: 142.18671636257415, max mem: 146.74609375, count: 55630"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.58664,
            "unit": "median cpu",
            "extra": "avg cpu: 19.663957379223145, max cpu: 74.05979, count: 55630"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 157.01171875,
            "unit": "median mem",
            "extra": "avg mem: 130.98335219867877, max mem: 165.88671875, count: 55630"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 21023,
            "unit": "median block_count",
            "extra": "avg block_count: 21168.882293726405, max block_count: 42068.0, count: 55630"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 4.476909589553717, max cpu: 4.660194, count: 55630"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 102.171875,
            "unit": "median mem",
            "extra": "avg mem: 90.50752446409311, max mem: 130.703125, count: 55630"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.234855293906165, max segment_count: 47.0, count: 55630"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.622696,
            "unit": "median cpu",
            "extra": "avg cpu: 21.12499968440357, max cpu: 70.17544, count: 111260"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 162.8359375,
            "unit": "median mem",
            "extra": "avg mem: 148.56282760397943, max mem: 173.66015625, count: 111260"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.9265,
            "unit": "median cpu",
            "extra": "avg cpu: 14.245812813270494, max cpu: 28.042841, count: 55630"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 153.99609375,
            "unit": "median mem",
            "extra": "avg mem: 151.98377730091678, max mem: 155.72265625, count: 55630"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "mdashti@gmail.com",
            "name": "Moe",
            "username": "mdashti"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "cc4fe5965d5f99254c795da6f95353b72f8876b7",
          "message": "feat: improved join performance when score and snippets are not used and `@@@` cannot be pushed down to custom scan (#2871)\n\n# Ticket(s) Closed\n\n- Closes #2807\n\n## What\n\nImproved join performance when score and snippets are not used and `@@@`\ncannot be pushed down to custom scan\n\n## Why\n\nA source of our JOIN performance degradation is that we’re pushing\n`joininfo` (i.e., JOIN-level predicates in\n[here](https://github.com/paradedb/paradedb/blob/801e88e9a966b465dee73a381b3902c1a173c5bb/pg_search/src/postgres/customscan/builders/custom_path.rs#L274-L275)\nand\n[here](https://github.com/paradedb/paradedb/blob/6464dc167c77f043bc01a684f8282467ee464cbf/pg_search/src/postgres/customscan/pdbscan/mod.rs#L264))\ndown to tantivy, to be able to produce scores and snippets. However,\nthis almost always leads to a full index scan, which is expected to\nproduce poor performance. As a quick follow-up, we should either:\n1) at least limit this down to the cases that the query is using\nsnippets or scores, instead of always pushing down the JOIN quals that\nalmost certainly lead to a full index scan\n2) or we can even get stricter and give up on scores and snippets in\nthis case, too. Then, this will be fixed as soon as CustomScan Join API\nis implemented.\n\nIn this PR, we go for (1), as it won't have any impact on query results,\nand will only improve the performance.\n\n## How\n\nWe fallback to index scan (as opposed to custom scan) if the join quals\ncannot be pushed down to custom scan, and `score` and `snippet` are not\nused in the query.\n\n## Tests\n\nAll current tests pass.",
          "timestamp": "2025-08-22T11:13:50-07:00",
          "tree_id": "a374e10b164d326326720111fc92435c5b04ce37",
          "url": "https://github.com/paradedb/paradedb/commit/cc4fe5965d5f99254c795da6f95353b72f8876b7"
        },
        "date": 1755889406219,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.550726,
            "unit": "median cpu",
            "extra": "avg cpu: 18.545834035308037, max cpu: 37.982197, count: 55496"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 154.80859375,
            "unit": "median mem",
            "extra": "avg mem: 142.14907908621794, max mem: 156.30859375, count: 55496"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6511626,
            "unit": "median cpu",
            "extra": "avg cpu: 7.636139401951247, max cpu: 41.941746, count: 55496"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.95703125,
            "unit": "median mem",
            "extra": "avg mem: 141.86950467150785, max mem: 146.95703125, count: 55496"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 18.568666,
            "unit": "median cpu",
            "extra": "avg cpu: 19.29707098463604, max cpu: 74.2029, count: 55496"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 154.66796875,
            "unit": "median mem",
            "extra": "avg mem: 129.63405336927437, max mem: 163.37890625, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 20706,
            "unit": "median block_count",
            "extra": "avg block_count: 20917.054706645526, max block_count: 41278.0, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.610951,
            "unit": "median cpu",
            "extra": "avg cpu: 2.9723179983266417, max cpu: 4.6647234, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 101.1953125,
            "unit": "median mem",
            "extra": "avg mem: 90.21058534905669, max mem: 130.0859375, count: 55496"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 30,
            "unit": "median segment_count",
            "extra": "avg segment_count: 30.238251405506702, max segment_count: 47.0, count: 55496"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 18.658894,
            "unit": "median cpu",
            "extra": "avg cpu: 21.21069940690526, max cpu: 78.537056, count: 110992"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 163.140625,
            "unit": "median mem",
            "extra": "avg mem: 149.02772148696528, max mem: 172.83984375, count: 110992"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.899614,
            "unit": "median cpu",
            "extra": "avg cpu: 13.116521282211561, max cpu: 28.042841, count: 55496"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 154.71875,
            "unit": "median mem",
            "extra": "avg mem: 152.58480882912733, max mem: 156.26953125, count: 55496"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "eebbrr@gmail.com",
            "name": "Eric Ridge",
            "username": "eeeebbbbrrrr"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "f29250c078214cab1c7785740c86fd032bce69d0",
          "message": "perf: optimize merging heuristics to prefer background merging (#3031)\n\nWe change the decisions around when/how/what to merge such that we\nprefer to merge in the background if we can. That is defined as all of\nthese being true:\n\n- the user has either a default or non-zero configuration for\n`background_layer_sizes`\n- no other merge (foreground or background) is currently happening\n- the layer policy simulation indicates there is merging work to do \n \n or\n\n- the request to merge came from a VACUUM\n\nOtherwise we'll merge in the foreground if:\n\n- the request to merge came from `aminsert`\n- the user has either a default or non-zero configuration for\n`layer_sizes`\n\nAdditionally, when merging in the background we now consider the\ncombined/unique set of layer sizes from both `layer_sizes` and\n`background_layer_sizes`, and ensure the maximum layer size is clamped\nto our estimate of what an ideal segment size would be based on the\ncurrent index size and the `target_segment_count`.\n\nThis still allows concurrent backends to merge into layers defined in\n`layer_sizes` in the foreground, but they'll prefer to launch that work\nin the background if it looks possible.\n\nIt does not necessarily ensure there'll only ever be one background\nmerger per index at any given time, but in practice it's pretty close.",
          "timestamp": "2025-08-22T15:09:29-04:00",
          "tree_id": "16d37ec0ce591b380c00e4a5aa6b8ed53ac0fa8b",
          "url": "https://github.com/paradedb/paradedb/commit/f29250c078214cab1c7785740c86fd032bce69d0"
        },
        "date": 1755892755980,
        "tool": "customSmallerIsBetter",
        "benches": [
          {
            "name": "Custom scan - Primary - cpu",
            "value": 18.514948,
            "unit": "median cpu",
            "extra": "avg cpu: 18.90785374158384, max cpu: 42.72997, count: 55532"
          },
          {
            "name": "Custom scan - Primary - mem",
            "value": 155.96484375,
            "unit": "median mem",
            "extra": "avg mem: 143.91612950922442, max mem: 155.96484375, count: 55532"
          },
          {
            "name": "Delete value - Primary - cpu",
            "value": 4.6332045,
            "unit": "median cpu",
            "extra": "avg cpu: 7.615780556880089, max cpu: 28.070175, count: 55532"
          },
          {
            "name": "Delete value - Primary - mem",
            "value": 146.94921875,
            "unit": "median mem",
            "extra": "avg mem: 143.96496494982622, max mem: 146.94921875, count: 55532"
          },
          {
            "name": "Insert value - Primary - cpu",
            "value": 9.338522,
            "unit": "median cpu",
            "extra": "avg cpu: 12.181161524834081, max cpu: 37.17328, count: 55532"
          },
          {
            "name": "Insert value - Primary - mem",
            "value": 159.421875,
            "unit": "median mem",
            "extra": "avg mem: 136.2603723978967, max mem: 168.9375, count: 55532"
          },
          {
            "name": "Monitor Segment Count - Primary - block_count",
            "value": 20569,
            "unit": "median block_count",
            "extra": "avg block_count: 21004.67143268746, max block_count: 41233.0, count: 55532"
          },
          {
            "name": "Monitor Segment Count - Primary - cpu",
            "value": 4.6153846,
            "unit": "median cpu",
            "extra": "avg cpu: 3.9297667067372357, max cpu: 4.6421666, count: 55532"
          },
          {
            "name": "Monitor Segment Count - Primary - mem",
            "value": 119.26953125,
            "unit": "median mem",
            "extra": "avg mem: 101.42682625558237, max mem: 136.1640625, count: 55532"
          },
          {
            "name": "Monitor Segment Count - Primary - segment_count",
            "value": 23,
            "unit": "median segment_count",
            "extra": "avg segment_count: 23.230533746308435, max segment_count: 99.0, count: 55532"
          },
          {
            "name": "Update random values - Primary - cpu",
            "value": 13.913043,
            "unit": "median cpu",
            "extra": "avg cpu: 14.863894689450504, max cpu: 37.137333, count: 111064"
          },
          {
            "name": "Update random values - Primary - mem",
            "value": 167.109375,
            "unit": "median mem",
            "extra": "avg mem: 154.3566241578617, max mem: 176.28515625, count: 111064"
          },
          {
            "name": "Vacuum - Primary - cpu",
            "value": 13.88621,
            "unit": "median cpu",
            "extra": "avg cpu: 13.781712685696064, max cpu: 28.125, count: 55532"
          },
          {
            "name": "Vacuum - Primary - mem",
            "value": 155.3671875,
            "unit": "median mem",
            "extra": "avg mem: 153.58827363896043, max mem: 157.79296875, count: 55532"
          }
        ]
      }
    ]
  }
}