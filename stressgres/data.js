window.BENCHMARK_DATA = {
  "lastUpdate": 1755811648077,
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
      }
    ]
  }
}