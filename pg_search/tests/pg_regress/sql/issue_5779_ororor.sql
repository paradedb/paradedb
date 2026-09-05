-- Regression test for issue #5779, `|||` operator variant.
-- The `|||` operator paired with `pdb.fuzzy` failed under generic prepared plans.
-- The RHS `$1::pdb.fuzzy(...)` is not folded to a `Const` under a generic plan,
-- so the operator's `exec_rewrite` path saw the RHS type as `pdb.fuzzy` and
-- raised: "The right-hand side of the `|||(field, TEXT)` operator must be a
-- text value".
--
-- Both cases below must return the same rows as a non-parameterized query.

CREATE TABLE issue_5779_ororor_repro (
    id int,
    title_x text
);

INSERT INTO issue_5779_ororor_repro (id, title_x) VALUES
    (1, 'the quick brown fox'),
    (2, 'the qwick brown fox'),
    (3, 'lazy dog'),
    (4, 'quiick brown'),
    (5, 'nothing here');

CREATE INDEX issue_5779_ororor_idx ON issue_5779_ororor_repro
    USING paradedb (id, title_x)
    WITH (key_field = 'id');

-- Baseline: the same query as a literal must return the fuzzy match set.
SELECT id, title_x
FROM issue_5779_ororor_repro
WHERE (title_x ||| 'quick'::pdb.fuzzy(2, f, t))
  AND (id @@@ pdb.all())
ORDER BY id;

-- Case 1: plan_cache_mode = auto.
SET plan_cache_mode = auto;

PREPARE issue_5779_ororor_auto(text) AS
SELECT id, title_x
FROM issue_5779_ororor_repro
WHERE (title_x ||| $1::pdb.fuzzy(2, f, t))
  AND (id @@@ pdb.all())
ORDER BY id;

EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');
EXECUTE issue_5779_ororor_auto('quick');

DEALLOCATE issue_5779_ororor_auto;

-- Case 2: plan_cache_mode = force_generic_plan.
SET plan_cache_mode = force_generic_plan;

PREPARE issue_5779_ororor_generic(text) AS
SELECT id, title_x
FROM issue_5779_ororor_repro
WHERE (title_x ||| $1::pdb.fuzzy(2, f, t))
  AND (id @@@ pdb.all())
ORDER BY id;

EXECUTE issue_5779_ororor_generic('quick');

DEALLOCATE issue_5779_ororor_generic;

RESET plan_cache_mode;

DROP TABLE issue_5779_ororor_repro CASCADE;
