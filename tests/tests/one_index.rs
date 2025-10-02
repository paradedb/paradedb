mod fixtures;

use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn only_one_index_allowed(mut conn: PgConnection) {
    r#"
    CALL paradedb.create_bm25_test_table(
      schema_name => 'public',
      table_name => 'mock_items'
    )
    "#
    .execute(&mut conn);

    r#"
    CREATE INDEX index_one ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute(&mut conn);

    match r#"
    CREATE INDEX index_two ON public.mock_items
    USING bm25 (id, description)
    WITH (key_field = 'id');
    "#
    .execute_result(&mut conn)
    {
        Ok(_) => panic!("created a second `USING bm25` index"),
        Err(e) if format!("{e}").contains("a relation may only have one `USING bm25` index") => (), // all good
        Err(e) => panic!("{}", e),
    }
}
