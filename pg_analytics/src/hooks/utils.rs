use pgrx::*;

pub unsafe fn relation_from_rangevar(rangevar: *mut pg_sys::RangeVar) -> pg_sys::Relation {
    let oid = match (*rangevar).schemaname.is_null() {
        true => pg_sys::RelnameGetRelid((*rangevar).relname),
        false => pg_sys::get_relname_relid(
            (*rangevar).relname,
            pg_sys::get_namespace_oid((*rangevar).schemaname, true),
        ),
    };

    pg_sys::RelationIdGetRelation(oid)
}
