mod fixtures;

use core::panic;

use fixtures::*;
use pretty_assertions::assert_eq;
use rstest::*;
use sqlx::PgConnection;

#[rstest]
fn boolean_tree(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boolean(
		    should => ARRAY[
			    paradedb.term(field => 'category', value => 'Electronics')
		    ]
	    ),
	    stable_sort => true
	);
    "#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn datetime_search(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
        query => paradedb.boolean(
            should => ARRAY[
                paradedb.term(field => 'last_updated_date', value => '2023-05-03'::date))
            ]
        ),
        stable_sort => true
    );

    SELECT * FROM bm25_search.search(
        query => paradedb.boolean(
            should => ARRAY[
                paradedb.range(field => 'last_updated_date', range => '[2023-05-01,2023-05-03]'::daterange)
            ]
        ),
        stable_sort => true
    );
    "#
    .fetch_collect(&mut conn);
    // TODO: not correct???
    assert_eq!(columns.id, vec![32, 5, 3, 4, 7, 34, 37, 10, 33, 39, 41]);
}

#[rstest]
fn fuzzy_fields(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(field => 'category', value => 'elector'),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.id, vec![1, 2, 12, 22, 32], "wrong results");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term(field => 'category', value => 'electornics'),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert!(columns.is_empty(), "without fuzzy field should be empty");

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(
    	    field => 'description',
    	    value => 'keybaord',
    	    tranposition_cost_one => false,
    	    distance => 1
    	),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert!(
        columns.is_empty(),
        "tranposition_cost_one false should be empty"
    );

    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(
    	    field => 'description',
    	    value => 'keybaord',
    	    tranposition_cost_one => true,
    	    distance => 1
    	),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(
        columns.id,
        vec![1, 2],
        "incorrect tranposition_cost_one true"
    );
}

#[rstest]
fn single_queries(mut conn: PgConnection) {
    SimpleProductsTable::setup().execute(&mut conn);

    // All
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.all(),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boost(query => paradedb.all(), boost => 1.5),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.const_score(query => paradedb.all(), score => 3.9),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')]),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.empty(),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(field => 'description', value => 'wolo'),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.parse('description:teddy'),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.phrase_prefix(field => 'description', phrases => ARRAY['har']),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Phrase with invalid term list
    match r#"
       SELECT * FROM bm25_search.search(
       	query => paradedb.phrase(field => 'description', phrases => ARRAY['robot']),
	    stable_sort => true
    )"#
    .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("required to have strictly more than one term")),
        _ => panic!("phrase prefix query should require multiple terms"),
    }

    // Phrase
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.phrase(
    		field => 'description',
    		phrases => ARRAY['robot', 'building', 'kit']
    	),
	    stable_sort => true

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.regex(
    		field => 'description',
    		pattern => '(hardcover|plush|leather|running|wireless)'
    	),
	    stable_sort => true

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term(field => 'description', value => 'shoes'),
	    stable_sort => true

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Term with no field (should search all columns)
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term(value => 'shoes'),
	    stable_sort => true

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet with invalid term list
    match r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term_set(
    	    terms => ARRAY[
    	        paradedb.regex(field => 'description', pattern => '.+')
    	    ]
    	),
	    stable_sort => true
	)"#
    .fetch_result::<SimpleProductsTable>(&mut conn)
    {
        Err(err) => assert!(err
            .to_string()
            .contains("only term queries can be passed to term_set")),
        _ => panic!("term set query should only accept terms"),
    }

    // TermSet
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term_set(
    	    terms => ARRAY[
    	        paradedb.term(field => 'description', value => 'shoes'),
    	        paradedb.term(field => 'description', value => 'novel')
    	    ]
    	),
	    stable_sort => true
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}
