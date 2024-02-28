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
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // Boost
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.boost(query => paradedb.all(), boost => 1.5)
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // ConstScore
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
	    query => paradedb.const_score(query => paradedb.all(), score => 3.9)
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 41);

    // DisjunctionMax
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.disjunction_max(disjuncts => ARRAY[paradedb.parse('description:shoes')])
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // Empty
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.empty()
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 0);

    // FuzzyTerm
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.fuzzy_term(field => 'description', value => 'wolo')
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 4);

    // Parse
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.parse('description:teddy')
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.phrase_prefix(field => 'description', phrases => ARRAY['har'])
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // PhrasePrefix with invalid term list
    match r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.phrase(field => 'description', phrases => ARRAY['robot'])
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
    	)

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 1);

    // Regex
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.regex(
    		field => 'description',
    		pattern => '(hardcover|plush|leather|running|wireless)'
    	)

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);

    // Term
    let columns: SimpleProductsTableVec = r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term(field => 'description', value => 'shoes')

	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 3);

    // TermSet with invalid term list
    match r#"
    SELECT * FROM bm25_search.search(
    	query => paradedb.term_set(
    	    fields => ARRAY[
    	        paradedb.regex(field => 'description', pattern => '.+')
    	    ]
    	)
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
    	    fields => ARRAY[
    	        paradedb.term(field => 'description', value => 'shoes'),
    	        paradedb.term(field => 'description', value => 'novel')
    	    ]
    	)
	)"#
    .fetch_collect(&mut conn);
    assert_eq!(columns.len(), 5);
}
