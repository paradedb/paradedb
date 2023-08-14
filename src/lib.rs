use pgrx::prelude::*;

pgrx::pg_module_magic!();

#[pg_extern]
fn hello_retake_extension() -> &'static str {
    "Hello, retake_extension"
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_hello_retake_extension() {
        assert_eq!("Hello, retake_extension", crate::hello_retake_extension());
    }

}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
        vec![]
    }
}
