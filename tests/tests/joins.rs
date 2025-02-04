mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

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

    let (snippet,) = r#"select paradedb.snippet(a.value) from a left join b on a.id = b.id where a.value @@@ 'beer';"#
    .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    let (snippet,) = r#"select paradedb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' and b.value @@@ 'beer';"#
    .fetch_one::<(String,)>(&mut conn);
    assert_eq!(snippet, String::from("<b>beer</b>"));

    // NB:  the result of this is wrong for now...
    let results = r#"select a.id, b.id, paradedb.snippet(a.value), paradedb.snippet(b.value) from a left join b on a.id = b.id where a.value @@@ 'beer' or b.value @@@ 'wine' order by a.id, b.id;"#
        .fetch_result::<(i64, i64, Option<String>, Option<String>)>(&mut conn)?;

    // ... this is what we'd actually expect from the above query
    /*
    let expected = vec![
        (1, 1, Some(String::from("<b>beer</b>")), None),
        (2, 2, None, Some(String::from("<b>wine</b>"))),
    ];
     */
    // but due to query planning weirdness that I haven't figured out yet, this is what we actually get
    // Even tho this "expected" result is not correct I want to encode validating this result
    // so that when we do improve our custom scan/planner integrations this test will light up,
    // and we can then confirm that it's actually the above
    let expected = vec![(1, 1, None, None), (2, 2, None, None)];

    assert_eq!(results, expected);

    Ok(())
}
