use pgrx::Spi;

pub const SETUP_SQL: &str = include_str!("sql/index_setup.sql");
pub const QUERY_SQL: &str = include_str!("sql/search_query.sql");

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
