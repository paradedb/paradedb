use pgrx::spi::SpiTupleTable;
use pgrx::{JsonB, Spi};
use serde_json::Value as JsonValue;
use std::error::Error;

pub const SETUP_SQL: &str = include_str!("sql/index_setup.sql");
pub const QUERY_SQL: &str = include_str!("sql/search_query.sql");

/// Define a struct to represent the expected row structure of our bm25_test_table,
/// with optional fields for testing flexibility.
pub struct ExpectedRow {
    pub rank_bm25: Option<f32>,
    pub id: Option<i32>,
    pub description: Option<&'static str>,
    pub rating: Option<i32>,
    pub category: Option<&'static str>,
    pub in_stock: Option<bool>,
    pub metadata: Option<JsonValue>,
    pub highlight_bm25: Option<&'static str>,
}

/// We default the struct to None for all fields, to avoid needing to pass None to all
/// the fields which aren't being tested in particular tests.
impl Default for ExpectedRow {
    fn default() -> Self {
        ExpectedRow {
            rank_bm25: None,
            id: None,
            description: None,
            rating: None,
            category: None,
            in_stock: None,
            metadata: None,
            highlight_bm25: None,
        }
    }
}

fn get_column_names(table: &SpiTupleTable) -> Result<Vec<String>, Box<dyn Error>> {
    let mut column_names = Vec::new();
    let mut ordinal = 1;

    while let Ok(name) = table.column_name(ordinal) {
        column_names.push(name);
        ordinal += 1;
    }

    Ok(column_names)
}

/// Compares the output of Spi::connect() query on our bm25_test_table to the expected output.
///
/// NOTE: This function assume that the query is executed against the bm25_search schema created
/// by the index_setup.sql script.
pub fn test_table(
    mut table: SpiTupleTable,
    expect: Vec<ExpectedRow>,
) -> Result<(), Box<dyn Error>> {
    let column_names = get_column_names(&table)?;

    let mut i = 0;
    while let Some(_) = table.next() {
        // Initialize fields with default values
        let mut rank_bm25 = None;
        let mut id = None;
        let mut description = None;
        let mut rating = None;
        let mut category = None;
        let mut in_stock = None;
        let mut metadata = None;
        let mut highlight_bm25 = None;

        for (index, col_name) in column_names.iter().enumerate() {
            match col_name.as_str() {
                "rank_bm25" => {
                    rank_bm25 = table.get(index + 1).ok().flatten();
                }
                "id" => {
                    id = table.get(index + 1).ok().flatten();
                }
                "description" => {
                    description = table.get(index + 1).ok().flatten();
                }
                "rating" => {
                    rating = table.get(index + 1).ok().flatten();
                }
                "category" => {
                    category = table.get(index + 1).ok().flatten();
                }
                "in_stock" => {
                    in_stock = table.get(index + 1).ok().flatten();
                }
                "metadata" => {
                    metadata = table
                        .get(index + 1)
                        .ok()
                        .flatten()
                        .map(|jsonb: JsonB| jsonb.0);
                }
                "highlight_bm25" => {
                    highlight_bm25 = table.get(index + 1).ok().flatten();
                }
                _ => {
                    panic!("Unexpected column name: {}", col_name); // Should never happen
                }
            }
        }

        // Create a tuple from the retrieved values
        let row = ExpectedRow {
            rank_bm25,
            id,
            description,
            rating,
            category,
            in_stock,
            metadata,
            highlight_bm25,
        };

        // Compare each field individually with the expected row
        let expected = &expect[i];
        assert_eq!(row.rank_bm25, expected.rank_bm25);
        assert_eq!(row.id, expected.id);
        assert_eq!(row.description, expected.description);
        assert_eq!(row.rating, expected.rating);
        assert_eq!(row.category, expected.category);
        assert_eq!(row.in_stock, expected.in_stock);
        assert_eq!(row.metadata, expected.metadata);
        assert_eq!(row.highlight_bm25, expected.highlight_bm25);

        i += 1;
    }
    assert_eq!(expect.len(), i, "Number of rows does not match expected");
    Ok(())
}

/// Executes a query on a remote PostgreSQL database using dblink.
///
/// `dblink` is a PostgreSQL extension that allows a user to connect to a different PostgreSQL
/// database from within a database session. It can be used to query data from a remote database
/// without having to establish a new database connection. This is particularly useful for
/// accessing databases on other servers or different databases on the same server that the
/// current session is not connected to.
///
/// # Arguments
/// * `query` - A reference to a string slice that holds the SQL query to be executed on the remote database.
///
/// # Returns
/// A `String` that contains the dblink function call, which can be executed within a PostgreSQL
/// environment to perform the remote database query.
///
/// # Panics
/// The function panics if:
/// - It cannot retrieve the current database name.
/// - It cannot retrieve the current port from the PostgreSQL settings.
/// - It cannot parse the retrieved port into an unsigned 32-bit integer.
///
/// # Examples
/// ```
/// let query = "SELECT * FROM my_table WHERE id = 1";
/// let dblink_query = dblink(query);
/// println!("DBLink Query: {}", dblink_query);
/// // Output: DBLink Query: dblink('host=localhost port=5432 dbname=mydb', 'SELECT * FROM my_table WHERE id = 1')
/// ```
pub fn dblink(query: &str) -> String {
    // Retrieve the current database name from the PostgreSQL environment.
    let current_db_name: String = Spi::get_one("SELECT current_database()::text")
        .expect("couldn't get current database for postgres connection")
        .unwrap();

    // Retrieve the current port number on which the PostgreSQL server is listening.
    let current_port: u32 =
        Spi::get_one::<String>("SELECT setting FROM pg_settings WHERE name = 'port'")
            .expect("couldn't get current port for postgres connection")
            .unwrap()
            .parse()
            .expect("couldn't parse current port into u32");

    // Prepare the connection string for dblink. This string contains the host (assumed to be
    // localhost in this function), the port number, and the database name to connect to.
    let connection_string = format!(
        "host=localhost port={} dbname={}",
        current_port, current_db_name
    );

    // Escape single quotes in the SQL query since it will be nested inside another SQL string
    // in the dblink function call. Single quotes in SQL strings are escaped by doubling them.
    let escaped_query_string = query.replace('\'', "''");

    // Construct the dblink function call with the connection string and the escaped query.
    // This function call is what can be executed within a PostgreSQL environment.
    format!("dblink('{connection_string}', '{escaped_query_string}')")
}
