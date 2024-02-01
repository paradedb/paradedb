mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
#[ignore]
async fn basic_search_query(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    TestTable::setup(&mut conn).await;
    let result = TestTable::fetch_all(
        &mut conn,
        r#"
        SELECT * FROM bm25_search.search('description:keyboard OR category:electronics')
        "#,
    )
    .await?;

    let descriptions: Vec<&str> = result.iter().map(|r| r.description.as_ref()).collect();
    assert_eq!(
        descriptions,
        vec![
            "Plastic Keyboard",
            "Ergonomic metal keyboard",
            "Innovative wireless earbuds",
            "Fast charging power bank",
            "Bluetooth-enabled speaker"
        ]
    );

    let categories: Vec<&str> = result.iter().map(|r| r.category.as_ref()).collect();
    assert_eq!(
        categories,
        vec![
            "Electronics",
            "Electronics",
            "Electronics",
            "Electronics",
            "Electronics",
        ]
    );

    Ok(())
}

/// Test various queries, ensuring that ids come back in the expected order.
#[rstest]
#[ignore]
async fn basic_search_ids(mut conn: PgConnection) -> Result<(), sqlx::Error> {
    TestTable::setup(&mut conn).await;

    let cases = vec![
        (
            "description:keyboard OR category:electronics",
            vec![2, 1, 12, 22, 32],
        ),
        ("description:keyboard", vec![2, 1]),
    ];

    for (query, expected) in cases {
        let result = TestTable::fetch_all(
            &mut conn,
            &format!("SELECT * FROM bm25_search.search('{query}')"),
        )
        .await?;
        assert_eq!(
            result.iter().map(|r| r.id).collect::<Vec<_>>(),
            expected,
            "incorrect ids returned bm25_search.search('{query}')"
        );
    }

    Ok(())
}

#[rstest]
#[ignore]
#[should_panic]
async fn fail_to_scan_index(mut conn: PgConnection) {
    TestTable::setup_no_index(&mut conn).await;
    TestTable::fetch_all(
        &mut conn,
        "SELECT * FROM bm25_search.search('description:shoes');",
    )
    .await
    .unwrap();
}
