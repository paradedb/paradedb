window.BENCHMARK_DATA = {
  "lastUpdate": 1755725040851,
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
      }
    ]
  }
}