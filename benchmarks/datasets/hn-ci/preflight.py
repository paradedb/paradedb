"""Emit SQL that validates every HN benchmark fixture before timing."""

import json
import re
from pathlib import Path

manifest = json.loads(
    Path(__file__).with_name("fixtures.json").read_text(encoding="utf-8")
)
cases = []
names = set()
for scenario in manifest["scenarios"]:
    name = scenario["name"]
    assert name not in names, name
    names.add(name)
    assert len(scenario["fixtures"]) == 40, name
    for i, fixture in enumerate(scenario["fixtures"]):
        assert fixture["matches"] > 0, (name, i)
        literals = []
        for value in fixture["params"]:
            if value is None:
                literals.append("NULL")
            elif isinstance(value, bool):
                literals.append("TRUE" if value else "FALSE")
            elif isinstance(value, (int, float)):
                literals.append(str(value))
            elif isinstance(value, str):
                literals.append("'" + value.replace("'", "''") + "'")
            else:
                raise TypeError(type(value))
        query = re.sub(
            r"\$(\d+)",
            lambda match, values=literals: values[int(match[1]) - 1],
            scenario["sql"],
        )
        cases.append({"name": name, "fixture": i, "sql": query})
assert len(names) == 29 and len(cases) == 1160
payload = json.dumps(cases)
assert "$fixtures$" not in payload
print("SET statement_timeout = '10min';")
print(
    """DO $preflight$
DECLARE
    fixture jsonb;
    plan jsonb;
    hits bigint;
    documents bigint;
BEGIN
    SELECT count(*) INTO documents FROM hn_items WHERE id @@@ pdb.all();
    IF documents <> 28737557 THEN
        RAISE EXCEPTION 'Unexpected HN dataset size: %, expected 28737557', documents;
    END IF;
    IF (SELECT count(*) FROM paradedb.schema('hn_items_idx')
        WHERE fast AND name IN ('time', 'score', 'type', 'descendants', 'deleted')) <> 5 THEN
        RAISE EXCEPTION 'HN index is missing required fast fields';
    END IF;
    FOR fixture IN SELECT value FROM jsonb_array_elements($fixtures$"""
    + payload
    + """$fixtures$::jsonb)
    LOOP
        EXECUTE 'EXPLAIN (FORMAT JSON, COSTS OFF) ' || (fixture->>'sql') INTO plan;
        IF position('TopKScanExecState' IN plan::text) = 0 THEN
            RAISE EXCEPTION 'HN fixture %/% does not use ParadeDB Top-K: %',
                fixture->>'name', fixture->>'fixture', plan;
        END IF;
        IF jsonb_path_exists(plan, '$.**."Filter"') THEN
            RAISE EXCEPTION 'HN fixture %/% has a residual filter: %',
                fixture->>'name', fixture->>'fixture', plan;
        END IF;
        EXECUTE fixture->>'sql';
        GET DIAGNOSTICS hits = ROW_COUNT;
        IF hits = 0 THEN
            RAISE EXCEPTION 'Empty HN fixture %/%: %',
                fixture->>'name', fixture->>'fixture', fixture->>'sql';
        END IF;
    END LOOP;
    RAISE NOTICE 'All 1160 HN fixtures return rows and use ParadeDB Top-K';
END
$preflight$;"""
)
