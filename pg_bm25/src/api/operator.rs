use crate::env::needs_commit;
use crate::schema::SearchConfig;
use crate::{globals::WriterGlobal, postgres::utils::get_search_index};
use pgrx::{prelude::PgHeapTuple, *};
use rustc_hash::FxHashSet;

#[pg_extern]
fn search_tantivy(
    element: AnyElement,
    config_json: JsonB,
    fcinfo: pg_sys::FunctionCallInfo,
) -> bool {
    let default_hash_set = || {
        let JsonB(search_config_json) = &config_json;
        let search_config: SearchConfig = serde_json::from_value(search_config_json.clone())
            .expect("could not parse search config");

        let writer_client = WriterGlobal::client();
        let search_index = get_search_index(&search_config.index_name);
        let mut scan_state = search_index
            .search_state(&writer_client, &search_config, needs_commit())
            .unwrap();
        let top_docs = scan_state.search();
        let mut hs = FxHashSet::default();

        for (_, doc_address) in top_docs {
            let key_field_value = scan_state.key_field_value(doc_address);
            hs.insert(key_field_value);
        }

        (search_config, hs)
    };

    let cached = unsafe { pg_func_extra(fcinfo, default_hash_set) };
    let search_config = &cached.0;
    let hash_set = &cached.1;

    let heap_tuple = unsafe { PgHeapTuple::from_composite_datum(element.datum()) };
    let key_field_name = &search_config.key_field;

    // Only i64 values (bigint in Postgres) are currently supported for the key_field.
    // We'll panic below if what's passed is anything other than an i64.
    let key_field_value: i64 = match heap_tuple.get_by_name(key_field_name) {
        Err(TryFromDatumError::NoSuchAttributeName(_))
        | Err(TryFromDatumError::NoSuchAttributeNumber(_)) => {
            panic!("no key_field '{key_field_name}' found on tuple");
        }
        Err(TryFromDatumError::IncompatibleTypes { .. }) => {
            panic!("could not parse key_field {key_field_name} from tuple, incorrect type");
        }
        Ok(None) => {
            panic!("no value present in key_field {key_field_name} in tuple")
        }
        Ok(Some(value)) => value,
    };

    hash_set.contains(&key_field_value)
}

extension_sql!(
    r#"
CREATE OPERATOR pg_catalog.@@@ (
    PROCEDURE = search_tantivy,
    LEFTARG = anyelement,
    RIGHTARG = jsonb
);

CREATE OPERATOR CLASS anyelement_bm25_ops DEFAULT FOR TYPE anyelement USING bm25 AS
    OPERATOR 1 pg_catalog.@@@(anyelement, jsonb),
    STORAGE anyelement;

"#,
    name = "bm25_ops_anyelement_operator"
);
