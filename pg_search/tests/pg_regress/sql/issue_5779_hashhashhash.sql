-- Regression test for issue #5779, `###` operator variant.
-- The `###` operator paired with `pdb.slop` failed under generic prepared plans.
-- The RHS `$1::pdb.slop(...)` is not folded to a `Const` under a generic plan,
-- so the operator's `exec_rewrite` path saw the RHS type as `pdb.slop` and
-- raised: "The right-hand side of the `###(field, TEXT)` operator must be a
-- text value".
--
-- Both cases below must return the same rows as a non-parameterized query.
-- `###` takes `pdb.slop` (not `pdb.fuzzy`); phrases have no per-token fuzzy
-- distance in Tantivy.

CREATE TABLE issue_5779_hashhashhash_repro (
    id int,
    title_x text
);

INSERT INTO issue_5779_hashhashhash_repro (id, title_x) VALUES
    (1, 'the quick brown fox jumps'),
    (2, 'quick fox brown'),
    (3, 'brown fox quick'),
    (4, 'quick jumps over brown fox'),
    (5, 'nothing here');

CREATE INDEX issue_5779_hashhashhash_idx ON issue_5779_hashhashhash_repro
    USING paradedb (id, title_x)
    WITH (key_field = 'id');

-- Baseline: the same query as a literal must return the phrase match set.
SELECT id, title_x
FROM issue_5779_hashhashhash_repro
WHERE (title_x ### 'quick brown'::pdb.slop(3))
  AND (id @@@ pdb.all())
ORDER BY id;

-- Case 1: plan_cache_mode = auto.
SET plan_cache_mode = auto;

PREPARE issue_5779_hashhashhash_auto(text) AS
SELECT id, title_x
FROM issue_5779_hashhashhash_repro
WHERE (title_x ### $1::pdb.slop(3))
  AND (id @@@ pdb.all())
ORDER BY id;

EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');
EXECUTE issue_5779_hashhashhash_auto('quick brown');

DEALLOCATE issue_5779_hashhashhash_auto;

-- Case 2: plan_cache_mode = force_generic_plan.
SET plan_cache_mode = force_generic_plan;

PREPARE issue_5779_hashhashhash_generic(text) AS
SELECT id, title_x
FROM issue_5779_hashhashhash_repro
WHERE (title_x ### $1::pdb.slop(3))
  AND (id @@@ pdb.all())
ORDER BY id;

EXECUTE issue_5779_hashhashhash_generic('quick brown');

DEALLOCATE issue_5779_hashhashhash_generic;

RESET plan_cache_mode;

DROP TABLE issue_5779_hashhashhash_repro CASCADE;
