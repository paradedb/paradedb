-- Regression test for issue #5710.
-- top_hits.sort on a text field silently returned null sort values with no
-- ordering, because Tantivy's top_hits sort accessor is numeric/date-only and
-- text fields fell back to an empty accessor. The plan-time validator now
-- rejects unsortable sort keys with a clear error.

CREATE TABLE issue_5710_repro (
    id uuid PRIMARY KEY,
    group_id integer NOT NULL,
    label text NOT NULL,
    created_at timestamp NOT NULL
);

INSERT INTO issue_5710_repro (id, group_id, label, created_at) VALUES
    ('00000000-0000-0000-0000-000000000001', 1, 'zebra',  '2026-01-01 00:00:00'),
    ('00000000-0000-0000-0000-000000000002', 1, 'alpha',  '2026-01-02 00:00:00'),
    ('00000000-0000-0000-0000-000000000003', 1, 'mango',  '2026-01-03 00:00:00');

-- `(label::pdb.literal)` makes label a fast field so docvalue_fields can
-- return it. Without this, top_hits errors on docvalue_fields resolution
-- rather than on the sort validator we are testing.
CREATE INDEX issue_5710_idx ON issue_5710_repro
    USING paradedb (id, group_id, (label::pdb.literal), created_at)
    WITH (key_field = 'id');

-- Case 1: sort on a text field must raise a clear error.
-- Before the fix, this returned three hits each with "sort": [null] and no
-- ordering. After the fix, planning rejects the query.
SELECT pdb.agg($$
{
  "top_hits": {
    "size": 3,
    "sort": [{"label": "asc"}],
    "docvalue_fields": ["label"]
  }
}
$$::jsonb)
FROM issue_5710_repro
WHERE id @@@ pdb.all()
GROUP BY group_id;

-- Case 2: sort on a date field still works. Baseline for the positive path.
SELECT pdb.agg($$
{
  "top_hits": {
    "size": 3,
    "sort": [{"created_at": "asc"}],
    "docvalue_fields": ["label", "created_at"]
  }
}
$$::jsonb)
FROM issue_5710_repro
WHERE id @@@ pdb.all()
GROUP BY group_id;

-- Case 3: sort on an integer field still works.
SELECT pdb.agg($$
{
  "top_hits": {
    "size": 3,
    "sort": [{"group_id": "desc"}],
    "docvalue_fields": ["label"]
  }
}
$$::jsonb)
FROM issue_5710_repro
WHERE id @@@ pdb.all()
GROUP BY group_id;

-- Case 4: sort on the Elasticsearch-style pseudo field `_score` is not a
-- schema field and must not trip the new validator.
SELECT pdb.agg($$
{
  "top_hits": {
    "size": 3,
    "sort": [{"_score": "desc"}],
    "docvalue_fields": ["label"]
  }
}
$$::jsonb)
FROM issue_5710_repro
WHERE id @@@ pdb.all()
GROUP BY group_id;

-- Case 5: mixed sort with one bad field must still error, since Tantivy
-- would produce null sort values for the text key while sorting on the date.
SELECT pdb.agg($$
{
  "top_hits": {
    "size": 3,
    "sort": [{"created_at": "asc"}, {"label": "asc"}],
    "docvalue_fields": ["label"]
  }
}
$$::jsonb)
FROM issue_5710_repro
WHERE id @@@ pdb.all()
GROUP BY group_id;

DROP TABLE issue_5710_repro CASCADE;
