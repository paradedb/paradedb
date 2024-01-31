use crate::env::register_commit_callback;
use crate::globals::WriterGlobal;
use crate::index::SearchIndex;
use crate::postgres::options::SearchIndexCreateOptions;
use crate::postgres::utils::{get_search_index, lookup_index_tupdesc};
use crate::schema::{SearchFieldConfig, SearchFieldName, SearchFieldType};
use crate::writer::WriterDirectory;
use pgrx::*;
use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};

// For now just pass the count on the build callback state
struct BuildState {
    count: usize,
}

impl BuildState {
    fn new() -> Self {
        BuildState { count: 0 }
    }
}

#[pg_guard]
// TODO: remove the unsafe
pub extern "C" fn ambuild(
    heaprel: pg_sys::Relation,
    indexrel: pg_sys::Relation,
    index_info: *mut pg_sys::IndexInfo,
) -> *mut pg_sys::IndexBuildResult {
    let heap_relation = unsafe { PgRelation::from_pg(heaprel) };
    let index_relation = unsafe { PgRelation::from_pg(indexrel) };
    let index_name = index_relation.name().to_string();

    let rdopts: PgBox<SearchIndexCreateOptions> = if !index_relation.rd_options.is_null() {
        unsafe { PgBox::from_pg(index_relation.rd_options as *mut SearchIndexCreateOptions) }
    } else {
        let ops = unsafe { PgBox::<SearchIndexCreateOptions>::alloc0() };
        ops.into_pg_boxed()
    };

    // Create a map from column name to column type. We'll use this to verify that index
    // configurations passed by the user reference the correct types for each column.
    let name_type_map: HashMap<SearchFieldName, SearchFieldType> = heap_relation
        .tuple_desc()
        .into_iter()
        .map(|attribute| {
            let attname = attribute.name();
            let attribute_type_oid = attribute.type_oid();
            let array_type = unsafe { pg_sys::get_element_type(attribute_type_oid.value()) };
            let base_oid = if array_type != pg_sys::InvalidOid {
                PgOid::from(array_type)
            } else {
                attribute_type_oid
            };
            let search_field_type =
                SearchFieldType::try_from(&base_oid).expect("unrecognized field type");

            (attname.into(), search_field_type)
        })
        .collect();

    // Parse and validate the index configurations for each column.

    let text_fields =
        rdopts
            .get_text_fields()
            .into_iter()
            .map(|(name, config)| match name_type_map.get(&name) {
                Some(SearchFieldType::Text) => (name, config),
                Some(wrong_type) => panic!("wrong type for field '{name}': {wrong_type:?}"),
                None => panic!("no field named '{name}'"),
            });

    let numeric_fields = rdopts
        .get_numeric_fields()
        .into_iter()
        .map(|(name, config)| match name_type_map.get(&name) {
            Some(SearchFieldType::I64) | Some(SearchFieldType::F64) => (name, config),
            Some(wrong_type) => panic!("wrong type for field '{name}': {wrong_type:?}"),
            None => panic!("no field named '{name}'"),
        });

    let boolean_fields = rdopts
        .get_boolean_fields()
        .into_iter()
        .map(|(name, config)| match name_type_map.get(&name) {
            Some(SearchFieldType::Bool) => (name, config),
            Some(wrong_type) => panic!("wrong type for field '{name}': {wrong_type:?}"),
            None => panic!("no field named '{name}'"),
        });

    let json_fields =
        rdopts
            .get_json_fields()
            .into_iter()
            .map(|(name, config)| match name_type_map.get(&name) {
                Some(SearchFieldType::Json) => (name, config),
                Some(wrong_type) => panic!("wrong type for field '{name}': {wrong_type:?}"),
                None => panic!("no field named '{name}'"),
            });

    let key_field = rdopts.get_key_field().expect("must specify key field");

    match name_type_map.get(&key_field) {
        Some(SearchFieldType::I64) => {}
        None => panic!("key field does not exist"),
        _ => panic!("key field must be an integer"),
    };

    // Concatenate the separate lists of fields.
    let fields: Vec<_> = text_fields
        .chain(numeric_fields)
        .chain(boolean_fields)
        .chain(json_fields)
        .chain(std::iter::once((key_field, SearchFieldConfig::Key)))
        // "ctid" is a reserved column name in Postgres, so we don't need to worry about
        // creating a name conflict with a user-named column.
        .chain(std::iter::once(("ctid".into(), SearchFieldConfig::Ctid)))
        .collect();

    // If there's only two fields in the vector, then those are just the Key and Ctid fields,
    // which we added above, and the user has not specified any fields to index.
    if fields.len() == 2 {
        panic!("no fields specified")
    }

    let directory = WriterDirectory::from_index_name(&index_name);
    SearchIndex::new(directory, fields).expect("could not build search index");

    let state = do_heap_scan(index_info, &heap_relation, &index_relation);

    let mut result = unsafe { PgBox::<pg_sys::IndexBuildResult>::alloc0() };
    result.heap_tuples = state.count as f64;
    result.index_tuples = state.count as f64;

    result.into_pg()
}

#[pg_guard]
pub extern "C" fn ambuildempty(_index_relation: pg_sys::Relation) {}

fn do_heap_scan<'a>(
    index_info: *mut pg_sys::IndexInfo,
    heap_relation: &'a PgRelation,
    index_relation: &'a PgRelation,
) -> BuildState {
    let mut state = BuildState::new();
    let _ = panic::catch_unwind(AssertUnwindSafe(|| unsafe {
        pg_sys::IndexBuildHeapScan(
            heap_relation.as_ptr(),
            index_relation.as_ptr(),
            index_info,
            Some(build_callback),
            &mut state,
        );
    }));
    state
}

#[cfg(feature = "pg12")]
#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    htup: pg_sys::HeapTuple,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    let htup = htup.as_ref().unwrap();

    build_callback_internal(htup.t_self, values, state, index);
}

#[cfg(any(feature = "pg13", feature = "pg14", feature = "pg15", feature = "pg16"))]
#[pg_guard]
unsafe extern "C" fn build_callback(
    index: pg_sys::Relation,
    ctid: pg_sys::ItemPointer,
    values: *mut pg_sys::Datum,
    _isnull: *mut bool,
    _tuple_is_alive: bool,
    state: *mut std::os::raw::c_void,
) {
    build_callback_internal(*ctid, values, state, index);
}

#[inline(always)]
unsafe fn build_callback_internal(
    ctid: pg_sys::ItemPointerData,
    values: *mut pg_sys::Datum,
    _state: *mut std::os::raw::c_void,
    index: pg_sys::Relation,
) {
    check_for_interrupts!();

    let index_relation_ref: PgRelation = PgRelation::from_pg(index);
    let tupdesc = lookup_index_tupdesc(&index_relation_ref);
    let index_name = index_relation_ref.name();
    let search_index = get_search_index(index_name);
    let search_document = search_index
        .row_to_search_document(ctid, &tupdesc, values)
        .unwrap_or_else(|err| {
            panic!("error creating index entries for index '{index_name}': {err:?}",)
        });

    let writer_client = WriterGlobal::client();

    search_index
        .insert(&writer_client, search_document)
        .unwrap_or_else(|err| panic!("error inserting document during build callback: {err:?}"));

    register_commit_callback(&writer_client)
        .expect("could not register commit callbacks for build operation");
}
