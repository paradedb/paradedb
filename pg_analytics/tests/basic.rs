mod fixtures;

use anyhow::Result;
use fixtures::*;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
async fn basic_select(mut conn: PgConnection) -> Result<()> {
    UserSessionLogTable::setup(&mut conn).await?;
    let query = r#"
        SELECT * FROM user_session_log_table ORDER BY id
    "#;
    let result: UserSessionLogTableVec = UserSessionLogTable::fetch_all(&mut conn, query)
        .await?
        .into_iter()
        .collect();

    // Check that the first ten ids are in order.
    let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    assert_eq!(&result.id[0..10], ids, "ids are in expected order");

    Ok(())
}

#[rstest]
async fn array(mut conn: PgConnection) -> Result<()> {
    ResearchProjectArraysTable::setup(&mut conn).await?;
    let query = r#"
        SELECT * FROM research_project_arrays_table ORDER BY project_id
    "#;
    let _result: ResearchProjectArraysTableVec =
        ResearchProjectArraysTable::fetch_all(&mut conn, query)
            .await?
            .into_iter()
            .collect();

    // // Check that the first ten ids are in order.
    // let ids = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    // assert_eq!(&result.id[0..10], ids, "ids are in expected order");

    Ok(())
}
