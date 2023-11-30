#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::*;
    use shared::testing::dblink;

    const SETUP_SQL: &str = include_str!("tokenizer_chinese_compatible_setup.sql");
    const QUERY_SQL: &str = include_str!("tokenizer_chinese_compatible_query.sql");

    #[pgrx::pg_test]
    fn test_chinese_compatible_tokenizer() {
        // In this test, the index is created and the tokenizer is used in the same transaction.

        Spi::run(SETUP_SQL).expect("error running setup query");

        let highlight = Spi::get_one(QUERY_SQL);

        // Assert that the highlight returned by the tokenizer is as expected.
        assert_eq!(
            highlight,
            Ok(Some("<b>张</b>伟")),
            "incorrect result for chinese compatible tokenizer highlight"
        );
    }

    #[pgrx::pg_test]
    fn test_chinese_compatible_tokenizer_in_new_connection() {
        // In this test, the index is created and the tokenizer is used in separate connections.
        // Because we retrieve the index from disk for new connections, we want to make
        // sure that the tokenizers are set up properly. We're going to make use of a
        // Postgres extension that lets us create 'sub-connections' to the database.

        // Create the dblink extension if it doesn't already exist.
        // dblink allows us to establish a 'sub-connection' to the current database
        // and execute queries. This is necessary, because the test context otherwise
        // is executed within a single Postgres transaction.
        Spi::run("CREATE EXTENSION IF NOT EXISTS dblink").expect("error creating dblink extension");

        // Set up the test environment using dblink to run the setup SQL in a separate connection.
        // The setup SQL is expected to prepare the database with the necessary configuration for the tokenizer.
        let setup_query = format!("SELECT * FROM {} AS (_ text)", &dblink(SETUP_SQL));
        Spi::run(&setup_query).expect("error running dblink setup query");

        // Run the test query using dblink to ensure the tokenizer works in a separate connection.
        // The query SQL is expected to test the tokenizer functionality and return a highlighted result.
        let test_query = format!(
            "SELECT * FROM {} AS (highlight_bm25 text)",
            &dblink(QUERY_SQL)
        );
        let highlight = Spi::get_one(&test_query);

        // Assert that the highlight returned by the tokenizer is as expected.
        assert_eq!(
            highlight,
            Ok(Some("<b>张</b>伟")),
            "incorrect result for chinese compatible tokenizer highlight"
        );
    }
}
