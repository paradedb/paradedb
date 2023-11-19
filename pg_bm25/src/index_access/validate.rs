use pgrx::*;

#[pg_guard]
pub extern "C" fn amvalidate(_opclassoid: pg_sys::Oid) -> bool {
    true
}

#[cfg(feature = "pg_test")]
#[pgrx::pg_schema]
mod tests {
    use super::amvalidate;

    #[pgrx::pg_test]
    fn test_amvalidate() {
        assert!(amvalidate(pgrx::pg_sys::Oid::default()))
    }
}
