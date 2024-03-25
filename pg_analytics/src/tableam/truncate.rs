use pgrx::*;

#[pg_guard]
pub extern "C" fn deltalake_relation_nontransactional_truncate(_rel: pg_sys::Relation) {
    todo!()
}
