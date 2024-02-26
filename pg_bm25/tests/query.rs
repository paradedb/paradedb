mod fixtures;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn multi_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boolean(
		    should => ARRAY[
			    paradedb.parse('description:shoes'),
			    paradedb.phrase_prefix(field => 'description', phrases => ARRAY['book']),
			    paradedb.term(field => 'description', value => 'speaker'),
			    paradedb.fuzzy_term(field => 'description', value => 'wolo')
		    ]
	    )
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.all()
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boost(query => paradedb.all(), boost => 1.5)
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.const_score(query => paradedb.all(), score => 3.9)
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')])
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.empty()
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(field => 'description', value => 'wolo')
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // MoreLikeThis with invalid query
    match r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.more_like_this(
    		fields => ARRAY[
    			paradedb.parse('description:shoes')
    		]
    	)
	)"#
    .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err.to_string().contains("only term queries")),
        _ => panic!("only term queries should be supported in bm25_search"),
    };

    // MoreLikeThis with correct query
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.more_like_this(
    		fields => ARRAY[
    			paradedb.term(field => 'description', value => 'shoes')
    		]
    	)
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);
}
