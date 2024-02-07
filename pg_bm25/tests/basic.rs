mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use shared::sqlx::{self};

#[rstest]
#[ignore]
async fn basic_search_query(
    mut simple_products_table: TableConnection<SimpleProductsTable>,
) -> Result<(), sqlx::Error> {
    let columns: SimpleProductsTableVec = simple_products_table.fetch_collect(
        "SELECT * FROM bm25_search.search('description:keyboard OR category:electronics')",
    );

    assert_eq!(
        columns.description,
        concat!(
            "Plastic Keyboard,Ergonomic metal keyboard,Innovative wireless earbuds,",
            "Fast charging power bank,Bluetooth-enabled speaker"
        )
        .split(',')
        .collect::<Vec<_>>()
    );

    assert_eq!(
        columns.category,
        "Electronics,Electronics,Electronics,Electronics,Electronics"
            .split(',')
            .collect::<Vec<_>>()
    );

    Ok(())
}

/// Test various queries, ensuring that ids come back in the expected order.
#[rstest]
#[ignore]
async fn basic_search_ids(mut simple_products_table: TableConnection<SimpleProductsTable>) {
    let columns: SimpleProductsTableVec = simple_products_table.fetch_collect(
        "SELECT * FROM bm25_search.search('description:keyboard OR category:electronics')",
    );
    assert_eq!(columns.id, vec![2, 1, 12, 22, 32]);

    let columns: SimpleProductsTableVec = simple_products_table
        .fetch_collect("SELECT * FROM bm25_search.search('description:keyboard')");
    assert_eq!(columns.id, vec![2, 1]);
}
