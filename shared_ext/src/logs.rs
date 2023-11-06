#[pgrx::pg_schema]
mod tests {
    use pgrx::{prelude::*, JsonString};
    use shared::gucs::PARADEDB_LOGS;
    use shared::plog;

    #[pg_test]
    fn test_bool_guc() {
        // Default should be false.
        assert!(!PARADEDB_LOGS.get(), "default is not set to false");

        // Setting to on should work.
        Spi::run("SET paradedb.logs = on").expect("SPI failed");
        assert!(PARADEDB_LOGS.get(), "setting parameter to on didn't work");

        // Setting to default should set to off.
        Spi::run("SET paradedb.logs TO DEFAULT;").expect("SPI failed");
        assert!(
            !PARADEDB_LOGS.get(),
            "setting parameter to default produced wrong value"
        );
    }

    #[pg_test]
    fn test_log_table() {
        // Each test starts with a fresh database connection, so the logs parameter
        // should return to false each time. We'll validate that here.
        assert!(
            !PARADEDB_LOGS.get(),
            "fresh database connection has logs set to true"
        );

        // We'll log a few things in each of the valid forms of plog!.
        // The expectation here is that the call is skipped entirely,
        // and nothing is inserted into the database.
        plog!("message only");
        plog!("message and data", vec![1, 2, 3]);
        plog!(LogLevel::DEBUG, "message and data and enum", vec![1, 2, 3]);

        let row_count = Spi::get_one("SELECT count(*) from paradedb.logs");
        assert_eq!(
            row_count,
            Ok(Some(0i64)), // counts must be i64
            "should be no rows before paradedb.logs is set to true"
        );

        // Now we'll set paradedb.logs to on, and we expect rows to be written.
        Spi::run("SET paradedb.logs = on").expect("error setting logs parameter to on");

        // Test just message
        plog!("message only");
        let message = Spi::get_one("SELECT message from paradedb.logs where ID = 1");
        assert_eq!(
            message,
            Ok(Some("message only")),
            "incorrect message in message only query"
        );

        // Test message and data
        plog!("message and data", vec![1, 2, 3]);
        let message = Spi::get_one("SELECT message FROM paradedb.logs WHERE ID = 2");
        let json = Spi::get_one("SELECT json FROM paradedb.logs WHERE ID = 2");
        assert_eq!(
            message,
            Ok(Some("message and data")),
            "incorrect message in messsage and data query"
        );
        match json {
            Ok(Some(JsonString(s))) => assert_eq!(
                s, "{\"data\":[1,2,3]}",
                "incorrect message in message and data query"
            ),
            _ => panic!("Unable to retrieve json data from message and data query"),
        }

        // Test level and message and data
        plog!(LogLevel::ERROR, "level and message and data", vec![1, 2, 3]);
        let message = Spi::get_one("SELECT message FROM paradedb.logs WHERE ID = 3");
        let level = Spi::get_one("SELECT level FROM paradedb.logs WHERE ID = 3");
        let json = Spi::get_one("SELECT json FROM paradedb.logs WHERE ID = 3");
        assert_eq!(
            message,
            Ok(Some("level and message and data")),
            "incorrect message in level and message and data query"
        );
        assert_eq!(
            level,
            Ok(Some(format!("{}", shared::logs::LogLevel::ERROR))),
            "incorrect level in level and message and data query"
        );
        match json {
            Ok(Some(JsonString(s))) => assert_eq!(
                s, "{\"data\":[1,2,3]}",
                "incorrect message in level and message and data query"
            ),
            _ => panic!("Unable to retrieve json data from message and data query"),
        }

        // Confirm that only 3 rows were written.
        let row_count = Spi::get_one("SELECT count(*) from paradedb.logs");
        assert_eq!(
            row_count,
            Ok(Some(3i64)), // counts must be i64
            "wrong number of rows written during plog! test"
        );
    }
}
