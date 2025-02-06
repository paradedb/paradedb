mod fixtures;

use fixtures::*;
use rstest::*;
use serde_json::Value;
use sqlx::PgConnection;

#[rstest]
fn joins_return_correct_results(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    DROP TABLE IF EXISTS a;
    DROP TABLE IF EXISTS b;
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );
    
    INSERT INTO public.a VALUES (1, 'beer wine');
    INSERT INTO public.a VALUES (2, 'beer wine');
    INSERT INTO public.a VALUES (3, 'cheese');
    INSERT INTO public.a VALUES (4, 'food stuff');
    INSERT INTO public.a VALUES (5, 'only_in_a');

    INSERT INTO public.b VALUES (1, 'beer');
    INSERT INTO public.b VALUES (2, 'wine');
    INSERT INTO public.b VALUES (3, 'cheese');
    INSERT INTO public.b VALUES (4, 'wine beer cheese');
                            -- mind the gap
    INSERT INTO public.b VALUES (6, 'only_in_b');

    INSERT INTO a (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
    INSERT INTO b (id, value) SELECT x, md5(random()::text) FROM generate_series(7, 10000) x;
        
    CREATE INDEX idxa ON public.a USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    CREATE INDEX idxb ON public.b USING bm25 (id, value) WITH (key_field=id, text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    type RowType = (Option<i64>, Option<i64>, Option<String>, Option<String>);
    // the pg_search queries also ORDER BY paradedb.score() to ensure we get a paradedb CustomScan
    let queries = [
        [
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, paradedb.score(a.id), paradedb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a left join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, paradedb.score(a.id), paradedb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a right join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, paradedb.score(a.id), paradedb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a inner join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
        [
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value @@@   'beer'   or b.value @@@   'wine'   or a.value @@@ 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id, paradedb.score(a.id), paradedb.score(b.id);",
            "select a.id, b.id, a.value a, b.value b from a full join b on a.id = b.id where a.value ilike '%beer%' or b.value ilike '%wine%' or a.value   = 'only_in_a' or b.value @@@ 'only_in_b' order by a.id, b.id;",
        ],
    ];

    for [pg_search, postgres] in queries {
        eprintln!("pg_search: {pg_search:?}");
        eprintln!("postgres: {postgres:?}");

        let (pg_search_plan,) = format!("EXPLAIN (ANALYZE, FORMAT JSON) {}", pg_search)
            .fetch_one::<(Value,)>(&mut conn);
        eprintln!("pg_search_plan: {pg_search_plan:#?}");
        assert!(format!("{pg_search_plan:?}").contains("ParadeDB Scan"));

        let pg_search = pg_search.fetch_result::<RowType>(&mut conn)?;
        let postgres = postgres.fetch_result::<RowType>(&mut conn)?;

        assert_eq!(pg_search, postgres);
    }

    Ok(())
}

#[rstest]
fn snippet_from_join(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    r#"
    CREATE TABLE a (
        id bigint,
        value text
    );
    CREATE TABLE b (
        id bigint,
        value text
    );

    INSERT INTO a (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');
    INSERT INTO b (id, value) VALUES (1, 'beer'), (2, 'wine'), (3, 'cheese');

    CREATE INDEX idxa ON a USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    CREATE INDEX idxb ON b USING bm25 (id, value) WITH (key_field='id', text_fields='{"value": {}}');
    "#
        .execute(&mut conn);

    let (snippet, ) = r#"select paradedb.snippet(a.value) from a left join b on a.id = b.id where a.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    let (snippet, ) = r#"select paradedb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' and b.value @@@ 'beer';"#
        .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    // NB:  the result of this is wrong for now...
    let results = r#"select a.id, b.id, paradedb.snippet(a.value), paradedb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' or b.value @@@ 'wine' order by a.id, b.id;"#
        .fetch_result::<(i64, i64, Option<String>, Option<String>)>(&mut conn)?;

    // ... this is what we'd actually expect from the above query
    let expected = vec![
        (1, 1, Some(String::from("<b>beer</b>")), None),
        (2, 2, None, Some(String::from("<b>wine</b>"))),
    ];

    assert_eq!(results, expected);

    Ok(())
}
